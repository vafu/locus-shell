use std::{future::Future, marker::PhantomData};

use locus_provider::{
    KindFilteredNodeListBinding, NodeId, NodeListBinding, NodePropertyBinding, NodeRef, Path,
    TargetBinding,
};
use providers::{Provider, ProviderContext, ProviderExt, ProviderSender, SwitchMapError};

pub mod generated;

pub use generated::{model, paths};

pub trait WindowNodeExt {
    fn title(&self) -> NodePropertyBinding<model::Window, String>;
    fn is_selected(&self) -> impl Provider<bool>;
}

impl WindowNodeExt for NodeRef<model::Window> {
    fn title(&self) -> NodePropertyBinding<model::Window, String> {
        self.property(model::Window::TITLE)
    }

    fn is_selected(&self) -> impl Provider<bool> {
        let node = self.id().to_owned();

        paths::SELECTED_WINDOW
            .target()
            .map(move |selected| selected == node)
    }
}

pub trait WorkspacePathExt {
    fn windows(self) -> WorkspaceWindowsProvider<TargetBinding<model::Workspace>>;
}

impl WorkspacePathExt for Path<model::Workspace> {
    fn windows(self) -> WorkspaceWindowsProvider<TargetBinding<model::Workspace>> {
        WorkspaceWindowsProvider::new(self.target())
    }
}

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
    type Error = SwitchMapError<P::Error, locus_provider::WatchError>;

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
