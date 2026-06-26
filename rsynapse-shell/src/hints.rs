use std::sync::{Mutex, OnceLock};

use shell_core::source::{
    Observable,
    rx::{
        BoxedSubscriptionSend, Context, CoreObservable, IntoBoxedSubscription, Observable as _,
        ObservableFactory as _, ObservableType, Observer, Shared, SharedSubject,
    },
};

use crate::request::HintsAction;

pub(crate) fn hints_active() -> Observable<bool> {
    Shared::<()>::lift(HintsActive).box_it()
}

pub(crate) fn apply(action: HintsAction) {
    match action {
        HintsAction::Set(active) => set_active(active),
        HintsAction::Toggle => set_active(!active()),
    }
}

fn active() -> bool {
    *state().active.lock().expect("hints active lock poisoned")
}

fn set_active(active: bool) {
    let state = state();
    let changed = {
        let mut current = state.active.lock().expect("hints active lock poisoned");
        if *current == active {
            false
        } else {
            *current = active;
            true
        }
    };
    if changed {
        state
            .subject
            .lock()
            .expect("hints subject lock poisoned")
            .next(active);
    }
}

fn state() -> &'static HintsState {
    static STATE: OnceLock<HintsState> = OnceLock::new();
    STATE.get_or_init(HintsState::default)
}

struct HintsState {
    active: Mutex<bool>,
    subject: Mutex<SharedSubject<'static, bool, String>>,
}

impl Default for HintsState {
    fn default() -> Self {
        Self {
            active: Mutex::new(false),
            subject: Mutex::new(Shared::subject()),
        }
    }
}

struct HintsActive;

impl ObservableType for HintsActive {
    type Item<'a> = bool;
    type Err = String;
}

impl<C> CoreObservable<C> for HintsActive
where
    C: Context,
    C::Inner: Observer<bool, String> + Send + 'static,
{
    type Unsub = BoxedSubscriptionSend;

    fn subscribe(self, context: C) -> Self::Unsub {
        let state = state();
        let mut observer = context.into_inner();
        observer.next(active());
        state
            .subject
            .lock()
            .expect("hints subject lock poisoned")
            .clone()
            .subscribe_with(observer)
            .into_boxed()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use shell_core::source::rx::{Observable as _, Observer, Subscription};

    use super::{HintsAction, apply, hints_active};

    #[derive(Clone)]
    struct Capture(Arc<Mutex<Vec<bool>>>);

    impl Observer<bool, String> for Capture {
        fn next(&mut self, value: bool) {
            self.0.lock().unwrap().push(value);
        }

        fn error(self, _err: String) {}

        fn complete(self) {}

        fn is_closed(&self) -> bool {
            false
        }
    }

    #[test]
    fn hints_observable_emits_initial_and_changes() {
        apply(HintsAction::Set(false));
        let values = Arc::new(Mutex::new(Vec::new()));
        let subscription = hints_active().subscribe_with(Capture(values.clone()));

        apply(HintsAction::Set(true));
        apply(HintsAction::Set(true));
        apply(HintsAction::Toggle);

        subscription.unsubscribe();
        assert_eq!(*values.lock().unwrap(), vec![false, true, false]);
    }
}
