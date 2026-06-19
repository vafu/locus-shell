use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

use shell_core::source::{Observable, SourceError, rx::Observable as _};

use super::watch::{self, WatchSpec};

const ROOT_ENV: &str = "LOCUSFS_ROOT";
const DEFAULT_ROOT: &str = "/tmp/rsynapse";
const BATTERY_OBJECT_PATH: &str = "dbus-service/upower/object/battery_BAT1";

type ReadFuture<T> = Pin<Box<dyn Future<Output = Result<T, SourceError>> + Send>>;

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
    let battery_path = battery_object_path();
    let present = battery_property(battery_path.join("IsPresent"), read_bool);
    let percent = battery_property(battery_path.join("Percentage"), read_percent);
    let state = battery_property(battery_path.join("State"), read_battery_state);

    present
        .combine_latest(percent, |present, percent| (present, percent))
        .combine_latest(state, |(present, percent), state| BatteryView {
            present,
            percent,
            state,
        })
        .distinct_until_changed()
        .box_it()
}

fn battery_property<Value>(
    path: PathBuf,
    read: fn(PathBuf) -> ReadFuture<Value>,
) -> Observable<Value>
where
    Value: Send + PartialEq + Clone + 'static,
{
    watch::read_on_change_async(WatchSpec::value(path.clone()), move || read(path.clone()))
        .distinct_until_changed()
        .box_it()
}

fn read_percent(path: PathBuf) -> ReadFuture<u8> {
    Box::pin(async move { Ok(read_f64(&path).await?.round().clamp(0.0, 100.0) as u8) })
}

fn read_battery_state(path: PathBuf) -> ReadFuture<BatteryState> {
    Box::pin(async move { Ok(battery_state(read_u32(&path).await?)) })
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

fn read_bool(path: PathBuf) -> ReadFuture<bool> {
    Box::pin(async move {
        let value = read_trimmed(&path).await?;
        match scalar_value(&value) {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            value => Err(SourceError::new(format!("invalid bool value: {value}"))),
        }
    })
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
