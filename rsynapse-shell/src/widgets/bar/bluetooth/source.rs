use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

use crate::locusfs_paths::{BLUEZ, UPOWER};

use super::{
    BluetoothDeviceGroup, BluetoothDeviceView, BluetoothStatusView, BluetoothView, DeviceGroupView,
    device_type::{BluetoothDeviceKind, device_kind},
};

const BLUEZ_ADAPTER_PATH: &str = "org/bluez/hci0";
const UPOWER_DEVICES_PATH: &str = "devices";

#[derive(Clone, Debug, Eq, PartialEq)]
struct BluezObject {
    path: LocusPath,
    powered: Option<bool>,
    discovering: Option<bool>,
    connected: Option<bool>,
    name: Option<String>,
    address: Option<String>,
    class: Option<u32>,
    appearance: Option<u32>,
    battery: Option<u8>,
    connecting: Option<bool>,
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
    source::shared_by_key("rsynapse.bluetooth-status", BLUEZ_ADAPTER_PATH, || {
        bluez_summary_objects()
            .map(bluetooth_view)
            .distinct_until_changed()
            .box_it()
    })
}

pub(super) fn bluetooth_group_devices(
    group: BluetoothDeviceGroup,
) -> Observable<Vec<BluetoothDeviceView>> {
    source::shared_by_key(
        "rsynapse.bluetooth-group-devices",
        format!("{group:?}"),
        move || {
            combine_latest!(
                bluez_detail_objects(),
                upower_devices() => move |(bluez_objects, upower_devices)| {
                    device_snapshots(&bluez_objects, &upower_devices)
                        .into_iter()
                        .filter(|device| group.matches(device.kind))
                        .map(|device| device_view(&device))
                        .collect::<Vec<_>>()
                },
            )
            .distinct_until_changed()
            .box_it()
        },
    )
}

fn bluez_summary_objects() -> Observable<Vec<BluezObject>> {
    source::shared_by_key("rsynapse.bluez-summary-objects", BLUEZ_ADAPTER_PATH, || {
        let adapter = BLUEZ.object(BLUEZ_ADAPTER_PATH);

        adapter
            .as_children()
            .switch_map(move |objects| {
                let adapter = adapter.clone();
                let devices = objects
                    .into_iter()
                    .filter(is_device)
                    .map(bluez_device_summary_object);
                source::combine_latest_vec(
                    std::iter::once(bluez_adapter_summary_object(adapter))
                        .chain(devices)
                        .collect(),
                )
            })
            .map(|objects| objects)
            .distinct_until_changed()
            .box_it()
    })
}

fn bluez_detail_objects() -> Observable<Vec<BluezObject>> {
    source::shared_by_key("rsynapse.bluez-detail-objects", BLUEZ_ADAPTER_PATH, || {
        BLUEZ
            .object(BLUEZ_ADAPTER_PATH)
            .as_children()
            .switch_map(|objects| {
                source::combine_latest_vec(
                    objects
                        .into_iter()
                        .filter(is_device)
                        .map(bluez_detail_object)
                        .collect(),
                )
            })
            .map(|objects| objects)
            .distinct_until_changed()
            .box_it()
    })
}

fn upower_devices() -> Observable<Vec<UpowerDevice>> {
    source::shared_by_key("rsynapse.upower-devices", UPOWER_DEVICES_PATH, || {
        UPOWER
            .object(UPOWER_DEVICES_PATH)
            .as_children()
            .switch_map(|objects| {
                source::combine_latest_vec(objects.into_iter().map(upower_device).collect())
            })
            .map(|devices| devices)
            .distinct_until_changed()
            .box_it()
    })
}

fn bluez_adapter_summary_object(object: LocusPath) -> Observable<BluezObject> {
    let properties = properties(&object);
    combine_latest!(
        properties.observe_prop_or::<bool>("Powered", false).map(Some),
        properties.observe_prop_or::<bool>("Discovering", false).map(Some)
            => move |(powered, discovering)| BluezObject {
                path: object.clone(),
                powered,
                discovering,
                connected: None,
                name: None,
                address: None,
                class: None,
                appearance: None,
                battery: None,
                connecting: None,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn bluez_device_summary_object(object: LocusPath) -> Observable<BluezObject> {
    let properties = properties(&object);
    combine_latest!(
        properties.observe_prop_or::<bool>("Connected", false).map(Some),
        properties.observe_prop_or::<u32>("Class", 0).map(Some),
        properties.observe_prop_or::<u32>("Appearance", 0).map(Some),
        properties.observe_prop_or::<u8>("BatteryPercentage", 0).map(Some)
            => move |(connected, class, appearance, battery)| BluezObject {
                path: object.clone(),
                powered: None,
                discovering: None,
                connected,
                name: None,
                address: None,
                class,
                appearance,
                battery,
                connecting: None,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn bluez_detail_object(object: LocusPath) -> Observable<BluezObject> {
    let properties = properties(&object);
    combine_latest!(
        properties.observe_prop_or::<bool>("Connected", false).map(Some),
        properties.observe_prop_or::<bool>("Connecting", false).map(Some),
        properties.observe_prop_or::<String>("Name", String::new()).map(Some),
        properties.observe_prop_or::<String>("Address", String::new()).map(Some),
        properties.observe_prop_or::<u32>("Class", 0).map(Some),
        properties.observe_prop_or::<u32>("Appearance", 0).map(Some),
        properties.observe_prop_or::<u8>("BatteryPercentage", 0).map(Some)
            => move |(connected, connecting, name, address, class, appearance, battery)| BluezObject {
                path: object.clone(),
                powered: None,
                discovering: None,
                connected,
                name,
                address,
                class,
                appearance,
                battery,
                connecting,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn is_device(path: &LocusPath) -> bool {
    path.as_path()
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with("dev_"))
        .unwrap_or(false)
}

fn upower_device(object: LocusPath) -> Observable<UpowerDevice> {
    let properties = properties(&object);
    combine_latest!(
        properties.observe_prop_or::<String>("NativePath", String::new()).map(Some),
        properties.observe_prop_or::<f64>("Percentage", 0.0).map(percent)
            => |(native_path, percentage)| UpowerDevice {
                native_path,
                percentage,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn properties(object: &LocusPath) -> LocusPath {
    object.clone()
}

fn bluetooth_view(bluez_objects: Vec<BluezObject>) -> BluetoothView {
    let adapter = bluez_objects.iter().find(|object| object.powered.is_some());
    let powered = adapter
        .and_then(|object| object.powered)
        .or_else(|| bluez_objects.iter().find_map(|object| object.powered))
        .unwrap_or(false);
    let power_path = adapter.map(|object| properties(&object.path).prop("Powered"));
    let discovering = bluez_objects
        .iter()
        .find_map(|object| object.discovering)
        .unwrap_or(false);
    let devices = device_snapshots(&bluez_objects, &[]);
    let connected_count = devices.iter().filter(|device| device.connected).count() as u8;

    BluetoothView {
        status: BluetoothStatusView {
            icon: status_icon(powered, discovering, connected_count).to_owned(),
            connected_count,
            powered,
            power_path,
        },
        keyboard: group_view(&devices, BluetoothDeviceGroup::Keyboard),
        audio: group_view(&devices, BluetoothDeviceGroup::Audio),
        pointer: group_view(&devices, BluetoothDeviceGroup::Pointer),
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
            let address = object
                .address
                .clone()
                .or_else(|| device_key(&object.path))?;
            let kind = device_kind(object.class, object.appearance);
            Some(DeviceSnapshot {
                name: device_name(object, &address),
                battery: object
                    .battery
                    .or_else(|| upower_battery_for(upower_devices, &address)),
                address,
                path: object.path.clone(),
                connected: object.connected.unwrap_or(false),
                connecting: object
                    .connected
                    .is_some_and(|_| object.connecting.unwrap_or(false)),
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

fn device_key(path: &LocusPath) -> Option<String> {
    path.as_path()
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
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

fn group_view(devices: &[DeviceSnapshot], group: BluetoothDeviceGroup) -> DeviceGroupView {
    let devices = devices
        .iter()
        .filter(|device| device.connected)
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
    BLUEZ
        .method_for_object(object, method)
        .expect("BlueZ method target must be under the BlueZ objects tree")
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
