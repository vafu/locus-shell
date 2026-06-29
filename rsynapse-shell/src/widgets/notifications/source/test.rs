use shell_core::locus_path::LocusPath;

use super::{
    NotificationCenterRowVm, NotificationUrgency, NotificationVm, PopupActivationEvent,
    PopupActivationState, action_children, grouping, notification_children, parse_urgency,
};

#[test]
fn notification_children_filter_command_files() {
    assert_eq!(
        notification_children(vec![
            LocusPath::new("/notifyd/notifications/10"),
            LocusPath::new("/notifyd/notifications/Close.call"),
            LocusPath::new("/notifyd/notifications/@internal"),
        ]),
        vec![LocusPath::new("/notifyd/notifications/10")]
    );
}

#[test]
fn action_children_filter_command_files() {
    assert_eq!(
        action_children(vec![
            LocusPath::new("/notifyd/notifications/10/actions/default"),
            LocusPath::new("/notifyd/notifications/10/actions/invoke.call"),
        ]),
        vec![LocusPath::new("/notifyd/notifications/10/actions/default")]
    );
}

#[test]
fn notification_payload_children_detect_ready_records() {
    assert!(super::notification_has_payload_children(&[LocusPath::new(
        "/notifyd/notifications/10/summary"
    ),]));
    assert!(!super::notification_has_payload_children(&[
        LocusPath::new("/notifyd/notifications/10/actions"),
        LocusPath::new("/notifyd/notifications/10/discard"),
    ]));
}

#[test]
fn parses_freedesktop_urgency_values() {
    assert_eq!(parse_urgency("0"), NotificationUrgency::Low);
    assert_eq!(parse_urgency("low"), NotificationUrgency::Low);
    assert_eq!(parse_urgency("2"), NotificationUrgency::Critical);
    assert_eq!(parse_urgency("critical"), NotificationUrgency::Critical);
    assert_eq!(parse_urgency("1"), NotificationUrgency::Normal);
}

#[test]
fn notification_sorting_uses_latest_timestamp_first() {
    let notifications = grouping::sort_notifications(vec![
        notification_at("1", "Build", 1000, 1000),
        notification_at("2", "Chat", 900, 4000),
    ]);

    assert_eq!(notifications[0].id, "2");
    assert_eq!(notifications[1].id, "1");
}

#[test]
fn center_rows_group_by_application_and_sort_groups_by_latest_time() {
    let rows = grouping::grouped_rows(vec![
        notification_at("1", "Mail", 1000, 1000),
        notification_at("2", "Chat", 2000, 2000),
        notification_at("3", "Mail", 3000, 3000),
    ]);

    assert!(matches!(
        &rows[0],
        NotificationCenterRowVm::Header(header) if header.app_name == "Mail" && header.count == 2
    ));
    assert!(matches!(
        &rows[3],
        NotificationCenterRowVm::Header(header) if header.app_name == "Chat" && header.count == 1
    ));
}

#[test]
fn popup_snapshot_seeds_baseline_without_showing_existing_notifications() {
    let state = super::update_popup_activation_state(
        PopupActivationState::default(),
        popup_event(
            shell_core::source::ChildrenEvent::Snapshot(vec![notification_path("1")]),
            1000,
        ),
    );

    let visible = super::popup_visible_notifications(vec![notification("1", "Mail")], &state, 1000);

    assert!(visible.is_empty());
}

#[test]
fn popup_later_snapshot_activates_new_notification_ids() {
    let state = super::update_popup_activation_state(
        PopupActivationState::default(),
        popup_event(
            shell_core::source::ChildrenEvent::Snapshot(vec![notification_path("1")]),
            1000,
        ),
    );
    let state = super::update_popup_activation_state(
        state,
        popup_event(
            shell_core::source::ChildrenEvent::Snapshot(vec![
                notification_path("1"),
                notification_path("2"),
            ]),
            2000,
        ),
    );

    let visible = super::popup_visible_notifications(
        vec![notification("1", "Mail"), notification("2", "Chat")],
        &state,
        2000,
    );

    assert_eq!(visible, vec![notification("2", "Chat")]);
}

#[test]
fn popup_added_notification_is_visible_until_timeout() {
    let state = super::update_popup_activation_state(
        PopupActivationState::default(),
        popup_event(
            shell_core::source::ChildrenEvent::Added(notification_path("1")),
            1000,
        ),
    );
    let mut record = notification("1", "Mail");
    record.expire_timeout_ms = 5000;

    let visible = super::popup_visible_notifications(vec![record.clone()], &state, 5999);
    assert_eq!(visible, vec![record.clone()]);

    let visible = super::popup_visible_notifications(vec![record], &state, 6000);
    assert!(visible.is_empty());
}

#[test]
fn popup_zero_timeout_stays_visible_until_removed() {
    let mut state = super::update_popup_activation_state(
        PopupActivationState::default(),
        popup_event(
            shell_core::source::ChildrenEvent::Added(notification_path("1")),
            1000,
        ),
    );
    let mut record = notification("1", "Mail");
    record.expire_timeout_ms = 0;

    let visible = super::popup_visible_notifications(vec![record], &state, 120_000);
    assert_eq!(visible.len(), 1);

    state = super::update_popup_activation_state(
        state,
        popup_event(
            shell_core::source::ChildrenEvent::Removed(notification_path("1")),
            120_000,
        ),
    );
    let visible =
        super::popup_visible_notifications(vec![notification("1", "Mail")], &state, 120_000);
    assert!(visible.is_empty());
}

#[test]
fn popup_added_command_file_is_ignored() {
    let state = super::update_popup_activation_state(
        PopupActivationState::default(),
        popup_event(
            shell_core::source::ChildrenEvent::Added(LocusPath::new(
                "/notifyd/notifications/Close.call",
            )),
            1000,
        ),
    );

    assert!(state.known_ids.is_empty());
    assert!(state.shown_at_by_id.is_empty());
}

fn popup_event(kind: shell_core::source::ChildrenEvent, now_unix_ms: u64) -> PopupActivationEvent {
    PopupActivationEvent { kind, now_unix_ms }
}

fn notification_path(id: &str) -> LocusPath {
    LocusPath::new(format!("/notifyd/notifications/{id}"))
}

fn notification(id: &str, app_name: &str) -> NotificationVm {
    notification_at(id, app_name, 1000, 1000)
}

fn notification_at(id: &str, app_name: &str, created_at: u64, updated_at: u64) -> NotificationVm {
    NotificationVm {
        path: notification_path(id),
        id: id.to_owned(),
        created_at_unix_ms: created_at,
        updated_at_unix_ms: updated_at,
        app_name: app_name.to_owned(),
        summary: "Summary".to_owned(),
        body: "Body".to_owned(),
        icon_name: String::new(),
        image_path: String::new(),
        urgency: NotificationUrgency::Normal,
        expire_timeout_ms: 5000,
        actions: Vec::new(),
    }
}
