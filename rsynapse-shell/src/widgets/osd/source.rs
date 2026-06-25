use std::{
    convert::Infallible,
    fs, io,
    path::{Path, PathBuf},
};

use futures_util::stream;
use notify::{RecursiveMode, Watcher as _};
use shell_core::source::{
    self, Observable,
    rx::{Observable as _, ObservableFactory as _, Shared},
};
use shell_rx_macros::combine_latest;

const PIPEWIRE_PATH: &str = "pipewire";

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct OsdLevel {
    pub(super) value: f64,
    pub(super) icon_name: String,
}

pub(super) fn osd_level() -> Observable<OsdLevel> {
    audio_events()
        .merge(brightness_events())
        .distinct_until_changed()
        .box_it()
}

fn audio_events() -> Observable<OsdLevel> {
    let sink = source::root().child(PIPEWIRE_PATH).child("default/sink");

    combine_latest!(
        sink.observe_prop_or::<u32>("volume-percent", 0),
        sink.observe_prop_or::<bool>("muted", false),
        sink.observe_prop_or::<String>("icon-name", String::new())
            => |(volume, muted, icon)| OsdLevel {
                value: (f64::from(volume) / 100.0).clamp(0.0, 1.0),
                icon_name: (!icon.is_empty())
                    .then_some(icon)
                    .unwrap_or_else(|| audio_icon_name(volume, muted).to_owned()),
            },
    )
    .distinct_until_changed()
    .skip(1)
    .box_it()
}

fn brightness_events() -> Observable<OsdLevel> {
    let Some(device) = backlight_device() else {
        return empty();
    };

    let brightness_path = device.join("brightness");
    let max = read_backlight_max(&device).unwrap_or(1.0);
    Shared::<()>::from_stream_result(brightness_stream(brightness_path, max))
        .map(|value| OsdLevel {
            value,
            icon_name: "display-brightness-symbolic".to_owned(),
        })
        .skip(1)
        .map_err(|error| {
            eprintln!("[rsynapse-shell/osd] brightness source error: {error}");
            error
        })
        .distinct_until_changed()
        .box_it()
}

fn empty<T>() -> Observable<T>
where
    T: Clone + Send + 'static,
{
    Shared::<()>::from_iter(Vec::<T>::new())
        .map_err(|error: Infallible| match error {})
        .box_it()
}

fn audio_icon_name(volume: u32, muted: bool) -> &'static str {
    if muted || volume == 0 {
        "audio-volume-muted-symbolic"
    } else if volume < 33 {
        "audio-volume-low-symbolic"
    } else if volume < 67 {
        "audio-volume-medium-symbolic"
    } else {
        "audio-volume-high-symbolic"
    }
}

fn backlight_device() -> Option<PathBuf> {
    fs::read_dir("/sys/class/backlight")
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.join("brightness").exists())
}

fn read_backlight_max(device: &Path) -> Result<f64, String> {
    let max = fs::read_to_string(device.join("max_brightness"))
        .map_err(|error| format!("failed to read max_brightness: {error}"))?
        .trim()
        .parse::<f64>()
        .map_err(|error| format!("failed to parse max_brightness: {error}"))?;
    Ok(max.max(1.0))
}

fn read_brightness(path: &Path, max: f64) -> Result<f64, String> {
    let value = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?
        .trim()
        .parse::<f64>()
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    Ok((value / max).clamp(0.0, 1.0))
}

// Brightness is still listed as a missing provider in the migration docs. Keep
// the adapter local to OSD until locusfs exposes a normalized brightness node.
fn brightness_stream(
    brightness_path: PathBuf,
    max: f64,
) -> impl futures_util::Stream<Item = Result<f64, String>> {
    stream::unfold(
        BrightnessStreamState::new(brightness_path, max),
        |mut state| async move {
            if state.done {
                return None;
            }

            if !state.initial_emitted {
                state.initial_emitted = true;
                return Some((read_brightness(&state.brightness_path, state.max), state));
            }

            loop {
                let event = match state.receiver.recv().await {
                    Ok(event) => event,
                    Err(error) => {
                        state.done = true;
                        return Some((
                            Err(format!("brightness watch channel closed: {error}")),
                            state,
                        ));
                    }
                };

                match event {
                    Ok(_) => {
                        return Some((read_brightness(&state.brightness_path, state.max), state));
                    }
                    Err(error) => {
                        return Some((Err(format!("brightness watch error: {error}")), state));
                    }
                }
            }
        },
    )
}

struct BrightnessStreamState {
    brightness_path: PathBuf,
    max: f64,
    receiver: async_channel::Receiver<notify::Result<notify::Event>>,
    _watcher: notify::RecommendedWatcher,
    initial_emitted: bool,
    done: bool,
}

impl BrightnessStreamState {
    fn new(brightness_path: PathBuf, max: f64) -> Self {
        let (sender, receiver) = async_channel::bounded(8);
        let watch_sender = sender.clone();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = watch_sender.try_send(event);
        })
        .expect("failed to create brightness file watcher");

        if let Err(error) = watcher.watch(&brightness_path, RecursiveMode::NonRecursive) {
            let _ = sender.try_send(Err(io::Error::other(error).into()));
        }

        Self {
            brightness_path,
            max,
            receiver,
            _watcher: watcher,
            initial_emitted: false,
            done: false,
        }
    }
}
