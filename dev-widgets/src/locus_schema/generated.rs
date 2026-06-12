#![allow(dead_code)]

pub mod model {
    use locus_provider::Property;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Unknown;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct AgentSession;

    impl AgentSession {
        pub const CWD: Property<Self, String> = Property::new("cwd");
        pub const ID: Property<Self, String> = Property::new("id");
        pub const MODEL: Property<Self, String> = Property::new("model");
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct AppInstance;

    impl AppInstance {
        pub const ICON: Property<Self, String> = Property::new("icon");
        pub const NAME: Property<Self, String> = Property::new("name");
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Context;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Output;

    impl Output {
        pub const CONNECTOR: Property<Self, String> = Property::new("connector");
        pub const SOURCE: Property<Self, String> = Property::new("source");
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Project;

    impl Project {
        pub const BRANCH: Property<Self, String> = Property::new("branch");
        pub const DISPLAY_ICON: Property<Self, String> = Property::new("display-icon");
        pub const DISPLAY_MAIN: Property<Self, String> = Property::new("display-main");
        pub const DISPLAY_SECONDARY: Property<Self, String> = Property::new("display-secondary");
        pub const ICON: Property<Self, String> = Property::new("icon");
        pub const NAME: Property<Self, String> = Property::new("name");
        pub const NOTEBOOK_PATH: Property<Self, String> = Property::new("notebook_path");
        pub const PATH: Property<Self, String> = Property::new("path");
        pub const SUBPROJ: Property<Self, String> = Property::new("subproj");
        pub const TASK: Property<Self, String> = Property::new("task");
        pub const WORKTREE: Property<Self, String> = Property::new("worktree");
        pub const WORKTREE_PATH: Property<Self, String> = Property::new("worktree-path");
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Window;

    impl Window {
        pub const ID: Property<Self, u32> = Property::new("id");
        pub const SOURCE: Property<Self, String> = Property::new("source");
        pub const TITLE: Property<Self, String> = Property::new("title");
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Workspace;

    impl Workspace {
        pub const ACTIVE: Property<Self, bool> = Property::new("active");
        pub const FOCUSED: Property<Self, bool> = Property::new("focused");
        pub const ID: Property<Self, u32> = Property::new("id");
        pub const INDEX: Property<Self, u32> = Property::new("index");
        pub const NAME: Property<Self, String> = Property::new("name");
        pub const SOURCE: Property<Self, String> = Property::new("source");
        pub const URGENT: Property<Self, bool> = Property::new("urgent");
    }
}

pub mod paths {
    use locus_provider::Path;

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

pub mod relations {
    use locus_provider::Relation;

    use super::model;

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
