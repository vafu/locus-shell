use std::marker::PhantomData;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DbusBus {
    Session,
    System,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Object<Target> {
    pub bus: DbusBus,
    pub service: &'static str,
    pub path: &'static str,
    pub interface: &'static str,
    _target: PhantomData<fn() -> Target>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Property<Target, Value> {
    pub key: &'static str,
    _target: PhantomData<fn() -> Target>,
    _value: PhantomData<fn() -> Value>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PropertyBinding<T> {
    pub bus: DbusBus,
    pub service: &'static str,
    pub path: &'static str,
    pub interface: &'static str,
    pub property: &'static str,
    _value: PhantomData<fn() -> T>,
}

impl<Target> Object<Target> {
    pub const fn session(
        service: &'static str,
        path: &'static str,
        interface: &'static str,
    ) -> Self {
        Self::new(DbusBus::Session, service, path, interface)
    }

    pub const fn system(
        service: &'static str,
        path: &'static str,
        interface: &'static str,
    ) -> Self {
        Self::new(DbusBus::System, service, path, interface)
    }

    pub const fn new(
        bus: DbusBus,
        service: &'static str,
        path: &'static str,
        interface: &'static str,
    ) -> Self {
        Self {
            bus,
            service,
            path,
            interface,
            _target: PhantomData,
        }
    }

    pub const fn bind<Value>(self, property: Property<Target, Value>) -> PropertyBinding<Value> {
        PropertyBinding::new(
            self.bus,
            self.service,
            self.path,
            self.interface,
            property.key,
        )
    }
}

impl<Target, Value> Property<Target, Value> {
    pub const fn new(key: &'static str) -> Self {
        Self {
            key,
            _target: PhantomData,
            _value: PhantomData,
        }
    }
}

impl<T> PropertyBinding<T> {
    const fn new(
        bus: DbusBus,
        service: &'static str,
        path: &'static str,
        interface: &'static str,
        property: &'static str,
    ) -> Self {
        Self {
            bus,
            service,
            path,
            interface,
            property,
            _value: PhantomData,
        }
    }
}
