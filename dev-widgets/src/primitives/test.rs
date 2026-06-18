use super::window_title::{WINDOW_ROW_CLASSES, WINDOW_ROW_SELECTED_CLASSES, window_title_classes};

use crate::locus;
use shell_core::source::IntoObservable;

#[test]
fn selected_workspace_windows_are_semantic_source() {
    fn assert_source<T: Send + 'static, P: IntoObservable<T>>(_source: P) {}

    let source = locus::selected_workspace_windows();

    assert_source::<Vec<String>, _>(source);
}

#[test]
fn window_title_is_local_to_window_node() {
    fn assert_source<T: Send + 'static, P: IntoObservable<T>>(_source: P) {}

    let window = "window:1".to_owned();

    assert_source::<String, _>(locus::window_title(window.clone()));
    assert_source::<bool, _>(locus::window_is_selected(window));
}

#[test]
fn window_title_classes_track_selection() {
    assert_eq!(window_title_classes(false), WINDOW_ROW_CLASSES);
    assert_eq!(window_title_classes(true), WINDOW_ROW_SELECTED_CLASSES);
}
