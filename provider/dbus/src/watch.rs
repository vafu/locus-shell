use std::pin::Pin;

use futures_util::StreamExt;
use providers::{CancellationToken, Provider};
use tokio_stream::Stream;
use zbus::Proxy;
use zbus::proxy::{Builder as ProxyBuilder, CacheProperties};
use zbus::zvariant::OwnedValue;

use crate::{DbusBus, PropertyBinding, WatchError};

pub type WatchStream<T> = Pin<Box<dyn Stream<Item = Result<T, WatchError>> + Send>>;

pub fn watch_property<Target, T>(
    binding: PropertyBinding<Target, T>,
    cancellation: CancellationToken,
) -> WatchStream<T>
where
    Target: Send + 'static,
    T: TryFrom<OwnedValue> + Send + Sync + Unpin + 'static,
    T::Error: Into<zbus::Error>,
{
    binding.stream(cancellation)
}

impl<Target, T> Provider<T> for PropertyBinding<Target, T>
where
    Target: Send + 'static,
    T: TryFrom<OwnedValue> + Send + Sync + Unpin + 'static,
    T::Error: Into<zbus::Error>,
{
    type Error = WatchError;
    type Stream = WatchStream<T>;

    fn stream(self, cancellation: CancellationToken) -> Self::Stream {
        Box::pin(async_stream::stream! {
            if cancellation.is_cancelled() {
                return;
            }

            let connection = match self.bus() {
                DbusBus::Session => match zbus::Connection::session().await {
                    Ok(connection) => connection,
                    Err(error) => {
                        yield Err(error.into());
                        return;
                    }
                },
                DbusBus::System => match zbus::Connection::system().await {
                    Ok(connection) => connection,
                    Err(error) => {
                        yield Err(error.into());
                        return;
                    }
                },
            };

            let proxy = match ProxyBuilder::<Proxy<'_>>::new(&connection)
                .destination(self.service())
                .and_then(|builder| builder.path(self.path()))
                .and_then(|builder| builder.interface(self.interface()))
            {
                Ok(builder) => match builder
                    .cache_properties(CacheProperties::Yes)
                    .build()
                    .await
                {
                    Ok(proxy) => proxy,
                    Err(error) => {
                        yield Err(error.into());
                        return;
                    }
                },
                Err(error) => {
                    yield Err(error.into());
                    return;
                }
            };

            match proxy.get_property::<T>(self.property_name()).await {
                Ok(value) => {
                    if !cancellation.is_cancelled() {
                        yield Ok(value);
                    }
                }
                Err(error) => {
                    yield Err(error.into());
                    return;
                }
            }

            if cancellation.is_cancelled() {
                return;
            }

            let mut updates = proxy.receive_property_changed::<T>(self.property_name()).await;

            loop {
                let update = tokio::select! {
                    _ = cancellation.cancelled() => break,
                    update = updates.next() => update,
                };
                let Some(update) = update else {
                    break;
                };

                let value = tokio::select! {
                    _ = cancellation.cancelled() => break,
                    value = update.get() => value,
                };

                match value {
                    Ok(value) => {
                        if !cancellation.is_cancelled() {
                            yield Ok(value);
                        }
                    }
                    Err(error) => {
                        yield Err(error.into());
                    }
                }
            }
        })
    }
}

impl<Target, T> property_provider::PropertyBinding<T> for PropertyBinding<Target, T>
where
    Target: Send + 'static,
    T: TryFrom<OwnedValue> + Send + Sync + Unpin + 'static,
    T::Error: Into<zbus::Error>,
{
    type Target = Target;
    type Key = crate::DbusPropertyKey;

    fn property(&self) -> property_provider::Property<Self::Target, T> {
        self.property_descriptor()
    }

    fn key(&self) -> Self::Key {
        self.binding_key()
    }
}
