use relm4::prelude::*;
use shell_core::gtk::{self, prelude::*};

use super::{
    card::{
        center_card_classes, connect_command, mount_actions, notification_app_name,
        notification_summary, set_notification_image,
    },
    source::{NotificationCenterRowVm, NotificationVm},
};
use crate::widgets::material_icon;

#[derive(Debug)]
pub(crate) struct NotificationCenterRow {
    row: NotificationCenterRowVm,
}

#[relm4::component(pub(crate))]
impl SimpleComponent for NotificationCenterRow {
    type Init = NotificationCenterRowVm;
    type Input = ();
    type Output = ();

    view! {
        gtk::Box {
            #[watch]
            set_css_classes: &center_row_classes(&model.row),
            set_orientation: gtk::Orientation::Vertical,

            gtk::Box {
                add_css_class: "notification-group-header",
                #[watch]
                set_visible: row_is_header(&model.row),
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,

                gtk::Label {
                    add_css_class: "notification-group-app",
                    set_hexpand: true,
                    set_halign: gtk::Align::Start,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_max_width_chars: 36,
                    #[watch]
                    set_label: center_header_app(&model.row).as_str(),
                },

                gtk::Label {
                    add_css_class: "notification-group-count",
                    #[watch]
                    set_label: center_header_count(&model.row).as_str(),
                }
            },

            gtk::Box {
                #[watch]
                set_css_classes: &center_row_card_classes(&model.row),
                #[watch]
                set_visible: row_notification(&model.row).is_some(),
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 10,

                    #[name = "image"]
                    gtk::Image {
                        add_css_class: "notification-image",
                        set_pixel_size: 42,
                        set_valign: gtk::Align::Start,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_spacing: 2,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 8,

                            gtk::Label {
                                add_css_class: "notification-app",
                                set_halign: gtk::Align::Start,
                                set_hexpand: true,
                                set_ellipsize: gtk::pango::EllipsizeMode::End,
                                set_max_width_chars: 30,
                                #[watch]
                                set_label: center_row_app_name(&model.row).as_str(),
                            },

                            #[name = "close_button"]
                            gtk::Button {
                                add_css_class: "flat",
                                add_css_class: "circular",
                                add_css_class: "notification-close",
                                set_tooltip_text: Some("Discard"),

                                gtk::Image {
                                    add_css_class: "materialicon",
                                    set_icon_name: Some(material_icon::icon_name("close").as_str()),
                                }
                            }
                        },

                        gtk::Label {
                            add_css_class: "notification-summary",
                            set_halign: gtk::Align::Start,
                            set_wrap: true,
                            set_wrap_mode: gtk::pango::WrapMode::WordChar,
                            set_max_width_chars: 42,
                            #[watch]
                            set_label: center_row_summary(&model.row).as_str(),
                        },

                        gtk::Label {
                            add_css_class: "notification-body",
                            set_halign: gtk::Align::Start,
                            set_wrap: true,
                            set_wrap_mode: gtk::pango::WrapMode::WordChar,
                            set_max_width_chars: 42,
                            #[watch]
                            set_visible: center_row_body_visible(&model.row),
                            #[watch]
                            set_label: center_row_body(&model.row).as_str(),
                        }
                    }
                },

                #[name = "actions_box"]
                gtk::Box {
                    add_css_class: "notification-actions",
                    set_halign: gtk::Align::End,
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,
                    #[watch]
                    set_visible: center_row_has_actions(&model.row),
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = NotificationCenterRow { row: init };
        let widgets = view_output!();
        if let Some(notification) = row_notification(&model.row) {
            set_notification_image(&widgets.image, notification);
            connect_command(
                &widgets.close_button,
                notification,
                "discard",
                "discarded\n",
            );
            mount_actions(&widgets.actions_box, &notification.actions);
        }
        ComponentParts { model, widgets }
    }
}

fn center_row_classes(row: &NotificationCenterRowVm) -> Vec<&'static str> {
    match row {
        NotificationCenterRowVm::Header(_) => vec!["notification-center-row", "group-header"],
        NotificationCenterRowVm::Notification(_) => vec!["notification-center-row"],
    }
}

fn center_row_card_classes(row: &NotificationCenterRowVm) -> Vec<&'static str> {
    row_notification(row)
        .map(center_card_classes)
        .unwrap_or_default()
}

fn row_is_header(row: &NotificationCenterRowVm) -> bool {
    matches!(row, NotificationCenterRowVm::Header(_))
}

fn row_notification(row: &NotificationCenterRowVm) -> Option<&NotificationVm> {
    match row {
        NotificationCenterRowVm::Notification(notification) => Some(notification),
        NotificationCenterRowVm::Header(_) => None,
    }
}

fn center_header_app(row: &NotificationCenterRowVm) -> String {
    match row {
        NotificationCenterRowVm::Header(header) => header.app_name.clone(),
        NotificationCenterRowVm::Notification(_) => String::new(),
    }
}

fn center_header_count(row: &NotificationCenterRowVm) -> String {
    match row {
        NotificationCenterRowVm::Header(header) => header.count.to_string(),
        NotificationCenterRowVm::Notification(_) => String::new(),
    }
}

fn center_row_app_name(row: &NotificationCenterRowVm) -> String {
    row_notification(row)
        .map(notification_app_name)
        .unwrap_or_default()
}

fn center_row_summary(row: &NotificationCenterRowVm) -> String {
    row_notification(row)
        .map(notification_summary)
        .unwrap_or_default()
}

fn center_row_body(row: &NotificationCenterRowVm) -> String {
    row_notification(row)
        .map(|notification| notification.body.clone())
        .unwrap_or_default()
}

fn center_row_body_visible(row: &NotificationCenterRowVm) -> bool {
    row_notification(row).is_some_and(|notification| !notification.body.trim().is_empty())
}

fn center_row_has_actions(row: &NotificationCenterRowVm) -> bool {
    row_notification(row).is_some_and(|notification| !notification.actions.is_empty())
}
