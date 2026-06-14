use relm4::prelude::*;
use shell_core::gtk::{self, prelude::*};

use locus_provider::NodeRef;

use crate::schema::{OptionalProjectNodeExt, WorkspaceNodeExt, model};
use crate::widgets::material_icon;

#[derive(Debug)]
#[shell_macros::model(module = project_label_sources)]
pub(super) struct ProjectLabel {
    pub workspace: NodeRef<model::Workspace>,

    #[source(workspace.index())]
    pub index: u32,

    #[source(workspace.name())]
    pub workspace_name: String,

    #[source(workspace.urgent())]
    pub urgent: bool,

    #[source(workspace.is_selected())]
    pub active: bool,

    #[model(source = workspace.project())]
    pub project: ProjectLabelProject,
}

#[derive(Debug)]
#[shell_macros::model(module = project_label_project_sources)]
pub(super) struct ProjectLabelProject {
    pub project: Option<NodeRef<model::Project>>,

    #[source(project.display_icon())]
    pub display_icon: Option<String>,

    #[source(project.icon())]
    pub icon: Option<String>,

    #[source(project.display_main())]
    pub display_main: Option<String>,

    #[source(project.display_secondary())]
    pub display_secondary: Option<String>,

    #[source(project.name())]
    pub name: Option<String>,

    #[source(project.path())]
    pub path: Option<String>,

    #[source(project.subproj())]
    pub subproj: Option<String>,

    #[source(project.task())]
    pub task: Option<String>,

    #[source(project.branch())]
    pub branch: Option<String>,
}

#[shell_macros::component(
    module = project_label_sources,
    model = ProjectLabel
)]
#[relm4::component(pub(crate))]
impl SimpleComponent for ProjectLabel {
    type Init = NodeRef<model::Workspace>;
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
                            set_icon_name: Some(material_icon::icon_name(project_icon(&model.project).as_str()).as_str()),
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
                                set_visible: !project_secondary(&model.project).is_empty(),

                                set_label: "·",
                                set_xalign: 0.0,
                            },

                            gtk::Label {
                                add_css_class: "projects-secondary",
                                add_css_class: "workspaces-secondary",
                                set_ellipsize: gtk::pango::EllipsizeMode::End,

                                #[watch]
                                set_label: project_secondary(&model.project).as_str(),

                                #[watch]
                                set_visible: !project_secondary(&model.project).is_empty(),

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
        let icon = material_icon::image(&project_icon(&model.project));
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

fn project_icon(project: &ProjectLabelProject) -> String {
    if project.project.is_none() {
        return "view_quilt".to_owned();
    }

    first_text([project.display_icon.as_deref(), project.icon.as_deref()])
        .unwrap_or("folder_code")
        .to_owned()
}

fn project_primary(model: &ProjectLabel) -> String {
    if model.project.project.is_none() {
        return workspace_title(&model.workspace_name, model.workspace.id(), model.index);
    }

    let project_id = model
        .project
        .project
        .as_ref()
        .map(|project| project.id())
        .unwrap_or("");
    let name = first_text([
        model.project.display_main.as_deref(),
        model.project.name.as_deref(),
        model.project.path.as_deref(),
    ])
    .map(path_basename)
    .unwrap_or(project_id)
    .trim()
    .to_owned();

    let subproject = optional_text(model.project.subproj.as_deref()).unwrap_or("");
    let task = optional_text(model.project.task.as_deref()).unwrap_or("");

    if !subproject.is_empty() && !task.is_empty() {
        subproject.to_owned()
    } else {
        name
    }
}

fn project_secondary(project: &ProjectLabelProject) -> String {
    if project.project.is_none() {
        return String::new();
    }

    if let Some(display_secondary) = optional_text(project.display_secondary.as_deref()) {
        return display_secondary.to_owned();
    }

    let subproject = optional_text(project.subproj.as_deref()).unwrap_or("");
    let task = optional_text(project.task.as_deref()).unwrap_or("");
    let branch = optional_text(project.branch.as_deref()).unwrap_or("");

    if !subproject.is_empty() && !task.is_empty() {
        return task.to_owned();
    }

    if !subproject.is_empty() {
        return subproject.to_owned();
    }

    branch.to_owned()
}

fn project_tooltip(model: &ProjectLabel) -> String {
    let primary = project_primary(model);
    let secondary = project_secondary(&model.project);

    if secondary.is_empty() {
        primary
    } else {
        format!("{primary} · {secondary}")
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

fn first_text<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Option<&'a str> {
    values.into_iter().find_map(optional_text)
}

fn optional_text(value: Option<&str>) -> Option<&str> {
    let value = value?.trim();
    (!value.is_empty()).then_some(value)
}

fn path_basename(path: &str) -> &str {
    path.rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(path)
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
