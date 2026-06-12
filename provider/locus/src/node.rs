use std::{future::Future, marker::PhantomData};

use providers::{Provider, ProviderContext, ProviderSender};

use crate::{
    DecodeLocusValue, NodeId, WatchError, binding::Property,
    watch::watch_node_property_with_context,
};

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

    pub fn property<Value>(
        self,
        property: Property<Model, Value>,
    ) -> NodePropertyBinding<Model, Value> {
        NodePropertyBinding {
            node: self.id,
            property: property.key,
            _model: PhantomData,
            _value: PhantomData,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodePropertyBinding<Model, Value> {
    pub node: NodeId,
    pub property: &'static str,
    _model: PhantomData<fn() -> Model>,
    _value: PhantomData<fn() -> Value>,
}

pub fn node<Model>(id: impl Into<NodeId>) -> NodeRef<Model> {
    NodeRef::new(id)
}

impl<Model, Value> Provider<Value> for NodePropertyBinding<Model, Value>
where
    Model: Send + 'static,
    Value: DecodeLocusValue + Default + Send + 'static,
{
    type Error = WatchError;

    fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<Value>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            watch_node_property_with_context(self.node, self.property, context, move |value| {
                sender.send(value);
            })
            .await
        }
    }
}
