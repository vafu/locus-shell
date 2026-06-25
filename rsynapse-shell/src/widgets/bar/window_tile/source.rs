use shell_core::source::{Observable, rx::Observable as _};
use shell_rx_macros::combine_latest;

use crate::desktop_icon;

use super::super::WindowNode;
use super::agent::{self, Agent};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::widgets::bar) enum Kind {
    Plain,
    Neovim,
    Agent(Agent),
}

impl Default for Kind {
    fn default() -> Self {
        Self::Plain
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::widgets::bar) struct ViewModel {
    pub(super) kind: Kind,
    pub(super) icon: String,
    pub(super) tooltip: String,
    pub(super) active: bool,
    pub(super) urgent: bool,
}

pub(super) fn window_tile_vm(window: WindowNode) -> Observable<Option<ViewModel>> {
    let window_id = window.observe_prop_or::<u32>("id", u32::MAX);
    let title = window
        .observe_prop_or::<String>("title", String::new())
        .map(non_empty);
    let app_id = window
        .observe_prop_or::<String>("app-id", String::new())
        .map(non_empty);
    let active = window.observe_prop_or::<bool>("selected", false);
    let urgent = window.observe_prop_or::<bool>("urgent", false);
    let agent = agent::agent_for_window(window);

    combine_latest!(
        window_id,
        title,
        app_id,
        active,
        urgent,
        agent
            => move |(_window_id, title, app_id, active, urgent, agent)| {
                let app_id = app_id.unwrap_or_default();
                let title = title.unwrap_or_default();
                Some(ViewModel {
                    tooltip: window_tooltip(&title, agent.as_ref()),
                    kind: window_kind(&title, agent),
                    icon: desktop_icon::icon_for_app_id(&app_id),
                    active,
                    urgent,
                })
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn window_kind(title: &str, agent: Option<Agent>) -> Kind {
    if let Some(agent) = agent {
        return Kind::Agent(agent);
    }

    let title = title.to_ascii_lowercase();
    if title.contains("nvim") || title.contains("neovim") {
        Kind::Neovim
    } else {
        Kind::Plain
    }
}

fn window_tooltip(title: &str, agent: Option<&Agent>) -> String {
    let Some(agent) = agent else {
        return title.to_owned();
    };

    let mut lines = vec![title.to_owned(), format!("Agent: {:?}", agent.state)];
    if agent.context_pct > 0 {
        lines.push(format!("Context: {}%", agent.context_pct));
    }

    lines.join("\n")
}

pub(super) fn non_empty(value: String) -> Option<String> {
    let value = value.trim().to_owned();
    (!value.is_empty()).then_some(value)
}
