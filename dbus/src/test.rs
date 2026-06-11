use std::sync::{Arc, Mutex};

use super::{DbusBus, Object, Property, PropertyBinding, watch::emit_value_if_active};
use providers::{CancellationToken, Provider, ProviderContext, ProviderSender};

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
fn typed_object_property_is_provider() {
    fn assert_provider<T: Send + 'static, P: providers::Provider<T>>(_provider: P) {}

    let binding: PropertyBinding<f64> = BATTERY.bind(Battery::PERCENTAGE);

    assert_provider::<f64, _>(binding);
}

#[test]
fn cancelled_property_provider_exits_before_dbus_setup() {
    let binding: PropertyBinding<f64> = BATTERY.bind(Battery::PERCENTAGE);
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let captured = sent.clone();

    let result = futures::executor::block_on(binding.run(
        ProviderContext::new(cancellation),
        ProviderSender::new(move |value| {
            captured.lock().expect("sent lock").push(value);
        }),
    ));

    assert!(result.is_ok());
    assert!(sent.lock().expect("sent lock").is_empty());
}

#[test]
fn initial_value_respects_cancellation() {
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let context = ProviderContext::new(cancellation);
    let mut sent = Vec::new();

    emit_value_if_active(&context, 42_u32, &mut |value| sent.push(value));

    assert!(sent.is_empty());
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
