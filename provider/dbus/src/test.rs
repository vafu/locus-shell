use futures::StreamExt;
use providers::{CancellationToken, Provider};

use super::{DbusBus, Object, Property, PropertyBinding};

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
fn typed_object_property_creates_typed_dbus_binding() {
    let binding: PropertyBinding<Battery, f64> = BATTERY.bind(Battery::PERCENTAGE);

    assert_eq!(binding.bus(), DbusBus::System);
    assert_eq!(binding.service(), "org.freedesktop.UPower");
    assert_eq!(
        binding.path(),
        "/org/freedesktop/UPower/devices/DisplayDevice"
    );
    assert_eq!(binding.interface(), "org.freedesktop.UPower.Device");
    assert_eq!(binding.property_name(), "Percentage");
    assert_eq!(binding.property_descriptor().key(), "Percentage");
}

#[test]
fn typed_object_property_is_provider() {
    fn assert_provider<T: Send + 'static, P: providers::Provider<T>>(_provider: P) {}

    let binding: PropertyBinding<Battery, f64> = BATTERY.bind(Battery::PERCENTAGE);

    assert_provider::<f64, _>(binding);
}

#[test]
fn typed_object_property_is_property_binding() {
    fn assert_property_binding<T, P>(_provider: P)
    where
        T: Send + 'static,
        P: property_provider::PropertyBinding<T>,
    {
    }

    let binding: PropertyBinding<Battery, f64> = BATTERY.bind(Battery::PERCENTAGE);

    assert_property_binding::<f64, _>(binding);
}

#[test]
fn cancelled_property_provider_exits_before_dbus_setup() {
    let binding: PropertyBinding<Battery, f64> = BATTERY.bind(Battery::PERCENTAGE);
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let mut stream = binding.stream(cancellation);

    let result = futures::executor::block_on(stream.next());

    assert!(result.is_none());
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
    let binding: PropertyBinding<Notifications, String> = NOTIFICATIONS.bind(Notifications::NAME);

    assert_eq!(binding.bus(), DbusBus::Session);
}
