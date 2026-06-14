use std::pin::Pin;

use locus_provider::{NodeId, NodeListBinding, Property, node};
use providers::{CancellationToken, Provider, ProviderError};
use tokio_stream::{Stream, StreamExt, StreamMap};

use crate::schema::{model, relations};

use super::{WindowTileView, hydrate::hydrate_window_tile};

type ProviderStream<T> = Pin<Box<dyn Stream<Item = Result<T, ProviderError>> + Send>>;
type WatchStream = Pin<Box<dyn Stream<Item = Result<(), ProviderError>> + Send>>;

const WINDOW_TILE_PROPERTIES: &[&str] = &[
    "icon",
    "app-icon",
    "icon-name",
    "title",
    "urgent",
    "app-id",
    "app_id",
    "class",
    "instance",
];

#[derive(Clone, Debug)]
pub(crate) struct WindowTileProvider {
    window_id: NodeId,
}

pub(crate) fn window_tile_for_window(window_id: NodeId) -> WindowTileProvider {
    WindowTileProvider { window_id }
}

impl Provider<WindowTileView> for WindowTileProvider {
    type Error = ProviderError;
    type Stream = ProviderStream<WindowTileView>;

    fn stream(self, cancellation: CancellationToken) -> Self::Stream {
        Box::pin(async_stream::stream! {
            let connection = match zbus::Connection::session().await {
                Ok(connection) => connection,
                Err(error) => {
                    yield Err(provider_error(error));
                    return;
                }
            };
            let mut window_updates = property_updates::<model::Window>(
                self.window_id.clone(),
                WINDOW_TILE_PROPERTIES,
                cancellation.child_token(),
            );
            let mut app_instances = NodeListBinding::<model::AppInstance>::targets(
                self.window_id.clone(),
                relations::APP_INSTANCE.name(),
            )
            .filter_kind("app-instance")
            .stream(cancellation.child_token());

            yield hydrate_window_tile(&connection, &self.window_id).await;

            loop {
                tokio::select! {
                    _ = cancellation.cancelled() => break,
                    update = window_updates.next() => {
                        match update {
                            Some(Ok(())) => yield hydrate_window_tile(&connection, &self.window_id).await,
                            Some(Err(error)) => yield Err(error),
                            None => break,
                        }
                    }
                    targets = app_instances.next() => {
                        match targets {
                            Some(Ok(_)) => yield hydrate_window_tile(&connection, &self.window_id).await,
                            Some(Err(error)) => yield Err(provider_error(error)),
                            None => break,
                        }
                    }
                }
            }
        })
    }
}

fn property_updates<Model>(
    node_id: NodeId,
    properties: &'static [&'static str],
    cancellation: CancellationToken,
) -> WatchStream
where
    Model: Send + 'static,
{
    Box::pin(async_stream::stream! {
        let mut streams = StreamMap::new();
        for property in properties {
            streams.insert(
                *property,
                node::<Model>(node_id.clone())
                    .raw_property(Property::<Model, String>::new(property))
                    .stream(cancellation.child_token()),
            );
        }

        loop {
            let update = tokio::select! {
                _ = cancellation.cancelled() => break,
                update = streams.next() => update,
            };

            match update {
                Some((_property, Ok(_value))) => yield Ok(()),
                Some((_property, Err(error))) => yield Err(provider_error(error)),
                None => break,
            }
        }
    })
}

fn provider_error(error: impl std::fmt::Display) -> ProviderError {
    ProviderError::new(error.to_string())
}
