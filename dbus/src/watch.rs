use std::future::Future;

use futures_util::StreamExt;
use locus_dbus::{BUS_NAME, GraphReadProxy, GraphResolveProxy, NONE_STRING, WATCH_INTERFACE};
use providers::{Provider, ProviderContext, ProviderSender};
use zbus::Proxy;
use zbus::proxy::{Builder as ProxyBuilder, CacheProperties};
use zbus::zvariant::OwnedValue;

use crate::{
    DbusBus, DecodeLocusValue, FieldBinding, PropertyBinding, WatchError, decode_wire_field,
};

const PROPERTIES_UPDATED_SIGNAL: &str = "PropertiesUpdated";

pub async fn watch_field<T, OnValue>(
    binding: FieldBinding<T>,
    on_value: OnValue,
) -> Result<(), WatchError>
where
    T: DecodeLocusValue + Default + Send + 'static,
    OnValue: FnMut(T) + Send + 'static,
{
    watch_field_with_context(binding, ProviderContext::default(), on_value).await
}

async fn watch_field_with_context<T, OnValue>(
    binding: FieldBinding<T>,
    context: ProviderContext,
    mut on_value: OnValue,
) -> Result<(), WatchError>
where
    T: DecodeLocusValue + Default + Send + 'static,
    OnValue: FnMut(T) + Send + 'static,
{
    if context.is_cancelled() {
        return Ok(());
    }

    let connection = zbus::Connection::session().await?;
    let read = GraphReadProxy::new(&connection).await?;
    let resolve = GraphResolveProxy::new(&connection).await?;
    let object_path = resolve
        .watch_node(
            binding.source,
            binding
                .relations
                .iter()
                .map(|relation| (*relation).to_owned())
                .collect(),
        )
        .await?;
    let watch = ProxyBuilder::<Proxy<'_>>::new(&connection)
        .destination(BUS_NAME)?
        .path(object_path.as_str())?
        .interface(WATCH_INTERFACE)?
        .cache_properties(CacheProperties::No)
        .build()
        .await?;

    let watch_result =
        stream_field_updates(&read, &watch, binding.property, &context, &mut on_value).await;
    let close_result = watch.call::<_, _, ()>("Close", &()).await;

    match (watch_result, close_result) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
        (Ok(()), Ok(())) => Ok(()),
    }
}

pub async fn watch_property<T, OnValue>(
    binding: PropertyBinding<T>,
    on_value: OnValue,
) -> Result<(), WatchError>
where
    T: TryFrom<OwnedValue> + Send + Sync + Unpin + 'static,
    T::Error: Into<zbus::Error>,
    OnValue: FnMut(T) + Send + 'static,
{
    watch_property_with_context(binding, ProviderContext::default(), on_value).await
}

async fn watch_property_with_context<T, OnValue>(
    binding: PropertyBinding<T>,
    context: ProviderContext,
    mut on_value: OnValue,
) -> Result<(), WatchError>
where
    T: TryFrom<OwnedValue> + Send + Sync + Unpin + 'static,
    T::Error: Into<zbus::Error>,
    OnValue: FnMut(T) + Send + 'static,
{
    if context.is_cancelled() {
        return Ok(());
    }

    let connection = match binding.bus {
        DbusBus::Session => zbus::Connection::session().await?,
        DbusBus::System => zbus::Connection::system().await?,
    };
    let proxy = ProxyBuilder::<Proxy<'_>>::new(&connection)
        .destination(binding.service)?
        .path(binding.path)?
        .interface(binding.interface)?
        .cache_properties(CacheProperties::Yes)
        .build()
        .await?;

    emit_value_if_active(
        &context,
        proxy.get_property::<T>(binding.property).await?,
        &mut on_value,
    );
    if context.is_cancelled() {
        return Ok(());
    }

    let mut updates = proxy.receive_property_changed::<T>(binding.property).await;

    while let Some(update) = updates.next().await {
        if context.is_cancelled() {
            break;
        }
        emit_value_if_active(&context, update.get().await?, &mut on_value);
    }

    Ok(())
}

async fn stream_field_updates<T, OnValue>(
    read: &GraphReadProxy<'_>,
    watch: &Proxy<'_>,
    property: &str,
    context: &ProviderContext,
    on_value: &mut OnValue,
) -> Result<(), WatchError>
where
    T: DecodeLocusValue + Default + Send + 'static,
    OnValue: FnMut(T) + Send,
{
    if context.is_cancelled() {
        return Ok(());
    }

    emit_current_field(read, watch, property, context, on_value).await?;
    if context.is_cancelled() {
        return Ok(());
    }

    let mut updated = watch.receive_signal(PROPERTIES_UPDATED_SIGNAL).await?;

    while let Some(signal) = updated.next().await {
        if context.is_cancelled() {
            break;
        }
        let (changed, removed) = signal
            .body()
            .deserialize::<(std::collections::HashMap<String, String>, Vec<String>)>()?;
        if let Some(value) = changed.get(property) {
            emit_field_value(context, value, on_value)?;
        } else if removed.iter().any(|key| key == property) {
            emit_value_if_active(context, T::default(), on_value);
        }
    }

    Ok(())
}

pub(crate) fn emit_value_if_active<T, OnValue>(
    context: &ProviderContext,
    value: T,
    on_value: &mut OnValue,
) where
    OnValue: FnMut(T) + Send,
{
    if !context.is_cancelled() {
        on_value(value);
    }
}

pub(crate) fn emit_field_value<T, OnValue>(
    context: &ProviderContext,
    value: impl AsRef<str>,
    on_value: &mut OnValue,
) -> Result<(), WatchError>
where
    T: DecodeLocusValue + Default,
    OnValue: FnMut(T) + Send,
{
    if !context.is_cancelled() {
        on_value(decode_wire_field(value.as_ref())?);
    }

    Ok(())
}

async fn emit_current_field<T, OnValue>(
    read: &GraphReadProxy<'_>,
    watch: &Proxy<'_>,
    property: &str,
    context: &ProviderContext,
    on_value: &mut OnValue,
) -> Result<(), WatchError>
where
    T: DecodeLocusValue + Default,
    OnValue: FnMut(T) + Send,
{
    let target = watch.get_property::<String>("Target").await?;
    if target == NONE_STRING {
        emit_value_if_active(context, T::default(), on_value);
        return Ok(());
    }

    let value = read.get_property(&target, property).await?;
    emit_field_value(context, value, on_value)
}

impl<T> Provider<T> for FieldBinding<T>
where
    T: DecodeLocusValue + Default + Send + 'static,
{
    type Error = WatchError;

    fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<T>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            watch_field_with_context(self, context, move |value| {
                sender.send(value);
            })
            .await
        }
    }
}

impl<T> Provider<T> for PropertyBinding<T>
where
    T: TryFrom<OwnedValue> + Send + Sync + Unpin + 'static,
    T::Error: Into<zbus::Error>,
{
    type Error = WatchError;

    fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<T>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            watch_property_with_context(self, context, move |value| {
                sender.send(value);
            })
            .await
        }
    }
}
