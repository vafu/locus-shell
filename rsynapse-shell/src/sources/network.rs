use std::path::{Path, PathBuf};

use shell_core::source::{Observable, SourceError, rx::Observable as _};

use super::watch::{self, WatchSpec};

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
    watch::read_on_any_change_async(open_network_watch_specs, read_network)
        .distinct_until_changed()
        .box_it()
}

async fn read_network() -> Result<NetworkView, SourceError> {
    let object_dir = network_object_path();
    if !locusfs_client::exists(&object_dir).await {
        return Ok(NetworkView::default());
    }

    let devices = read_devices(&object_dir).await?;
    let wifi = if let Some(device) = devices
        .iter()
        .find(|device| device.device_type == DEVICE_TYPE_WIFI)
    {
        read_wifi(device).await?
    } else {
        WifiView::default()
    };
    let ethernet = devices
        .iter()
        .filter(|device| device.device_type == DEVICE_TYPE_ETHERNET)
        .find(|device| device.state == DEVICE_STATE_ACTIVATED)
        .or_else(|| {
            devices
                .iter()
                .filter(|device| device.device_type == DEVICE_TYPE_ETHERNET)
                .find(|device| {
                    !matches!(
                        device.state,
                        DEVICE_STATE_DISCONNECTED | DEVICE_STATE_UNAVAILABLE
                    )
                })
        })
        .or_else(|| {
            devices
                .iter()
                .find(|device| device.device_type == DEVICE_TYPE_ETHERNET)
        })
        .map(read_ethernet)
        .unwrap_or_default();

    Ok(NetworkView { wifi, ethernet })
}

async fn read_devices(object_dir: &Path) -> Result<Vec<NetworkDevice>, SourceError> {
    let mut devices = Vec::new();

    for name in locusfs_client::read_dir_names(object_dir)
        .await
        .map_err(|error| {
            SourceError::new(format!("failed to read NetworkManager objects: {error}"))
        })?
    {
        let path = object_dir.join(name);
        let Ok(device_type) = read_u32(&path.join("DeviceType")).await else {
            continue;
        };
        let state = read_u32(&path.join("State")).await.unwrap_or(0);
        let interface = read_string(&path.join("Interface"))
            .await
            .unwrap_or_default();

        devices.push(NetworkDevice {
            path,
            device_type,
            state,
            interface,
        });
    }

    Ok(devices)
}

async fn open_network_watch_specs() -> Result<Vec<WatchSpec>, SourceError> {
    if !locusfs_client::exists(network_object_path()).await {
        return Ok(vec![WatchSpec::directory(dbus_service_path())]);
    }

    let mut specs = vec![
        WatchSpec::directory(network_object_path()),
        WatchSpec::optional_directory(network_root_object_path()),
    ];

    for device in read_devices(&network_object_path()).await? {
        specs.push(WatchSpec::optional_directory(device.path.clone()));
        if let Some(active_ap) = read_object_path(&device.path.join("ActiveAccessPoint"))
            .await
            .ok()
            .and_then(|path| dbus_path_to_object_path(&path))
            && locusfs_client::exists(&active_ap).await
        {
            specs.push(WatchSpec::optional_directory(active_ap));
        }
    }

    Ok(specs)
}

async fn read_wifi(device: &NetworkDevice) -> Result<WifiView, SourceError> {
    let active_ap = match read_object_path(&device.path.join("ActiveAccessPoint")).await {
        Ok(path) => dbus_path_to_object_path(&path),
        Err(_) => None,
    };
    let (ssid, strength) = if let Some(path) = active_ap {
        read_access_point(&path)
            .await
            .unwrap_or_else(|_| (String::new(), 0))
    } else {
        (String::new(), 0)
    };

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

async fn read_access_point(path: &Path) -> Result<(String, u8), SourceError> {
    let ssid = read_ssid(&path.join("Ssid")).await.unwrap_or_default();
    let strength = read_u32(&path.join("Strength"))
        .await
        .unwrap_or(0)
        .clamp(0, 100) as u8;
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

fn dbus_path_to_object_path(path: &str) -> Option<PathBuf> {
    if path == "/" {
        return None;
    }

    let local = path
        .strip_prefix("/org/freedesktop/NetworkManager/")
        .unwrap_or(path.trim_start_matches('/'))
        .replace('/', "%2F");
    Some(network_object_path().join(local))
}

async fn read_object_path(path: &Path) -> Result<String, SourceError> {
    read_string(path).await
}

async fn read_string(path: &Path) -> Result<String, SourceError> {
    let value = read_trimmed(path).await?;
    Ok(strip_scalar_prefix(&value).trim_matches('"').to_owned())
}

async fn read_ssid(path: &Path) -> Result<String, SourceError> {
    let value = read_trimmed(path).await?;
    let value = strip_scalar_prefix(&value);
    if value.is_empty() {
        return Ok(String::new());
    }

    let bytes = if value.contains("U8(") {
        parse_owned_u8_array(value)?
    } else {
        value
            .split_whitespace()
            .map(|part| {
                part.parse::<u8>()
                    .map_err(|error| SourceError::new(format!("invalid SSID byte {part}: {error}")))
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn parse_owned_u8_array(value: &str) -> Result<Vec<u8>, SourceError> {
    let mut bytes = Vec::new();
    let mut rest = value;

    while let Some((_, after_prefix)) = rest.split_once("U8(") {
        let Some((byte, after_byte)) = after_prefix.split_once(')') else {
            return Err(SourceError::new(format!("invalid U8 array value: {value}")));
        };
        bytes.push(
            byte.parse::<u8>()
                .map_err(|error| SourceError::new(format!("invalid SSID byte {byte}: {error}")))?,
        );
        rest = after_byte;
    }

    Ok(bytes)
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

fn network_object_path() -> PathBuf {
    root().join(NETWORK_OBJECT_PATH)
}

fn network_root_object_path() -> PathBuf {
    network_object_path().join("%40")
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
