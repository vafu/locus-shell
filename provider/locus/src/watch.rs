use std::{collections::HashMap, pin::Pin};

use futures_util::StreamExt;
use locus_dbus::{BUS_NAME, GraphReadProxy, GraphResolveProxy, NONE_STRING, WATCH_INTERFACE};
use providers::{CancellationToken, Provider};
use tokio_stream::Stream;
use zbus::Proxy;
use zbus::proxy::{Builder as ProxyBuilder, CacheProperties};

use crate::{LocusPropertyBinding, NodeId, WatchError};

const PROPERTIES_UPDATED_SIGNAL: &str = "PropertiesUpdated";

pub type WatchStream<T> = Pin<Box<dyn Stream<Item = Result<T, WatchError>> + Send>>;

pub fn watch_field<Target>(
    binding: LocusPropertyBinding<Target>,
    cancellation: CancellationToken,
) -> WatchStream<String>
where
    Target: Send + 'static,
{
    binding.stream(cancellation)
}

pub(crate) fn watch_node_property(
    node: NodeId,
    property: &'static str,
    cancellation: CancellationToken,
) -> WatchStream<String> {
    watch_node_path_property(node, Vec::new(), property, cancellation)
}

fn watch_node_path_property(
    source: String,
    relations: Vec<String>,
    property: &'static str,
    cancellation: CancellationToken,
) -> WatchStream<String> {
    Box::pin(async_stream::stream! {
        if cancellation.is_cancelled() {
            return;
        }

        let connection = match zbus::Connection::session().await {
            Ok(connection) => connection,
            Err(error) => {
                yield Err(error.into());
                return;
            }
        };
        let read = match GraphReadProxy::new(&connection).await {
            Ok(read) => read,
            Err(error) => {
                yield Err(error.into());
                return;
            }
        };
        let resolve = match GraphResolveProxy::new(&connection).await {
            Ok(resolve) => resolve,
            Err(error) => {
                yield Err(error.into());
                return;
            }
        };
        let object_path = match resolve.watch_node(&source, relations).await {
            Ok(object_path) => object_path,
            Err(error) => {
                yield Err(error.into());
                return;
            }
        };
        let watch = match ProxyBuilder::<Proxy<'_>>::new(&connection)
            .destination(BUS_NAME)
            .and_then(|builder| builder.path(object_path.as_str()))
            .and_then(|builder| builder.interface(WATCH_INTERFACE))
        {
            Ok(builder) => match builder
                .cache_properties(CacheProperties::No)
                .build()
                .await
            {
                Ok(watch) => watch,
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

        if let Some(value) = current_field(&read, &watch, property).await {
            yield value;
        }

        if cancellation.is_cancelled() {
            close_watch(&watch, &cancellation).await;
            return;
        }

        let mut updated = match watch.receive_signal(PROPERTIES_UPDATED_SIGNAL).await {
            Ok(updated) => updated,
            Err(error) => {
                yield Err(error.into());
                close_watch(&watch, &cancellation).await;
                return;
            }
        };

        loop {
            let signal = tokio::select! {
                _ = cancellation.cancelled() => break,
                signal = updated.next() => signal,
            };
            let Some(signal) = signal else {
                break;
            };

            let body = signal
                .body()
                .deserialize::<(HashMap<String, String>, Vec<String>)>();
            let (changed, removed) = match body {
                Ok(body) => body,
                Err(error) => {
                    yield Err(error.into());
                    continue;
                }
            };

            if let Some(value) = changed.get(property) {
                yield Ok(value.clone());
            } else if removed.iter().any(|key| key == property) {
                yield Ok(NONE_STRING.to_owned());
            }
        }

        close_watch(&watch, &cancellation).await;
    })
}

async fn current_field(
    read: &GraphReadProxy<'_>,
    watch: &Proxy<'_>,
    property: &str,
) -> Option<Result<String, WatchError>> {
    let target = match watch.get_property::<String>("Target").await {
        Ok(target) => target,
        Err(error) => return Some(Err(error.into())),
    };
    if target == NONE_STRING {
        return Some(Ok(NONE_STRING.to_owned()));
    }

    Some(
        read.get_property(&target, property)
            .await
            .map_err(Into::into),
    )
}

async fn close_watch(watch: &Proxy<'_>, cancellation: &CancellationToken) {
    if let Err(error) = watch.call::<_, _, ()>("Close", &()).await
        && !cancellation.is_cancelled()
    {
        eprintln!("failed to close Locus watch: {error}");
    }
}

impl<Target> Provider<String> for LocusPropertyBinding<Target>
where
    Target: Send + 'static,
{
    type Error = WatchError;
    type Stream = WatchStream<String>;

    fn stream(self, cancellation: CancellationToken) -> Self::Stream {
        watch_node_path_property(
            self.source().to_owned(),
            self.relations()
                .iter()
                .map(|relation| (*relation).to_owned())
                .collect(),
            self.property_name(),
            cancellation,
        )
    }
}

impl<Target> property_provider::PropertyBinding<String> for LocusPropertyBinding<Target>
where
    Target: Send + 'static,
{
    type Target = Target;
    type Key = crate::LocusPropertyKey;

    fn property(&self) -> property_provider::Property<Self::Target, String> {
        property_provider::Property::new(self.property_name())
    }

    fn key(&self) -> Self::Key {
        self.binding_key()
    }
}
