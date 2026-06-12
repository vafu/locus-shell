use std::{
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt, wrappers::WatchStream};

use crate::{Provider, Subscription};

/// Cloneable provider handle returned by [`ProviderExt::shared`](crate::ProviderExt::shared).
#[derive(Debug)]
pub struct SharedProvider<P, T>
where
    P: Provider<T>,
    T: Clone + Send + 'static,
{
    state: Arc<SharedState<P, T>>,
}

#[derive(Debug)]
struct SharedState<P, T>
where
    P: Provider<T>,
    T: Clone + Send + 'static,
{
    provider: P,
    events: watch::Sender<SharedEvent<T, P::Error>>,
    active_subscribers: AtomicUsize,
    subscription: Mutex<Option<Subscription>>,
}

#[derive(Clone, Debug)]
enum SharedEvent<T, E> {
    Pending,
    Value(T),
    Error(E),
    Finished,
}

impl<P, T> SharedProvider<P, T>
where
    P: Provider<T> + Sync,
    T: Clone + Send + Sync + 'static,
    P::Error: Clone,
{
    /// Creates a shared provider handle around `provider`.
    pub fn new(provider: P) -> Self {
        let (events, _) = watch::channel(SharedEvent::Pending);

        Self {
            state: Arc::new(SharedState {
                provider,
                events,
                active_subscribers: AtomicUsize::new(0),
                subscription: Mutex::new(None),
            }),
        }
    }

    fn subscribe(&self) -> watch::Receiver<SharedEvent<T, P::Error>>
    where
        P: Clone,
    {
        let receiver = self.state.events.subscribe();

        if self.state.active_subscribers.fetch_add(1, Ordering::SeqCst) == 0 {
            self.start_upstream();
        }

        receiver
    }

    fn start_upstream(&self)
    where
        P: Clone,
    {
        let provider = self.state.provider.clone();
        let events = self.state.events.clone();
        let mut subscription = self
            .state
            .subscription
            .lock()
            .expect("shared provider subscription lock");

        if let Some(mut previous) = subscription.take() {
            previous.cancel();
        }

        let _ = events.send_replace(SharedEvent::Pending);

        *subscription = Some(Subscription::spawn(move |cancellation| async move {
            let stream = provider.stream(cancellation.clone());
            tokio::pin!(stream);

            loop {
                let item = tokio::select! {
                    _ = cancellation.cancelled() => return,
                    item = stream.next() => item,
                };

                match item {
                    Some(Ok(value)) => {
                        let _ = events.send_replace(SharedEvent::Value(value));
                    }
                    Some(Err(error)) => {
                        let _ = events.send_replace(SharedEvent::Error(error));
                        return;
                    }
                    None => {
                        let _ = events.send_replace(SharedEvent::Finished);
                        return;
                    }
                }
            }
        }));
    }
}

impl<P, T> Clone for SharedProvider<P, T>
where
    P: Provider<T>,
    T: Clone + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl<P, T> Provider<T> for SharedProvider<P, T>
where
    P: Provider<T> + Clone + Sync,
    T: Clone + Send + Sync + 'static,
    P::Error: Clone,
{
    type Error = P::Error;
    type Stream = SharedProviderStream<P, T>;

    fn stream(self, _cancellation: crate::CancellationToken) -> Self::Stream {
        SharedProviderStream {
            events: WatchStream::new(self.subscribe()),
            provider: self,
            finished: false,
        }
    }
}

/// Stream returned by [`SharedProvider`].
#[derive(Debug)]
pub struct SharedProviderStream<P, T>
where
    P: Provider<T> + Sync,
    T: Clone + Send + 'static,
{
    events: WatchStream<SharedEvent<T, P::Error>>,
    provider: SharedProvider<P, T>,
    finished: bool,
}

impl<P, T> Stream for SharedProviderStream<P, T>
where
    P: Provider<T> + Clone + Sync,
    T: Clone + Send + Sync + 'static,
    P::Error: Clone,
{
    type Item = Result<T, P::Error>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if this.finished {
            return Poll::Ready(None);
        }

        loop {
            match Pin::new(&mut this.events).poll_next(context) {
                Poll::Ready(Some(SharedEvent::Pending)) => {}
                Poll::Ready(Some(SharedEvent::Value(value))) => {
                    return Poll::Ready(Some(Ok(value)));
                }
                Poll::Ready(Some(SharedEvent::Error(error))) => {
                    this.finished = true;
                    return Poll::Ready(Some(Err(error)));
                }
                Poll::Ready(Some(SharedEvent::Finished)) | Poll::Ready(None) => {
                    this.finished = true;
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl<P, T> Drop for SharedProviderStream<P, T>
where
    P: Provider<T> + Sync,
    T: Clone + Send + 'static,
{
    fn drop(&mut self) {
        self.provider.unsubscribe();
    }
}

impl<P, T> Drop for SharedState<P, T>
where
    P: Provider<T>,
    T: Clone + Send + 'static,
{
    fn drop(&mut self) {
        if let Some(mut subscription) = self
            .subscription
            .lock()
            .expect("shared provider subscription lock")
            .take()
        {
            subscription.cancel();
        }
    }
}

impl<P, T> SharedProvider<P, T>
where
    P: Provider<T> + Sync,
    T: Clone + Send + 'static,
{
    fn unsubscribe(&self) {
        if self.state.active_subscribers.fetch_sub(1, Ordering::SeqCst) == 1 {
            if let Some(mut subscription) = self
                .state
                .subscription
                .lock()
                .expect("shared provider subscription lock")
                .take()
            {
                subscription.cancel();
            }
        }
    }
}

impl<P, T> Unpin for SharedProviderStream<P, T>
where
    P: Provider<T> + Sync,
    T: Clone + Send,
{
}
