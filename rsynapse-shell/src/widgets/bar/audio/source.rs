use shell_core::{
    locus_path::LocusPath,
    source::{self, Observable, rx::Observable as _},
};
use shell_rx_macros::combine_latest;

use super::{AudioRouteView, AudioView};

const PIPEWIRE_PATH: &str = "pipewire";

#[derive(Clone, Debug, Eq, PartialEq)]
struct SinkSnapshot {
    path: LocusPath,
    state: Option<String>,
    id: u32,
    name: Option<String>,
    description: Option<String>,
    nick: Option<String>,
    form_factor: Option<String>,
    view: AudioView,
}

pub(in crate::widgets::bar) fn audio_status() -> Observable<AudioView> {
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

pub(in crate::widgets::bar) fn audio_routes() -> Observable<Vec<AudioRouteView>> {
    let pipewire = source::root().child(PIPEWIRE_PATH);
    let default_sink = pipewire.child("default").observe_rel("sink");
    let sinks = pipewire.child("sink").as_children().switch_map(|sinks| {
        source::combine_latest_vec(sinks.into_iter().map(sink_snapshot).collect())
    });

    combine_latest!(
        default_sink,
        sinks => |(default_sink, sinks)| route_views(default_sink, sinks),
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
        sink.observe_prop::<String>("name"),
        sink.observe_prop::<String>("description"),
        sink.observe_prop::<String>("nick"),
        sink.observe_prop::<String>("icon-name"),
        sink.observe_prop_or::<bool>("muted", false),
        sink.observe_prop_or::<u32>("volume-percent", 0),
        sink.observe_prop::<String>("form-factor")
            => move |(state, id, name, description, nick, icon, muted, volume, form_factor)| {
                let view = sink_view_from_props(
                    description.as_deref(),
                    icon.as_deref(),
                    muted,
                    volume,
                );
                SinkSnapshot {
                    path: sink.clone(),
                    state,
                    id,
                    name,
                    description: description.clone(),
                    nick,
                    form_factor,
                    view,
                }
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

fn route_views(
    default_sink: Option<LocusPath>,
    mut sinks: Vec<SinkSnapshot>,
) -> Vec<AudioRouteView> {
    let default_path = default_sink.as_ref().map(LocusPath::as_path);
    sinks.sort_by(|left, right| {
        let left_default = Some(left.path.as_path()) == default_path;
        let right_default = Some(right.path.as_path()) == default_path;
        right_default
            .cmp(&left_default)
            .then_with(|| left.id.cmp(&right.id))
            .then_with(|| left.path.as_path().cmp(right.path.as_path()))
    });

    sinks
        .into_iter()
        .filter_map(|sink| {
            let name = non_empty(sink.name.as_deref())?.to_owned();
            let title = non_empty(sink.description.as_deref())
                .or_else(|| non_empty(sink.nick.as_deref()))
                .unwrap_or(name.as_str())
                .to_owned();
            let subtitle = if title == name {
                format!("PipeWire endpoint {}", sink.id)
            } else {
                name.clone()
            };
            let is_default = Some(sink.path.as_path()) == default_path;
            let icon = sink_type_icon(
                sink.form_factor.as_deref(),
                Some(title.as_str()),
                Some(subtitle.as_str()),
            )
            .to_owned();

            Some(AudioRouteView {
                id: sink.id,
                name,
                title,
                subtitle,
                icon,
                is_default,
            })
        })
        .collect()
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
    let description = non_empty(description).unwrap_or("Audio Output");
    let icon = non_empty(icon)
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

fn sink_type_icon(
    form_factor: Option<&str>,
    description: Option<&str>,
    name: Option<&str>,
) -> &'static str {
    let haystack = [form_factor, description, name]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();

    if haystack.contains("headphone") || haystack.contains("headset") {
        "headphones"
    } else if haystack.contains("card") || haystack.contains("pci") {
        "settings_input_component"
    } else {
        "speaker"
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}
