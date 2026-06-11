use std::{
    future::Future,
    sync::{Arc, Mutex},
};

use tokio::sync::watch;

use crate::{
    Provider, ProviderContext, ProviderError, ProviderSender, Subscription, run_provider, spawn,
};

/// Cloneable provider handle returned by [`ProviderExt::shared`](crate::ProviderExt::shared).
#[derive(Debug)]
pub struct SharedProvider<P, T> {
    state: Arc<SharedState<P, T>>,
}

#[derive(Debug)]
struct SharedState<P, T> {
    provider: Mutex<Option<P>>,
    latest: Arc<Mutex<Option<T>>>,
    events: watch::Sender<SharedEvent<T>>,
    subscription: Mutex<Option<Subscription>>,
}

#[derive(Clone, Debug)]
enum SharedEvent<T> {
    Pending,
    Value(T),
    Failed(ProviderError),
    Finished,
}

impl<P, T> SharedProvider<P, T> {
    /// Creates a shared provider handle around `provider`.
    pub fn new(provider: P) -> Self {
        let (events, _) = watch::channel(SharedEvent::Pending);

        Self {
            state: Arc::new(SharedState {
                provider: Mutex::new(Some(provider)),
                latest: Arc::new(Mutex::new(None)),
                events,
                subscription: Mutex::new(None),
            }),
        }
    }
}

impl<P, T> Clone for SharedProvider<P, T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl<P, T> SharedProvider<P, T>
where
    P: Provider<T>,
    T: Clone + Send + Sync + 'static,
{
    fn start_once(&self) {
        let Some(provider) = self
            .state
            .provider
            .lock()
            .expect("shared provider lock")
            .take()
        else {
            return;
        };

        let mut subscription = Subscription::new();
        let context = subscription.context();
        let latest = self.state.latest.clone();
        let events = self.state.events.clone();
        let task = spawn(async move {
            let value_events = events.clone();
            let result = run_provider(provider, context, move |value: T| {
                *latest.lock().expect("shared provider latest lock") = Some(value.clone());
                let _ = value_events.send(SharedEvent::Value(value));
            })
            .await;

            match result {
                Ok(()) => {
                    let _ = events.send(SharedEvent::Finished);
                }
                Err(error) => {
                    let _ = events.send(SharedEvent::Failed(error));
                }
            }
        });

        subscription.set_task(task);
        *self
            .state
            .subscription
            .lock()
            .expect("shared provider subscription lock") = Some(subscription);
    }
}

impl<P, T> Provider<T> for SharedProvider<P, T>
where
    P: Provider<T>,
    T: Clone + Send + Sync + 'static,
{
    type Error = ProviderError;

    fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<T>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            self.start_once();

            let mut receiver = self.state.events.subscribe();
            let mut skip_initial_value = false;
            let mut sent_value = false;
            if let Some(value) = self
                .state
                .latest
                .lock()
                .expect("shared provider latest lock")
                .clone()
            {
                sender.send(value);
                skip_initial_value = true;
                sent_value = true;
            }

            loop {
                match receiver.borrow_and_update().clone() {
                    SharedEvent::Pending => {}
                    SharedEvent::Value(value) => {
                        if skip_initial_value {
                            skip_initial_value = false;
                        } else {
                            sender.send(value);
                            sent_value = true;
                        }
                    }
                    SharedEvent::Failed(error) => return Err(error),
                    SharedEvent::Finished => {
                        if !sent_value {
                            if let Some(value) = self
                                .state
                                .latest
                                .lock()
                                .expect("shared provider latest lock")
                                .clone()
                            {
                                sender.send(value);
                            }
                        }
                        return Ok(());
                    }
                }

                tokio::select! {
                    _ = context.cancelled() => return Ok(()),
                    changed = receiver.changed() => {
                        if changed.is_err() {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}

impl<P, T> Drop for SharedState<P, T> {
    fn drop(&mut self) {
        if let Some(subscription) = self
            .subscription
            .lock()
            .expect("shared provider subscription lock")
            .take()
        {
            drop(subscription);
        }
    }
}
