use std::error::Error as _;

use super::{
    DbusBus, DecodeLocusValue, FieldBinding, Object, Property, PropertyBinding, decode_wire_field,
    schema,
};

struct Battery;

const BATTERY: Object<Battery> = Object::system(
    "org.freedesktop.UPower",
    "/org/freedesktop/UPower/devices/DisplayDevice",
    "org.freedesktop.UPower.Device",
);

impl Battery {
    const PERCENTAGE: Property<Self, f64> = Property::new("Percentage");
}

#[test]
fn decodes_primitive_values() {
    assert_eq!(String::decode_locus("title").unwrap(), "title");
    assert!(bool::decode_locus("true").unwrap());
    assert!(!bool::decode_locus("false").unwrap());
    assert_eq!(u32::decode_locus("42").unwrap(), 42);
    assert_eq!(i32::decode_locus("-4").unwrap(), -4);
    assert_eq!(f64::decode_locus("1.25").unwrap(), 1.25);
}

#[test]
fn rejects_invalid_bool() {
    let error = bool::decode_locus("yes").unwrap_err();

    assert_eq!(error.to_string(), "invalid bool value from Locus: \"yes\"");
    assert!(error.source().is_none());
}

#[test]
fn preserves_numeric_parse_sources() {
    let error = u32::decode_locus("abc").unwrap_err();

    assert_eq!(error.to_string(), "invalid u32 value from Locus: \"abc\"");
    assert!(error.source().is_some());
}

#[test]
fn decodes_none_wire_value_as_default() {
    assert_eq!(decode_wire_field::<String>("").unwrap(), "");
    assert!(!decode_wire_field::<bool>("").unwrap());
    assert_eq!(decode_wire_field::<u32>("").unwrap(), 0);
    assert_eq!(decode_wire_field::<i32>("").unwrap(), 0);
    assert_eq!(decode_wire_field::<f64>("").unwrap(), 0.0);
}

#[test]
fn generated_path_property_creates_typed_binding() {
    let binding: FieldBinding<String> =
        schema::paths::SELECTED_WINDOW.property(schema::model::Window::TITLE);

    assert_eq!(binding.source, "context:selected");
    assert_eq!(binding.relations, &["window"]);
    assert_eq!(binding.property, "title");
}

#[test]
fn generated_numeric_properties_keep_value_type() {
    let binding: FieldBinding<u32> =
        schema::paths::SELECTED_WINDOW.property(schema::model::Window::ID);

    assert_eq!(binding.property, "id");
}

#[test]
fn typed_object_property_creates_typed_dbus_binding() {
    let binding: PropertyBinding<f64> = BATTERY.bind(Battery::PERCENTAGE);

    assert_eq!(binding.bus, DbusBus::System);
    assert_eq!(binding.service, "org.freedesktop.UPower");
    assert_eq!(
        binding.path,
        "/org/freedesktop/UPower/devices/DisplayDevice"
    );
    assert_eq!(binding.interface, "org.freedesktop.UPower.Device");
    assert_eq!(binding.property, "Percentage");
}

#[test]
fn session_object_targets_session_bus() {
    struct Notifications;
    const NOTIFICATIONS: Object<Notifications> = Object::session(
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    );
    impl Notifications {
        const NAME: Property<Self, String> = Property::new("Name");
    }
    let binding: PropertyBinding<String> = NOTIFICATIONS.bind(Notifications::NAME);

    assert_eq!(binding.bus, DbusBus::Session);
}
