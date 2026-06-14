use std::collections::HashMap;

use locus_dbus::NONE_STRING;

use crate::schema::model;

pub(super) fn project_icon(project: &HashMap<String, String>) -> String {
    first_property(
        project,
        &[
            model::Project::DISPLAY_ICON.key(),
            model::Project::ICON.key(),
            "icon-name",
            "symbolicIcon",
        ],
    )
    .unwrap_or("folder_code")
    .to_owned()
}

pub(super) fn project_title_parts(
    project: &HashMap<String, String>,
    project_id: &str,
) -> (String, String) {
    let name = first_property(
        project,
        &[
            model::Project::DISPLAY_MAIN.key(),
            model::Project::NAME.key(),
            model::Project::PATH.key(),
        ],
    )
    .map(path_basename)
    .unwrap_or(project_id)
    .trim()
    .to_owned();

    if let Some(display_secondary) =
        optional_property(project, model::Project::DISPLAY_SECONDARY.key())
    {
        return (name, display_secondary.to_owned());
    }

    let subproject = optional_property(project, model::Project::SUBPROJ.key()).unwrap_or("");
    let task = optional_property(project, model::Project::TASK.key()).unwrap_or("");
    let branch = optional_property(project, model::Project::BRANCH.key()).unwrap_or("");

    if !subproject.is_empty() && !task.is_empty() {
        return (subproject.to_owned(), task.to_owned());
    }

    if !subproject.is_empty() {
        return (name, subproject.to_owned());
    }

    (name, branch.to_owned())
}

pub(super) fn workspace_title(
    workspace: &HashMap<String, String>,
    workspace_id: &str,
    sort_index: u32,
) -> String {
    optional_property(workspace, model::Workspace::NAME.key())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            if workspace_id.is_empty() {
                format!("Workspace {}", sort_index + 1)
            } else {
                workspace_id.to_owned()
            }
        })
}

pub(super) fn optional_property<'a>(
    properties: &'a HashMap<String, String>,
    key: &str,
) -> Option<&'a str> {
    let value = properties.get(key)?.trim();
    (!value.is_empty() && value != NONE_STRING).then_some(value)
}

fn first_property<'a>(properties: &'a HashMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| optional_property(properties, key))
}

fn path_basename(path: &str) -> &str {
    path.rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(path)
}
