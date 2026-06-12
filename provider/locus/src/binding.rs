use std::marker::PhantomData;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Property<Model, Value> {
    pub key: &'static str,
    _model: PhantomData<fn() -> Model>,
    _value: PhantomData<fn() -> Value>,
}

impl<Model, Value> Property<Model, Value> {
    pub const fn new(key: &'static str) -> Self {
        Self {
            key,
            _model: PhantomData,
            _value: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Path<Target> {
    pub name: &'static str,
    pub source: &'static str,
    pub relations: &'static [&'static str],
    pub many: bool,
    _target: PhantomData<fn() -> Target>,
}

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

    pub const fn property<Value>(self, property: Property<Target, Value>) -> FieldBinding<Value> {
        FieldBinding {
            source: self.source,
            relations: self.relations,
            property: property.key,
            _value: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldBinding<Value> {
    pub source: &'static str,
    pub relations: &'static [&'static str],
    pub property: &'static str,
    _value: PhantomData<fn() -> Value>,
}
