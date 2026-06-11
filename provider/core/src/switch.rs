use std::{error::Error as StdError, fmt, future::Future, marker::PhantomData};

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::{StreamExt, wrappers::UnboundedReceiverStream};

use crate::{CancellationToken, Provider, ProviderContext, ProviderSender, spawn};

/// Provider returned by [`ProviderExt::switch_map`](crate::ProviderExt::switch_map).
#[derive(Debug)]
pub struct SwitchMapProvider<Upstream, F, Input, Downstream, Output> {
    upstream: Upstream,
    map: F,
    marker: PhantomData<fn(Input, Downstream) -> Output>,
}

impl<Upstream, F, Input, Downstream, Output>
    SwitchMapProvider<Upstream, F, Input, Downstream, Output>
{
    /// Creates a provider that switches downstream providers on upstream values.
    pub fn new(upstream: Upstream, map: F) -> Self {
        Self {
            upstream,
            map,
            marker: PhantomData,
        }
    }
}

/// Error returned by a switched provider when either side fails.
#[derive(Debug)]
pub enum SwitchMapError<Upstream, Downstream> {
    Upstream(Upstream),
    Downstream(Downstream),
}

impl<Upstream, Downstream> fmt::Display for SwitchMapError<Upstream, Downstream>
where
    Upstream: fmt::Display,
    Downstream: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Upstream(error) => write!(formatter, "upstream provider failed: {error}"),
            Self::Downstream(error) => write!(formatter, "downstream provider failed: {error}"),
        }
    }
}

impl<Upstream, Downstream> StdError for SwitchMapError<Upstream, Downstream>
where
    Upstream: StdError + 'static,
    Downstream: StdError + 'static,
{
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Upstream(error) => Some(error),
            Self::Downstream(error) => Some(error),
        }
    }
}

enum SwitchEvent<Input, UpstreamError, Output, DownstreamError> {
    UpstreamValue(Input),
    UpstreamDone(Result<(), UpstreamError>),
    DownstreamValue(usize, Output),
    DownstreamDone(usize, Result<(), DownstreamError>),
}

struct ActiveDownstream {
    id: usize,
    cancellation: CancellationToken,
    task: JoinHandle<()>,
}

impl ActiveDownstream {
    fn cancel(self) {
        self.cancellation.cancel();
        self.task.abort();
    }
}

impl Drop for ActiveDownstream {
    fn drop(&mut self) {
        self.cancellation.cancel();
        self.task.abort();
    }
}

impl<Upstream, F, Input, Downstream, Output> Provider<Output>
    for SwitchMapProvider<Upstream, F, Input, Downstream, Output>
where
    Upstream: Provider<Input>,
    F: Fn(Input) -> Downstream + Send + Sync + 'static,
    Input: Send + 'static,
    Downstream: Provider<Output>,
    Output: Send + 'static,
{
    type Error = SwitchMapError<Upstream::Error, Downstream::Error>;

    fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<Output>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            let (event_sender, receiver) = mpsc::unbounded_channel();
            let mut events = UnboundedReceiverStream::new(receiver);
            let upstream_events = event_sender.clone();
            let upstream_context = context.clone();
            let upstream = self.upstream;
            let upstream_task = spawn(async move {
                let value_events = upstream_events.clone();
                let result = upstream
                    .run(
                        upstream_context,
                        ProviderSender::new(move |value| {
                            let _ = value_events.send(SwitchEvent::UpstreamValue(value));
                        }),
                    )
                    .await;

                let _ = upstream_events.send(SwitchEvent::UpstreamDone(result));
            });

            let mut next_downstream_id = 0;
            let mut active_downstream: Option<ActiveDownstream> = None;
            let mut upstream_done = false;

            let result = loop {
                let event = tokio::select! {
                    _ = context.cancelled() => break Ok(()),
                    event = events.next() => event,
                };

                match event {
                    Some(SwitchEvent::UpstreamValue(value)) => {
                        if let Some(active) = active_downstream.take() {
                            active.cancel();
                        }

                        let id = next_downstream_id;
                        next_downstream_id += 1;
                        let cancellation = CancellationToken::new();
                        let downstream_context = ProviderContext::new(cancellation.clone());
                        let downstream = (self.map)(value);
                        let downstream_events = event_sender.clone();
                        let task = spawn(async move {
                            let value_events = downstream_events.clone();
                            let result = downstream
                                .run(
                                    downstream_context,
                                    ProviderSender::new(move |value| {
                                        let _ = value_events
                                            .send(SwitchEvent::DownstreamValue(id, value));
                                    }),
                                )
                                .await;

                            let _ = downstream_events.send(SwitchEvent::DownstreamDone(id, result));
                        });

                        active_downstream = Some(ActiveDownstream {
                            id,
                            cancellation,
                            task,
                        });
                    }
                    Some(SwitchEvent::UpstreamDone(Ok(()))) => {
                        upstream_done = true;
                        if active_downstream.is_none() {
                            break Ok(());
                        }
                    }
                    Some(SwitchEvent::UpstreamDone(Err(error))) => {
                        context.cancellation().cancel();
                        break Err(SwitchMapError::Upstream(error));
                    }
                    Some(SwitchEvent::DownstreamValue(id, value)) => {
                        if active_downstream
                            .as_ref()
                            .is_some_and(|active| active.id == id)
                        {
                            sender.send(value);
                        }
                    }
                    Some(SwitchEvent::DownstreamDone(id, Ok(()))) => {
                        if active_downstream
                            .as_ref()
                            .is_some_and(|active| active.id == id)
                        {
                            active_downstream = None;
                            if upstream_done {
                                break Ok(());
                            }
                        }
                    }
                    Some(SwitchEvent::DownstreamDone(id, Err(error))) => {
                        if active_downstream
                            .as_ref()
                            .is_some_and(|active| active.id == id)
                        {
                            context.cancellation().cancel();
                            break Err(SwitchMapError::Downstream(error));
                        }
                    }
                    None => break Ok(()),
                }
            };

            upstream_task.abort();
            if let Some(active) = active_downstream {
                active.cancel();
            }

            result
        }
    }
}
