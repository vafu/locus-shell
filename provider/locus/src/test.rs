use std::{
    error::Error as _,
    sync::{Arc, Mutex},
};

use super::{
    DecodeLocusValue, FieldBinding, decode_wire_field, model, paths,
    watch::{emit_field_value, emit_value_if_active},
};
use providers::{CancellationToken, Provider, ProviderContext, ProviderSender};

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
    let binding: FieldBinding<String> = paths::SELECTED_WINDOW.property(model::Window::TITLE);

    assert_eq!(binding.source, "context:selected");
    assert_eq!(binding.relations, &["window"]);
    assert_eq!(binding.property, "title");
}

#[test]
fn generated_path_property_is_provider() {
    fn assert_provider<T: Send + 'static, P: providers::Provider<T>>(_provider: P) {}

    let binding: FieldBinding<String> = paths::SELECTED_WINDOW.property(model::Window::TITLE);

    assert_provider::<String, _>(binding);
}

#[test]
fn generated_numeric_properties_keep_value_type() {
    let binding: FieldBinding<u32> = paths::SELECTED_WINDOW.property(model::Window::ID);

    assert_eq!(binding.property, "id");
}

#[test]
fn cancelled_field_provider_exits_before_dbus_setup() {
    let binding: FieldBinding<String> = paths::SELECTED_WINDOW.property(model::Window::TITLE);
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
fn field_value_respects_cancellation_before_decoding() {
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let context = ProviderContext::new(cancellation);
    let mut sent = Vec::new();

    let result = emit_field_value::<bool, _>(&context, "not-a-bool", &mut |value| {
        sent.push(value);
    });

    assert!(result.is_ok());
    assert!(sent.is_empty());
}
