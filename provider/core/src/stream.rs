use std::{error::Error as StdError, future::Future, marker::PhantomData};

use tokio_stream::{Stream, StreamExt};

use crate::{Provider, ProviderContext, ProviderSender};

/// Provider backed by a Tokio stream of result values.
#[derive(Debug)]
pub struct StreamProvider<S, T, E> {
    stream: S,
    marker: PhantomData<fn() -> (T, E)>,
}

/// Creates a provider from a stream of `Result<T, E>` values.
pub fn stream_provider<T, E, S>(stream: S) -> StreamProvider<S, T, E>
where
    T: Send + 'static,
    E: StdError + Send + Sync + 'static,
    S: Stream<Item = Result<T, E>> + Send + Unpin + 'static,
{
    StreamProvider::new(stream)
}

impl<S, T, E> StreamProvider<S, T, E> {
    /// Creates a stream-backed provider.
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            marker: PhantomData,
        }
    }
}

impl<S, T, E> Provider<T> for StreamProvider<S, T, E>
where
    T: Send + 'static,
    E: StdError + Send + Sync + 'static,
    S: Stream<Item = Result<T, E>> + Send + Unpin + 'static,
{
    type Error = E;

    fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<T>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            let mut stream = self.stream;

            loop {
                let value = tokio::select! {
                    _ = context.cancelled() => break,
                    value = stream.next() => value,
                };

                let Some(value) = value else {
                    break;
                };

                sender.send(value?);
            }

            Ok(())
        }
    }
}
