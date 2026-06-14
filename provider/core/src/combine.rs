use std::{
    collections::VecDeque,
    error::Error as StdError,
    fmt,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use tokio_stream::Stream;

use crate::{CancellationToken, Provider};

/// Error emitted by a two-source combined stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CombineLatestError<Left, Right> {
    Left(Left),
    Right(Right),
}

impl<Left, Right> fmt::Display for CombineLatestError<Left, Right>
where
    Left: fmt::Display,
    Right: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left(error) => write!(formatter, "left provider failed: {error}"),
            Self::Right(error) => write!(formatter, "right provider failed: {error}"),
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

/// Combines two result streams by emitting the latest successful values from both.
///
/// The combined stream waits until both inputs have emitted at least one
/// successful value. Source errors are forwarded and do not replace the last
/// successful value for that source.
pub fn combine_latest2_stream<LeftValue, RightValue, LeftError, RightError, Left, Right>(
    left: Left,
    right: Right,
) -> CombineLatest2Stream<Left, Right, LeftValue, RightValue, LeftError, RightError>
where
    Left: Stream<Item = Result<LeftValue, LeftError>>,
    Right: Stream<Item = Result<RightValue, RightError>>,
    LeftValue: Clone,
    RightValue: Clone,
{
    CombineLatest2Stream {
        left: Box::pin(left),
        right: Box::pin(right),
        latest_left: None,
        latest_right: None,
        left_done: false,
        right_done: false,
        terminated: false,
        pending: VecDeque::new(),
    }
}

/// Stream returned by [`combine_latest2_stream`].
pub struct CombineLatest2Stream<Left, Right, LeftValue, RightValue, LeftError, RightError>
where
    Left: Stream<Item = Result<LeftValue, LeftError>>,
    Right: Stream<Item = Result<RightValue, RightError>>,
{
    left: Pin<Box<Left>>,
    right: Pin<Box<Right>>,
    latest_left: Option<LeftValue>,
    latest_right: Option<RightValue>,
    left_done: bool,
    right_done: bool,
    terminated: bool,
    pending: VecDeque<Result<(LeftValue, RightValue), CombineLatestError<LeftError, RightError>>>,
}

impl<Left, Right, LeftValue, RightValue, LeftError, RightError> Stream
    for CombineLatest2Stream<Left, Right, LeftValue, RightValue, LeftError, RightError>
where
    Left: Stream<Item = Result<LeftValue, LeftError>>,
    Right: Stream<Item = Result<RightValue, RightError>>,
    LeftValue: Clone,
    RightValue: Clone,
{
    type Item = Result<(LeftValue, RightValue), CombineLatestError<LeftError, RightError>>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if let Some(item) = this.pending.pop_front() {
            return Poll::Ready(Some(item));
        }

        if this.terminated {
            return Poll::Ready(None);
        }

        if !this.left_done {
            match this.left.as_mut().poll_next(context) {
                Poll::Ready(Some(Ok(value))) => {
                    this.latest_left = Some(value);
                    push_latest(
                        &mut this.pending,
                        this.latest_left.as_ref(),
                        this.latest_right.as_ref(),
                    );
                }
                Poll::Ready(Some(Err(error))) => {
                    this.pending.push_back(Err(CombineLatestError::Left(error)));
                }
                Poll::Ready(None) => {
                    this.left_done = true;
                }
                Poll::Pending => {}
            }
        }

        if !this.right_done {
            match this.right.as_mut().poll_next(context) {
                Poll::Ready(Some(Ok(value))) => {
                    this.latest_right = Some(value);
                    push_latest(
                        &mut this.pending,
                        this.latest_left.as_ref(),
                        this.latest_right.as_ref(),
                    );
                }
                Poll::Ready(Some(Err(error))) => {
                    this.pending
                        .push_back(Err(CombineLatestError::Right(error)));
                }
                Poll::Ready(None) => {
                    this.right_done = true;
                }
                Poll::Pending => {}
            }
        }

        if let Some(item) = this.pending.pop_front() {
            return Poll::Ready(Some(item));
        }

        if this.left_done && this.latest_left.is_none()
            || this.right_done && this.latest_right.is_none()
            || this.left_done && this.right_done
        {
            this.terminated = true;
            return Poll::Ready(None);
        }

        Poll::Pending
    }
}

impl<Left, Right, LeftValue, RightValue, LeftError, RightError> Unpin
    for CombineLatest2Stream<Left, Right, LeftValue, RightValue, LeftError, RightError>
where
    Left: Stream<Item = Result<LeftValue, LeftError>>,
    Right: Stream<Item = Result<RightValue, RightError>>,
{
}

fn push_latest<LeftValue, RightValue, LeftError, RightError>(
    pending: &mut VecDeque<
        Result<(LeftValue, RightValue), CombineLatestError<LeftError, RightError>>,
    >,
    latest_left: Option<&LeftValue>,
    latest_right: Option<&RightValue>,
) where
    LeftValue: Clone,
    RightValue: Clone,
{
    if let (Some(left), Some(right)) = (latest_left, latest_right) {
        pending.push_back(Ok((left.clone(), right.clone())));
    }
}

/// Provider returned by [`combine_latest2`].
#[derive(Debug)]
pub struct CombineLatest2<Left, Right, LeftValue, RightValue>
where
    Left: Provider<LeftValue>,
    Right: Provider<RightValue>,
    LeftValue: Send + 'static,
    RightValue: Send + 'static,
{
    left: Left,
    right: Right,
    _values: PhantomData<fn() -> (LeftValue, RightValue)>,
}

/// Combines two providers by emitting the latest successful values from both.
pub fn combine_latest2<LeftValue, RightValue, Left, Right>(
    left: Left,
    right: Right,
) -> CombineLatest2<Left, Right, LeftValue, RightValue>
where
    Left: Provider<LeftValue>,
    Right: Provider<RightValue>,
    LeftValue: Send + 'static,
    RightValue: Send + 'static,
{
    CombineLatest2 {
        left,
        right,
        _values: PhantomData,
    }
}

impl<Left, Right, LeftValue, RightValue> Provider<(LeftValue, RightValue)>
    for CombineLatest2<Left, Right, LeftValue, RightValue>
where
    Left: Provider<LeftValue>,
    Right: Provider<RightValue>,
    Left::Stream: Send + 'static,
    Right::Stream: Send + 'static,
    LeftValue: Clone + Send + 'static,
    RightValue: Clone + Send + 'static,
{
    type Error = CombineLatestError<Left::Error, Right::Error>;
    type Stream = CombineLatest2Stream<
        Left::Stream,
        Right::Stream,
        LeftValue,
        RightValue,
        Left::Error,
        Right::Error,
    >;

    fn stream(self, cancellation: CancellationToken) -> Self::Stream {
        let left = self.left.stream(cancellation.clone());
        let right = self.right.stream(cancellation);
        combine_latest2_stream(left, right)
    }
}
