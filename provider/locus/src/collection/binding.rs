use std::marker::PhantomData;

use crate::Path;

pub type NodeId = String;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Relation<Source, Target> {
    name: &'static str,
    _source: PhantomData<fn() -> Source>,
    _target: PhantomData<fn() -> Target>,
}

impl<Source, Target> Relation<Source, Target> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _source: PhantomData,
            _target: PhantomData,
        }
    }

    pub fn sources(self, target: impl Into<NodeId>) -> NodeListBinding<Source> {
        NodeListBinding::sources(target.into(), self.name)
    }

    pub fn targets(self, source: impl Into<NodeId>) -> NodeListBinding<Target> {
        NodeListBinding::targets(source.into(), self.name)
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetBinding<Target> {
    source: &'static str,
    relations: &'static [&'static str],
    _target: PhantomData<fn() -> Target>,
}

impl<Target> TargetBinding<Target> {
    pub const fn new(source: &'static str, relations: &'static [&'static str]) -> Self {
        Self {
            source,
            relations,
            _target: PhantomData,
        }
    }

    pub const fn source(&self) -> &'static str {
        self.source
    }

    pub const fn relations(&self) -> &'static [&'static str] {
        self.relations
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeListBinding<Target> {
    query: NodeListQuery,
    _target: PhantomData<fn() -> Target>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeListQuery {
    ResolveAll {
        source: NodeId,
        relations: Vec<String>,
    },
    Sources {
        target: NodeId,
        relation: &'static str,
    },
    Targets {
        source: NodeId,
        relation: &'static str,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KindFilteredNodeListBinding<Target> {
    binding: NodeListBinding<Target>,
    kind: &'static str,
}

impl<Target> NodeListBinding<Target> {
    pub fn resolve_all(
        source: impl Into<NodeId>,
        relations: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            query: NodeListQuery::ResolveAll {
                source: source.into(),
                relations: relations.into_iter().collect(),
            },
            _target: PhantomData,
        }
    }

    pub fn sources(target: impl Into<NodeId>, relation: &'static str) -> Self {
        Self {
            query: NodeListQuery::Sources {
                target: target.into(),
                relation,
            },
            _target: PhantomData,
        }
    }

    pub fn targets(source: impl Into<NodeId>, relation: &'static str) -> Self {
        Self {
            query: NodeListQuery::Targets {
                source: source.into(),
                relation,
            },
            _target: PhantomData,
        }
    }

    pub fn filter_kind(self, kind: &'static str) -> KindFilteredNodeListBinding<Target> {
        KindFilteredNodeListBinding {
            binding: self,
            kind,
        }
    }

    pub fn query(&self) -> &NodeListQuery {
        &self.query
    }
}

impl<Target> Path<Target> {
    pub fn target(self) -> TargetBinding<Target> {
        TargetBinding::new(self.source(), self.relations())
    }

    pub fn all(self) -> NodeListBinding<Target> {
        NodeListBinding::resolve_all(
            self.source(),
            self.relations()
                .iter()
                .map(|relation| (*relation).to_owned()),
        )
    }
}

impl<Target> KindFilteredNodeListBinding<Target> {
    pub fn binding(&self) -> &NodeListBinding<Target> {
        &self.binding
    }

    pub const fn kind(&self) -> &'static str {
        self.kind
    }

    pub fn into_parts(self) -> (NodeListBinding<Target>, &'static str) {
        (self.binding, self.kind)
    }
}
