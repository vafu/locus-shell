use std::collections::BTreeMap;

use super::{NotificationCenterRowVm, NotificationGroupHeaderVm, NotificationVm};

pub(super) fn grouped_rows(notifications: Vec<NotificationVm>) -> Vec<NotificationCenterRowVm> {
    let mut groups = BTreeMap::<String, Vec<NotificationVm>>::new();
    for notification in sort_notifications(notifications) {
        groups
            .entry(notification_app_name(&notification).to_owned())
            .or_default()
            .push(notification);
    }

    let mut groups = groups
        .into_iter()
        .map(|(app_name, notifications)| {
            let latest = notifications
                .iter()
                .map(notification_sort_time)
                .max()
                .unwrap_or_default();
            (app_name, latest, notifications)
        })
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    let mut rows = Vec::new();
    for (app_name, _latest, notifications) in groups {
        rows.push(NotificationCenterRowVm::Header(NotificationGroupHeaderVm {
            app_name,
            count: notifications.len(),
        }));
        rows.extend(
            notifications
                .into_iter()
                .map(NotificationCenterRowVm::Notification),
        );
    }
    rows
}

pub(super) fn sort_notifications(mut notifications: Vec<NotificationVm>) -> Vec<NotificationVm> {
    notifications.sort_by(|left, right| {
        notification_sort_time(right)
            .cmp(&notification_sort_time(left))
            .then_with(|| left.id.cmp(&right.id))
    });
    notifications
}

fn notification_sort_time(notification: &NotificationVm) -> u64 {
    notification
        .updated_at_unix_ms
        .max(notification.created_at_unix_ms)
}

fn notification_app_name(notification: &NotificationVm) -> &str {
    non_empty(&notification.app_name).unwrap_or("Application")
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}
