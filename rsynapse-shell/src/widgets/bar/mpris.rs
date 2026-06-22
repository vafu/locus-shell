use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

const MPRIS_PLAYERS_PATH: &str = "mpris/player";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct MprisView {
    pub(super) visible: bool,
    pub(super) metadata: String,
    pub(super) tooltip: String,
    pub(super) state_class: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlayerView {
    metadata: String,
    status: String,
    can_play: bool,
}

pub(super) fn mpris_status() -> Observable<MprisView> {
    source::root()
        .child(MPRIS_PLAYERS_PATH)
        .as_children()
        .switch_map(|players| source::combine_latest_vec(players.into_iter().map(player).collect()))
        .map(selected_player)
        .distinct_until_changed()
        .box_it()
}

fn player(path: LocusPath) -> Observable<PlayerView> {
    combine_latest!(
        path.observe_prop::<String>("artist"),
        path.observe_prop::<String>("title"),
        path.observe_prop_or::<String>("playback-status", String::new()),
        path.observe_prop_or::<bool>("can-play", false)
            => |(artist, title, status, can_play)| PlayerView {
                metadata: metadata(artist.as_deref(), title.as_deref()),
                status,
                can_play,
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn selected_player(players: Vec<PlayerView>) -> MprisView {
    let Some(player) = players
        .iter()
        .find(|player| player.status == "Playing")
        .or_else(|| players.iter().find(|player| player.can_play))
    else {
        return MprisView::default();
    };

    if player.metadata.is_empty() {
        return MprisView::default();
    }

    MprisView {
        visible: true,
        metadata: player.metadata.clone(),
        tooltip: player.metadata.clone(),
        state_class: playback_state_class(player.status.as_str()),
    }
}

fn metadata(artist: Option<&str>, title: Option<&str>) -> String {
    match (non_empty(artist), non_empty(title)) {
        (Some(artist), Some(title)) => format!("{artist} - {title}"),
        (None, Some(title)) => title.to_owned(),
        (Some(artist), None) => artist.to_owned(),
        (None, None) => String::new(),
    }
}

fn playback_state_class(status: &str) -> &'static str {
    match status {
        "Playing" | "playing" => "playing",
        "Paused" | "paused" => "paused",
        "Stopped" | "stopped" => "stopped",
        _ => "",
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}
