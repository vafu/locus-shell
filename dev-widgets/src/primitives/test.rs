use super::window_title::{WINDOW_ROW_CLASSES, WINDOW_ROW_SELECTED_CLASSES, window_title_classes};

use crate::locus_schema::{WindowNodeExt, WorkspacePathExt, model, paths};
use providers::Provider;

#[test]
fn selected_workspace_windows_are_semantic_provider() {
    fn assert_provider<T: Send + 'static, P: Provider<T>>(_provider: P) {}

    let provider = paths::SELECTED_WORKSPACE.windows();

    assert_provider::<Vec<locus_provider::NodeRef<model::Window>>, _>(provider);
}

#[test]
fn window_title_is_local_to_window_node() {
    fn assert_provider<T: Send + 'static, P: Provider<T>>(_provider: P) {}

    let window = locus_provider::node::<model::Window>("window:1");

    assert_provider::<String, _>(window.title());
    assert_provider::<bool, _>(window.is_selected());
}

#[test]
fn window_title_classes_track_selection() {
    assert_eq!(window_title_classes(false), WINDOW_ROW_CLASSES);
    assert_eq!(window_title_classes(true), WINDOW_ROW_SELECTED_CLASSES);
}
