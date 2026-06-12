use std::marker::PhantomData;

use property_provider::Property;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DbusBus {
    Session,
    System,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Object<Target> {
    bus: DbusBus,
    service: &'static str,
    path: &'static str,
    interface: &'static str,
    _target: PhantomData<fn() -> Target>,
}

impl<Target> Clone for Object<Target> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Target> Copy for Object<Target> {}

#[derive(Debug, Eq, PartialEq)]
pub struct PropertyBinding<Target, Value> {
    bus: DbusBus,
    service: &'static str,
    path: &'static str,
    interface: &'static str,
    property: &'static str,
    _target: PhantomData<fn() -> Target>,
    _value: PhantomData<fn() -> Value>,
}

impl<Target, Value> Clone for PropertyBinding<Target, Value> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Target, Value> Copy for PropertyBinding<Target, Value> {}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DbusPropertyKey {
    bus: DbusBus,
    service: &'static str,
    path: &'static str,
    interface: &'static str,
    property: &'static str,
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

    pub const fn bind<Value>(
        self,
        property: Property<Target, Value>,
    ) -> PropertyBinding<Target, Value> {
        PropertyBinding::new(
            self.bus,
            self.service,
            self.path,
            self.interface,
            property.key(),
        )
    }

    pub const fn bus(&self) -> DbusBus {
        self.bus
    }

    pub const fn service(&self) -> &'static str {
        self.service
    }

    pub const fn path(&self) -> &'static str {
        self.path
    }

    pub const fn interface(&self) -> &'static str {
        self.interface
    }
}

impl<Target, Value> PropertyBinding<Target, Value> {
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
            _target: PhantomData,
            _value: PhantomData,
        }
    }

    pub const fn property_descriptor(&self) -> Property<Target, Value> {
        Property::new(self.property)
    }

    pub const fn bus(&self) -> DbusBus {
        self.bus
    }

    pub const fn service(&self) -> &'static str {
        self.service
    }

    pub const fn path(&self) -> &'static str {
        self.path
    }

    pub const fn interface(&self) -> &'static str {
        self.interface
    }

    pub const fn property_name(&self) -> &'static str {
        self.property
    }

    pub const fn binding_key(&self) -> DbusPropertyKey {
        DbusPropertyKey {
            bus: self.bus,
            service: self.service,
            path: self.path,
            interface: self.interface,
            property: self.property,
        }
    }
}

impl DbusPropertyKey {
    pub const fn bus(&self) -> DbusBus {
        self.bus
    }

    pub const fn service(&self) -> &'static str {
        self.service
    }

    pub const fn path(&self) -> &'static str {
        self.path
    }

    pub const fn interface(&self) -> &'static str {
        self.interface
    }

    pub const fn property_name(&self) -> &'static str {
        self.property
    }
}
