use std::marker::PhantomData;

use providers::{CancellationToken, Provider};

use crate::{NodeId, Property, WatchError, watch::watch_node_property};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeRef<Model> {
    id: NodeId,
    _model: PhantomData<fn() -> Model>,
}

impl<Model> NodeRef<Model> {
    pub fn new(id: impl Into<NodeId>) -> Self {
        Self {
            id: id.into(),
            _model: PhantomData,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn raw_property<Value>(
        &self,
        property: Property<Model, Value>,
    ) -> NodePropertyBinding<Model> {
        NodePropertyBinding {
            node: self.id.clone(),
            property: property.key(),
            _model: PhantomData,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NodePropertyBinding<Model> {
    node: NodeId,
    property: &'static str,
    _model: PhantomData<fn() -> Model>,
}

impl<Model> PartialEq for NodePropertyBinding<Model> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node && self.property == other.property
    }
}

impl<Model> Eq for NodePropertyBinding<Model> {}

impl<Model> NodePropertyBinding<Model> {
    pub fn node(&self) -> &str {
        &self.node
    }

    pub const fn property_name(&self) -> &'static str {
        self.property
    }
}

pub fn node<Model>(id: impl Into<NodeId>) -> NodeRef<Model> {
    NodeRef::new(id)
}

impl<Model> Provider<String> for NodePropertyBinding<Model>
where
    Model: Send + 'static,
{
    type Error = WatchError;
    type Stream = crate::watch::WatchStream<String>;

    fn stream(self, cancellation: CancellationToken) -> Self::Stream {
        watch_node_property(self.node, self.property, cancellation)
    }
}

impl<Model> property_provider::PropertyBinding<String> for NodePropertyBinding<Model>
where
    Model: Send + 'static,
{
    type Target = Model;
    type Key = NodePropertyKey;

    fn property(&self) -> property_provider::Property<Self::Target, String> {
        property_provider::Property::new(self.property)
    }

    fn key(&self) -> Self::Key {
        NodePropertyKey {
            node: self.node.clone(),
            property: self.property,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NodePropertyKey {
    node: NodeId,
    property: &'static str,
}

impl NodePropertyKey {
    pub fn node(&self) -> &str {
        &self.node
    }

    pub const fn property_name(&self) -> &'static str {
        self.property
    }
}
