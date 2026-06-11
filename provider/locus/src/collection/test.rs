use std::sync::{Arc, Mutex};

use providers::{CancellationToken, Provider, ProviderContext, ProviderSender};

use crate::{
    NodeListBinding, NodeListDiffCommand, TargetBinding, collection::diff::apply_commands, model,
    paths, relations,
};

#[test]
fn generated_path_target_creates_typed_binding() {
    let binding: TargetBinding<model::Workspace> = paths::SELECTED_WORKSPACE.target();

    assert_eq!(binding.source, "context:selected");
    assert_eq!(binding.relations, &["workspace"]);
}

#[test]
fn generated_path_all_creates_resolve_all_binding() {
    let binding: NodeListBinding<model::Workspace> = paths::SELECTED_WORKSPACE.all();

    assert_eq!(
        binding.query,
        super::binding::NodeListQuery::ResolveAll {
            source: "context:selected".to_owned(),
            relations: vec!["workspace".to_owned()],
        }
    );
}

#[test]
fn relation_sources_creates_source_list_binding() {
    let binding = relations::WORKSPACE.sources("workspace:1");

    assert_eq!(
        binding.query,
        super::binding::NodeListQuery::Sources {
            target: "workspace:1".to_owned(),
            relation: "workspace",
        }
    );
}

#[test]
fn relation_targets_creates_target_list_binding() {
    let binding = relations::APP_INSTANCE.targets("window:1");

    assert_eq!(
        binding.query,
        super::binding::NodeListQuery::Targets {
            source: "window:1".to_owned(),
            relation: "app-instance",
        }
    );
}

#[test]
fn target_binding_is_provider() {
    fn assert_provider<T: Send + 'static, P: providers::Provider<T>>(_provider: P) {}

    assert_provider::<String, _>(paths::SELECTED_WORKSPACE.target());
}

#[test]
fn node_list_binding_is_provider() {
    fn assert_provider<T: Send + 'static, P: providers::Provider<T>>(_provider: P) {}

    assert_provider::<Vec<String>, _>(relations::WORKSPACE.sources("workspace:1"));
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
    let sent = Arc::new(Mutex::new(Vec::new()));
    let captured = sent.clone();

    let result = futures::executor::block_on(paths::SELECTED_WORKSPACE.target().run(
        ProviderContext::new(cancellation),
        ProviderSender::new(move |value| {
            captured.lock().expect("sent lock").push(value);
        }),
    ));

    assert!(result.is_ok());
    assert!(sent.lock().expect("sent lock").is_empty());
}

#[test]
fn cancelled_node_list_provider_exits_before_dbus_setup() {
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let captured = sent.clone();

    let result = futures::executor::block_on(relations::WORKSPACE.sources("workspace:1").run(
        ProviderContext::new(cancellation),
        ProviderSender::new(move |value| {
            captured.lock().expect("sent lock").push(value);
        }),
    ));

    assert!(result.is_ok());
    assert!(sent.lock().expect("sent lock").is_empty());
}
