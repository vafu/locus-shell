use locus_dbus::GraphReadProxy;
use locus_provider::NodeId;
use providers::ProviderError;

use crate::schema::{model, relations};

use super::{
    ProjectLabelView,
    format::{optional_property, project_icon, project_title_parts, workspace_title},
};

pub(super) async fn hydrate_project_label(
    connection: &zbus::Connection,
    workspace_id: &str,
) -> Result<ProjectLabelView, ProviderError> {
    let read = GraphReadProxy::new(connection)
        .await
        .map_err(provider_error)?;
    workspace_view(&read, workspace_id, 0).await
}

async fn workspace_view(
    read: &GraphReadProxy<'_>,
    workspace_id: &str,
    position: usize,
) -> Result<ProjectLabelView, ProviderError> {
    let workspace = read
        .get_properties(workspace_id)
        .await
        .map_err(provider_error)?;
    let sort_index = optional_property(&workspace, model::Workspace::INDEX.key())
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(position as u32);
    let urgent = matches!(
        optional_property(&workspace, model::Workspace::URGENT.key()),
        Some("true")
    );
    let project_id = first_project(read, workspace_id).await?;

    match project_id {
        Some(project_id) => {
            let project = read
                .get_properties(&project_id)
                .await
                .map_err(provider_error)?;
            let (primary, secondary) = project_title_parts(&project, &project_id);
            let tooltip = label_tooltip(&primary, &secondary);

            Ok(ProjectLabelView {
                project_id: Some(project_id),
                sort_index,
                icon: project_icon(&project),
                primary,
                secondary,
                tooltip,
                urgent,
            })
        }
        None => {
            let primary = workspace_title(&workspace, workspace_id, sort_index);

            Ok(ProjectLabelView {
                project_id: None,
                sort_index,
                icon: "view_quilt".to_owned(),
                tooltip: primary.clone(),
                primary,
                secondary: String::new(),
                urgent,
            })
        }
    }
}

async fn first_project(
    read: &GraphReadProxy<'_>,
    workspace_id: &str,
) -> Result<Option<NodeId>, ProviderError> {
    let targets = read
        .get_targets(workspace_id, relations::PROJECT.name())
        .await
        .map_err(provider_error)?;

    for target in targets {
        let properties = read.get_properties(&target).await.map_err(provider_error)?;
        if matches!(optional_property(&properties, "kind"), Some("project")) {
            return Ok(Some(target));
        }
    }

    Ok(None)
}

fn provider_error(error: impl std::fmt::Display) -> ProviderError {
    ProviderError::new(error.to_string())
}

fn label_tooltip(primary: &str, secondary: &str) -> String {
    if secondary.is_empty() {
        primary.to_owned()
    } else {
        format!("{primary} · {secondary}")
    }
}
