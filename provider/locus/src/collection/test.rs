use futures::StreamExt;
use providers::{CancellationToken, Provider};

use crate::{
    NodeListBinding, NodeListDiffCommand, Path, Relation, TargetBinding,
    collection::diff::apply_commands,
};

mod schema {
    use super::{Path, Relation};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Unknown;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Workspace;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Window;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct AppInstance;

    pub const SELECTED_WORKSPACE: Path<Workspace> = Path::new(
        "selected-workspace",
        "context:selected",
        &["workspace"],
        false,
    );
    pub const WORKSPACE: Relation<Unknown, Workspace> = Relation::new("workspace");
    pub const APP_INSTANCE: Relation<Window, AppInstance> = Relation::new("app-instance");
}

#[test]
fn path_target_creates_typed_binding() {
    let binding: TargetBinding<schema::Workspace> = schema::SELECTED_WORKSPACE.target();

    assert_eq!(binding.source(), "context:selected");
    assert_eq!(binding.relations(), &["workspace"]);
}

#[test]
fn path_all_creates_resolve_all_binding() {
    let binding: NodeListBinding<schema::Workspace> = schema::SELECTED_WORKSPACE.all();

    assert_eq!(
        binding.query(),
        &super::binding::NodeListQuery::ResolveAll {
            source: "context:selected".to_owned(),
            relations: vec!["workspace".to_owned()],
        }
    );
}

#[test]
fn relation_sources_creates_source_list_binding() {
    let binding = schema::WORKSPACE.sources("workspace:1");

    assert_eq!(
        binding.query(),
        &super::binding::NodeListQuery::Sources {
            target: "workspace:1".to_owned(),
            relation: "workspace",
        }
    );
}

#[test]
fn relation_targets_creates_target_list_binding() {
    let binding = schema::APP_INSTANCE.targets("window:1");

    assert_eq!(
        binding.query(),
        &super::binding::NodeListQuery::Targets {
            source: "window:1".to_owned(),
            relation: "app-instance",
        }
    );
}

#[test]
fn target_binding_is_provider() {
    fn assert_provider<T: Send + 'static, P: providers::Provider<T>>(_provider: P) {}

    assert_provider::<String, _>(schema::SELECTED_WORKSPACE.target());
}

#[test]
fn node_list_binding_is_provider() {
    fn assert_provider<T: Send + 'static, P: providers::Provider<T>>(_provider: P) {}

    assert_provider::<Vec<String>, _>(schema::WORKSPACE.sources("workspace:1"));
}

#[test]
fn kind_filtered_node_list_binding_is_provider() {
    fn assert_provider<T: Send + 'static, P: providers::Provider<T>>(_provider: P) {}

    assert_provider::<Vec<String>, _>(
        schema::WORKSPACE
            .sources("workspace:1")
            .filter_kind("window"),
    );
}

#[test]
fn node_list_commands_materialize_nodes() {
    let mut nodes = Vec::new();

    apply_commands(
        &mut nodes,
        vec![
            NodeListDiffCommand::Reset {
                nodes: vec!["a".to_owned(), "c".to_owned()],
            },
            NodeListDiffCommand::NodeAdded {
                node: "b".to_owned(),
                index: 1,
            },
            NodeListDiffCommand::NodeRemoved {
                node: "c".to_owned(),
                index: 2,
            },
        ],
    )
    .unwrap();

    assert_eq!(nodes, ["a".to_owned(), "b".to_owned()]);
}

#[test]
fn node_list_commands_reject_unknown_command() {
    let error =
        NodeListDiffCommand::from_tuple(("moved".to_owned(), "a".to_owned(), 0, Vec::new()))
            .unwrap_err();

    assert_eq!(
        error.to_string(),
        "unknown Locus node-list diff command: \"moved\""
    );
}

#[test]
fn cancelled_target_provider_exits_before_dbus_setup() {
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let mut stream = schema::SELECTED_WORKSPACE.target().stream(cancellation);

    let result = futures::executor::block_on(stream.next());

    assert!(result.is_none());
}

#[test]
fn cancelled_node_list_provider_exits_before_dbus_setup() {
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let mut stream = schema::WORKSPACE
        .sources("workspace:1")
        .stream(cancellation);

    let result = futures::executor::block_on(stream.next());

    assert!(result.is_none());
}

#[test]
fn cancelled_kind_filtered_node_list_provider_exits_before_dbus_setup() {
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let mut stream = schema::WORKSPACE
        .sources("workspace:1")
        .filter_kind("window")
        .stream(cancellation);

    let result = futures::executor::block_on(stream.next());

    assert!(result.is_none());
}
