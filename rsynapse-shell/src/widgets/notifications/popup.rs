use relm4::prelude::*;
use shell_core::gtk::{self, prelude::*};

use super::{
    card::{
        connect_command, mount_actions, notification_app_name, notification_card_classes,
        notification_summary, set_notification_image,
    },
    source::NotificationVm,
};
use crate::widgets::material_icon;

#[derive(Debug)]
pub(crate) struct NotificationCard {
    notification: NotificationVm,
}

#[relm4::component(pub(crate))]
impl SimpleComponent for NotificationCard {
    type Init = NotificationVm;
    type Input = ();
    type Output = ();

    view! {
        gtk::Box {
            #[watch]
            set_css_classes: &notification_card_classes(&model.notification),
            set_orientation: gtk::Orientation::Vertical,
            set_width_request: 380,
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
                            set_label: notification_app_name(&model.notification).as_str(),
                        },

                        #[name = "close_button"]
                        gtk::Button {
                            add_css_class: "flat",
                            add_css_class: "circular",
                            add_css_class: "notification-close",
                            set_tooltip_text: Some("Dismiss"),

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
                        set_label: notification_summary(&model.notification).as_str(),
                    },

                    gtk::Label {
                        add_css_class: "notification-body",
                        set_halign: gtk::Align::Start,
                        set_wrap: true,
                        set_wrap_mode: gtk::pango::WrapMode::WordChar,
                        set_visible: !model.notification.body.trim().is_empty(),
                        set_label: model.notification.body.as_str(),
                    }
                }
            },

            #[name = "actions_box"]
            gtk::Box {
                add_css_class: "notification-actions",
                set_halign: gtk::Align::End,
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 6,
                set_visible: !model.notification.actions.is_empty(),
            }
        }
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = NotificationCard { notification: init };
        let widgets = view_output!();
        set_notification_image(&widgets.image, &model.notification);
        connect_command(
            &widgets.close_button,
            &model.notification,
            "discard",
            "discarded\n",
        );
        mount_actions(&widgets.actions_box, &model.notification.actions);
        ComponentParts { model, widgets }
    }
}
