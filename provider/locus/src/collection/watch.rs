use std::future::Future;

use futures_util::StreamExt;
use locus_dbus::{
    BUS_NAME, GRAPH_READ_INTERFACE, GRAPH_RESOLVE_INTERFACE, GraphReadProxy, GraphResolveProxy,
    NodeListDiffCommandTuple, ROOT_PATH,
};
use providers::{Provider, ProviderContext, ProviderSender};
use zbus::Proxy;
use zbus::proxy::Builder as ProxyBuilder;

use crate::{
    NodeId, NodeListBinding, TargetBinding, WatchError,
    collection::binding::NodeListQuery,
    collection::diff::{apply_commands, commands_from_tuples},
};

const RESOLVE_CHANGED_SIGNAL: &str = "ResolveChanged";
const RESOLVE_ALL_CHANGED_SIGNAL: &str = "ResolveAllChanged";
const SOURCES_CHANGED_SIGNAL: &str = "SourcesChanged";
const TARGETS_CHANGED_SIGNAL: &str = "TargetsChanged";

pub async fn watch_target<Target, OnValue>(
    binding: TargetBinding<Target>,
    on_value: OnValue,
) -> Result<(), WatchError>
where
    Target: Send + 'static,
    OnValue: FnMut(NodeId) + Send + 'static,
{
    watch_target_with_context(binding, ProviderContext::default(), on_value).await
}

async fn watch_target_with_context<Target, OnValue>(
    binding: TargetBinding<Target>,
    context: ProviderContext,
    mut on_value: OnValue,
) -> Result<(), WatchError>
where
    Target: Send + 'static,
    OnValue: FnMut(NodeId) + Send + 'static,
{
    if context.is_cancelled() {
        return Ok(());
    }

    let connection = zbus::Connection::session().await?;
    let resolve = GraphResolveProxy::new(&connection).await?;
    let proxy = signal_proxy(&connection, GRAPH_RESOLVE_INTERFACE).await?;
    let mut updates = proxy.receive_signal(RESOLVE_CHANGED_SIGNAL).await?;
    let relations = relations_to_vec(binding.relations);
    let current = resolve
        .subscribe_resolve(binding.source, relations.clone())
        .await?;
    emit_value_if_active(&context, current, &mut on_value);

    loop {
        let signal = tokio::select! {
            _ = context.cancelled() => break,
            signal = updates.next() => signal,
        };
        let Some(signal) = signal else {
            break;
        };
        let (source, path, target) = signal
            .body()
            .deserialize::<(String, Vec<String>, String)>()?;
        if source == binding.source && path == relations {
            emit_value_if_active(&context, target, &mut on_value);
        }
    }

    Ok(())
}

pub async fn watch_node_list<Target, OnValue>(
    binding: NodeListBinding<Target>,
    on_value: OnValue,
) -> Result<(), WatchError>
where
    Target: Send + 'static,
    OnValue: FnMut(Vec<NodeId>) + Send + 'static,
{
    watch_node_list_with_context(binding, ProviderContext::default(), on_value).await
}

async fn watch_node_list_with_context<Target, OnValue>(
    binding: NodeListBinding<Target>,
    context: ProviderContext,
    mut on_value: OnValue,
) -> Result<(), WatchError>
where
    Target: Send + 'static,
    OnValue: FnMut(Vec<NodeId>) + Send + 'static,
{
    if context.is_cancelled() {
        return Ok(());
    }

    let connection = zbus::Connection::session().await?;
    let signal = ListSignal::new(&connection, &binding.query).await?;
    let mut updates = signal.proxy.receive_signal(signal.name).await?;
    let initial = subscribe_node_list(&connection, &binding.query).await?;
    let mut nodes = Vec::new();
    apply_and_emit(&context, &mut nodes, initial, &mut on_value)?;

    loop {
        let signal = tokio::select! {
            _ = context.cancelled() => break,
            signal = updates.next() => signal,
        };
        let Some(signal) = signal else {
            break;
        };
        if let Some(commands) = changed_commands(&binding.query, &signal)? {
            apply_and_emit(&context, &mut nodes, commands, &mut on_value)?;
        }
    }

    Ok(())
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

fn apply_and_emit<OnValue>(
    context: &ProviderContext,
    nodes: &mut Vec<NodeId>,
    commands: Vec<NodeListDiffCommandTuple>,
    on_value: &mut OnValue,
) -> Result<(), WatchError>
where
    OnValue: FnMut(Vec<NodeId>) + Send,
{
    apply_commands(nodes, commands_from_tuples(commands)?)?;
    emit_value_if_active(context, nodes.clone(), on_value);
    Ok(())
}

fn emit_value_if_active<T, OnValue>(context: &ProviderContext, value: T, on_value: &mut OnValue)
where
    OnValue: FnMut(T) + Send,
{
    if !context.is_cancelled() {
        on_value(value);
    }
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

    fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<NodeId>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            watch_target_with_context(self, context, move |value| {
                sender.send(value);
            })
            .await
        }
    }
}

impl<Target> Provider<Vec<NodeId>> for NodeListBinding<Target>
where
    Target: Send + 'static,
{
    type Error = WatchError;

    fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<Vec<NodeId>>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            watch_node_list_with_context(self, context, move |value| {
                sender.send(value);
            })
            .await
        }
    }
}
