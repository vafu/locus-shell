use futures_util::StreamExt;
use locus_dbus::{BUS_NAME, GraphReadProxy, GraphResolveProxy, NONE_STRING, WATCH_INTERFACE};
use zbus::Proxy;
use zbus::proxy::{Builder as ProxyBuilder, CacheProperties};
use zbus::zvariant::OwnedValue;

use crate::{
    DbusBus, DecodeLocusValue, FieldBinding, PropertyBinding, WatchError, decode_wire_field,
};

const PROPERTIES_UPDATED_SIGNAL: &str = "PropertiesUpdated";

pub async fn watch_field<T, OnValue>(
    binding: FieldBinding<T>,
    mut on_value: OnValue,
) -> Result<(), WatchError>
where
    T: DecodeLocusValue + Default,
    OnValue: FnMut(T) + Send,
{
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

    let watch_result = stream_field_updates(&read, &watch, binding.property, &mut on_value).await;
    let close_result = watch.call::<_, _, ()>("Close", &()).await;

    match (watch_result, close_result) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
        (Ok(()), Ok(())) => Ok(()),
    }
}

pub async fn watch_property<T, OnValue>(
    binding: PropertyBinding<T>,
    mut on_value: OnValue,
) -> Result<(), WatchError>
where
    T: TryFrom<OwnedValue> + Send + Unpin + 'static,
    T::Error: Into<zbus::Error>,
    OnValue: FnMut(T) + Send,
{
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

    let _ = proxy.get_property::<T>(binding.property).await?;
    let mut updates = proxy.receive_property_changed::<T>(binding.property).await;

    while let Some(update) = updates.next().await {
        on_value(update.get().await?);
    }

    Ok(())
}

async fn stream_field_updates<T, OnValue>(
    read: &GraphReadProxy<'_>,
    watch: &Proxy<'_>,
    property: &str,
    on_value: &mut OnValue,
) -> Result<(), WatchError>
where
    T: DecodeLocusValue + Default,
    OnValue: FnMut(T) + Send,
{
    emit_current_field(read, watch, property, on_value).await?;
    let mut updated = watch.receive_signal(PROPERTIES_UPDATED_SIGNAL).await?;

    while let Some(signal) = updated.next().await {
        let (changed, removed) = signal
            .body()
            .deserialize::<(std::collections::HashMap<String, String>, Vec<String>)>()?;
        if let Some(value) = changed.get(property) {
            on_value(decode_wire_field(value)?);
        } else if removed.iter().any(|key| key == property) {
            on_value(T::default());
        }
    }

    Ok(())
}

async fn emit_current_field<T, OnValue>(
    read: &GraphReadProxy<'_>,
    watch: &Proxy<'_>,
    property: &str,
    on_value: &mut OnValue,
) -> Result<(), WatchError>
where
    T: DecodeLocusValue + Default,
    OnValue: FnMut(T) + Send,
{
    let target = watch.get_property::<String>("Target").await?;
    if target == NONE_STRING {
        on_value(T::default());
        return Ok(());
    }

    let value = read.get_property(&target, property).await?;
    on_value(decode_wire_field(&value)?);
    Ok(())
}
