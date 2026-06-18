use std::{
    fs, io,
    path::{Path, PathBuf},
};

use shell_core::source::{self, Observable, SourceError};

const ROOT_ENV: &str = "LOCUSFS_ROOT";
const DEFAULT_ROOT: &str = "/tmp/rsynapse";
const NETWORK_OBJECT_PATH: &str = "dbus-service/networkmanager/object";
const DBUS_SERVICE_PATH: &str = "dbus-service";

const DEVICE_TYPE_ETHERNET: u32 = 1;
const DEVICE_TYPE_WIFI: u32 = 2;

const DEVICE_STATE_UNAVAILABLE: u32 = 20;
const DEVICE_STATE_DISCONNECTED: u32 = 30;
const DEVICE_STATE_PREPARE: u32 = 40;
const DEVICE_STATE_CONFIG: u32 = 50;
const DEVICE_STATE_NEED_AUTH: u32 = 60;
const DEVICE_STATE_IP_CONFIG: u32 = 70;
const DEVICE_STATE_IP_CHECK: u32 = 80;
const DEVICE_STATE_SECONDARIES: u32 = 90;
const DEVICE_STATE_ACTIVATED: u32 = 100;
const DEVICE_STATE_FAILED: u32 = 120;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct NetworkView {
    pub(crate) wifi: WifiView,
    pub(crate) ethernet: EthernetView,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WifiView {
    pub(crate) visible: bool,
    pub(crate) icon: String,
    pub(crate) tooltip: String,
}

impl Default for WifiView {
    fn default() -> Self {
        Self {
            visible: false,
            icon: "network-wireless-offline-symbolic".to_owned(),
            tooltip: String::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EthernetView {
    pub(crate) visible: bool,
    pub(crate) icon: String,
    pub(crate) tooltip: String,
}

impl Default for EthernetView {
    fn default() -> Self {
        Self {
            visible: false,
            icon: "network-wired-disconnected-symbolic".to_owned(),
            tooltip: String::new(),
        }
    }
}

pub(crate) fn network_status() -> Observable<NetworkView> {
    source::from_async_loop(|emitter| async move {
        loop {
            let watch_path = if network_object_path().exists() {
                network_object_path()
            } else {
                dbus_service_path()
            };

            let mut watch = match open_directory_watch(&watch_path).await {
                Ok(watch) => watch,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    emitter.next(NetworkView::default());
                    return;
                }
                Err(error) => {
                    emitter.error(SourceError::new(format!(
                        "failed to watch {}: {error}",
                        watch_path.display()
                    )));
                    return;
                }
            };

            match read_network() {
                Ok(network) => emitter.next(network),
                Err(error) => {
                    emitter.error(error);
                    return;
                }
            }

            if let Err(error) = watch.wait_event_to_string().await {
                emitter.error(SourceError::new(format!(
                    "watch failed for {}: {error}",
                    watch_path.display()
                )));
                return;
            }
        }
    })
}

fn read_network() -> Result<NetworkView, SourceError> {
    let object_dir = network_object_path();
    if !object_dir.exists() {
        return Ok(NetworkView::default());
    }

    let devices = read_devices(&object_dir)?;
    let wifi = devices
        .iter()
        .find(|device| device.device_type == DEVICE_TYPE_WIFI)
        .map(read_wifi)
        .transpose()?
        .unwrap_or_default();
    let ethernet = devices
        .iter()
        .filter(|device| device.device_type == DEVICE_TYPE_ETHERNET)
        .find(|device| device.state != DEVICE_STATE_DISCONNECTED)
        .or_else(|| {
            devices
                .iter()
                .find(|device| device.device_type == DEVICE_TYPE_ETHERNET)
        })
        .map(read_ethernet)
        .unwrap_or_default();

    Ok(NetworkView { wifi, ethernet })
}

fn read_devices(object_dir: &Path) -> Result<Vec<NetworkDevice>, SourceError> {
    let mut devices = Vec::new();

    for entry in fs::read_dir(object_dir).map_err(|error| {
        SourceError::new(format!("failed to read NetworkManager objects: {error}"))
    })? {
        let path = entry
            .map_err(|error| SourceError::new(error.to_string()))?
            .path();
        let Ok(device_type) = read_u32(&path.join("DeviceType")) else {
            continue;
        };
        let state = read_u32(&path.join("State")).unwrap_or(0);
        let interface = read_string(&path.join("Interface")).unwrap_or_default();

        devices.push(NetworkDevice {
            path,
            device_type,
            state,
            interface,
        });
    }

    Ok(devices)
}

fn read_wifi(device: &NetworkDevice) -> Result<WifiView, SourceError> {
    let active_ap = read_object_path(&device.path.join("ActiveAccessPoint")).ok();
    let (ssid, strength) = active_ap
        .as_deref()
        .and_then(|path| object_for_dbus_path(path))
        .map(|path| read_access_point(&path))
        .transpose()?
        .unwrap_or_else(|| (String::new(), 0));

    Ok(WifiView {
        visible: true,
        icon: wifi_icon_name(device.state, strength),
        tooltip: if ssid.is_empty() {
            device.interface.clone()
        } else {
            ssid
        },
    })
}

fn read_ethernet(device: &NetworkDevice) -> EthernetView {
    EthernetView {
        visible: !matches!(
            device.state,
            DEVICE_STATE_DISCONNECTED | DEVICE_STATE_UNAVAILABLE
        ),
        icon: ethernet_icon_name(device.state).to_owned(),
        tooltip: device.interface.clone(),
    }
}

fn read_access_point(path: &Path) -> Result<(String, u8), SourceError> {
    let ssid = read_ssid(&path.join("Ssid")).unwrap_or_default();
    let strength = read_u32(&path.join("Strength")).unwrap_or(0).clamp(0, 100) as u8;
    Ok((ssid, strength))
}

fn wifi_icon_name(state: u32, strength: u8) -> String {
    match state {
        DEVICE_STATE_ACTIVATED => {
            let level = match strength {
                80..=100 => "excellent",
                55..=79 => "good",
                30..=54 => "ok",
                1..=29 => "weak",
                _ => "none",
            };
            format!("network-wireless-signal-{level}-symbolic")
        }
        DEVICE_STATE_PREPARE
        | DEVICE_STATE_CONFIG
        | DEVICE_STATE_IP_CONFIG
        | DEVICE_STATE_IP_CHECK
        | DEVICE_STATE_SECONDARIES => "network-wireless-acquiring-symbolic".to_owned(),
        DEVICE_STATE_NEED_AUTH => "network-wireless-encrypted-symbolic".to_owned(),
        DEVICE_STATE_FAILED => "network-wireless-offline-symbolic".to_owned(),
        _ => "network-wireless-offline-symbolic".to_owned(),
    }
}

fn ethernet_icon_name(state: u32) -> &'static str {
    match state {
        DEVICE_STATE_ACTIVATED => "network-wired-symbolic",
        DEVICE_STATE_IP_CHECK
        | DEVICE_STATE_IP_CONFIG
        | DEVICE_STATE_CONFIG
        | DEVICE_STATE_SECONDARIES
        | DEVICE_STATE_PREPARE => "network-wired-acquiring-symbolic",
        DEVICE_STATE_FAILED | DEVICE_STATE_NEED_AUTH => "network-wired-no-route-symbolic",
        _ => "network-wired-disconnected-symbolic",
    }
}

fn object_for_dbus_path(path: &str) -> Option<PathBuf> {
    if path == "/" {
        return None;
    }

    let local = path.rsplit('/').next()?;
    let object = network_object_path().join(local);
    object.exists().then_some(object)
}

fn read_object_path(path: &Path) -> Result<String, SourceError> {
    read_string(path)
}

fn read_string(path: &Path) -> Result<String, SourceError> {
    let value = read_trimmed(path)?;
    Ok(strip_scalar_prefix(&value).trim_matches('"').to_owned())
}

fn read_ssid(path: &Path) -> Result<String, SourceError> {
    let value = read_trimmed(path)?;
    let value = strip_scalar_prefix(&value);
    if value.is_empty() {
        return Ok(String::new());
    }

    let bytes = value
        .split_whitespace()
        .map(|part| {
            part.parse::<u8>()
                .map_err(|error| SourceError::new(format!("invalid SSID byte {part}: {error}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(String::from_utf8_lossy(&bytes).into_owned())
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

fn network_object_path() -> PathBuf {
    root().join(NETWORK_OBJECT_PATH)
}

fn dbus_service_path() -> PathBuf {
    root().join(DBUS_SERVICE_PATH)
}

fn root() -> PathBuf {
    std::env::var_os(ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROOT))
}

struct NetworkDevice {
    path: PathBuf,
    device_type: u32,
    state: u32,
    interface: String,
}
