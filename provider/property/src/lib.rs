use std::{hash::Hash, marker::PhantomData};

#[cfg(test)]
mod test;

pub use providers;

#[derive(Debug, Eq, PartialEq)]
pub struct Property<Target, Value> {
    key: &'static str,
    _target: PhantomData<fn() -> Target>,
    _value: PhantomData<fn() -> Value>,
}

impl<Target, Value> Clone for Property<Target, Value> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Target, Value> Copy for Property<Target, Value> {}

impl<Target, Value> Property<Target, Value> {
    pub const fn new(key: &'static str) -> Self {
        Self {
            key,
            _target: PhantomData,
            _value: PhantomData,
        }
    }

    pub const fn key(&self) -> &'static str {
        self.key
    }
}

pub trait PropertyBinding<T>: providers::Provider<T>
where
    T: Send + 'static,
{
    type Target;
    type Key: Clone + Eq + Hash;

    fn property(&self) -> Property<Self::Target, T>;
    fn key(&self) -> Self::Key;
}
