use relm4::prelude::*;
use shell_core::gtk::{self, prelude::*};

use crate::sources::WorkspaceNode;
use crate::widgets::material_icon;

#[derive(Debug)]
#[shell_macros::model(module = project_label_sources)]
pub(super) struct ProjectLabel {
    pub workspace: WorkspaceNode,

    #[source(crate::sources::workspace_index(workspace.clone()))]
    pub index: u32,

    #[source(crate::sources::workspace_name(workspace.clone()))]
    pub workspace_name: String,

    #[source(crate::sources::workspace_urgent(workspace.clone()))]
    pub urgent: bool,

    #[source(crate::sources::workspace_active(workspace.clone()))]
    pub active: bool,

    #[source(crate::sources::workspace_project_name(workspace.clone()))]
    pub project_name: Option<String>,

    #[source(crate::sources::workspace_project_branch(workspace.clone()))]
    pub project_branch: Option<String>,

    #[source(crate::sources::workspace_project_icon(workspace.clone()))]
    pub project_icon: Option<String>,
}

#[shell_macros::component(
    module = project_label_sources,
    model = ProjectLabel
)]
#[relm4::component(pub(crate))]
impl SimpleComponent for ProjectLabel {
    type Init = WorkspaceNode;
    type Input = project_label_sources::Msg;
    type Output = ();

    view! {
        gtk::Overlay {
            set_halign: gtk::Align::Start,
            set_hexpand: false,

            #[name = "group"]
            gtk::Box {
                #[watch]
                set_css_classes: project_group_classes(model.active, model.urgent),

                set_halign: gtk::Align::Start,
                set_hexpand: false,
                set_orientation: gtk::Orientation::Horizontal,

                #[name = "root_button"]
                gtk::Button {
                    #[watch]
                    set_css_classes: root_button_classes(model.active),

                    #[watch]
                    set_tooltip_text: Some(project_tooltip(&model).as_str()),

                    set_halign: gtk::Align::Start,
                    set_hexpand: false,

                    gtk::Box {
                        add_css_class: "projects-collapsed-icon",
                        add_css_class: "workspaces-collapsed-icon",
                        set_halign: gtk::Align::Center,
                        set_hexpand: false,

                        #[local_ref]
                        icon -> gtk::Image {

                            #[watch]
                            set_icon_name: Some(material_icon::icon_name(project_icon(&model).as_str()).as_str()),
                        }
                    }
                },

                #[name = "title_revealer"]
                gtk::Revealer {
                    #[watch]
                    set_reveal_child: model.active,

                    set_halign: gtk::Align::Start,
                    set_hexpand: false,
                    set_transition_type: gtk::RevealerTransitionType::SlideRight,

                    gtk::Box {
                        add_css_class: "button-subgroup",
                        set_halign: gtk::Align::Start,
                        set_hexpand: false,
                        set_orientation: gtk::Orientation::Horizontal,

                        gtk::Box {
                            add_css_class: "projects-title",
                            add_css_class: "workspaces-title",
                            set_halign: gtk::Align::Start,
                            set_hexpand: false,
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 4,

                            gtk::Label {
                                add_css_class: "projects-primary",
                                add_css_class: "workspaces-primary",
                                set_ellipsize: gtk::pango::EllipsizeMode::End,

                                #[watch]
                                set_label: project_primary(&model).as_str(),

                                set_max_width_chars: 18,
                                set_xalign: 0.0,
                            },

                            gtk::Label {
                                add_css_class: "projects-delimiter",
                                add_css_class: "workspaces-delimiter",

                                #[watch]
                                set_visible: project_secondary(&model).is_some(),

                                set_label: "·",
                                set_xalign: 0.0,
                            },

                            gtk::Label {
                                add_css_class: "projects-secondary",
                                add_css_class: "workspaces-secondary",
                                set_ellipsize: gtk::pango::EllipsizeMode::End,

                                #[watch]
                                set_label: project_secondary(&model).unwrap_or_default().as_str(),

                                #[watch]
                                set_visible: project_secondary(&model).is_some(),

                                set_max_width_chars: 18,
                                set_xalign: 0.0,
                            }
                        }
                    }
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ProjectLabel::new(init);
        let icon = material_icon::image(&project_icon(&model));
        let widgets = view_output!();

        root.add_overlay(&workspace_badge(model.index));

        ComponentParts { model, widgets }
    }
}

const PROJECT_GROUP_CLASSES: &[&str] = &[
    "projects-project",
    "workspaces-workspace",
    "button-subgroup-expand-right",
];
const PROJECT_GROUP_ACTIVE_CLASSES: &[&str] = &[
    "projects-project",
    "workspaces-workspace",
    "button-subgroup-expand-right",
    "current-workspace",
];
const PROJECT_GROUP_URGENT_CLASSES: &[&str] = &[
    "projects-project",
    "workspaces-workspace",
    "button-subgroup-expand-right",
    "has-attention",
];
const PROJECT_GROUP_ACTIVE_URGENT_CLASSES: &[&str] = &[
    "projects-project",
    "workspaces-workspace",
    "button-subgroup-expand-right",
    "current-workspace",
    "has-attention",
];

const ROOT_BUTTON_CLASSES: &[&str] = &[
    "projects-root-button",
    "workspaces-root-button",
    "flat",
    "circular",
    "panel-widget",
    "button-subgroup-main",
];
const ROOT_BUTTON_OPEN_CLASSES: &[&str] = &[
    "projects-root-button",
    "workspaces-root-button",
    "flat",
    "circular",
    "panel-widget",
    "button-subgroup-main",
    "opened",
];

fn project_group_classes(active: bool, urgent: bool) -> &'static [&'static str] {
    match (active, urgent) {
        (true, true) => PROJECT_GROUP_ACTIVE_URGENT_CLASSES,
        (true, false) => PROJECT_GROUP_ACTIVE_CLASSES,
        (false, true) => PROJECT_GROUP_URGENT_CLASSES,
        (false, false) => PROJECT_GROUP_CLASSES,
    }
}

fn root_button_classes(active: bool) -> &'static [&'static str] {
    if active {
        ROOT_BUTTON_OPEN_CLASSES
    } else {
        ROOT_BUTTON_CLASSES
    }
}

fn project_icon(model: &ProjectLabel) -> String {
    model
        .project_icon
        .as_deref()
        .and_then(non_empty_text)
        .unwrap_or("view_quilt")
        .to_owned()
}

fn project_primary(model: &ProjectLabel) -> String {
    model
        .project_name
        .as_deref()
        .and_then(non_empty_text)
        .map(str::to_owned)
        .unwrap_or_else(|| workspace_title(&model.workspace_name, &model.workspace, model.index))
}

fn project_secondary(model: &ProjectLabel) -> Option<String> {
    model
        .project_branch
        .as_deref()
        .and_then(non_empty_text)
        .map(str::to_owned)
}

fn project_tooltip(model: &ProjectLabel) -> String {
    let primary = project_primary(model);
    match project_secondary(model) {
        Some(secondary) => format!("{primary} · {secondary}"),
        None => primary,
    }
}

fn workspace_title(workspace_name: &str, workspace_id: &str, index: u32) -> String {
    optional_text(Some(workspace_name))
        .map(str::to_owned)
        .unwrap_or_else(|| {
            if workspace_id.is_empty() {
                format!("Workspace {}", index + 1)
            } else {
                workspace_id.to_owned()
            }
        })
}

fn optional_text(value: Option<&str>) -> Option<&str> {
    non_empty_text(value?)
}

fn non_empty_text(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn workspace_badge(sort_index: u32) -> gtk::Label {
    let badge = gtk::Label::new(Some(&sort_index.to_string()));
    badge.set_halign(gtk::Align::End);
    badge.set_valign(gtk::Align::Start);
    badge.add_css_class("barblock-badge");
    badge.add_css_class("workspace-number-badge");
    badge.set_visible(false);
    badge
}
