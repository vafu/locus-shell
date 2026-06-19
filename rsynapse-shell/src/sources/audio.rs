use std::path::{Path, PathBuf};

use shell_core::source::{Observable, SourceError, rx::Observable as _};

use super::watch::{self, WatchSpec};

const ROOT_ENV: &str = "LOCUSFS_ROOT";
const DEFAULT_ROOT: &str = "/tmp/rsynapse";
const PIPEWIRE_PATH: &str = "pipewire";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AudioView {
    pub(crate) visible: bool,
    pub(crate) icon: String,
    pub(crate) tooltip: String,
}

impl Default for AudioView {
    fn default() -> Self {
        Self {
            visible: false,
            icon: "audio-volume-medium-symbolic".to_owned(),
            tooltip: String::new(),
        }
    }
}

pub(crate) fn audio_status() -> Observable<AudioView> {
    watch::read_on_any_change_async(open_audio_watch_specs, read_audio)
        .distinct_until_changed()
        .box_it()
}

async fn read_audio() -> Result<AudioView, SourceError> {
    let sink_path = match read_active_sink_path().await {
        Ok(path) => path,
        Err(_) => return Ok(AudioView::default()),
    };

    let description = read_string(&sink_path.join("description"))
        .await
        .unwrap_or_else(|_| "Audio Output".to_owned());
    let icon = match read_string(&sink_path.join("icon-name")).await {
        Ok(icon) => icon,
        Err(_) => {
            let muted = read_bool(&sink_path.join("muted")).await.unwrap_or(false);
            let volume = read_u32(&sink_path.join("volume-percent"))
                .await
                .unwrap_or(0);
            audio_icon_name(muted, volume).to_owned()
        }
    };
    let volume = read_u32(&sink_path.join("volume-percent")).await.ok();
    let muted = read_bool(&sink_path.join("muted")).await.unwrap_or(false);

    Ok(AudioView {
        visible: true,
        icon,
        tooltip: audio_tooltip(&description, muted, volume),
    })
}

async fn open_audio_watch_specs() -> Result<Vec<WatchSpec>, SourceError> {
    if !locusfs_client::exists(pipewire_path()).await {
        return Ok(vec![WatchSpec::directory(root())]);
    }

    let mut specs = vec![
        WatchSpec::directory(pipewire_path()),
        WatchSpec::directory(pipewire_path().join("sink")),
    ];
    let default = pipewire_path().join("default");
    if locusfs_client::exists(&default).await {
        specs.push(WatchSpec::optional_directory(default));
    }
    if let Ok(sink_path) = read_active_sink_path().await {
        specs.push(WatchSpec::optional_directory(sink_path));
    }
    Ok(specs)
}

async fn read_active_sink_path() -> Result<PathBuf, SourceError> {
    match read_default_sink_path().await {
        Ok(path) => Ok(path),
        Err(_) => read_fallback_sink_path().await,
    }
}

async fn read_default_sink_path() -> Result<PathBuf, SourceError> {
    let default_sink = pipewire_path().join("default/sink");
    locusfs_client::read_link(&default_sink)
        .await
        .map_err(|error| {
            SourceError::new(format!(
                "failed to resolve default sink {}: {error}",
                default_sink.display()
            ))
        })
}

async fn read_fallback_sink_path() -> Result<PathBuf, SourceError> {
    let mut sinks = locusfs_client::read_dir_names(pipewire_path().join("sink"))
        .await
        .map_err(|error| SourceError::new(format!("failed to read PipeWire sinks: {error}")))?
        .into_iter()
        .filter(|name| name.chars().all(|char| char.is_ascii_digit()))
        .map(|name| pipewire_path().join("sink").join(name))
        .collect::<Vec<_>>();

    let mut indexed = Vec::with_capacity(sinks.len());
    for path in sinks.drain(..) {
        let state = read_string(&path.join("state")).await.unwrap_or_default();
        let id = read_u32(&path.join("id")).await.unwrap_or(u32::MAX);
        indexed.push((sink_state_rank(&state), id, path));
    }
    indexed.sort_by_key(|(rank, id, _)| (*rank, *id));

    indexed
        .into_iter()
        .map(|(_, _, path)| path)
        .next()
        .ok_or_else(|| SourceError::new("no PipeWire sink found"))
}

fn sink_state_rank(state: &str) -> u8 {
    match state {
        "running" => 0,
        "idle" => 1,
        "suspended" => 2,
        _ => 3,
    }
}

fn audio_tooltip(description: &str, muted: bool, volume: Option<u32>) -> String {
    match (muted, volume) {
        (true, Some(volume)) => format!("{description} - muted ({volume}%)"),
        (true, None) => format!("{description} - muted"),
        (false, Some(volume)) => format!("{description} - {volume}%"),
        (false, None) => description.to_owned(),
    }
}

fn audio_icon_name(muted: bool, volume: u32) -> &'static str {
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

async fn read_string(path: &Path) -> Result<String, SourceError> {
    let value = read_trimmed(path).await?;
    Ok(strip_scalar_prefix(&value).trim_matches('"').to_owned())
}

async fn read_bool(path: &Path) -> Result<bool, SourceError> {
    let value = read_trimmed(path).await?;
    match strip_scalar_prefix(&value) {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        value => Err(SourceError::new(format!("invalid bool value: {value}"))),
    }
}

async fn read_u32(path: &Path) -> Result<u32, SourceError> {
    let value = read_trimmed(path).await?;
    strip_scalar_prefix(&value)
        .parse()
        .map_err(|error| SourceError::new(format!("invalid u32 value {value}: {error}")))
}

async fn read_trimmed(path: &Path) -> Result<String, SourceError> {
    let value = locusfs_client::read_to_string(path)
        .await
        .map_err(|error| SourceError::new(format!("failed to read {}: {error}", path.display())))?;
    Ok(value.trim().to_owned())
}

fn strip_scalar_prefix(value: &str) -> &str {
    let mut chars = value.chars();
    match (chars.next(), chars.next()) {
        (Some(kind), Some(separator))
            if kind.is_ascii_alphabetic() && separator.is_whitespace() =>
        {
            chars.as_str().trim()
        }
        _ => value,
    }
}

fn pipewire_path() -> PathBuf {
    root().join(PIPEWIRE_PATH)
}

fn root() -> PathBuf {
    std::env::var_os(ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROOT))
}
