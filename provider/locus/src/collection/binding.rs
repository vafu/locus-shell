use std::marker::PhantomData;

use crate::{binding::Path, model};

pub type NodeId = String;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Relation<Source, Target> {
    pub name: &'static str,
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetBinding<Target> {
    pub source: &'static str,
    pub relations: &'static [&'static str],
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeListBinding<Target> {
    pub query: NodeListQuery,
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
}

impl<Target> Path<Target> {
    pub fn target(self) -> TargetBinding<Target> {
        TargetBinding::new(self.source, self.relations)
    }

    pub fn all(self) -> NodeListBinding<Target> {
        NodeListBinding::resolve_all(
            self.source,
            self.relations.iter().map(|relation| (*relation).to_owned()),
        )
    }
}

pub mod relations {
    use super::{Relation, model};

    pub const WINDOW: Relation<model::Context, model::Window> = Relation::new("window");
    pub const WORKSPACE: Relation<model::Unknown, model::Workspace> = Relation::new("workspace");
    pub const OUTPUT: Relation<model::Workspace, model::Output> = Relation::new("output");
    pub const SESSION_PROJECT: Relation<model::AgentSession, model::Project> =
        Relation::new("session-project");
    pub const AGENT_SESSION: Relation<model::AppInstance, model::AgentSession> =
        Relation::new("agent-session");
    pub const APP_INSTANCE: Relation<model::Window, model::AppInstance> =
        Relation::new("app-instance");
}
