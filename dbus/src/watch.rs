use std::future::Future;

use futures_util::StreamExt;
use providers::{Provider, ProviderContext, ProviderSender};
use zbus::Proxy;
use zbus::proxy::{Builder as ProxyBuilder, CacheProperties};
use zbus::zvariant::OwnedValue;

use crate::{DbusBus, PropertyBinding, WatchError};

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
