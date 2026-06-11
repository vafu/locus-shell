//! UPower D-Bus object and property definitions.
//!
//! Use these definitions with `dbus-provider` to create typed provider sources.

/// Marker type for UPower's display battery device.
pub struct DisplayDevice;

/// UPower's aggregate display device.
pub const DISPLAY_DEVICE: dbus_provider::Object<DisplayDevice> = dbus_provider::Object::system(
    "org.freedesktop.UPower",
    "/org/freedesktop/UPower/devices/DisplayDevice",
    "org.freedesktop.UPower.Device",
);

impl DisplayDevice {
    pub const PERCENTAGE: dbus_provider::Property<Self, f64> =
        dbus_provider::Property::new("Percentage");
    pub const STATE: dbus_provider::Property<Self, u32> = dbus_provider::Property::new("State");
    pub const TIME_TO_EMPTY: dbus_provider::Property<Self, i64> =
        dbus_provider::Property::new("TimeToEmpty");
    pub const TIME_TO_FULL: dbus_provider::Property<Self, i64> =
        dbus_provider::Property::new("TimeToFull");
    pub const IS_PRESENT: dbus_provider::Property<Self, bool> =
        dbus_provider::Property::new("IsPresent");
}
