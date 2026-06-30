use std::{fs, path::Path, thread};

use shell_core::gtk::{self, prelude::*};

use super::NOTIFICATION_CARD_BLUR_CLASS;
use super::source::{NotificationActionVm, NotificationUrgency, NotificationVm};

pub(super) const NOTIFICATION_APP_MAX_CHARS: i32 = 30;
pub(super) const NOTIFICATION_TEXT_MAX_CHARS: i32 = 42;

const NOTIFICATION_ACTION_MAX_CHARS: i32 = 24;

pub(super) fn notification_card_classes(notification: &NotificationVm) -> Vec<&'static str> {
    let mut classes = vec!["notification-card", NOTIFICATION_CARD_BLUR_CLASS];
    append_notification_state_classes(&mut classes, notification);
    classes
}

pub(super) fn center_card_classes(notification: &NotificationVm) -> Vec<&'static str> {
    let mut classes = vec![
        "notification-card",
        NOTIFICATION_CARD_BLUR_CLASS,
        "notification-center-card",
    ];
    append_notification_state_classes(&mut classes, notification);
    classes
}

fn append_notification_state_classes(
    classes: &mut Vec<&'static str>,
    notification: &NotificationVm,
) {
    match notification.urgency {
        NotificationUrgency::Low => classes.push("low"),
        NotificationUrgency::Normal => {}
        NotificationUrgency::Critical => classes.push("critical"),
    }
}

pub(super) fn notification_app_name(notification: &NotificationVm) -> String {
    non_empty(&notification.app_name)
        .unwrap_or("Application")
        .to_owned()
}

pub(super) fn notification_summary(notification: &NotificationVm) -> String {
    non_empty(&notification.summary)
        .or_else(|| non_empty(&notification.body))
        .unwrap_or("Notification")
        .to_owned()
}

pub(super) fn set_notification_image(image: &gtk::Image, notification: &NotificationVm) {
    if let Some(path) = notification_image_path(notification) {
        image.set_from_file(Some(path));
        image.set_visible(true);
        return;
    }

    let icon_name = non_empty(&notification.icon_name).unwrap_or("dialog-information-symbolic");
    image.set_icon_name(Some(icon_name));
    image.set_visible(true);
}

fn notification_image_path(notification: &NotificationVm) -> Option<&Path> {
    let value = non_empty(&notification.image_path)?;
    let value = value.strip_prefix("file://").unwrap_or(value);
    if value.starts_with('/') {
        Some(Path::new(value))
    } else {
        None
    }
}

pub(super) fn connect_command(
    button: &gtk::Button,
    notification: &NotificationVm,
    property: &'static str,
    payload: &'static str,
) {
    let command_path = notification.path.child(property).into_path_buf();
    button.connect_clicked(move |_| {
        let command_path = command_path.clone();
        thread::spawn(move || {
            let _ = fs::write(command_path, payload);
        });
    });
}

pub(super) fn mount_actions(actions_box: &gtk::Box, actions: &[NotificationActionVm]) {
    for action in actions
        .iter()
        .filter(|action| !action.label.trim().is_empty())
    {
        let label = gtk::Label::new(Some(action.label.as_str()));
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.set_max_width_chars(NOTIFICATION_ACTION_MAX_CHARS);

        let button = gtk::Button::new();
        button.add_css_class("notification-action");
        button.set_child(Some(&label));
        if action.is_default {
            button.add_css_class("default");
        }

        let invoke_path = action.path.child("invoke").into_path_buf();
        let key = action.key.clone();
        button.connect_clicked(move |_| {
            let invoke_path = invoke_path.clone();
            let key = key.clone();
            thread::spawn(move || {
                let _ = fs::write(invoke_path, format!("{key}\n"));
            });
        });

        actions_box.append(&button);
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}
