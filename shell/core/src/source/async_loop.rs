//! Legacy async-loop bridge.
//!
//! This exists only for older handwritten source functions that push values
//! through an emitter. New source primitives should use RxRust-native
//! `Shared::from_stream_result` or `Shared::from_future_result`.

use std::{future::Future, marker::PhantomData};

use rxrust::{
    context::Context,
    observable::{CoreObservable, ObservableType},
    observer::{BoxedObserverSend, IntoBoxedObserver},
    prelude::{Observable as _, Shared},
    scheduler::{Scheduler, TaskHandle},
};

use super::{AsyncEmitter, Observable};

pub(super) fn from_async_loop<T, Run, RunFuture>(run: Run) -> Observable<T>
where
    T: Send + 'static,
    Run: FnOnce(AsyncEmitter<T>) -> RunFuture + Send + 'static,
    RunFuture: Future<Output = ()> + Send + 'static,
{
    Shared::<()>::lift(AsyncLoop::new(run)).box_it()
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
