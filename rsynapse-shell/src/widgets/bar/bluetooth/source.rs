use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

use super::{
    BluetoothDeviceView, BluetoothStatusView, BluetoothView, DeviceGroupView,
    device_type::{BluetoothDeviceKind, device_kind},
};

const BLUEZ_OBJECT_PATH: &str = "dbus-service/bluez/object";
const UPOWER_OBJECT_PATH: &str = "dbus-service/upower/object";

#[derive(Clone, Debug, Eq, PartialEq)]
struct BluezObject {
    path: LocusPath,
    powered: Option<bool>,
    discovering: Option<bool>,
    connected: Option<bool>,
    connecting: Option<bool>,
    name: Option<String>,
    address: Option<String>,
    class: Option<u32>,
    appearance: Option<u32>,
    battery: Option<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct UpowerDevice {
    native_path: Option<String>,
    percentage: Option<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeviceSnapshot {
    name: String,
    address: String,
    path: LocusPath,
    connected: bool,
    connecting: bool,
    kind: BluetoothDeviceKind,
    battery: Option<u8>,
}

pub(super) fn bluetooth_status() -> Observable<BluetoothView> {
    let root = source::root();
    let bluez = root.child(BLUEZ_OBJECT_PATH);
    let upower = root.child(UPOWER_OBJECT_PATH);

    let bluez_objects = bluez.as_children().switch_map(|objects| {
        source::combine_latest_vec(
            objects
                .into_iter()
                .filter(is_adapter_or_device)
                .map(bluez_object)
                .collect(),
        )
    });
    let upower_devices = upower.as_children().switch_map(|objects| {
        source::combine_latest_vec(objects.into_iter().map(upower_device).collect())
    });

    combine_latest!(
        bluez_objects,
        upower_devices => |(bluez_objects, upower_devices)| bluetooth_view(bluez_objects, upower_devices),
    )
    .distinct_until_changed()
    .box_it()
}

fn bluez_object(object: LocusPath) -> Observable<BluezObject> {
    let state = combine_latest!(
        object.observe_prop::<bool>("Powered"),
        object.observe_prop::<bool>("Discovering"),
        object.observe_prop::<bool>("Connected"),
        object.observe_prop::<bool>("Connecting"),
        object.observe_prop::<String>("Name"),
    );
    let identity = combine_latest!(
        object.observe_prop::<String>("Address"),
        object.observe_prop::<u32>("Class"),
        object.observe_prop::<u32>("Appearance"),
        object.observe_prop::<u8>("BatteryPercentage"),
    );

    combine_latest!(
        state,
        identity => move |(
            (powered, discovering, connected, connecting, name),
            (address, class, appearance, battery),
        )| BluezObject {
                path: object.clone(),
                powered,
                discovering,
                connected,
                connecting,
                name,
                address,
                class,
                appearance,
                battery,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn is_adapter_or_device(path: &LocusPath) -> bool {
    path.as_path()
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with("hci") || name.starts_with("dev_"))
        .unwrap_or(false)
}

fn upower_device(object: LocusPath) -> Observable<UpowerDevice> {
    combine_latest!(
        object.observe_prop::<String>("NativePath"),
        object.observe_prop::<f64>("Percentage").map(|value| value.and_then(percent))
            => |(native_path, percentage)| UpowerDevice {
                native_path,
                percentage,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn bluetooth_view(
    bluez_objects: Vec<BluezObject>,
    upower_devices: Vec<UpowerDevice>,
) -> BluetoothView {
    let adapter = bluez_objects.iter().find(|object| object.powered.is_some());
    let powered = adapter
        .and_then(|object| object.powered)
        .or_else(|| bluez_objects.iter().find_map(|object| object.powered))
        .unwrap_or(false);
    let power_path = adapter.map(|object| object.path.prop("Powered"));
    let discovering = bluez_objects
        .iter()
        .find_map(|object| object.discovering)
        .unwrap_or(false);
    let devices = device_snapshots(&bluez_objects, &upower_devices);
    let connected_count = devices.iter().filter(|device| device.connected).count() as u8;

    BluetoothView {
        status: BluetoothStatusView {
            icon: status_icon(powered, discovering, connected_count).to_owned(),
            connected_count,
            powered,
            power_path,
        },
        keyboard: group_view(&devices, DeviceGroup::Keyboard),
        audio: group_view(&devices, DeviceGroup::Audio),
        pointer: group_view(&devices, DeviceGroup::Pointer),
    }
}

fn device_snapshots(
    bluez_objects: &[BluezObject],
    upower_devices: &[UpowerDevice],
) -> Vec<DeviceSnapshot> {
    let mut devices = bluez_objects
        .iter()
        .filter(|object| object.powered.is_none())
        .filter_map(|object| {
            let address = object.address.clone()?;
            let kind = device_kind(object.class, object.appearance);
            Some(DeviceSnapshot {
                name: device_name(object, &address),
                battery: object
                    .battery
                    .or_else(|| upower_battery_for(upower_devices, &address)),
                address,
                path: object.path.clone(),
                connected: object.connected.unwrap_or(false),
                connecting: object.connecting.unwrap_or(false),
                kind,
            })
        })
        .collect::<Vec<_>>();

    devices.sort_by(|left, right| {
        right
            .connected
            .cmp(&left.connected)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.address.cmp(&right.address))
    });
    devices
}

fn device_name(object: &BluezObject, address: &str) -> String {
    object
        .name
        .as_deref()
        .filter(|name| !name.is_empty())
        .unwrap_or(address)
        .to_owned()
}

fn upower_battery_for(devices: &[UpowerDevice], address: &str) -> Option<u8> {
    let needle = address_to_bluez_suffix(address);
    devices
        .iter()
        .find(|device| {
            device
                .native_path
                .as_deref()
                .map(|path| path.ends_with(&needle) || path.contains(&address.to_ascii_lowercase()))
                .unwrap_or(false)
        })
        .and_then(|device| device.percentage)
}

fn group_view(devices: &[DeviceSnapshot], group: DeviceGroup) -> DeviceGroupView {
    let devices = devices
        .iter()
        .filter(|device| group.matches(device.kind))
        .map(device_view)
        .collect::<Vec<_>>();
    let primary = devices.first();

    DeviceGroupView {
        visible: !devices.is_empty(),
        icon: primary
            .map(|device| device.icon.clone())
            .unwrap_or_else(|| group.fallback_icon().to_owned()),
        tinted: primary.map(|device| !device.connected).unwrap_or(true),
        tooltip: primary
            .map(|device| device.name.clone())
            .unwrap_or_else(|| group.tooltip().to_owned()),
        battery: primary.and_then(|device| device.battery),
        devices,
    }
}

fn device_view(device: &DeviceSnapshot) -> BluetoothDeviceView {
    BluetoothDeviceView {
        name: device.name.clone(),
        address: device.address.clone(),
        icon: device.kind.icon().to_owned(),
        connected: device.connected,
        connecting: device.connecting,
        battery: device.battery,
        connect_path: method_call_path(&device.path, "Connect"),
        disconnect_path: method_call_path(&device.path, "Disconnect"),
    }
}

fn method_call_path(object: &LocusPath, method: &str) -> LocusPath {
    object.rel("methods").child(method).prop("call")
}

fn status_icon(powered: bool, discovering: bool, connected_count: u8) -> &'static str {
    if !powered {
        "bluetooth_disabled"
    } else if discovering {
        "bluetooth_searching"
    } else if connected_count > 0 {
        "bluetooth_connected"
    } else {
        "bluetooth"
    }
}

fn percent(value: f64) -> Option<u8> {
    value
        .is_finite()
        .then(|| value.round().clamp(0.0, 100.0) as u8)
}

fn address_to_bluez_suffix(address: &str) -> String {
    format!("dev_{}", address.replace(':', "_"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeviceGroup {
    Keyboard,
    Audio,
    Pointer,
}

impl DeviceGroup {
    fn matches(self, kind: BluetoothDeviceKind) -> bool {
        match self {
            Self::Keyboard => kind == BluetoothDeviceKind::InputKeyboard,
            Self::Audio => matches!(
                kind,
                BluetoothDeviceKind::AudioHeadphones
                    | BluetoothDeviceKind::AudioHeadset
                    | BluetoothDeviceKind::AudioCard
            ),
            Self::Pointer => matches!(
                kind,
                BluetoothDeviceKind::InputMouse | BluetoothDeviceKind::InputTablet
            ),
        }
    }

    fn fallback_icon(self) -> &'static str {
        match self {
            Self::Keyboard => "keyboard",
            Self::Audio => "headphones",
            Self::Pointer => "mouse",
        }
    }

    fn tooltip(self) -> &'static str {
        match self {
            Self::Keyboard => "Bluetooth keyboard",
            Self::Audio => "Bluetooth audio",
            Self::Pointer => "Bluetooth pointer",
        }
    }
}
