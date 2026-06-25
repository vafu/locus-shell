mod actual;
mod mock;

use shell_core::source::{Observable, rx::Observable as _};
use shell_rx_macros::combine_latest;

use super::Agent;
use crate::widgets::bar::WindowNode;

pub(in crate::widgets::bar::window_tile) fn agent_for_window(
    window: WindowNode,
) -> Observable<Option<Agent>> {
    combine_latest!(
        actual::agent_for_window(window.clone()),
        mock::agent_for_window(window)
            => |(actual, mock)| actual.or(mock),
    )
    .distinct_until_changed()
    .box_it()
}
