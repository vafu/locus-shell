use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

const PIPEWIRE_PATH: &str = "pipewire";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AudioView {
    pub(super) visible: bool,
    pub(super) icon: String,
    pub(super) tooltip: String,
}

impl Default for AudioView {
    fn default() -> Self {
        Self {
            visible: false,
            icon: "audio-volume-medium-symbolic".to_owned(),
            tooltip: String::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SinkSnapshot {
    path: LocusPath,
    state: Option<String>,
    id: u32,
    view: AudioView,
}

pub(super) fn audio_status() -> Observable<AudioView> {
    let pipewire = source::root().child(PIPEWIRE_PATH);
    let default_sink = pipewire
        .child("default")
        .observe_rel("sink")
        .switch_map(default_sink_view);
    let sinks = pipewire.child("sink").as_children().switch_map(|sinks| {
        source::combine_latest_vec(sinks.into_iter().map(sink_snapshot).collect())
    });

    combine_latest!(
        default_sink,
        sinks => |(default_sink, sinks)| active_sink_view(default_sink, sinks),
    )
    .distinct_until_changed()
    .box_it()
}

fn default_sink_view(sink: Option<LocusPath>) -> Observable<Option<AudioView>> {
    let Some(sink) = sink else {
        return source::once(None);
    };

    combine_latest!(
        sink.observe_prop::<String>("description"),
        sink.observe_prop::<String>("icon-name"),
        sink.observe_prop_or::<bool>("muted", false),
        sink.observe_prop_or::<u32>("volume-percent", 0)
            => |(description, icon, muted, volume)| {
                Some(sink_view_from_props(
                    description.as_deref(),
                    icon.as_deref(),
                    muted,
                    volume,
                ))
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn sink_snapshot(sink: LocusPath) -> Observable<SinkSnapshot> {
    combine_latest!(
        sink.observe_prop::<String>("state"),
        sink.observe_prop_or::<u32>("id", u32::MAX),
        sink.observe_prop::<String>("description"),
        sink.observe_prop::<String>("icon-name"),
        sink.observe_prop_or::<bool>("muted", false),
        sink.observe_prop_or::<u32>("volume-percent", 0)
            => move |(state, id, description, icon, muted, volume)| SinkSnapshot {
                path: sink.clone(),
                state,
                id,
                view: sink_view_from_props(
                    description.as_deref(),
                    icon.as_deref(),
                    muted,
                    volume,
                ),
            },
    )
    .distinct_until_changed()
    .box_it()
}

fn active_sink_view(default_sink: Option<AudioView>, sinks: Vec<SinkSnapshot>) -> AudioView {
    if let Some(default_sink) = default_sink {
        return default_sink;
    }

    let Some(sink) = fallback_sink(&sinks) else {
        return AudioView::default();
    };

    sink.view.clone()
}

fn fallback_sink(sinks: &[SinkSnapshot]) -> Option<&SinkSnapshot> {
    sinks.iter().min_by(|left, right| {
        (
            sink_state_rank(left.state.as_deref().unwrap_or_default()),
            left.id,
        )
            .cmp(&(
                sink_state_rank(right.state.as_deref().unwrap_or_default()),
                right.id,
            ))
            .then_with(|| left.path.as_path().cmp(right.path.as_path()))
    })
}

fn sink_view_from_props(
    description: Option<&str>,
    icon: Option<&str>,
    muted: bool,
    volume: u32,
) -> AudioView {
    let description = description
        .filter(|value| !value.is_empty())
        .unwrap_or("Audio Output");
    let icon = icon
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| audio_icon_name(muted, volume).to_owned());

    AudioView {
        visible: true,
        icon,
        tooltip: audio_tooltip(description, muted, Some(volume)),
    }
}

fn sink_state_rank(state: &str) -> u8 {
    match state {
        "running" => 0,
        "idle" => 1,
        "suspended" => 2,
        _ => 3,
    }
}

fn audio_tooltip(description: &str, muted: bool, volume: Option<u32>) -> String {
    match (muted, volume) {
        (true, Some(volume)) => format!("{description} - muted ({volume}%)"),
        (true, None) => format!("{description} - muted"),
        (false, Some(volume)) => format!("{description} - {volume}%"),
        (false, None) => description.to_owned(),
    }
}

fn audio_icon_name(muted: bool, volume: u32) -> &'static str {
    if muted || volume == 0 {
        "audio-volume-muted-symbolic"
    } else if volume < 33 {
        "audio-volume-low-symbolic"
    } else if volume < 67 {
        "audio-volume-medium-symbolic"
    } else {
        "audio-volume-high-symbolic"
    }
}
