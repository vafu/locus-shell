mod workspace_fallback;

use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

use self::workspace_fallback::workspace_window_fallback_source;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(in crate::widgets::bar) struct ProjectLabelVm {
    pub(super) index: u32,
    pub(super) workspace_name: String,
    pub(super) urgent: bool,
    pub(super) active: bool,
    pub(super) project_name: Option<String>,
    pub(super) project_branch: Option<String>,
    pub(super) project_icon: Option<String>,
    pub(super) project_icon_is_app: bool,
    pub(super) empty: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ProjectDetails {
    has_project: bool,
    name: Option<String>,
    branch: Option<String>,
    icon: Option<String>,
}

pub(super) fn project_label_vm(workspace: LocusPath) -> Observable<ProjectLabelVm> {
    let project = workspace
        .observe_rel("project")
        .switch_map(project_details_source);
    let workspace_fallback = workspace_window_fallback_source(workspace.clone());

    combine_latest!(
        workspace
            .observe_prop_or::<u32>("index", u32::MAX),
        workspace
            .observe_prop_or::<String>("name", String::new()),
        workspace
            .observe_prop_or::<bool>("urgent", false),
        workspace
            .observe_prop_or::<bool>("selected", false),
        project,
        workspace_fallback
            => |(index, workspace_name, urgent, active, project, fallback)| {
                let fallback_icon = (!project.has_project).then_some(fallback.icon).flatten();
                let project_icon_is_app = fallback_icon.is_some();
                let project_icon = project.icon.or(fallback_icon);
                ProjectLabelVm {
                    index,
                    workspace_name,
                    urgent,
                    active,
                    project_name: project.name,
                    project_branch: project.branch,
                    project_icon,
                    project_icon_is_app,
                    empty: !project.has_project && fallback.empty,
                }
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn project_details_source(project: Option<LocusPath>) -> Observable<ProjectDetails> {
    let Some(project) = project else {
        return source::once(ProjectDetails::default());
    };

    combine_latest!(
        project.observe_prop_or::<String>("display-name", String::new()).map(non_empty_value),
        project.observe_prop_or::<String>("title", String::new()).map(non_empty_value),
        project.observe_prop_or::<String>("name", String::new()).map(non_empty_value),
        project.observe_prop_or::<String>("path", String::new()).map(non_empty_value),
        project.observe_prop_or::<String>("branch", String::new()).map(non_empty_value),
        project.observe_prop_or::<String>("display-icon", String::new()).map(non_empty_value),
        project.observe_prop_or::<String>("icon", String::new()).map(non_empty_value)
            => |(display_name, title, name, path, branch, display_icon, icon)| ProjectDetails {
                    has_project: true,
                    name: display_name.or(title).or(name).or(path),
                    branch,
                    icon: display_icon.or(icon),
            },
    )
    .box_it()
}

fn non_empty_value(value: String) -> Option<String> {
    non_empty(Some(value))
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    })
}
