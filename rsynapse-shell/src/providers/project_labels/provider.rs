use std::{future::pending, pin::Pin};

use locus_provider::{NodeId, NodeListBinding, Property, node};
use providers::{CancellationToken, Provider, ProviderError};
use tokio_stream::{Stream, StreamExt, StreamMap};

use crate::schema::{model, relations};

use super::{ProjectLabelView, hydrate::hydrate_project_label};

type ProviderStream<T> = Pin<Box<dyn Stream<Item = Result<T, ProviderError>> + Send>>;
type WatchStream = Pin<Box<dyn Stream<Item = Result<(), ProviderError>> + Send>>;

const WORKSPACE_LABEL_PROPERTIES: &[&str] = &["index", "name", "urgent"];
const PROJECT_LABEL_PROPERTIES: &[&str] = &[
    "display-icon",
    "display-main",
    "display-secondary",
    "icon",
    "name",
    "path",
    "subproj",
    "task",
    "branch",
    "icon-name",
    "symbolicIcon",
];

#[derive(Clone, Debug)]
pub(crate) struct WorkspaceProjectLabel {
    workspace_id: NodeId,
}

pub(crate) fn project_label_for_workspace(workspace_id: NodeId) -> WorkspaceProjectLabel {
    WorkspaceProjectLabel { workspace_id }
}

impl Provider<ProjectLabelView> for WorkspaceProjectLabel {
    type Error = ProviderError;
    type Stream = ProviderStream<ProjectLabelView>;

    fn stream(self, cancellation: CancellationToken) -> Self::Stream {
        Box::pin(async_stream::stream! {
            let connection = match zbus::Connection::session().await {
                Ok(connection) => connection,
                Err(error) => {
                    yield Err(provider_error(error));
                    return;
                }
            };

            let mut workspace_updates = property_updates::<model::Workspace>(
                self.workspace_id.clone(),
                WORKSPACE_LABEL_PROPERTIES,
                cancellation.child_token(),
            );
            let mut project_targets = NodeListBinding::<model::Project>::targets(
                self.workspace_id.clone(),
                relations::PROJECT.name(),
            )
            .filter_kind("project")
            .stream(cancellation.child_token());
            let mut watched_project: Option<NodeId> = None;
            let mut project_cancellation: Option<CancellationToken> = None;
            let mut project_updates: Option<WatchStream> = None;

            match refresh_project_label(
                &connection,
                &self.workspace_id,
                &cancellation,
                &mut watched_project,
                &mut project_cancellation,
                &mut project_updates,
            )
            .await
            {
                Ok(view) => yield Ok(view),
                Err(error) => yield Err(error),
            }

            loop {
                let project_update = async {
                    match project_updates.as_mut() {
                        Some(project_updates) => project_updates.next().await,
                        None => pending().await,
                    }
                };

                tokio::select! {
                    _ = cancellation.cancelled() => break,
                    update = workspace_updates.next() => {
                        match update {
                            Some(Ok(())) => {
                                yield refresh_project_label(
                                    &connection,
                                    &self.workspace_id,
                                    &cancellation,
                                    &mut watched_project,
                                    &mut project_cancellation,
                                    &mut project_updates,
                                ).await;
                            }
                            Some(Err(error)) => yield Err(error),
                            None => break,
                        }
                    }
                    targets = project_targets.next() => {
                        match targets {
                            Some(Ok(_)) => {
                                yield refresh_project_label(
                                    &connection,
                                    &self.workspace_id,
                                    &cancellation,
                                    &mut watched_project,
                                    &mut project_cancellation,
                                    &mut project_updates,
                                ).await;
                            }
                            Some(Err(error)) => yield Err(provider_error(error)),
                            None => break,
                        }
                    }
                    update = project_update => {
                        match update {
                            Some(Ok(())) => {
                                yield refresh_project_label(
                                    &connection,
                                    &self.workspace_id,
                                    &cancellation,
                                    &mut watched_project,
                                    &mut project_cancellation,
                                    &mut project_updates,
                                ).await;
                            }
                            Some(Err(error)) => yield Err(error),
                            None => {
                                project_updates = None;
                                project_cancellation = None;
                                watched_project = None;
                            }
                        }
                    }
                }
            }

            if let Some(token) = project_cancellation {
                token.cancel();
            }
        })
    }
}

async fn refresh_project_label(
    connection: &zbus::Connection,
    workspace_id: &str,
    cancellation: &CancellationToken,
    watched_project: &mut Option<NodeId>,
    project_cancellation: &mut Option<CancellationToken>,
    project_updates: &mut Option<WatchStream>,
) -> Result<ProjectLabelView, ProviderError> {
    let view = hydrate_project_label(connection, workspace_id).await?;

    if view.project_id != *watched_project {
        if let Some(token) = project_cancellation.take() {
            token.cancel();
        }

        *project_updates = view.project_id.clone().map(|project_id| {
            let token = cancellation.child_token();
            *project_cancellation = Some(token.clone());
            property_updates::<model::Project>(project_id, PROJECT_LABEL_PROPERTIES, token)
        });
        *watched_project = view.project_id.clone();
    }

    Ok(view)
}

fn property_updates<Model>(
    node_id: NodeId,
    properties: &'static [&'static str],
    cancellation: CancellationToken,
) -> WatchStream
where
    Model: Send + 'static,
{
    Box::pin(async_stream::stream! {
        let mut streams = StreamMap::new();
        for property in properties {
            streams.insert(
                *property,
                node::<Model>(node_id.clone())
                    .raw_property(Property::<Model, String>::new(property))
                    .stream(cancellation.child_token()),
            );
        }

        loop {
            let update = tokio::select! {
                _ = cancellation.cancelled() => break,
                update = streams.next() => update,
            };

            match update {
                Some((_property, Ok(_value))) => yield Ok(()),
                Some((_property, Err(error))) => yield Err(provider_error(error)),
                None => break,
            }
        }
    })
}

fn provider_error(error: impl std::fmt::Display) -> ProviderError {
    ProviderError::new(error.to_string())
}
