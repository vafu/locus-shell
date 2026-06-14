use providers::{Provider, SubscriptionGroup};
use relm4::ComponentSender;

/// A model whose source context can be driven by another provider.
///
/// This is used by `shell-macros` for nested source models such as a workspace
/// row whose current project may change over time.
pub trait SourceModel: Sized + Send + 'static {
    type Context: Clone + Send + 'static;
    type Msg: Send + 'static;

    fn from_default_context() -> Self
    where
        Self::Context: Default;

    fn update_source_model(&mut self, msg: Self::Msg);

    fn start_source_model<Component, Source, Map>(
        source: Source,
        sender: ComponentSender<Component>,
        map: Map,
    ) -> SubscriptionGroup
    where
        Component: relm4::Component + 'static,
        Component::Input: Send,
        Component::Output: Send,
        Component::CommandOutput: Send,
        Source: Provider<Self::Context>,
        Map: Fn(Self::Msg) -> Component::Input + Clone + Send + 'static;
}
