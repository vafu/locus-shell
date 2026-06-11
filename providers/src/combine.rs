use std::{error::Error as StdError, fmt, future::Future, marker::PhantomData, sync::Arc};

use futures::{FutureExt, StreamExt, channel::mpsc, pin_mut, select};

use crate::{Provider, ProviderContext, ProviderSender};

/// Provider returned by [`ProviderExt::combine_latest`].
#[derive(Debug)]
pub struct CombineLatestProvider<Left, Right, F, LeftValue, RightValue, Output> {
    left: Left,
    right: Right,
    combine: F,
    marker: PhantomData<fn(LeftValue, RightValue) -> Output>,
}

impl<Left, Right, F, LeftValue, RightValue, Output>
    CombineLatestProvider<Left, Right, F, LeftValue, RightValue, Output>
{
    /// Creates a provider that derives values from the latest values of two providers.
    pub fn new(left: Left, right: Right, combine: F) -> Self {
        Self {
            left,
            right,
            combine,
            marker: PhantomData,
        }
    }
}

/// Error returned by a combined provider when either side fails.
#[derive(Debug)]
pub enum CombineLatestError<Left, Right> {
    Left(Left),
    Right(Right),
}

impl<Left, Right> fmt::Display for CombineLatestError<Left, Right>
where
    Left: fmt::Display,
    Right: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left(error) => write!(f, "left provider failed: {error}"),
            Self::Right(error) => write!(f, "right provider failed: {error}"),
        }
    }
}

impl<Left, Right> StdError for CombineLatestError<Left, Right>
where
    Left: StdError + 'static,
    Right: StdError + 'static,
{
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Left(error) => Some(error),
            Self::Right(error) => Some(error),
        }
    }
}

enum CombineEvent<Left, Right> {
    Left(Left),
    Right(Right),
}

impl<Left, Right, F, LeftValue, RightValue, Output> Provider<Output>
    for CombineLatestProvider<Left, Right, F, LeftValue, RightValue, Output>
where
    Left: Provider<LeftValue>,
    Right: Provider<RightValue>,
    F: Fn(&LeftValue, &RightValue) -> Output + Send + Sync + 'static,
    LeftValue: Send + 'static,
    RightValue: Send + 'static,
    Output: Send + 'static,
{
    type Error = CombineLatestError<Left::Error, Right::Error>;

    fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<Output>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            let (events, mut receiver) = mpsc::unbounded();
            let left_events = events.clone();
            let right_events = events.clone();
            drop(events);

            let left_context = context.clone();
            let right_context = context.clone();
            let left = self.left.run(
                left_context,
                ProviderSender::new(move |value| {
                    let _ = left_events.unbounded_send(CombineEvent::Left(value));
                }),
            );
            let right = self.right.run(
                right_context,
                ProviderSender::new(move |value| {
                    let _ = right_events.unbounded_send(CombineEvent::Right(value));
                }),
            );
            let left = left.fuse();
            let right = right.fuse();
            pin_mut!(left, right);

            let combine = Arc::new(self.combine);
            let mut left_latest = None;
            let mut right_latest = None;
            let mut left_done = false;
            let mut right_done = false;

            loop {
                select! {
                    event = receiver.next().fuse() => {
                        match event {
                            Some(CombineEvent::Left(value)) => left_latest = Some(value),
                            Some(CombineEvent::Right(value)) => right_latest = Some(value),
                            None => {
                                if left_done && right_done {
                                    break;
                                }
                            }
                        }

                        if let (Some(left), Some(right)) = (&left_latest, &right_latest) {
                            sender.send(combine(left, right));
                        }
                    },
                    result = left => {
                        left_done = true;
                        if let Err(error) = result {
                            context.cancellation().cancel();
                            return Err(CombineLatestError::Left(error));
                        }
                    },
                    result = right => {
                        right_done = true;
                        if let Err(error) = result {
                            context.cancellation().cancel();
                            return Err(CombineLatestError::Right(error));
                        }
                    },
                    complete => break,
                }
            }

            Ok(())
        }
    }
}
