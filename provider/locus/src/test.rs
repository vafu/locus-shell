use std::error::Error as _;

use futures::StreamExt;
use providers::{CancellationToken, Provider};

use super::{DecodeError, LocusPropertyBinding, NONE_STRING, Path, Property, node};

mod schema {
    use super::{Path, Property};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Window;

    impl Window {
        pub const ID: Property<Self, u32> = Property::new("id");
        pub const TITLE: Property<Self, String> = Property::new("title");
    }

    pub const SELECTED_WINDOW: Path<Window> =
        Path::new("selected-window", "context:selected", &["window"], false);
}

#[test]
fn decodes_primitive_values() {
    assert_eq!(decode_string("title").unwrap(), "title");
    assert!(decode_bool("true").unwrap());
    assert!(!decode_bool("false").unwrap());
    assert_eq!(decode_u32("42").unwrap(), 42);
    assert_eq!(decode_i32("-4").unwrap(), -4);
    assert_eq!(decode_f64("1.25").unwrap(), 1.25);
}

#[test]
fn rejects_invalid_bool() {
    let error = decode_bool("yes").unwrap_err();

    assert_eq!(error.to_string(), "invalid bool value from Locus: \"yes\"");
    assert!(error.source().is_none());
}

#[test]
fn preserves_numeric_parse_sources() {
    let error = decode_u32("abc").unwrap_err();

    assert_eq!(error.to_string(), "invalid u32 value from Locus: \"abc\"");
    assert!(error.source().is_some());
}

#[test]
fn decodes_none_wire_value_as_optional_absence() {
    assert_eq!(decode_optional_string("").unwrap(), None);
    assert_eq!(
        decode_string("").unwrap_err().to_string(),
        "missing Locus value for non-optional property"
    );
}

#[test]
fn path_property_creates_typed_binding() {
    let binding: LocusPropertyBinding<schema::Window> =
        schema::SELECTED_WINDOW.raw_property(schema::Window::TITLE);

    assert_eq!(binding.source(), "context:selected");
    assert_eq!(binding.relations(), &["window"]);
    assert_eq!(binding.property_name(), "title");
    assert_eq!(binding.property_descriptor::<String>().key(), "title");
}

#[test]
fn path_property_is_provider() {
    fn assert_provider<T: Send + 'static, P: providers::Provider<T>>(_provider: P) {}

    let binding = schema::SELECTED_WINDOW.raw_property(schema::Window::TITLE);

    assert_provider::<String, _>(binding);
}

#[test]
fn path_property_is_property_binding() {
    fn assert_property_binding<T, P>(_provider: P)
    where
        T: Send + 'static,
        P: property_provider::PropertyBinding<T>,
    {
    }

    let binding = schema::SELECTED_WINDOW.raw_property(schema::Window::TITLE);

    assert_property_binding::<String, _>(binding);
}

#[test]
fn numeric_properties_keep_value_type() {
    let binding: LocusPropertyBinding<schema::Window> =
        schema::SELECTED_WINDOW.raw_property(schema::Window::ID);

    assert_eq!(binding.property_name(), "id");
}

#[test]
fn direct_node_property_creates_typed_binding() {
    let binding = node::<schema::Window>("window:1").raw_property(schema::Window::TITLE);

    assert_eq!(binding.node(), "window:1");
    assert_eq!(binding.property_name(), "title");
}

#[test]
fn direct_node_property_is_provider() {
    fn assert_provider<T: Send + 'static, P: providers::Provider<T>>(_provider: P) {}

    let binding = node::<schema::Window>("window:1").raw_property(schema::Window::TITLE);

    assert_provider::<String, _>(binding);
}

#[test]
fn direct_node_property_is_property_binding() {
    fn assert_property_binding<T, P>(_provider: P)
    where
        T: Send + 'static,
        P: property_provider::PropertyBinding<T>,
    {
    }

    let binding = node::<schema::Window>("window:1").raw_property(schema::Window::TITLE);

    assert_property_binding::<String, _>(binding);
}

#[test]
fn cancelled_field_provider_exits_before_dbus_setup() {
    let binding = schema::SELECTED_WINDOW.raw_property(schema::Window::TITLE);
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let mut stream = binding.stream(cancellation);

    let result = futures::executor::block_on(stream.next());

    assert!(result.is_none());
}

fn decode_string(value: &str) -> Result<String, DecodeError> {
    if value == NONE_STRING {
        Err(DecodeError::MissingValue)
    } else {
        Ok(value.to_owned())
    }
}

fn decode_optional_string(value: &str) -> Result<Option<String>, DecodeError> {
    if value == NONE_STRING {
        Ok(None)
    } else {
        decode_string(value).map(Some)
    }
}

fn decode_bool(value: &str) -> Result<bool, DecodeError> {
    if value == NONE_STRING {
        return Err(DecodeError::MissingValue);
    }
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(DecodeError::Bool {
            value: value.to_owned(),
        }),
    }
}

fn decode_u32(value: &str) -> Result<u32, DecodeError> {
    if value == NONE_STRING {
        return Err(DecodeError::MissingValue);
    }
    value.parse().map_err(|source| DecodeError::U32 {
        value: value.to_owned(),
        source,
    })
}

fn decode_i32(value: &str) -> Result<i32, DecodeError> {
    if value == NONE_STRING {
        return Err(DecodeError::MissingValue);
    }
    value.parse().map_err(|source| DecodeError::I32 {
        value: value.to_owned(),
        source,
    })
}

fn decode_f64(value: &str) -> Result<f64, DecodeError> {
    if value == NONE_STRING {
        return Err(DecodeError::MissingValue);
    }
    value.parse().map_err(|source| DecodeError::F64 {
        value: value.to_owned(),
        source,
    })
}

#[test]
fn cancelled_direct_node_property_provider_exits_before_dbus_setup() {
    let binding = node::<schema::Window>("window:1").raw_property(schema::Window::TITLE);
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let mut stream = binding.stream(cancellation);

    let result = futures::executor::block_on(stream.next());

    assert!(result.is_none());
}
