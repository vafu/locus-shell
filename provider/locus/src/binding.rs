use std::marker::PhantomData;

pub type Property<Target, Value> = property_provider::Property<Target, Value>;

#[derive(Debug, Eq, PartialEq)]
pub struct Path<Target> {
    name: &'static str,
    source: &'static str,
    relations: &'static [&'static str],
    many: bool,
    _target: PhantomData<fn() -> Target>,
}

impl<Target> Clone for Path<Target> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Target> Copy for Path<Target> {}

impl<Target> Path<Target> {
    pub const fn new(
        name: &'static str,
        source: &'static str,
        relations: &'static [&'static str],
        many: bool,
    ) -> Self {
        Self {
            name,
            source,
            relations,
            many,
            _target: PhantomData,
        }
    }

    pub const fn raw_property<Value>(
        self,
        property: Property<Target, Value>,
    ) -> LocusPropertyBinding<Target> {
        LocusPropertyBinding {
            source: self.source,
            relations: self.relations,
            property: property.key(),
            _target: PhantomData,
        }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn source(&self) -> &'static str {
        self.source
    }

    pub const fn relations(&self) -> &'static [&'static str] {
        self.relations
    }

    pub const fn is_many(&self) -> bool {
        self.many
    }
}

#[derive(Debug)]
pub struct LocusPropertyBinding<Target> {
    source: &'static str,
    relations: &'static [&'static str],
    property: &'static str,
    _target: PhantomData<fn() -> Target>,
}

impl<Target> Clone for LocusPropertyBinding<Target> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Target> Copy for LocusPropertyBinding<Target> {}

impl<Target> PartialEq for LocusPropertyBinding<Target> {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
            && self.relations == other.relations
            && self.property == other.property
    }
}

impl<Target> Eq for LocusPropertyBinding<Target> {}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LocusPropertyKey {
    pub source: &'static str,
    pub relations: &'static [&'static str],
    pub property: &'static str,
}

impl<Target> LocusPropertyBinding<Target> {
    pub const fn property_descriptor<Value>(&self) -> Property<Target, Value> {
        Property::new(self.property)
    }

    pub const fn source(&self) -> &'static str {
        self.source
    }

    pub const fn relations(&self) -> &'static [&'static str] {
        self.relations
    }

    pub const fn property_name(&self) -> &'static str {
        self.property
    }

    pub const fn binding_key(&self) -> LocusPropertyKey {
        LocusPropertyKey {
            source: self.source,
            relations: self.relations,
            property: self.property,
        }
    }
}

impl LocusPropertyKey {
    pub const fn source(&self) -> &'static str {
        self.source
    }

    pub const fn relations(&self) -> &'static [&'static str] {
        self.relations
    }

    pub const fn property_name(&self) -> &'static str {
        self.property
    }
}
