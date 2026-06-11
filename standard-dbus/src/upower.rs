pub struct DisplayDevice;

pub const DISPLAY_DEVICE: dbus::Object<DisplayDevice> = dbus::Object::system(
    "org.freedesktop.UPower",
    "/org/freedesktop/UPower/devices/DisplayDevice",
    "org.freedesktop.UPower.Device",
);

impl DisplayDevice {
    pub const PERCENTAGE: dbus::Property<Self, f64> = dbus::Property::new("Percentage");
    pub const STATE: dbus::Property<Self, u32> = dbus::Property::new("State");
    pub const TIME_TO_EMPTY: dbus::Property<Self, i64> = dbus::Property::new("TimeToEmpty");
    pub const TIME_TO_FULL: dbus::Property<Self, i64> = dbus::Property::new("TimeToFull");
    pub const IS_PRESENT: dbus::Property<Self, bool> = dbus::Property::new("IsPresent");
}
