mod device_type;
mod source;

use std::{process::Command, thread};

use adw::prelude::*;
use shell_core::{gtk, source::Observable};

use crate::widgets::level_indicator::{
    self, LevelRenderStyle, LevelStage, LineStyle, TRACK_CLASSES,
};
use crate::widgets::material_icon;

const BATTERY_STAGES: &[LevelStage] = &[LevelStage {
    level: 5.0,
    class: "ok",
}];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct BluetoothView {
    pub(super) status: BluetoothStatusView,
    pub(super) keyboard: DeviceGroupView,
    pub(super) audio: DeviceGroupView,
    pub(super) pointer: DeviceGroupView,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct BluetoothStatusView {
    pub(super) icon: String,
    pub(super) connected_count: u8,
    pub(super) powered: bool,
}

impl Default for BluetoothStatusView {
    fn default() -> Self {
        Self {
            icon: "bluetooth_disabled".to_owned(),
            connected_count: 0,
            powered: false,
        }
    }
}

pub(super) fn bluetooth_status() -> Observable<BluetoothView> {
    source::bluetooth_status()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeviceGroupView {
    pub(super) visible: bool,
    pub(super) icon: String,
    pub(super) tinted: bool,
    pub(super) tooltip: String,
    pub(super) battery: Option<u8>,
    pub(super) devices: Vec<BluetoothDeviceView>,
}

impl Default for DeviceGroupView {
    fn default() -> Self {
        Self {
            visible: false,
            icon: "bluetooth".to_owned(),
            tinted: true,
            tooltip: String::new(),
            battery: None,
            devices: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct BluetoothDeviceView {
    pub(super) name: String,
    pub(super) address: String,
    pub(super) icon: String,
    pub(super) connected: bool,
    pub(super) connecting: bool,
    pub(super) battery: Option<u8>,
}

pub(super) fn status_count(status: &BluetoothStatusView) -> String {
    if status.connected_count == 0 {
        String::new()
    } else {
        status.connected_count.to_string()
    }
}

pub(super) fn status_tooltip(status: &BluetoothStatusView) -> String {
    if !status.powered {
        "Bluetooth disabled".to_owned()
    } else if status.connected_count == 0 {
        "Bluetooth".to_owned()
    } else {
        format!("{} Bluetooth device(s) connected", status.connected_count)
    }
}

pub(super) fn group_classes(group: &DeviceGroupView) -> Vec<&'static str> {
    let mut classes = vec!["flat", "circular", "panel-widget", "bt-device-button"];
    if group.tinted {
        classes.push("tinted");
    }
    classes
}

pub(super) fn battery_root_classes() -> Vec<&'static str> {
    level_indicator::root_classes(["line", "battery", "bt-battery-indicator"])
}

pub(super) fn battery_track_classes() -> &'static [&'static str] {
    TRACK_CLASSES
}

pub(super) fn battery_level_classes(group: &DeviceGroupView) -> Vec<&'static str> {
    level_indicator::level_classes(f64::from(group.battery.unwrap_or(0)), 0.0, BATTERY_STAGES)
}

pub(super) fn battery_track_draw_func()
-> impl Fn(&gtk::DrawingArea, &gtk::cairo::Context, i32, i32) + 'static {
    level_indicator::track_draw_func(LevelRenderStyle::Line(LineStyle::vertical(3.0)))
}

pub(super) fn battery_level_draw_func(
    group: &DeviceGroupView,
) -> impl Fn(&gtk::DrawingArea, &gtk::cairo::Context, i32, i32) + 'static {
    level_indicator::level_draw_func(
        f64::from(group.battery.unwrap_or(0)),
        0.0,
        100.0,
        LevelRenderStyle::Line(LineStyle::vertical(3.0)),
    )
}

pub(super) fn device_list(group: &DeviceGroupView) -> gtk::ListBox {
    let list = gtk::ListBox::new();
    list.add_css_class("bt-device-list");
    list.set_selection_mode(gtk::SelectionMode::None);

    if group.devices.is_empty() {
        let row = adw::ActionRow::builder()
            .title("No devices")
            .activatable(false)
            .sensitive(false)
            .build();
        list.append(&row);
        return list;
    }

    for device in &group.devices {
        let row = adw::ActionRow::builder()
            .title(device.name.as_str())
            .subtitle(device_subtitle(device))
            .activatable(true)
            .build();
        row.add_css_class("bt-device-row");
        row.add_prefix(
            &gtk::Image::builder()
                .css_classes(["materialicon"])
                .icon_name(material_icon::icon_name(device.icon.as_str()))
                .build(),
        );

        let address = device.address.clone();
        let connected = device.connected;
        row.connect_activated(move |row| {
            if let Some(popover) = row
                .ancestor(gtk::Popover::static_type())
                .and_then(|widget| widget.downcast::<gtk::Popover>().ok())
            {
                popover.popdown();
            }

            let address = address.clone();
            thread::spawn(move || {
                let action = if connected { "disconnect" } else { "connect" };
                let _ = Command::new("bluetoothctl")
                    .arg(action)
                    .arg(address)
                    .status();
            });
        });

        list.append(&row);
    }

    list
}

fn device_subtitle(device: &BluetoothDeviceView) -> String {
    let mut subtitle = if device.connected {
        "Connected"
    } else if device.connecting {
        "Connecting..."
    } else {
        "Disconnected"
    }
    .to_owned();

    if let Some(battery) = device.battery {
        subtitle.push_str(&format!(" - {battery}%"));
    }

    subtitle
}
