use std::collections::HashMap;

use locus_dbus::{GraphReadProxy, GraphResolveProxy, NONE_STRING};
use locus_provider::NodeId;
use providers::ProviderError;

use crate::schema::{model, relations};

use super::{WindowTileKind, WindowTileView};

const AGENT_SESSION_WORKSPACE_PROJECT_PATH: &[&str] =
    &["agent-session", "app-instance", "workspace", "project"];

pub(super) async fn hydrate_window_tile(
    connection: &zbus::Connection,
    window_id: &str,
) -> Result<WindowTileView, ProviderError> {
    let read = GraphReadProxy::new(connection)
        .await
        .map_err(provider_error)?;
    let window = read
        .get_properties(window_id)
        .await
        .map_err(provider_error)?;
    let agent_session = window_agent_session(&read, window_id).await?;
    let title = optional_property(&window, model::Window::TITLE.key()).unwrap_or(window_id);
    let urgent = matches!(
        optional_property(&window, model::Window::URGENT.key()),
        Some("true")
    );

    if let Some(agent_session) = agent_session {
        let substatus_count = read
            .get_targets(&agent_session, relations::SUBAGENT_SESSION.name())
            .await
            .map_err(provider_error)?
            .len() as u32;
        let icon = agent_project_icon(connection, &read, &agent_session)
            .await?
            .unwrap_or_else(|| window_icon(&window, "smart_toy"));

        return Ok(WindowTileView {
            kind: WindowTileKind::Agent,
            icon,
            tooltip: format!("{agent_session} · {title}"),
            urgent,
            context_pct: 0,
            substatus_count,
        });
    }

    let icon = window_icon(&window, "application-x-executable-symbolic");
    let kind = if is_neovim_window(&window, title) {
        WindowTileKind::Neovim
    } else {
        WindowTileKind::Plain
    };

    Ok(WindowTileView {
        kind,
        icon,
        tooltip: title.to_owned(),
        urgent,
        context_pct: 0,
        substatus_count: 0,
    })
}

async fn window_agent_session(
    read: &GraphReadProxy<'_>,
    window_id: &str,
) -> Result<Option<NodeId>, ProviderError> {
    let app_instances = read
        .get_targets(window_id, relations::APP_INSTANCE.name())
        .await
        .map_err(provider_error)?;

    for app_instance in app_instances {
        let agent_sessions = read
            .get_targets(&app_instance, relations::AGENT_SESSION.name())
            .await
            .map_err(provider_error)?;
        if let Some(agent_session) = agent_sessions.into_iter().next() {
            return Ok(Some(agent_session));
        }
    }

    Ok(None)
}

async fn agent_project_icon(
    connection: &zbus::Connection,
    read: &GraphReadProxy<'_>,
    agent_session: &str,
) -> Result<Option<String>, ProviderError> {
    let project = match resolve_agent_workspace_project(connection, agent_session).await {
        Some(project) => Some(project),
        None => read
            .get_targets(agent_session, relations::SESSION_PROJECT.name())
            .await
            .map_err(provider_error)?
            .into_iter()
            .next(),
    };

    let Some(project) = project else {
        return Ok(None);
    };

    let properties = read
        .get_properties(&project)
        .await
        .map_err(provider_error)?;
    Ok(first_property(
        &properties,
        &["display-icon", "icon", "icon-name", "symbolicIcon"],
    )
    .map(str::to_owned))
}

async fn resolve_agent_workspace_project(
    connection: &zbus::Connection,
    agent_session: &str,
) -> Option<NodeId> {
    let resolve = GraphResolveProxy::new(connection).await.ok()?;
    let project = resolve
        .resolve(
            agent_session,
            AGENT_SESSION_WORKSPACE_PROJECT_PATH
                .iter()
                .map(|relation| (*relation).to_owned())
                .collect(),
        )
        .await
        .ok()?;

    (project != NONE_STRING).then_some(project)
}

fn window_icon(window: &HashMap<String, String>, fallback: &str) -> String {
    first_property(
        window,
        &[model::Window::ICON.key(), "app-icon", "icon-name"],
    )
    .unwrap_or(fallback)
    .to_owned()
}

fn is_neovim_window(window: &HashMap<String, String>, title: &str) -> bool {
    first_property(window, &["app-id", "app_id", "class", "instance"])
        .is_some_and(|value| value.to_ascii_lowercase().contains("nvim"))
        || title.to_ascii_lowercase().contains("nvim")
        || title.to_ascii_lowercase().contains("neovim")
}

fn first_property<'a>(properties: &'a HashMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| optional_property(properties, key))
}

fn optional_property<'a>(properties: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    let value = properties.get(key)?.trim();
    (!value.is_empty() && value != NONE_STRING).then_some(value)
}

fn provider_error(error: impl std::fmt::Display) -> ProviderError {
    ProviderError::new(error.to_string())
}
