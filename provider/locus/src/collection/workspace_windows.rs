use std::{future::Future, marker::PhantomData};

use providers::{Provider, ProviderContext, ProviderExt, ProviderSender, SwitchMapError};

use crate::{
    NodeId, WatchError,
    collection::binding::{KindFilteredNodeListBinding, NodeListBinding},
    model,
};

/// Provider returned by semantic workspace window collection helpers.
#[derive(Clone, Debug)]
pub struct WorkspaceWindowsProvider<P> {
    workspace: P,
    _marker: PhantomData<fn() -> model::Window>,
}

impl<P> WorkspaceWindowsProvider<P> {
    pub fn new(workspace: P) -> Self {
        Self {
            workspace,
            _marker: PhantomData,
        }
    }
}

impl<P> Provider<Vec<NodeId>> for WorkspaceWindowsProvider<P>
where
    P: Provider<NodeId>,
{
    type Error = SwitchMapError<P::Error, WatchError>;

    fn run(
        self,
        context: ProviderContext,
        sender: ProviderSender<Vec<NodeId>>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.workspace
            .switch_map(workspace_windows)
            .run(context, sender)
    }
}

fn workspace_windows(workspace: NodeId) -> KindFilteredNodeListBinding<model::Window> {
    NodeListBinding::sources(workspace, "workspace").filter_kind("window")
}
