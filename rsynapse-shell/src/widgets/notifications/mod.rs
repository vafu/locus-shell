mod card;
mod center_row;
mod popup;
mod source;

use relm4::prelude::*;
use shell_core::{
    gtk::{self, prelude::*},
    list::ComponentListBoxExt,
    window::{self, Anchors, Edge, Layer, SurfaceMargins, WindowConfig},
};

use self::{
    center_row::NotificationCenterRow,
    popup::NotificationCard,
    source::{
        NotificationCenterRowVm, NotificationVm, notification_center_rows, popup_notifications,
    },
};

pub(crate) use self::source::has_notification_items;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationsInit {
    pub title: &'static str,
}

#[derive(Debug)]
#[shell_macros::model(module = notification_sources)]
pub struct NotificationsWindow {
    #[source(popup_notifications())]
    notifications: Vec<NotificationVm>,
}

#[derive(Debug)]
pub enum NotificationsInput {
    Source(notification_sources::Msg),
}

impl From<notification_sources::Msg> for NotificationsInput {
    fn from(msg: notification_sources::Msg) -> Self {
        Self::Source(msg)
    }
}

#[shell_macros::component(module = notification_sources, model = NotificationsWindow)]
#[relm4::component(pub, async)]
impl SimpleAsyncComponent for NotificationsWindow {
    type Init = NotificationsInit;
    type Input = NotificationsInput;
    type Output = ();

    view! {
        gtk::Window {
            add_css_class: "notifications-window",
            #[watch]
            set_visible: !model.notifications.is_empty(),

            gtk::Revealer {
                #[watch]
                set_reveal_child: !model.notifications.is_empty(),
                set_transition_type: gtk::RevealerTransitionType::SlideRight,
                set_transition_duration: 220,

                #[bind_list(notifications, row = NotificationCard)]
                notifications -> gtk::Box {
                    add_css_class: "notifications-stack",
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
                    set_valign: gtk::Align::End,
                }
            }
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        window::apply_layer_shell_config(&root, notifications_window_config());
        root.set_title(Some(init.title));

        let model = NotificationsWindow::new();
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, _sender: AsyncComponentSender<Self>) {
        match msg {
            NotificationsInput::Source(msg) => NotificationsWindow::update(self, msg),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationCenterInit {
    pub title: &'static str,
}

#[derive(Debug)]
#[shell_macros::model(module = notification_center_sources)]
pub struct NotificationCenterWindow {
    open: bool,

    #[source(notification_center_rows())]
    rows: Vec<NotificationCenterRowVm>,
}

#[derive(Debug)]
pub enum NotificationCenterInput {
    Source(notification_center_sources::Msg),
    Toggle,
    Close,
}

impl From<notification_center_sources::Msg> for NotificationCenterInput {
    fn from(msg: notification_center_sources::Msg) -> Self {
        Self::Source(msg)
    }
}

#[shell_macros::component(
    module = notification_center_sources,
    model = NotificationCenterWindow
)]
#[relm4::component(pub, async)]
impl SimpleAsyncComponent for NotificationCenterWindow {
    type Init = NotificationCenterInit;
    type Input = NotificationCenterInput;
    type Output = ();

    view! {
        gtk::Window {
            add_css_class: "notification-center-window",
            #[watch]
            set_visible: model.open,

            gtk::Revealer {
                #[watch]
                set_reveal_child: model.open,
                set_transition_type: gtk::RevealerTransitionType::SlideUp,
                set_transition_duration: 180,

                gtk::Box {
                    add_css_class: "notification-center",
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,

                    gtk::Box {
                        add_css_class: "notification-center-header",
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,

                        gtk::Label {
                            add_css_class: "notification-center-title",
                            set_hexpand: true,
                            set_halign: gtk::Align::Start,
                            set_label: "Notifications",
                        },

                        #[name = "center_close_button"]
                        gtk::Button {
                            add_css_class: "flat",
                            add_css_class: "circular",
                            add_css_class: "notification-close",
                            set_tooltip_text: Some("Close"),

                            gtk::Image {
                                add_css_class: "materialicon",
                                set_icon_name: Some(crate::widgets::material_icon::icon_name("close").as_str()),
                            }
                        }
                    },

                    gtk::Label {
                        add_css_class: "notification-empty",
                        #[watch]
                        set_visible: model.rows.is_empty(),
                        set_label: "No notifications",
                    },

                    #[bind_list(rows, row = NotificationCenterRow)]
                    rows -> gtk::Box {
                        add_css_class: "notification-center-list",
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,
                    }
                }
            }
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        window::apply_layer_shell_config(&root, notification_center_window_config());
        root.set_title(Some(init.title));

        let model = NotificationCenterWindow::new(false);
        let widgets = view_output!();
        let input_sender = sender.input_sender().clone();
        widgets.center_close_button.connect_clicked(move |_| {
            input_sender.emit(NotificationCenterInput::Close);
        });
        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, _sender: AsyncComponentSender<Self>) {
        match msg {
            NotificationCenterInput::Source(msg) => NotificationCenterWindow::update(self, msg),
            NotificationCenterInput::Toggle => self.open = !self.open,
            NotificationCenterInput::Close => self.open = false,
        }
    }
}

fn notifications_window_config() -> WindowConfig {
    WindowConfig::new(Layer::Overlay)
        .with_anchors(Anchors::NONE.with_edge(Edge::Bottom).with_edge(Edge::Left))
        .with_surface_margins(SurfaceMargins {
            bottom: 24,
            left: 24,
            ..SurfaceMargins::ZERO
        })
        .with_namespace("rsynapse-notifications")
}

fn notification_center_window_config() -> WindowConfig {
    WindowConfig::new(Layer::Overlay)
        .with_anchors(Anchors::NONE.with_edge(Edge::Bottom).with_edge(Edge::Right))
        .with_surface_margins(SurfaceMargins {
            bottom: 52,
            right: 16,
            ..SurfaceMargins::ZERO
        })
        .with_namespace("rsynapse-notification-center")
}
