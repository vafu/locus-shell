#![allow(dead_code, unused_imports)]
pub mod model {
    use locus_provider::Property;
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Unknown;
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct AgentSession;
    impl AgentSession {
        pub const CWD: Property<Self, ::std::string::String> = Property::new("cwd");
        pub const ID: Property<Self, ::std::string::String> = Property::new("id");
        pub const MODEL: Property<Self, ::std::string::String> = Property::new("model");
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct AppInstance;
    impl AppInstance {
        pub const ICON: Property<Self, ::std::string::String> = Property::new("icon");
        pub const NAME: Property<Self, ::std::string::String> = Property::new("name");
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Context;
    impl Context {}
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Output;
    impl Output {
        pub const CONNECTOR: Property<Self, ::std::string::String> = Property::new("connector");
        pub const SOURCE: Property<Self, ::std::string::String> = Property::new("source");
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Project;
    impl Project {
        pub const BRANCH: Property<Self, ::std::string::String> = Property::new("branch");
        pub const DISPLAY_ICON: Property<Self, ::std::string::String> =
            Property::new("display-icon");
        pub const DISPLAY_MAIN: Property<Self, ::std::string::String> =
            Property::new("display-main");
        pub const DISPLAY_SECONDARY: Property<Self, ::std::string::String> =
            Property::new("display-secondary");
        pub const ICON: Property<Self, ::std::string::String> = Property::new("icon");
        pub const NAME: Property<Self, ::std::string::String> = Property::new("name");
        pub const NOTEBOOK_PATH: Property<Self, ::std::string::String> =
            Property::new("notebook_path");
        pub const PATH: Property<Self, ::std::string::String> = Property::new("path");
        pub const SUBPROJ: Property<Self, ::std::string::String> = Property::new("subproj");
        pub const TASK: Property<Self, ::std::string::String> = Property::new("task");
        pub const WORKTREE: Property<Self, ::std::string::String> = Property::new("worktree");
        pub const WORKTREE_PATH: Property<Self, ::std::string::String> =
            Property::new("worktree-path");
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Window;
    impl Window {
        pub const APP_ICON: Property<Self, ::std::string::String> = Property::new("app-icon");
        pub const APP_ID: Property<Self, ::std::string::String> = Property::new("app-id");
        pub const CLASS: Property<Self, ::std::string::String> = Property::new("class");
        pub const ICON: Property<Self, ::std::string::String> = Property::new("icon");
        pub const ID: Property<Self, u32> = Property::new("id");
        pub const INSTANCE: Property<Self, ::std::string::String> = Property::new("instance");
        pub const SOURCE: Property<Self, ::std::string::String> = Property::new("source");
        pub const TITLE: Property<Self, ::std::string::String> = Property::new("title");
        pub const URGENT: Property<Self, bool> = Property::new("urgent");
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Workspace;
    impl Workspace {
        pub const ACTIVE: Property<Self, bool> = Property::new("active");
        pub const FOCUSED: Property<Self, bool> = Property::new("focused");
        pub const ID: Property<Self, u32> = Property::new("id");
        pub const INDEX: Property<Self, u32> = Property::new("index");
        pub const NAME: Property<Self, ::std::string::String> = Property::new("name");
        pub const SOURCE: Property<Self, ::std::string::String> = Property::new("source");
        pub const URGENT: Property<Self, bool> = Property::new("urgent");
    }
}
pub mod decode {
    use locus_provider::{DecodeError, NONE_STRING};
    pub fn string(value: &str) -> Result<::std::string::String, DecodeError> {
        if value == NONE_STRING {
            Err(DecodeError::MissingValue)
        } else {
            Ok(value.to_owned())
        }
    }
    pub fn bool(value: &str) -> Result<bool, DecodeError> {
        if value == NONE_STRING {
            return Err(DecodeError::MissingValue);
        }
        match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(DecodeError::Bool {
                value: value.to_owned(),
            }),
        }
    }
    pub fn u32(value: &str) -> Result<u32, DecodeError> {
        if value == NONE_STRING {
            return Err(DecodeError::MissingValue);
        }
        value.parse().map_err(|source| DecodeError::U32 {
            value: value.to_owned(),
            source,
        })
    }
    pub fn i32(value: &str) -> Result<i32, DecodeError> {
        if value == NONE_STRING {
            return Err(DecodeError::MissingValue);
        }
        value.parse().map_err(|source| DecodeError::I32 {
            value: value.to_owned(),
            source,
        })
    }
    pub fn f64(value: &str) -> Result<f64, DecodeError> {
        if value == NONE_STRING {
            return Err(DecodeError::MissingValue);
        }
        value.parse().map_err(|source| DecodeError::F64 {
            value: value.to_owned(),
            source,
        })
    }
}
pub mod source {
    use super::{model, paths};
    use locus_provider::{
        DecodeError, LocusPropertyKey, NONE_STRING, NodeId, NodeListBinding, NodePropertyKey,
        NodeRef, Path, Property, TargetBinding, WatchError,
    };
    use providers::{CancellationToken, Provider, ProviderError};
    use std::{future::pending, marker::PhantomData, pin::Pin};
    use tokio_stream::{Stream, StreamExt};
    pub type DecodeValue<Value> = fn(&str) -> Result<Value, DecodeError>;
    pub type WatchStream<Value> = Pin<Box<dyn Stream<Item = Result<Value, WatchError>> + Send>>;
    pub trait Decode<Value> {
        fn decode(value: &str) -> Result<Value, DecodeError>;
    }
    impl<Target> Decode<::std::string::String> for Target {
        fn decode(value: &str) -> Result<::std::string::String, DecodeError> {
            super::decode::string(value)
        }
    }
    impl<Target> Decode<bool> for Target {
        fn decode(value: &str) -> Result<bool, DecodeError> {
            super::decode::bool(value)
        }
    }
    impl<Target> Decode<u32> for Target {
        fn decode(value: &str) -> Result<u32, DecodeError> {
            super::decode::u32(value)
        }
    }
    impl<Target> Decode<i32> for Target {
        fn decode(value: &str) -> Result<i32, DecodeError> {
            super::decode::i32(value)
        }
    }
    impl<Target> Decode<f64> for Target {
        fn decode(value: &str) -> Result<f64, DecodeError> {
            super::decode::f64(value)
        }
    }
    pub trait PathPropertyExt<Target> {
        fn property<Value>(
            self,
            property: Property<Target, Value>,
        ) -> PropertyBinding<Target, Value>
        where
            Target: Decode<Value>,
            Value: Send + 'static;
    }
    impl<Target> PathPropertyExt<Target> for Path<Target> {
        fn property<Value>(
            self,
            property: Property<Target, Value>,
        ) -> PropertyBinding<Target, Value>
        where
            Target: Decode<Value>,
            Value: Send + 'static,
        {
            PropertyBinding {
                raw: self.raw_property(property),
                property,
                decode: Target::decode,
                _value: PhantomData,
            }
        }
    }
    #[derive(Debug)]
    pub struct PropertyBinding<Target, Value> {
        raw: locus_provider::LocusPropertyBinding<Target>,
        property: Property<Target, Value>,
        decode: DecodeValue<Value>,
        _value: PhantomData<fn() -> Value>,
    }
    impl<Target, Value> Provider<Value> for PropertyBinding<Target, Value>
    where
        Target: Send + 'static,
        Value: Send + 'static,
    {
        type Error = WatchError;
        type Stream = WatchStream<Value>;
        fn stream(self, cancellation: CancellationToken) -> Self::Stream {
            Box::pin(async_stream::stream! {
                let updates = self.raw.stream(cancellation.clone());
                tokio::pin!(updates); loop { let update = tokio::select! { _ =
                cancellation.cancelled() => break, update = updates.next() => update,
                }; match update { Some(Ok(value)) => yield (self.decode) (value
                .as_str()).map_err(Into::into), Some(Err(error)) => yield Err(error),
                None => break, } }
            })
        }
    }
    impl<Target, Value> locus_provider::property_provider::PropertyBinding<Value>
        for PropertyBinding<Target, Value>
    where
        Target: Send + 'static,
        Value: Send + 'static,
    {
        type Target = Target;
        type Key = LocusPropertyKey;
        fn property(&self) -> locus_provider::property_provider::Property<Self::Target, Value> {
            self.property
        }
        fn key(&self) -> Self::Key {
            self.raw.binding_key()
        }
    }
    pub trait NodePropertyExt<Model> {
        fn property<Value>(
            &self,
            property: Property<Model, Value>,
        ) -> NodePropertyBinding<Model, Value>
        where
            Model: Decode<Value>,
            Value: Send + 'static;
    }
    impl<Model> NodePropertyExt<Model> for locus_provider::NodeRef<Model> {
        fn property<Value>(
            &self,
            property: Property<Model, Value>,
        ) -> NodePropertyBinding<Model, Value>
        where
            Model: Decode<Value>,
            Value: Send + 'static,
        {
            NodePropertyBinding {
                raw: self.raw_property(property),
                property,
                decode: Model::decode,
                _value: PhantomData,
            }
        }
    }
    #[derive(Debug)]
    pub struct NodePropertyBinding<Model, Value> {
        raw: locus_provider::NodePropertyBinding<Model>,
        property: Property<Model, Value>,
        decode: DecodeValue<Value>,
        _value: PhantomData<fn() -> Value>,
    }
    impl<Model, Value> Provider<Value> for NodePropertyBinding<Model, Value>
    where
        Model: Send + 'static,
        Value: Send + 'static,
    {
        type Error = WatchError;
        type Stream = WatchStream<Value>;
        fn stream(self, cancellation: CancellationToken) -> Self::Stream {
            Box::pin(async_stream::stream! {
                let updates = self.raw.stream(cancellation.clone());
                tokio::pin!(updates); loop { let update = tokio::select! { _ =
                cancellation.cancelled() => break, update = updates.next() => update,
                }; match update { Some(Ok(value)) => yield (self.decode) (value
                .as_str()).map_err(Into::into), Some(Err(error)) => yield Err(error),
                None => break, } }
            })
        }
    }
    impl<Model, Value> locus_provider::property_provider::PropertyBinding<Value>
        for NodePropertyBinding<Model, Value>
    where
        Model: Send + 'static,
        Value: Send + 'static,
    {
        type Target = Model;
        type Key = NodePropertyKey;
        fn property(&self) -> locus_provider::property_provider::Property<Self::Target, Value> {
            self.property
        }
        fn key(&self) -> Self::Key {
            self.raw.key()
        }
    }
    #[derive(Clone, Debug)]
    pub struct SelectedNodeProvider<Target, P> {
        selected: P,
        node: NodeId,
        _target: PhantomData<fn() -> Target>,
    }
    impl<Target, P> SelectedNodeProvider<Target, P> {
        pub fn new(selected: P, node: NodeId) -> Self {
            Self {
                selected,
                node,
                _target: PhantomData,
            }
        }
    }
    impl<Target, P> Provider<bool> for SelectedNodeProvider<Target, P>
    where
        Target: Send + 'static,
        P: Provider<NodeId>,
    {
        type Error = ProviderError;
        type Stream = Pin<Box<dyn Stream<Item = Result<bool, ProviderError>> + Send>>;
        fn stream(self, cancellation: CancellationToken) -> Self::Stream {
            Box::pin(async_stream::stream! {
                let updates = self.selected.stream(cancellation.clone());
                tokio::pin!(updates); loop { let update = tokio::select! { _ =
                cancellation.cancelled() => break, update = updates.next() => update,
                }; match update { Some(Ok(selected)) => yield Ok(selected == self
                .node), Some(Err(error)) => yield Err(ProviderError::new(error
                .to_string())), None => break, } }
            })
        }
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum CollectionDirection {
        Sources,
        Targets,
    }
    #[derive(Clone, Debug)]
    pub struct RelatedNodesProvider<Owner, Target, P> {
        owner: P,
        relation: &'static str,
        direction: CollectionDirection,
        kind: Option<&'static str>,
        _owner: PhantomData<fn() -> Owner>,
        _target: PhantomData<fn() -> Target>,
    }
    impl<Owner, Target, P> RelatedNodesProvider<Owner, Target, P> {
        fn new(
            owner: P,
            relation: &'static str,
            direction: CollectionDirection,
            kind: Option<&'static str>,
        ) -> Self {
            Self {
                owner,
                relation,
                direction,
                kind,
                _owner: PhantomData,
                _target: PhantomData,
            }
        }
    }
    impl<Owner, Target, P> Provider<Vec<NodeRef<Target>>> for RelatedNodesProvider<Owner, Target, P>
    where
        Owner: Send + 'static,
        Target: Send + 'static,
        P: Provider<NodeId>,
    {
        type Error = ProviderError;
        type Stream =
            Pin<Box<dyn Stream<Item = Result<Vec<NodeRef<Target>>, ProviderError>> + Send>>;
        fn stream(self, cancellation: CancellationToken) -> Self::Stream {
            Box::pin(async_stream::stream! {
                let owners = self.owner.stream(cancellation.clone());
                tokio::pin!(owners); let mut node_cancellation : Option <
                CancellationToken > = None; let mut nodes : Option < Pin < Box < dyn
                Stream < Item = Result < Vec < NodeId >, locus_provider::WatchError
                >> + Send >> > = None; loop { let node_update = async { match nodes
                .as_mut() { Some(nodes) => nodes.next(). await, None => pending().
                await, } }; tokio::select! { _ = cancellation.cancelled() => break,
                owner = owners.next() => { match owner { Some(Ok(owner)) => { if let
                Some(token) = node_cancellation.take() { token.cancel(); } if owner
                == NONE_STRING { nodes = None; yield Ok(Vec::new()); continue; } let
                token = cancellation.child_token(); let binding = match self
                .direction { CollectionDirection::Sources => { NodeListBinding:: <
                Target > ::sources(owner, self.relation) }
                CollectionDirection::Targets => { NodeListBinding:: < Target >
                ::targets(owner, self.relation) } }; let stream : Pin < Box < dyn
                Stream < Item = Result < Vec < NodeId >, locus_provider::WatchError
                >> + Send >> = match self.kind { Some(kind) => Box::pin(binding
                .filter_kind(kind).stream(token.clone())), None => Box::pin(binding
                .stream(token.clone())), }; nodes = Some(stream); node_cancellation =
                Some(token); } Some(Err(error)) => { yield
                Err(ProviderError::new(error.to_string())); } None => break, } }
                node_ids = node_update => { match node_ids { Some(Ok(nodes)) => {
                yield Ok(nodes.into_iter().map(NodeRef:: < Target > ::new).collect())
                } Some(Err(error)) => yield Err(ProviderError::new(error
                .to_string())), None => { node_cancellation = None; nodes = None; } }
                } } } if let Some(token) = node_cancellation { token.cancel(); }
            })
        }
    }
    pub trait AgentSessionNodeExt {
        fn cwd(&self) -> NodePropertyBinding<model::AgentSession, ::std::string::String>;
        fn model(&self) -> NodePropertyBinding<model::AgentSession, ::std::string::String>;
        fn is_selected(
            &self,
        ) -> SelectedNodeProvider<model::AgentSession, TargetBinding<model::AgentSession>>;
    }
    impl AgentSessionNodeExt for locus_provider::NodeRef<model::AgentSession> {
        fn cwd(&self) -> NodePropertyBinding<model::AgentSession, ::std::string::String> {
            self.property(model::AgentSession::CWD)
        }
        fn model(&self) -> NodePropertyBinding<model::AgentSession, ::std::string::String> {
            self.property(model::AgentSession::MODEL)
        }
        fn is_selected(
            &self,
        ) -> SelectedNodeProvider<model::AgentSession, TargetBinding<model::AgentSession>> {
            SelectedNodeProvider::new(paths::SELECTED_AGENT_SESSION.target(), self.id().to_owned())
        }
    }
    pub trait AppInstanceNodeExt {
        fn icon(&self) -> NodePropertyBinding<model::AppInstance, ::std::string::String>;
        fn name(&self) -> NodePropertyBinding<model::AppInstance, ::std::string::String>;
    }
    impl AppInstanceNodeExt for locus_provider::NodeRef<model::AppInstance> {
        fn icon(&self) -> NodePropertyBinding<model::AppInstance, ::std::string::String> {
            self.property(model::AppInstance::ICON)
        }
        fn name(&self) -> NodePropertyBinding<model::AppInstance, ::std::string::String> {
            self.property(model::AppInstance::NAME)
        }
    }
    pub trait ContextNodeExt {}
    impl ContextNodeExt for locus_provider::NodeRef<model::Context> {}
    pub trait OutputNodeExt {
        fn connector(&self) -> NodePropertyBinding<model::Output, ::std::string::String>;
        fn source(&self) -> NodePropertyBinding<model::Output, ::std::string::String>;
        fn is_selected(&self) -> SelectedNodeProvider<model::Output, TargetBinding<model::Output>>;
    }
    impl OutputNodeExt for locus_provider::NodeRef<model::Output> {
        fn connector(&self) -> NodePropertyBinding<model::Output, ::std::string::String> {
            self.property(model::Output::CONNECTOR)
        }
        fn source(&self) -> NodePropertyBinding<model::Output, ::std::string::String> {
            self.property(model::Output::SOURCE)
        }
        fn is_selected(&self) -> SelectedNodeProvider<model::Output, TargetBinding<model::Output>> {
            SelectedNodeProvider::new(paths::SELECTED_OUTPUT.target(), self.id().to_owned())
        }
    }
    pub trait ProjectNodeExt {
        fn branch(&self) -> NodePropertyBinding<model::Project, ::std::string::String>;
        fn display_icon(&self) -> NodePropertyBinding<model::Project, ::std::string::String>;
        fn display_main(&self) -> NodePropertyBinding<model::Project, ::std::string::String>;
        fn display_secondary(&self) -> NodePropertyBinding<model::Project, ::std::string::String>;
        fn icon(&self) -> NodePropertyBinding<model::Project, ::std::string::String>;
        fn name(&self) -> NodePropertyBinding<model::Project, ::std::string::String>;
        fn notebook_path(&self) -> NodePropertyBinding<model::Project, ::std::string::String>;
        fn path(&self) -> NodePropertyBinding<model::Project, ::std::string::String>;
        fn subproj(&self) -> NodePropertyBinding<model::Project, ::std::string::String>;
        fn task(&self) -> NodePropertyBinding<model::Project, ::std::string::String>;
        fn worktree(&self) -> NodePropertyBinding<model::Project, ::std::string::String>;
        fn worktree_path(&self) -> NodePropertyBinding<model::Project, ::std::string::String>;
        fn is_selected(
            &self,
        ) -> SelectedNodeProvider<model::Project, TargetBinding<model::Project>>;
    }
    impl ProjectNodeExt for locus_provider::NodeRef<model::Project> {
        fn branch(&self) -> NodePropertyBinding<model::Project, ::std::string::String> {
            self.property(model::Project::BRANCH)
        }
        fn display_icon(&self) -> NodePropertyBinding<model::Project, ::std::string::String> {
            self.property(model::Project::DISPLAY_ICON)
        }
        fn display_main(&self) -> NodePropertyBinding<model::Project, ::std::string::String> {
            self.property(model::Project::DISPLAY_MAIN)
        }
        fn display_secondary(&self) -> NodePropertyBinding<model::Project, ::std::string::String> {
            self.property(model::Project::DISPLAY_SECONDARY)
        }
        fn icon(&self) -> NodePropertyBinding<model::Project, ::std::string::String> {
            self.property(model::Project::ICON)
        }
        fn name(&self) -> NodePropertyBinding<model::Project, ::std::string::String> {
            self.property(model::Project::NAME)
        }
        fn notebook_path(&self) -> NodePropertyBinding<model::Project, ::std::string::String> {
            self.property(model::Project::NOTEBOOK_PATH)
        }
        fn path(&self) -> NodePropertyBinding<model::Project, ::std::string::String> {
            self.property(model::Project::PATH)
        }
        fn subproj(&self) -> NodePropertyBinding<model::Project, ::std::string::String> {
            self.property(model::Project::SUBPROJ)
        }
        fn task(&self) -> NodePropertyBinding<model::Project, ::std::string::String> {
            self.property(model::Project::TASK)
        }
        fn worktree(&self) -> NodePropertyBinding<model::Project, ::std::string::String> {
            self.property(model::Project::WORKTREE)
        }
        fn worktree_path(&self) -> NodePropertyBinding<model::Project, ::std::string::String> {
            self.property(model::Project::WORKTREE_PATH)
        }
        fn is_selected(
            &self,
        ) -> SelectedNodeProvider<model::Project, TargetBinding<model::Project>> {
            SelectedNodeProvider::new(paths::SELECTED_PROJECT.target(), self.id().to_owned())
        }
    }
    pub trait WindowNodeExt {
        fn app_icon(&self) -> NodePropertyBinding<model::Window, ::std::string::String>;
        fn app_id(&self) -> NodePropertyBinding<model::Window, ::std::string::String>;
        fn class(&self) -> NodePropertyBinding<model::Window, ::std::string::String>;
        fn icon(&self) -> NodePropertyBinding<model::Window, ::std::string::String>;
        fn instance(&self) -> NodePropertyBinding<model::Window, ::std::string::String>;
        fn source(&self) -> NodePropertyBinding<model::Window, ::std::string::String>;
        fn title(&self) -> NodePropertyBinding<model::Window, ::std::string::String>;
        fn urgent(&self) -> NodePropertyBinding<model::Window, bool>;
        fn is_selected(&self) -> SelectedNodeProvider<model::Window, TargetBinding<model::Window>>;
    }
    impl WindowNodeExt for locus_provider::NodeRef<model::Window> {
        fn app_icon(&self) -> NodePropertyBinding<model::Window, ::std::string::String> {
            self.property(model::Window::APP_ICON)
        }
        fn app_id(&self) -> NodePropertyBinding<model::Window, ::std::string::String> {
            self.property(model::Window::APP_ID)
        }
        fn class(&self) -> NodePropertyBinding<model::Window, ::std::string::String> {
            self.property(model::Window::CLASS)
        }
        fn icon(&self) -> NodePropertyBinding<model::Window, ::std::string::String> {
            self.property(model::Window::ICON)
        }
        fn instance(&self) -> NodePropertyBinding<model::Window, ::std::string::String> {
            self.property(model::Window::INSTANCE)
        }
        fn source(&self) -> NodePropertyBinding<model::Window, ::std::string::String> {
            self.property(model::Window::SOURCE)
        }
        fn title(&self) -> NodePropertyBinding<model::Window, ::std::string::String> {
            self.property(model::Window::TITLE)
        }
        fn urgent(&self) -> NodePropertyBinding<model::Window, bool> {
            self.property(model::Window::URGENT)
        }
        fn is_selected(&self) -> SelectedNodeProvider<model::Window, TargetBinding<model::Window>> {
            SelectedNodeProvider::new(paths::SELECTED_WINDOW.target(), self.id().to_owned())
        }
    }
    pub trait WorkspaceNodeExt {
        fn active(&self) -> NodePropertyBinding<model::Workspace, bool>;
        fn focused(&self) -> NodePropertyBinding<model::Workspace, bool>;
        fn index(&self) -> NodePropertyBinding<model::Workspace, u32>;
        fn name(&self) -> NodePropertyBinding<model::Workspace, ::std::string::String>;
        fn source(&self) -> NodePropertyBinding<model::Workspace, ::std::string::String>;
        fn urgent(&self) -> NodePropertyBinding<model::Workspace, bool>;
        fn is_selected(
            &self,
        ) -> SelectedNodeProvider<model::Workspace, TargetBinding<model::Workspace>>;
    }
    impl WorkspaceNodeExt for locus_provider::NodeRef<model::Workspace> {
        fn active(&self) -> NodePropertyBinding<model::Workspace, bool> {
            self.property(model::Workspace::ACTIVE)
        }
        fn focused(&self) -> NodePropertyBinding<model::Workspace, bool> {
            self.property(model::Workspace::FOCUSED)
        }
        fn index(&self) -> NodePropertyBinding<model::Workspace, u32> {
            self.property(model::Workspace::INDEX)
        }
        fn name(&self) -> NodePropertyBinding<model::Workspace, ::std::string::String> {
            self.property(model::Workspace::NAME)
        }
        fn source(&self) -> NodePropertyBinding<model::Workspace, ::std::string::String> {
            self.property(model::Workspace::SOURCE)
        }
        fn urgent(&self) -> NodePropertyBinding<model::Workspace, bool> {
            self.property(model::Workspace::URGENT)
        }
        fn is_selected(
            &self,
        ) -> SelectedNodeProvider<model::Workspace, TargetBinding<model::Workspace>> {
            SelectedNodeProvider::new(paths::SELECTED_WORKSPACE.target(), self.id().to_owned())
        }
    }
    pub trait OutputPathExt {
        fn workspaces(
            self,
        ) -> RelatedNodesProvider<model::Output, model::Workspace, TargetBinding<model::Output>>;
    }
    impl OutputPathExt for Path<model::Output> {
        fn workspaces(
            self,
        ) -> RelatedNodesProvider<model::Output, model::Workspace, TargetBinding<model::Output>>
        {
            RelatedNodesProvider::new(
                self.target(),
                "output",
                CollectionDirection::Sources,
                Some("workspace"),
            )
        }
    }
    pub trait WorkspacePathExt {
        fn projects(
            self,
        ) -> RelatedNodesProvider<model::Workspace, model::Project, TargetBinding<model::Workspace>>;
        fn windows(
            self,
        ) -> RelatedNodesProvider<model::Workspace, model::Window, TargetBinding<model::Workspace>>;
    }
    impl WorkspacePathExt for Path<model::Workspace> {
        fn projects(
            self,
        ) -> RelatedNodesProvider<model::Workspace, model::Project, TargetBinding<model::Workspace>>
        {
            RelatedNodesProvider::new(
                self.target(),
                "project",
                CollectionDirection::Targets,
                Some("project"),
            )
        }
        fn windows(
            self,
        ) -> RelatedNodesProvider<model::Workspace, model::Window, TargetBinding<model::Workspace>>
        {
            RelatedNodesProvider::new(
                self.target(),
                "workspace",
                CollectionDirection::Sources,
                Some("window"),
            )
        }
    }
}
pub use source::{NodePropertyExt, PathPropertyExt};
pub mod relations {
    use super::model;
    use locus_provider::Relation;
    pub const AGENT_SESSION: Relation<model::AppInstance, model::AgentSession> =
        Relation::new("agent-session");
    pub const APP_INSTANCE: Relation<model::Window, model::AppInstance> =
        Relation::new("app-instance");
    pub const OUTPUT: Relation<model::Workspace, model::Output> = Relation::new("output");
    pub const PROJECT: Relation<model::Workspace, model::Project> = Relation::new("project");
    pub const SESSION_PROJECT: Relation<model::AgentSession, model::Project> =
        Relation::new("session-project");
    pub const SUBAGENT_SESSION: Relation<model::AgentSession, model::AgentSession> =
        Relation::new("subagent-session");
    pub const WINDOW: Relation<model::Unknown, model::Window> = Relation::new("window");
    pub const WORKSPACE: Relation<model::Unknown, model::Workspace> = Relation::new("workspace");
}
pub mod paths {
    use super::model;
    use locus_provider::Path;
    pub const AGENT_SESSION_PROJECT: Path<model::Project> = Path::new(
        "agent-session-project",
        "agent-session",
        &["session-project"],
        false,
    );
    pub const AGENT_SESSION_WORKSPACE: Path<model::Workspace> = Path::new(
        "agent-session-workspace",
        "agent-session",
        &["agent-session", "app-instance", "workspace"],
        false,
    );
    pub const AGENT_SESSION_WORKSPACE_PROJECT: Path<model::Project> = Path::new(
        "agent-session-workspace-project",
        "agent-session",
        &["agent-session", "app-instance", "workspace", "project"],
        false,
    );
    pub const SELECTED_AGENT_SESSION: Path<model::AgentSession> = Path::new(
        "selected-agent-session",
        "context:selected",
        &["window", "app-instance", "agent-session"],
        false,
    );
    pub const SELECTED_OUTPUT: Path<model::Output> = Path::new(
        "selected-output",
        "context:selected",
        &["workspace", "output"],
        false,
    );
    pub const SELECTED_PROJECT: Path<model::Project> = Path::new(
        "selected-project",
        "context:selected",
        &["workspace", "project"],
        false,
    );
    pub const SELECTED_WINDOW: Path<model::Window> =
        Path::new("selected-window", "context:selected", &["window"], false);
    pub const SELECTED_WORKSPACE: Path<model::Workspace> = Path::new(
        "selected-workspace",
        "context:selected",
        &["workspace"],
        false,
    );
    pub const WINDOW_AGENT_SESSION: Path<model::AgentSession> = Path::new(
        "window-agent-session",
        "window",
        &["app-instance", "agent-session"],
        false,
    );
}
