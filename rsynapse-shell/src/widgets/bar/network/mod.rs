use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

mod parse;

use parse::parse_ssid;

const NETWORK_MANAGER_OBJECT_PATH: &str = "dbus/networkmanager/object";
const NETWORK_MANAGER_DEVICE_PATH: &str = "dbus/networkmanager/object/Devices";

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
    state: u32,
    interface: Option<String>,
    access_point: Option<AccessPoint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AccessPoint {
    ssid: Option<String>,
    strength: u32,
}

pub(super) fn network_status() -> Observable<NetworkView> {
    source::root()
        .child(NETWORK_MANAGER_DEVICE_PATH)
        .as_children()
        .switch_map(|objects| {
            let mut devices = objects;
            devices.sort_by(|left, right| left.as_path().cmp(right.as_path()));
            source::combine_latest_vec(devices.into_iter().map(network_object).collect())
        })
        .map(|devices| NetworkView {
            wifi: wifi_status(&devices),
            ethernet: ethernet_view(&devices),
        })
        .distinct_until_changed()
        .box_it()
}

fn networkmanager_object_path(dbus_path: &str) -> Option<LocusPath> {
    if dbus_path == "/" {
        return None;
    }

    let local = dbus_path.strip_prefix("/org/freedesktop/NetworkManager/")?;
    Some(
        source::root()
            .child(NETWORK_MANAGER_OBJECT_PATH)
            .child(local),
    )
}

fn network_object(object: LocusPath) -> Observable<NetworkObject> {
    let properties = properties(&object);
    // TODO(rsynapse-shell): move `ActiveAccessPoint` behind the selected Wi-Fi
    // interface only. A direct `Interface -> switch_map(detail)` split currently
    // hits an RxRust boxing/GAT edge, so this stable path still opens the
    // relation-like property on every NetworkManager device while only following
    // AP object details when NetworkManager reports an active AP.
    let access_point = properties
        .observe_prop::<String>("ActiveAccessPoint")
        .map(|access_point| {
            access_point
                .as_deref()
                .and_then(networkmanager_object_path)
                .map(access_point_view)
                .unwrap_or_else(|| source::once(None))
        })
        .switch_map(|access_point| access_point);

    combine_latest!(
        properties.observe_prop::<String>("Interface"),
        properties.observe_prop_or::<u32>("State", 0),
        access_point
            => move |(interface, state, access_point)| NetworkObject {
                state,
                interface,
                access_point,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn access_point_view(access_point: LocusPath) -> Observable<Option<AccessPoint>> {
    let properties = properties(&access_point);
    combine_latest!(
        properties.observe_prop::<String>("Ssid"),
        properties.observe_prop_or::<u32>("Strength", 0)
            => move |(ssid, strength)| Some(AccessPoint {
                ssid,
                strength,
            }),
    )
    .distinct_until_changed()
    .box_it()
}

fn properties(object: &LocusPath) -> LocusPath {
    object.child("@properties")
}

fn wifi_status(devices: &[NetworkObject]) -> WifiView {
    let Some(device) = devices
        .iter()
        .find(|device| device.interface.as_deref().is_some_and(is_wifi_interface))
    else {
        return WifiView::default();
    };

    wifi_view(device, device.access_point.as_ref())
}

fn wifi_view(device: &NetworkObject, access_point: Option<&AccessPoint>) -> WifiView {
    let state = device.state;
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

fn ethernet_device_view(device: &NetworkObject) -> EthernetView {
    let state = device.state;
    EthernetView {
        visible: !matches!(state, DEVICE_STATE_DISCONNECTED | DEVICE_STATE_UNAVAILABLE),
        icon: ethernet_icon_name(state).to_owned(),
        tooltip: device.interface.clone().unwrap_or_default(),
    }
}

fn ethernet_view(devices: &[NetworkObject]) -> EthernetView {
    devices
        .iter()
        .filter(|device| {
            device
                .interface
                .as_deref()
                .is_some_and(is_ethernet_interface)
        })
        .find(|device| device.state == DEVICE_STATE_ACTIVATED)
        .or_else(|| {
            devices
                .iter()
                .filter(|device| {
                    device
                        .interface
                        .as_deref()
                        .is_some_and(is_ethernet_interface)
                })
                .find(|device| {
                    !matches!(
                        device.state,
                        DEVICE_STATE_DISCONNECTED | DEVICE_STATE_UNAVAILABLE
                    )
                })
        })
        .or_else(|| {
            devices.iter().find(|device| {
                device
                    .interface
                    .as_deref()
                    .is_some_and(is_ethernet_interface)
            })
        })
        .map(ethernet_device_view)
        .unwrap_or_default()
}

fn is_wifi_interface(interface: &str) -> bool {
    interface.starts_with("wl") || interface.starts_with("wlan")
}

fn is_ethernet_interface(interface: &str) -> bool {
    interface.starts_with("en") || interface.starts_with("eth")
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
