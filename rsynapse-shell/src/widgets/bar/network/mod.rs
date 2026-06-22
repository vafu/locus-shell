use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

mod parse;

use parse::parse_ssid;

const NETWORK_OBJECT_PATH: &str = "dbus-service/networkmanager/object";

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
    pub(super) wifi: WifiView,
    pub(super) ethernet: EthernetView,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct WifiView {
    pub(super) visible: bool,
    pub(super) icon: String,
    pub(super) tooltip: String,
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
pub(super) struct EthernetView {
    pub(super) visible: bool,
    pub(super) icon: String,
    pub(super) tooltip: String,
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct NetworkObject {
    path: LocusPath,
    device_type: Option<u32>,
    state: u32,
    interface: Option<String>,
    active_access_point: Option<String>,
    ssid: Option<String>,
    strength: u32,
}

pub(super) fn network_status() -> Observable<NetworkView> {
    let network = source::root().child(NETWORK_OBJECT_PATH);

    network
        .clone()
        .as_children()
        .switch_map(|objects| {
            source::combine_latest_vec(objects.into_iter().map(network_object).collect())
        })
        .map(move |objects| NetworkView {
            wifi: wifi_view(&objects, &network),
            ethernet: ethernet_view(&objects),
        })
        .distinct_until_changed()
        .box_it()
}

fn network_object(object: LocusPath) -> Observable<NetworkObject> {
    combine_latest!(
        object.observe_prop::<u32>("DeviceType"),
        object.observe_prop_or::<u32>("State", 0),
        object.observe_prop::<String>("Interface"),
        object.observe_prop::<String>("ActiveAccessPoint"),
        object.observe_prop::<String>("Ssid"),
        object.observe_prop_or::<u32>("Strength", 0)
            => move |(
                device_type,
                state,
                interface,
                active_access_point,
                ssid,
                strength,
            )| NetworkObject {
                path: object.clone(),
                device_type,
                state,
                interface,
                active_access_point,
                ssid,
                strength,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn wifi_view(objects: &[NetworkObject], network: &LocusPath) -> WifiView {
    let Some(device) = objects
        .iter()
        .find(|object| object.device_type == Some(DEVICE_TYPE_WIFI))
    else {
        return WifiView::default();
    };

    let state = device.state;
    let access_point = device
        .active_access_point
        .as_deref()
        .and_then(|path| find_dbus_object(objects, network, path));
    let ssid = access_point
        .and_then(|ap| ap.ssid.as_deref())
        .and_then(|ssid| parse_ssid(ssid).ok())
        .unwrap_or_default();
    let strength = access_point
        .map(|ap| ap.strength)
        .unwrap_or_default()
        .clamp(0, 100) as u8;

    WifiView {
        visible: true,
        icon: wifi_icon_name(state, strength),
        tooltip: if ssid.is_empty() {
            device.interface.clone().unwrap_or_default()
        } else {
            ssid
        },
    }
}

fn ethernet_view(objects: &[NetworkObject]) -> EthernetView {
    objects
        .iter()
        .filter(|device| device.device_type == Some(DEVICE_TYPE_ETHERNET))
        .find(|device| device.state == DEVICE_STATE_ACTIVATED)
        .or_else(|| {
            objects
                .iter()
                .filter(|device| device.device_type == Some(DEVICE_TYPE_ETHERNET))
                .find(|device| {
                    !matches!(
                        device.state,
                        DEVICE_STATE_DISCONNECTED | DEVICE_STATE_UNAVAILABLE
                    )
                })
        })
        .or_else(|| {
            objects
                .iter()
                .find(|device| device.device_type == Some(DEVICE_TYPE_ETHERNET))
        })
        .map(ethernet_device_view)
        .unwrap_or_default()
}

fn ethernet_device_view(device: &NetworkObject) -> EthernetView {
    let state = device.state;
    EthernetView {
        visible: !matches!(state, DEVICE_STATE_DISCONNECTED | DEVICE_STATE_UNAVAILABLE),
        icon: ethernet_icon_name(state).to_owned(),
        tooltip: device.interface.clone().unwrap_or_default(),
    }
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

fn dbus_path_to_object_path(network: &LocusPath, path: &str) -> Option<LocusPath> {
    if path == "/" {
        return None;
    }

    let local = path
        .strip_prefix("/org/freedesktop/NetworkManager/")
        .unwrap_or(path.trim_start_matches('/'))
        .replace('/', "%2F");
    Some(network.child(local))
}

fn find_dbus_object<'a>(
    objects: &'a [NetworkObject],
    network: &LocusPath,
    dbus_path: &str,
) -> Option<&'a NetworkObject> {
    dbus_path_to_object_path(network, dbus_path)
        .as_ref()
        .and_then(|path| objects.iter().find(|object| &object.path == path))
        .or_else(|| {
            let local = dbus_path.rsplit('/').next()?;
            objects.iter().find(|object| {
                object
                    .path
                    .as_path()
                    .file_name()
                    .and_then(|name| name.to_str())
                    == Some(local)
            })
        })
}
