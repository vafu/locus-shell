use std::{future::Future, marker::PhantomData, sync::Arc};

use crate::{Provider, ProviderContext, ProviderSender};

/// Provider returned by [`ProviderExt::map`].
#[derive(Debug)]
pub struct MapProvider<P, F, Input, Output> {
    provider: P,
    map: F,
    marker: PhantomData<fn(Input) -> Output>,
}

impl<P, F, Input, Output> MapProvider<P, F, Input, Output> {
    /// Creates a provider that maps values from `provider` through `map`.
    pub fn new(provider: P, map: F) -> Self {
        Self {
            provider,
            map,
            marker: PhantomData,
        }
    }
}

/// Extension methods for provider combinators.
pub trait ProviderExt<Input>: Provider<Input> + Sized
where
    Input: Send + 'static,
{
    /// Maps each provider value into another type.
    fn map<Output, F>(self, map: F) -> MapProvider<Self, F, Input, Output>
    where
        Output: Send + 'static,
        F: Fn(Input) -> Output + Send + Sync + 'static,
    {
        MapProvider::new(self, map)
    }
}

impl<Input, P> ProviderExt<Input> for P
where
    Input: Send + 'static,
    P: Provider<Input>,
{
}

impl<P, F, Input, Output> Provider<Output> for MapProvider<P, F, Input, Output>
where
    P: Provider<Input>,
    F: Fn(Input) -> Output + Send + Sync + 'static,
    Input: Send + 'static,
    Output: Send + 'static,
{
    type Error = P::Error;

    fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<Output>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let map = Arc::new(self.map);
        self.provider.run(
            context,
            ProviderSender::new(move |value| {
                sender.send(map(value));
            }),
        )
    }
}
