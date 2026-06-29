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
    shown: bool,
}

#[derive(Debug)]
pub(crate) enum NotificationCardInput {
    Reveal,
}

#[relm4::component(pub(crate))]
impl SimpleComponent for NotificationCard {
    type Init = NotificationVm;
    type Input = NotificationCardInput;
    type Output = ();

    view! {
        gtk::Revealer {
            #[watch]
            set_reveal_child: model.shown,
            set_transition_type: gtk::RevealerTransitionType::SlideLeft,
            set_transition_duration: 220,

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
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = NotificationCard {
            notification: init,
            shown: false,
        };
        let widgets = view_output!();
        set_notification_image(&widgets.image, &model.notification);
        connect_command(
            &widgets.close_button,
            &model.notification,
            "discard",
            "discarded\n",
        );
        mount_actions(&widgets.actions_box, &model.notification.actions);
        let reveal_sender = sender.clone();
        gtk::glib::idle_add_local_once(move || {
            reveal_sender.input(NotificationCardInput::Reveal);
        });
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            NotificationCardInput::Reveal => self.shown = true,
        }
    }
}
