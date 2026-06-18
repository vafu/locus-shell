use std::{
    io,
    path::{Path, PathBuf},
};

use shell_core::source::{self, Observable, SourceError};

const ROOT_ENV: &str = "LOCUSFS_ROOT";
const DEFAULT_ROOT: &str = "/tmp/rsynapse";
const BATTERY_OBJECT_PATH: &str = "dbus-service/upower/object/battery_BAT1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BatteryView {
    pub(crate) present: bool,
    pub(crate) percent: u8,
    pub(crate) state: BatteryState,
}

impl Default for BatteryView {
    fn default() -> Self {
        Self {
            present: false,
            percent: 0,
            state: BatteryState::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum BatteryState {
    Charging,
    Discharging,
    Empty,
    Full,
    PendingCharge,
    PendingDischarge,
    #[default]
    Unknown,
}

impl BatteryState {
    pub(crate) fn is_charging(self) -> bool {
        matches!(self, Self::Charging | Self::Full)
    }
}

pub(crate) fn battery_status() -> Observable<BatteryView> {
    source::from_async_loop(|emitter| async move {
        let battery_path = battery_object_path();

        loop {
            let mut watch = match open_directory_watch(&battery_path).await {
                Ok(watch) => watch,
                Err(error) => {
                    emitter.error(SourceError::new(format!(
                        "failed to watch {}: {error}",
                        battery_path.display()
                    )));
                    return;
                }
            };

            match read_battery(&battery_path).await {
                Ok(battery) => emitter.next(battery),
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            }

            if let Err(error) = watch.wait_event_to_string().await {
                emitter.error(SourceError::new(format!(
                    "watch failed for {}: {error}",
                    battery_path.display()
                )));
                return;
            }
        }
    })
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

async fn read_battery(object_path: &Path) -> Result<BatteryView, SourceError> {
    let percent = read_f64(&object_path.join("Percentage")).await?;
    let state = battery_state(read_u32(&object_path.join("State")).await?);
    let present = read_bool(&object_path.join("IsPresent")).await?;

    Ok(BatteryView {
        present,
        percent: percent.round().clamp(0.0, 100.0) as u8,
        state,
    })
}

async fn read_f64(path: &Path) -> Result<f64, SourceError> {
    let value = read_trimmed(path).await?;
    scalar_value(&value)
        .parse()
        .map_err(|error| SourceError::new(format!("invalid f64 value {value}: {error}")))
}

async fn read_u32(path: &Path) -> Result<u32, SourceError> {
    let value = read_trimmed(path).await?;
    scalar_value(&value)
        .parse()
        .map_err(|error| SourceError::new(format!("invalid u32 value {value}: {error}")))
}

async fn read_bool(path: &Path) -> Result<bool, SourceError> {
    let value = read_trimmed(path).await?;
    match scalar_value(&value) {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        value => Err(SourceError::new(format!("invalid bool value: {value}"))),
    }
}

async fn read_trimmed(path: &Path) -> Result<String, SourceError> {
    let value = locusfs_client::read_to_string(path)
        .await
        .map_err(|error| SourceError::new(format!("failed to read {}: {error}", path.display())))?;
    Ok(value.trim().to_owned())
}

fn scalar_value(value: &str) -> &str {
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

fn battery_object_path() -> PathBuf {
    std::env::var_os(ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROOT))
        .join(BATTERY_OBJECT_PATH)
}

fn battery_state(value: u32) -> BatteryState {
    match value {
        1 => BatteryState::Charging,
        2 => BatteryState::Discharging,
        3 => BatteryState::Empty,
        4 => BatteryState::Full,
        5 => BatteryState::PendingCharge,
        6 => BatteryState::PendingDischarge,
        _ => BatteryState::Unknown,
    }
}
