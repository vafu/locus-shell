use super::upower::{DISPLAY_DEVICE, DisplayDevice};

#[test]
fn display_device_percentage_binding_targets_upower() {
    let binding = DISPLAY_DEVICE.bind(DisplayDevice::PERCENTAGE);

    assert_eq!(binding.bus, dbus_provider::DbusBus::System);
    assert_eq!(binding.service, "org.freedesktop.UPower");
    assert_eq!(
        binding.path,
        "/org/freedesktop/UPower/devices/DisplayDevice"
    );
    assert_eq!(binding.interface, "org.freedesktop.UPower.Device");
    assert_eq!(binding.property, "Percentage");
}
