use std::{
    fs,
    future::Future,
    io,
    path::{Path, PathBuf},
    pin::Pin,
};

use futures_util::future;
use shell_core::source::{self, Observable, SourceError};

const ROOT_ENV: &str = "LOCUSFS_ROOT";
const DEFAULT_ROOT: &str = "/tmp/rsynapse";
const PIPEWIRE_PATH: &str = "pipewire";

type WatchFuture<'a> = Pin<Box<dyn Future<Output = io::Result<String>> + Send + 'a>>;

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
    source::from_async_loop(|emitter| async move {
        loop {
            let mut watches = match open_audio_watches().await {
                Ok(watches) => watches,
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            };

            match read_audio() {
                Ok(audio) => emitter.next(audio),
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            }

            if let Err(error) = wait_for_any_watch(&mut watches).await {
                emitter.error(error);
                return;
            }
        }
    })
}

fn read_audio() -> Result<AudioView, SourceError> {
    let sink_path = match read_active_sink_path() {
        Ok(path) => path,
        Err(_) => return Ok(AudioView::default()),
    };

    let description =
        read_string(&sink_path.join("description")).unwrap_or_else(|_| "Audio Output".to_owned());
    let icon = read_string(&sink_path.join("icon-name")).unwrap_or_else(|_| {
        let muted = read_bool(&sink_path.join("muted")).unwrap_or(false);
        let volume = read_u32(&sink_path.join("volume-percent")).unwrap_or(0);
        audio_icon_name(muted, volume).to_owned()
    });
    let volume = read_u32(&sink_path.join("volume-percent")).ok();
    let muted = read_bool(&sink_path.join("muted")).unwrap_or(false);

    Ok(AudioView {
        visible: true,
        icon,
        tooltip: audio_tooltip(&description, muted, volume),
    })
}

async fn open_audio_watches() -> Result<Vec<locusfs_client::Watch>, SourceError> {
    let mut watches = Vec::new();

    if !pipewire_path().exists() {
        open_optional_directory_watch(&mut watches, root()).await?;
        return Ok(watches);
    }

    open_optional_directory_watch(&mut watches, pipewire_path()).await?;
    open_optional_directory_watch(&mut watches, pipewire_path().join("default")).await?;
    open_optional_directory_watch(&mut watches, pipewire_path().join("sink")).await?;

    if let Ok(sink_path) = read_active_sink_path() {
        open_optional_directory_watch(&mut watches, sink_path).await?;
    }

    if watches.is_empty() {
        return Err(SourceError::new("no PipeWire watches registered"));
    }

    Ok(watches)
}

async fn open_optional_directory_watch(
    watches: &mut Vec<locusfs_client::Watch>,
    path: PathBuf,
) -> Result<(), SourceError> {
    match open_directory_watch(&path).await {
        Ok(watch) => {
            watches.push(watch);
            Ok(())
        }
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::Other | io::ErrorKind::InvalidData
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(SourceError::new(format!(
            "failed to watch {}: {error}",
            path.display()
        ))),
    }
}

async fn wait_for_any_watch(watches: &mut [locusfs_client::Watch]) -> Result<(), SourceError> {
    if watches.is_empty() {
        return Err(SourceError::new("no PipeWire watches registered"));
    }

    let paths = watches
        .iter()
        .map(|watch| watch.data_path().to_path_buf())
        .collect::<Vec<_>>();
    let waiters = watches
        .iter_mut()
        .map(|watch| Box::pin(watch.wait_event_to_string()) as WatchFuture<'_>)
        .collect::<Vec<_>>();
    let (result, index, _) = future::select_all(waiters).await;
    result.map_err(|error| {
        SourceError::new(format!(
            "watch failed for {}: {error}",
            paths[index].display()
        ))
    })?;
    Ok(())
}

fn read_active_sink_path() -> Result<PathBuf, SourceError> {
    read_default_sink_path().or_else(|_| read_fallback_sink_path())
}

fn read_default_sink_path() -> Result<PathBuf, SourceError> {
    let default_sink = pipewire_path().join("default/sink");
    let target = fs::read_link(&default_sink).map_err(|error| {
        SourceError::new(format!(
            "failed to resolve default sink {}: {error}",
            default_sink.display()
        ))
    })?;

    let path = if target.is_absolute() {
        target
    } else {
        default_sink
            .parent()
            .unwrap_or_else(|| Path::new("/"))
            .join(target)
    };

    Ok(path)
}

fn read_fallback_sink_path() -> Result<PathBuf, SourceError> {
    let mut sinks = fs::read_dir(pipewire_path().join("sink"))
        .map_err(|error| SourceError::new(format!("failed to read PipeWire sinks: {error}")))?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let name = path.file_name()?.to_str()?;
            name.chars()
                .all(|char| char.is_ascii_digit())
                .then_some(path)
        })
        .collect::<Vec<_>>();

    sinks.sort_by_key(|path| {
        let state = read_string(&path.join("state")).unwrap_or_default();
        let id = read_u32(&path.join("id")).unwrap_or(u32::MAX);
        (sink_state_rank(&state), id)
    });

    sinks
        .into_iter()
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

fn read_string(path: &Path) -> Result<String, SourceError> {
    let value = read_trimmed(path)?;
    Ok(strip_scalar_prefix(&value).trim_matches('"').to_owned())
}

fn read_bool(path: &Path) -> Result<bool, SourceError> {
    let value = read_trimmed(path)?;
    match strip_scalar_prefix(&value) {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        value => Err(SourceError::new(format!("invalid bool value: {value}"))),
    }
}

fn read_u32(path: &Path) -> Result<u32, SourceError> {
    let value = read_trimmed(path)?;
    strip_scalar_prefix(&value)
        .parse()
        .map_err(|error| SourceError::new(format!("invalid u32 value {value}: {error}")))
}

fn read_trimmed(path: &Path) -> Result<String, SourceError> {
    let value = fs::read_to_string(path)
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

async fn open_directory_watch(path: &Path) -> io::Result<locusfs_client::Watch> {
    let data_path = locusfs_client::absolute_path(path)?;
    let mount_root = locusfs_client::find_mount_root(&data_path).await?;
    let mut logical_path = locusfs_client::logical_watch_path(&mount_root, &data_path)?;

    if !logical_path.ends_with('/') {
        logical_path.push('/');
    }

    locusfs_client::Watch::open_with_parts(data_path, mount_root, logical_path).await
}

fn pipewire_path() -> PathBuf {
    root().join(PIPEWIRE_PATH)
}

fn root() -> PathBuf {
    std::env::var_os(ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROOT))
}
