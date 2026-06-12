use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};

use providers::{CancellationToken, Provider, Stream};

use super::{Property, PropertyBinding};

#[derive(Debug)]
struct Target;

#[derive(Clone)]
struct Binding {
    property: Property<Target, String>,
}

struct EmptyStream;

impl Stream for EmptyStream {
    type Item = Result<String, Infallible>;

    fn poll_next(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(None)
    }
}

impl Provider<String> for Binding {
    type Error = Infallible;
    type Stream = Pin<Box<dyn Stream<Item = Result<String, Self::Error>> + Send>>;

    fn stream(self, _cancellation: CancellationToken) -> Self::Stream {
        Box::pin(EmptyStream)
    }
}

impl PropertyBinding<String> for Binding {
    type Target = Target;
    type Key = &'static str;

    fn property(&self) -> Property<Self::Target, String> {
        self.property
    }

    fn key(&self) -> Self::Key {
        self.property.key()
    }
}

#[test]
fn property_preserves_static_key_and_value_type() {
    const TITLE: Property<Target, String> = Property::new("title");

    assert_eq!(TITLE.key(), "title");
}

#[test]
fn property_binding_is_also_provider() {
    fn assert_property_binding<T, P>(_binding: P)
    where
        T: Send + 'static,
        P: PropertyBinding<T>,
    {
    }

    let binding = Binding {
        property: Property::new("title"),
    };

    assert_property_binding::<String, _>(binding);
}
