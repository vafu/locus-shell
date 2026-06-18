use std::{
    error::Error as StdError,
    fmt,
    future::Future,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use rxrust::prelude::{
    BoxedSubscriptionSend, IntoBoxedSubscription as _, Observable as RxObservable,
    ObservableFactory as _, Observer as RxObserver, Shared, SharedBoxedObservable,
    Subscription as _,
};
use rxrust::{
    context::Context,
    observable::{CoreObservable, ObservableType},
    observer::{BoxedObserverSend, IntoBoxedObserver},
    scheduler::{Scheduler, TaskHandle},
};

pub use rxrust;
pub use rxrust::prelude as rx;

pub type Observable<T, E = SourceError> = SharedBoxedObservable<'static, T, E>;

pub trait IntoObservable<T>
where
    T: Send + 'static,
{
    type Error: StdError + Send + Sync + 'static;

    fn into_observable(self) -> Observable<T, Self::Error>;
}

impl<T, E> IntoObservable<T> for Observable<T, E>
where
    T: Send + 'static,
    E: StdError + Send + Sync + 'static,
{
    type Error = E;

    fn into_observable(self) -> Observable<T, Self::Error> {
        self
    }
}

pub fn into_observable<T, Source>(source: Source) -> Observable<T, Source::Error>
where
    T: Send + 'static,
    Source: IntoObservable<T>,
{
    source.into_observable()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceError {
    message: String,
}

impl SourceError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for SourceError {}

#[derive(Default)]
pub struct Subscriptions {
    subscriptions: Vec<BoxedSubscriptionSend>,
}

impl Subscriptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, subscription: BoxedSubscriptionSend) {
        self.subscriptions.push(subscription);
    }

    pub fn extend(&mut self, mut subscriptions: Self) {
        self.subscriptions.append(&mut subscriptions.subscriptions);
    }

    pub fn cancel(&mut self) {
        while let Some(subscription) = self.subscriptions.pop() {
            subscription.unsubscribe();
        }
    }
}

impl fmt::Debug for Subscriptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Subscriptions")
            .field("len", &self.subscriptions.len())
            .finish()
    }
}

impl Drop for Subscriptions {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub fn subscribe<T, E, OnResult>(
    observable: Observable<T, E>,
    on_result: OnResult,
) -> BoxedSubscriptionSend
where
    T: Send + 'static,
    E: StdError + Send + Sync + 'static,
    OnResult: FnMut(Result<T, SourceError>) + Send + 'static,
{
    observable
        .subscribe_with(ResultObserver { on_result })
        .into_boxed()
}

pub fn once<T>(value: T) -> Observable<T>
where
    T: Send + 'static,
{
    Shared::<()>::create::<T, SourceError, _, _>(move |emitter| {
        emitter.next(value);
        emitter.complete();
    })
    .box_it()
}

pub fn from_async_loop<T, Run, RunFuture>(run: Run) -> Observable<T>
where
    T: Send + 'static,
    Run: FnOnce(AsyncEmitter<T>) -> RunFuture + Send + 'static,
    RunFuture: Future<Output = ()> + Send + 'static,
{
    Shared::<()>::lift(AsyncLoop::new(run)).box_it()
}

struct ResultObserver<OnResult> {
    on_result: OnResult,
}

impl<T, E, OnResult> RxObserver<T, E> for ResultObserver<OnResult>
where
    OnResult: FnMut(Result<T, SourceError>),
    E: StdError,
{
    fn next(&mut self, value: T) {
        (self.on_result)(Ok(value));
    }

    fn error(mut self, error: E) {
        (self.on_result)(Err(SourceError::new(error.to_string())));
    }

    fn complete(self) {}

    fn is_closed(&self) -> bool {
        false
    }
}

pub struct AsyncEmitter<T, E = SourceError> {
    observer: Arc<Mutex<Option<BoxedObserverSend<'static, T, E>>>>,
}

impl<T, E> Clone for AsyncEmitter<T, E> {
    fn clone(&self) -> Self {
        Self {
            observer: self.observer.clone(),
        }
    }
}

impl<T, E> AsyncEmitter<T, E> {
    fn new(observer: BoxedObserverSend<'static, T, E>) -> Self {
        Self {
            observer: Arc::new(Mutex::new(Some(observer))),
        }
    }

    pub fn next(&self, value: T) {
        let Ok(mut observer) = self.observer.lock() else {
            return;
        };
        if let Some(observer) = observer.as_mut() {
            observer.next(value);
        }
    }

    pub fn error(&self, error: E) {
        let Ok(mut observer) = self.observer.lock() else {
            return;
        };
        if let Some(observer) = observer.take() {
            observer.error(error);
        }
    }

    pub fn complete(&self) {
        let Ok(mut observer) = self.observer.lock() else {
            return;
        };
        if let Some(observer) = observer.take() {
            observer.complete();
        }
    }
}

struct AsyncLoop<Run, T, E> {
    run: Run,
    _marker: PhantomData<fn() -> (T, E)>,
}

impl<Run, T, E> AsyncLoop<Run, T, E> {
    fn new(run: Run) -> Self {
        Self {
            run,
            _marker: PhantomData,
        }
    }
}

impl<Run, T, E> ObservableType for AsyncLoop<Run, T, E> {
    type Item<'a>
        = T
    where
        Self: 'a;
    type Err = E;
}

impl<C, Run, RunFuture, T, E> CoreObservable<C> for AsyncLoop<Run, T, E>
where
    C: Context,
    C::Inner: IntoBoxedObserver<BoxedObserverSend<'static, T, E>> + Send + 'static,
    C::Scheduler: Scheduler<RunFuture> + Clone,
    Run: FnOnce(AsyncEmitter<T, E>) -> RunFuture,
    RunFuture: Future<Output = ()> + Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    type Unsub = TaskHandle;

    fn subscribe(self, context: C) -> Self::Unsub {
        let scheduler = context.scheduler().clone();
        let observer = context.into_inner().into_boxed();
        scheduler.schedule((self.run)(AsyncEmitter::new(observer)), None)
    }
}
