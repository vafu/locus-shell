pub mod binding {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Property<Model, Value> {
        pub key: &'static str,
        _model: ::std::marker::PhantomData<fn() -> Model>,
        _value: ::std::marker::PhantomData<fn() -> Value>,
    }
    impl<Model, Value> Property<Model, Value> {
        pub const fn new(key: &'static str) -> Self {
            Self {
                key,
                _model: ::std::marker::PhantomData,
                _value: ::std::marker::PhantomData,
            }
        }
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Path<Target> {
        pub name: &'static str,
        pub source: &'static str,
        pub relations: &'static [&'static str],
        pub many: bool,
        _target: ::std::marker::PhantomData<fn() -> Target>,
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
                _target: ::std::marker::PhantomData,
            }
        }
        pub const fn property<Value>(
            self,
            property: Property<Target, Value>,
        ) -> FieldBinding<Value> {
            FieldBinding {
                source: self.source,
                relations: self.relations,
                property: property.key,
                _value: ::std::marker::PhantomData,
            }
        }
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct FieldBinding<Value> {
        pub source: &'static str,
        pub relations: &'static [&'static str],
        pub property: &'static str,
        _value: ::std::marker::PhantomData<fn() -> Value>,
    }
}
pub mod model {
    use super::binding::Property;
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
        pub const ID: Property<Self, u32> = Property::new("id");
        pub const SOURCE: Property<Self, ::std::string::String> = Property::new("source");
        pub const TITLE: Property<Self, ::std::string::String> = Property::new("title");
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
pub mod paths {
    use super::binding::Path;
    use super::model;
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
