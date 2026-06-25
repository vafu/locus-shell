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
    pub(super) art_url: String,
    pub(super) playerctl_name: String,
    pub(super) play_pause_icon: &'static str,
    pub(super) can_play_pause: bool,
    pub(super) can_go_next: bool,
    pub(super) can_go_previous: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlayerView {
    metadata: String,
    tooltip: String,
    status: String,
    can_play: bool,
    can_pause: bool,
    can_go_next: bool,
    can_go_previous: bool,
    art_url: String,
    playerctl_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlayerMetadata {
    metadata: String,
    tooltip: String,
    art_url: String,
    playerctl_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlayerPlayback {
    status: String,
    can_play: bool,
    can_pause: bool,
    can_go_next: bool,
    can_go_previous: bool,
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
    let metadata = combine_latest!(
        path.observe_prop_or::<String>("artist", String::new()),
        path.observe_prop_or::<String>("title", String::new()),
        path.observe_prop_or::<String>("album", String::new()),
        path.observe_prop_or::<String>("identity", String::new()),
        path.observe_prop_or::<String>("art-url", String::new()),
        path.observe_prop_or::<String>("playerctl-name", String::new())
            => |(artist, title, album, identity, art_url, playerctl_name)| PlayerMetadata {
                metadata: metadata(Some(artist.as_str()), Some(title.as_str())),
                tooltip: tooltip(
                    Some(identity.as_str()),
                    Some(artist.as_str()),
                    Some(title.as_str()),
                    Some(album.as_str()),
                ),
                art_url,
                playerctl_name,
            },
    );
    let playback = combine_latest!(
        path.observe_prop_or::<String>("playback-status", String::new()),
        path.observe_prop_or::<bool>("can-play", false),
        path.observe_prop_or::<bool>("can-pause", false),
        path.observe_prop_or::<bool>("can-go-next", false),
        path.observe_prop_or::<bool>("can-go-previous", false)
            => |(status, can_play, can_pause, can_go_next, can_go_previous)| PlayerPlayback {
                status,
                can_play,
                can_pause,
                can_go_next,
                can_go_previous,
            },
    );

    combine_latest!(
        metadata,
        playback => |(metadata, playback)| PlayerView {
            metadata: metadata.metadata,
            tooltip: metadata.tooltip,
            status: playback.status,
            can_play: playback.can_play,
            can_pause: playback.can_pause,
            can_go_next: playback.can_go_next,
            can_go_previous: playback.can_go_previous,
            art_url: metadata.art_url,
            playerctl_name: metadata.playerctl_name,
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

    if player.metadata.is_empty() && player.tooltip.is_empty() {
        return MprisView::default();
    }

    let paused = matches!(
        player.status.as_str(),
        "Paused" | "paused" | "Stopped" | "stopped"
    );
    MprisView {
        visible: true,
        metadata: player.metadata.clone(),
        tooltip: player.tooltip.clone(),
        state_class: playback_state_class(player.status.as_str()),
        art_url: player.art_url.clone(),
        playerctl_name: player.playerctl_name.clone(),
        play_pause_icon: if paused { "play_arrow" } else { "pause" },
        can_play_pause: player.can_play || player.can_pause,
        can_go_next: player.can_go_next,
        can_go_previous: player.can_go_previous,
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

fn tooltip(
    identity: Option<&str>,
    artist: Option<&str>,
    title: Option<&str>,
    album: Option<&str>,
) -> String {
    let metadata = metadata(artist, title);
    let album = non_empty(album);
    let identity = non_empty(identity);
    match (identity, non_empty(Some(metadata.as_str())), album) {
        (Some(identity), Some(metadata), Some(album)) => format!("{metadata}\n{album}\n{identity}"),
        (Some(identity), Some(metadata), None) => format!("{metadata}\n{identity}"),
        (Some(identity), None, Some(album)) => format!("{album}\n{identity}"),
        (Some(identity), None, None) => identity.to_owned(),
        (None, Some(metadata), Some(album)) => format!("{metadata}\n{album}"),
        (None, Some(metadata), None) => metadata.to_owned(),
        (None, None, Some(album)) => album.to_owned(),
        (None, None, None) => String::new(),
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
