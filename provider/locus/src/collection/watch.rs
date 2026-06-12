use std::pin::Pin;

use futures_util::StreamExt;
use locus_dbus::{
    BUS_NAME, GRAPH_READ_INTERFACE, GRAPH_RESOLVE_INTERFACE, GraphReadProxy, GraphResolveProxy,
    NodeListDiffCommandTuple, ROOT_PATH,
};
use providers::{CancellationToken, Provider};
use tokio_stream::Stream;
use zbus::Proxy;
use zbus::proxy::Builder as ProxyBuilder;

use crate::{
    NodeId, NodeListBinding, TargetBinding, WatchError,
    collection::binding::{KindFilteredNodeListBinding, NodeListQuery},
    collection::diff::{apply_commands, commands_from_tuples},
};

const RESOLVE_CHANGED_SIGNAL: &str = "ResolveChanged";
const RESOLVE_ALL_CHANGED_SIGNAL: &str = "ResolveAllChanged";
const SOURCES_CHANGED_SIGNAL: &str = "SourcesChanged";
const TARGETS_CHANGED_SIGNAL: &str = "TargetsChanged";

pub type WatchStream<T> = Pin<Box<dyn Stream<Item = Result<T, WatchError>> + Send>>;

pub fn watch_target<Target>(
    binding: TargetBinding<Target>,
    cancellation: CancellationToken,
) -> WatchStream<NodeId>
where
    Target: Send + 'static,
{
    binding.stream(cancellation)
}

pub fn watch_node_list<Target>(
    binding: NodeListBinding<Target>,
    cancellation: CancellationToken,
) -> WatchStream<Vec<NodeId>>
where
    Target: Send + 'static,
{
    binding.stream(cancellation)
}

fn target_stream<Target>(
    binding: TargetBinding<Target>,
    cancellation: CancellationToken,
) -> WatchStream<NodeId>
where
    Target: Send + 'static,
{
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
        let resolve = match GraphResolveProxy::new(&connection).await {
            Ok(resolve) => resolve,
            Err(error) => {
                yield Err(error.into());
                return;
            }
        };
        let proxy = match signal_proxy(&connection, GRAPH_RESOLVE_INTERFACE).await {
            Ok(proxy) => proxy,
            Err(error) => {
                yield Err(error);
                return;
            }
        };
        let mut updates = match proxy.receive_signal(RESOLVE_CHANGED_SIGNAL).await {
            Ok(updates) => updates,
            Err(error) => {
                yield Err(error.into());
                return;
            }
        };
        let relations = relations_to_vec(binding.relations());
        match resolve
            .subscribe_resolve(binding.source(), relations.clone())
            .await
        {
            Ok(current) => {
                if !cancellation.is_cancelled() {
                    yield Ok(current);
                }
            }
            Err(error) => {
                yield Err(error.into());
                return;
            }
        }

        loop {
            let signal = tokio::select! {
                _ = cancellation.cancelled() => break,
                signal = updates.next() => signal,
            };
            let Some(signal) = signal else {
                break;
            };
            let body = signal.body().deserialize::<(String, Vec<String>, String)>();
            let (source, path, target) = match body {
                Ok(body) => body,
                Err(error) => {
                    yield Err(error.into());
                    continue;
                }
            };
            if source == binding.source() && path == relations && !cancellation.is_cancelled() {
                yield Ok(target);
            }
        }
    })
}

fn node_list_stream<Target>(
    binding: NodeListBinding<Target>,
    kind_filter: Option<&'static str>,
    cancellation: CancellationToken,
) -> WatchStream<Vec<NodeId>>
where
    Target: Send + 'static,
{
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
        let signal = match ListSignal::new(&connection, binding.query()).await {
            Ok(signal) => signal,
            Err(error) => {
                yield Err(error);
                return;
            }
        };
        let mut updates = match signal.proxy.receive_signal(signal.name).await {
            Ok(updates) => updates,
            Err(error) => {
                yield Err(error.into());
                return;
            }
        };
        let initial = match subscribe_node_list(&connection, binding.query()).await {
            Ok(initial) => initial,
            Err(error) => {
                yield Err(error);
                return;
            }
        };
        let mut nodes = Vec::new();
        match apply_and_materialize(&connection, &mut nodes, initial, kind_filter).await {
            Ok(visible_nodes) => {
                if !cancellation.is_cancelled() {
                    yield Ok(visible_nodes);
                }
            }
            Err(error) => {
                yield Err(error);
            }
        }

        loop {
            let signal = tokio::select! {
                _ = cancellation.cancelled() => break,
                signal = updates.next() => signal,
            };
            let Some(signal) = signal else {
                break;
            };
            let commands = match changed_commands(binding.query(), &signal) {
                Ok(Some(commands)) => commands,
                Ok(None) => continue,
                Err(error) => {
                    yield Err(error);
                    continue;
                }
            };
            match apply_and_materialize(&connection, &mut nodes, commands, kind_filter).await {
                Ok(visible_nodes) => {
                    if !cancellation.is_cancelled() {
                        yield Ok(visible_nodes);
                    }
                }
                Err(error) => {
                    yield Err(error);
                }
            }
        }
    })
}

struct ListSignal<'proxy> {
    proxy: Proxy<'proxy>,
    name: &'static str,
}

impl<'proxy> ListSignal<'proxy> {
    async fn new(
        connection: &'proxy zbus::Connection,
        query: &NodeListQuery,
    ) -> Result<Self, WatchError> {
        let (interface, name) = match query {
            NodeListQuery::ResolveAll { .. } => {
                (GRAPH_RESOLVE_INTERFACE, RESOLVE_ALL_CHANGED_SIGNAL)
            }
            NodeListQuery::Sources { .. } => (GRAPH_READ_INTERFACE, SOURCES_CHANGED_SIGNAL),
            NodeListQuery::Targets { .. } => (GRAPH_READ_INTERFACE, TARGETS_CHANGED_SIGNAL),
        };

        Ok(Self {
            proxy: signal_proxy(connection, interface).await?,
            name,
        })
    }
}

async fn signal_proxy<'proxy>(
    connection: &'proxy zbus::Connection,
    interface: &'static str,
) -> Result<Proxy<'proxy>, WatchError> {
    Ok(ProxyBuilder::<Proxy<'_>>::new(connection)
        .destination(BUS_NAME)?
        .path(ROOT_PATH)?
        .interface(interface)?
        .build()
        .await?)
}

async fn subscribe_node_list(
    connection: &zbus::Connection,
    query: &NodeListQuery,
) -> Result<Vec<NodeListDiffCommandTuple>, WatchError> {
    match query {
        NodeListQuery::ResolveAll { source, relations } => {
            let resolve = GraphResolveProxy::new(connection).await?;
            Ok(resolve
                .subscribe_resolve_all(source, relations.clone())
                .await?)
        }
        NodeListQuery::Sources { target, relation } => {
            let read = GraphReadProxy::new(connection).await?;
            Ok(read.subscribe_sources(target, relation).await?)
        }
        NodeListQuery::Targets { source, relation } => {
            let read = GraphReadProxy::new(connection).await?;
            Ok(read.subscribe_targets(source, relation).await?)
        }
    }
}

fn changed_commands(
    query: &NodeListQuery,
    signal: &zbus::Message,
) -> Result<Option<Vec<NodeListDiffCommandTuple>>, WatchError> {
    match query {
        NodeListQuery::ResolveAll { source, relations } => {
            let (changed_source, changed_path, commands) =
                signal
                    .body()
                    .deserialize::<(String, Vec<String>, Vec<NodeListDiffCommandTuple>)>()?;
            Ok((changed_source == *source && changed_path == *relations).then_some(commands))
        }
        NodeListQuery::Sources { target, relation } => {
            let (changed_target, changed_relation, commands) =
                signal
                    .body()
                    .deserialize::<(String, String, Vec<NodeListDiffCommandTuple>)>()?;
            Ok((changed_target == *target && changed_relation == *relation).then_some(commands))
        }
        NodeListQuery::Targets { source, relation } => {
            let (changed_source, changed_relation, commands) =
                signal
                    .body()
                    .deserialize::<(String, String, Vec<NodeListDiffCommandTuple>)>()?;
            Ok((changed_source == *source && changed_relation == *relation).then_some(commands))
        }
    }
}

async fn apply_and_materialize(
    connection: &zbus::Connection,
    nodes: &mut Vec<NodeId>,
    commands: Vec<NodeListDiffCommandTuple>,
    kind_filter: Option<&str>,
) -> Result<Vec<NodeId>, WatchError> {
    apply_commands(nodes, commands_from_tuples(commands)?)?;
    let visible_nodes = match kind_filter {
        Some(kind) => filter_nodes_by_kind(connection, nodes, kind).await?,
        None => nodes.clone(),
    };
    Ok(visible_nodes)
}

async fn filter_nodes_by_kind(
    connection: &zbus::Connection,
    nodes: &[NodeId],
    expected: &str,
) -> Result<Vec<NodeId>, WatchError> {
    let read = GraphReadProxy::new(connection).await?;
    let mut filtered = Vec::new();

    for node in nodes {
        if read.get_property(node, "kind").await? == expected {
            filtered.push(node.clone());
        }
    }

    Ok(filtered)
}

fn relations_to_vec(relations: &[&str]) -> Vec<String> {
    relations
        .iter()
        .map(|relation| (*relation).to_owned())
        .collect()
}

impl<Target> Provider<NodeId> for TargetBinding<Target>
where
    Target: Send + 'static,
{
    type Error = WatchError;
    type Stream = WatchStream<NodeId>;

    fn stream(self, cancellation: CancellationToken) -> Self::Stream {
        target_stream(self, cancellation)
    }
}

impl<Target> Provider<Vec<NodeId>> for NodeListBinding<Target>
where
    Target: Send + 'static,
{
    type Error = WatchError;
    type Stream = WatchStream<Vec<NodeId>>;

    fn stream(self, cancellation: CancellationToken) -> Self::Stream {
        node_list_stream(self, None, cancellation)
    }
}

impl<Target> Provider<Vec<NodeId>> for KindFilteredNodeListBinding<Target>
where
    Target: Send + 'static,
{
    type Error = WatchError;
    type Stream = WatchStream<Vec<NodeId>>;

    fn stream(self, cancellation: CancellationToken) -> Self::Stream {
        let (binding, kind) = self.into_parts();
        node_list_stream(binding, Some(kind), cancellation)
    }
}
