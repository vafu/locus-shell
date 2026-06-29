#[cfg(test)]
mod test;

mod grouping;

use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use shell_core::{
    locus_path::LocusPath,
    source::{
        self, Observable,
        rx::{Observable as _, ObservableFactory as _, Shared},
    },
};
use shell_rx_macros::combine_latest;

use self::grouping::{grouped_rows, sort_notifications};

const POPUP_TICK: Duration = Duration::from_secs(1);
const DEFAULT_POPUP_TIMEOUT: Duration = Duration::from_secs(7);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum NotificationUrgency {
    Low,
    #[default]
    Normal,
    Critical,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NotificationActionVm {
    pub(crate) path: LocusPath,
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) is_default: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NotificationVm {
    pub(crate) path: LocusPath,
    pub(crate) id: String,
    pub(crate) created_at_unix_ms: u64,
    pub(crate) updated_at_unix_ms: u64,
    pub(crate) app_name: String,
    pub(crate) summary: String,
    pub(crate) body: String,
    pub(crate) icon_name: String,
    pub(crate) image_path: String,
    pub(crate) urgency: NotificationUrgency,
    pub(crate) expire_timeout_ms: i32,
    pub(crate) actions: Vec<NotificationActionVm>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NotificationGroupHeaderVm {
    pub(crate) app_name: String,
    pub(crate) count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NotificationCenterRowVm {
    Header(NotificationGroupHeaderVm),
    Notification(NotificationVm),
}

pub(crate) fn popup_notifications() -> Observable<Vec<NotificationVm>> {
    source::shared_by_key("rsynapse.notifyd.popups", "visible", || {
        combine_latest!(
            notification_collection("popups"),
            popup_activation_state(),
            popup_clock() => |(notifications, activation, now_unix_ms)| {
                popup_visible_notifications(notifications, &activation, now_unix_ms)
            },
        )
        .distinct_until_changed()
        .box_it()
    })
}

pub(crate) fn notification_center_rows() -> Observable<Vec<NotificationCenterRowVm>> {
    source::shared_by_key("rsynapse.notifyd.center", "rows", || {
        notification_collection("center")
            .map(grouped_rows)
            .distinct_until_changed()
            .box_it()
    })
}

pub(crate) fn has_notification_items() -> Observable<bool> {
    source::shared_by_key("rsynapse.notifyd.center", "has-items", || {
        notification_count()
            .map(|count| count > 0)
            .distinct_until_changed()
            .box_it()
    })
}

fn notification_collection(key: &'static str) -> Observable<Vec<NotificationVm>> {
    source::shared_by_key("rsynapse.notifyd.notifications", key, move || {
        notifyd_collection_path()
            .as_children()
            .map(notification_children)
            .switch_map(move |notifications| {
                source::combine_latest_vec(
                    notifications.into_iter().map(notification_entry).collect(),
                )
            })
            .map(ready_notifications)
            .map(sort_notifications)
            .distinct_until_changed()
            .box_it()
    })
}

fn notification_count() -> Observable<u32> {
    source::shared_by_key("rsynapse.notifyd.state", "notification-count", || {
        notifyd_state_path()
            .observe_prop_or::<u32>("notification-count", 0)
            .distinct_until_changed()
            .box_it()
    })
}

fn notifyd_state_path() -> LocusPath {
    source::root().child("notifyd/state")
}

fn notifyd_collection_path() -> LocusPath {
    source::root().child("notifyd/notifications")
}

fn notification_entry(path: LocusPath) -> Observable<Option<NotificationVm>> {
    path.as_children()
        .switch_map(move |children| {
            if notification_has_payload_children(&children) {
                notification(path.clone()).map(Some).box_it()
            } else {
                source::once(None)
            }
        })
        .map(|notification| notification)
        .distinct_until_changed()
        .box_it()
}

fn notification(path: LocusPath) -> Observable<NotificationVm> {
    let id_from_path = path_id(&path);
    let path_for_vm = path.clone();
    let actions = notification_actions(path.child("actions"));

    combine_latest!(
        path.observe_prop_or::<u64>("created-at-unix-ms", 0),
        path.observe_prop_or::<u64>("updated-at-unix-ms", 0),
        path.observe_prop_or::<String>("app-name", String::new()),
        path.observe_prop_or::<String>("summary", String::new()),
        path.observe_prop_or::<String>("body", String::new()),
        path.observe_prop_or::<String>("icon-name", String::new()),
        path.observe_prop_or::<String>("image-path", String::new()),
        path.observe_prop_or::<String>("urgency", String::new()),
        actions
            => move |(
                created_at_unix_ms,
                updated_at_unix_ms,
                app_name,
                summary,
                body,
                icon_name,
                image_path,
                urgency,
                actions,
            )| NotificationVm {
                path: path_for_vm.clone(),
                id: id_from_path.clone(),
                created_at_unix_ms,
                updated_at_unix_ms,
                app_name,
                summary,
                body,
                icon_name,
                image_path,
                urgency: parse_urgency(&urgency),
                expire_timeout_ms: 0,
                actions,
            },
    )
    .combine_latest(
        path.observe_prop_or::<i32>("expire-timeout-ms", 0),
        |mut notification, expire_timeout_ms| {
            notification.expire_timeout_ms = expire_timeout_ms;
            notification
        },
    )
    .distinct_until_changed()
    .box_it()
}

fn notification_actions(actions_path: LocusPath) -> Observable<Vec<NotificationActionVm>> {
    actions_path
        .as_children()
        .map(action_children)
        .switch_map(|actions| {
            source::combine_latest_vec(actions.into_iter().map(notification_action).collect())
        })
        .map(|actions| actions)
        .distinct_until_changed()
        .box_it()
}

fn notification_action(path: LocusPath) -> Observable<NotificationActionVm> {
    let key_from_path = path_id(&path);

    combine_latest!(
        path.observe_prop_or::<String>("key", key_from_path),
        path.observe_prop_or::<String>("label", String::new()),
        path.observe_prop_or::<bool>("default", false)
            => move |(key, label, is_default)| NotificationActionVm {
                path: path.clone(),
                key,
                label,
                is_default,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn notification_children(children: Vec<LocusPath>) -> Vec<LocusPath> {
    children.into_iter().filter(is_visible_child).collect()
}

fn notification_has_payload_children(children: &[LocusPath]) -> bool {
    children.iter().filter_map(child_name).any(|name| {
        matches!(
            name,
            "id" | "app-name" | "summary" | "body" | "icon-name" | "image-path"
        )
    })
}

fn action_children(children: Vec<LocusPath>) -> Vec<LocusPath> {
    children.into_iter().filter(is_visible_child).collect()
}

fn is_visible_child(child: &LocusPath) -> bool {
    child_name(child).is_some_and(|name| !name.ends_with(".call") && !name.starts_with('@'))
}

fn child_name(child: &LocusPath) -> Option<&str> {
    child.as_path().file_name().and_then(|name| name.to_str())
}

fn ready_notifications(notifications: Vec<Option<NotificationVm>>) -> Vec<NotificationVm> {
    notifications.into_iter().flatten().collect()
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct PopupActivationState {
    initialized: bool,
    known_ids: BTreeSet<String>,
    shown_at_by_id: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PopupActivationEvent {
    kind: source::ChildrenEvent,
    now_unix_ms: u64,
}

fn popup_activation_state() -> Observable<PopupActivationState> {
    source::shared_by_key("rsynapse.notifyd.popups", "activation", || {
        notifyd_collection_path()
            .as_children_events()
            .map(|kind| PopupActivationEvent {
                kind,
                now_unix_ms: now_unix_ms(),
            })
            .scan(
                PopupActivationState::default(),
                update_popup_activation_state,
            )
            .distinct_until_changed()
            .box_it()
    })
}

fn update_popup_activation_state(
    mut state: PopupActivationState,
    event: PopupActivationEvent,
) -> PopupActivationState {
    match event.kind {
        source::ChildrenEvent::Snapshot(children) => {
            let ids = notification_children(children)
                .iter()
                .map(path_id)
                .collect::<BTreeSet<_>>();

            if state.initialized {
                for id in ids.difference(&state.known_ids) {
                    state.shown_at_by_id.insert(id.clone(), event.now_unix_ms);
                }
            }

            state.initialized = true;
            state.known_ids = ids;
            state
                .shown_at_by_id
                .retain(|id, _| state.known_ids.contains(id));
        }
        source::ChildrenEvent::Added(path) => {
            if is_visible_child(&path) {
                let id = path_id(&path);
                state.known_ids.insert(id.clone());
                state.shown_at_by_id.insert(id, event.now_unix_ms);
            }
        }
        source::ChildrenEvent::Changed(path) => {
            if is_visible_child(&path) {
                let id = path_id(&path);
                if state.known_ids.insert(id.clone()) {
                    state.shown_at_by_id.insert(id, event.now_unix_ms);
                }
            }
        }
        source::ChildrenEvent::Removed(path) => {
            let id = path_id(&path);
            state.known_ids.remove(&id);
            state.shown_at_by_id.remove(&id);
        }
    }

    state
}

fn path_id(path: &LocusPath) -> String {
    path.as_path()
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned()
}

fn parse_urgency(value: &str) -> NotificationUrgency {
    match value.trim().to_ascii_lowercase().as_str() {
        "0" | "low" => NotificationUrgency::Low,
        "2" | "critical" => NotificationUrgency::Critical,
        _ => NotificationUrgency::Normal,
    }
}

fn popup_clock() -> Observable<u64> {
    source::shared_by_key("rsynapse.notifyd.popups", "clock", || {
        Shared::<()>::interval(POPUP_TICK)
            .start_with(vec![0])
            .map(|_| now_unix_ms())
            .map_err(|error| error.to_string())
            .box_it()
    })
}

fn popup_visible_notifications(
    notifications: Vec<NotificationVm>,
    activation: &PopupActivationState,
    now_unix_ms: u64,
) -> Vec<NotificationVm> {
    notifications
        .into_iter()
        .filter(|notification| {
            activation
                .shown_at_by_id
                .get(&notification.id)
                .is_some_and(|shown_at| popup_is_visible(notification, *shown_at, now_unix_ms))
        })
        .collect()
}

fn popup_is_visible(
    notification: &NotificationVm,
    shown_at_unix_ms: u64,
    now_unix_ms: u64,
) -> bool {
    let Some(timeout_ms) = popup_timeout_ms(notification) else {
        return true;
    };

    now_unix_ms.saturating_sub(shown_at_unix_ms) < timeout_ms
}

fn popup_timeout_ms(notification: &NotificationVm) -> Option<u64> {
    match notification.expire_timeout_ms {
        timeout if timeout < 0 => Some(DEFAULT_POPUP_TIMEOUT.as_millis() as u64),
        0 => None,
        timeout => Some(timeout as u64),
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
