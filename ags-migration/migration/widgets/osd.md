# OSD Migration Proposal

## AGS Sources Reviewed

- `/home/v47/.config/ags/widgets/osd/index.tsx`
- `/home/v47/.config/ags/widgets/osd/OSD.tsx`
- `/home/v47/.config/ags/services/brightness.tsx`
- `/home/v47/.config/ags/style/osd.scss`
- `/home/v47/.config/ags/app.ts`
- `/home/v47/.config/ags/services/locus.ts`
- `/home/v47/.config/ags/widgets/bar/audio_route.tsx`

## Native Structure

Implement this as an external consumer binary, for example `rsynapse-osd`, not inside `shell/core`. The binary owns the OSD role, placement, event policy, and CSS. `shell-core` should only provide the GTK app, stylesheet registration, provider runtime, and layer-shell window creation primitives described in `PLAN.md`.

Initial component tree:

```text
OsdApp
└── OsdWindow
    └── gtk::Revealer
        └── gtk::Box.osd-shell
            ├── gtk::Image
            └── gtk::LevelBar
```

`OsdWindow` should create one bottom-anchored overlay layer surface whose monitor follows the active Locus output. It should use `Layer::Overlay`, no exclusive zone, no focus, bottom anchor, namespace/name `OSD`, and CSS class `OSD`.

## Initial Models

Start with a small event model that preserves behavior without preserving AGS's RxJS shape:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum OsdPayload {
    Hidden,
    Level {
        value: f64,
        icon_name: String,
        source: OsdSource,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsdSource {
    Brightness,
    Audio,
}

#[shell_macros::model]
pub struct OsdWindowModel {
    #[source(active_monitor_provider())]
    pub active_monitor: Option<MonitorRef>,

    #[source(osd_payload_provider())]
    pub payload: OsdPayload,

    pub window_visible: bool,
}
```

If macro support for timer-derived fields is not ready, keep `window_visible` as local Relm4 state updated by explicit messages rather than a provider-bound field.

The view bindings should derive:

- `revealer.reveal_child` from `payload != OsdPayload::Hidden`.
- `image.icon_name` from the latest level payload, falling back to an empty or neutral icon while hidden.
- `levelbar.value` from the latest level payload, falling back to `0.0` while hidden.
- `window.visible` from `window_visible`, with a short hide delay so the revealer crossfade can finish.

## Provider And Stream Dependencies

Required providers:

- `BrightnessProvider`: normalized display brightness level, driven by the backlight device. Initial implementation can mirror the AGS behavior by reading `brightnessctl max/get` and watching `/sys/class/backlight/<device>/brightness`, but the provider should expose a typed `Provider<f64>`.
- `DefaultSpeakerProvider`: current default audio endpoint and changes to its volume and volume icon. Prefer a WirePlumber/PipeWire-backed provider or a focused D-Bus provider instead of carrying AstalWp concepts into Rust UI code.
- `ActiveMonitorProvider`: maps Locus selected output connector to the GTK monitor used by the layer surface.
- `OsdPayloadProvider`: merges brightness and audio events into a single `Provider<OsdPayload>` stream.

Stream behavior:

```text
brightness level changes -> Level { source: Brightness, value, icon }
audio volume/icon changes -> Level { source: Audio, value, icon }
merged level event -> emit Level immediately
merged level event -> schedule Hidden after 1s
new level event before timeout -> cancel previous hide and restart timer
Hidden -> set reveal_child false immediately
Hidden -> keep window visible briefly for crossfade, then hide surface
```

The hide timer is part of the OSD policy, so it belongs in the consumer widget or a consumer-owned provider. If multiple level sources emit during startup, consider suppressing the first snapshot unless it corresponds to an actual user-visible change.

## D-Bus, Locus, And Provider Dependencies

Locus dependencies:

- Selected output connector from the Locus graph, equivalent to AGS `locus.selectedOutputProperty$('connector')`.
- A monitor resolver that maps connector strings to the current GTK monitor list.

D-Bus/system dependencies:

- Backlight device discovery and change notification from `/sys/class/backlight`, `brightnessctl`, or a future typed display-brightness provider.
- WirePlumber/PipeWire default sink volume and icon/semantic volume state. If icon names are not available from the backend, derive the symbolic icon from volume and mute state in the provider.

Provider dependencies:

- `providers::Provider<T>` for all async value streams.
- Shared latest/fanout support if audio endpoint identity, volume, mute state, and icon are exposed as separate fields from one upstream connection.
- A thin stream helper for switch/restart timer behavior, or a local Relm4 command task if provider combinators are not ready.

## CSS

Place OSD styling in the consumer stylesheet. Preserve the AGS visual contract:

- `window.OSD box.osd-shell`
- `border-radius: 24px`
- OSD background mapped to the theme sidebar background
- elevation shadow matching `elevation-2`
- `padding: 13px 16px`
- `margin: 50px`
- icon padding `12px`
- level bar width around `100px`
- level bar block minimum height `2rem`

## Missing Shell Features

- [Layer window monitor rebinding](../../missing-shell-features/layer-window-monitor-rebinding.md): OSD needs one overlay surface whose `Gdk.Monitor` follows the active Locus output.
- [Transient timer stream combinators](../../missing-shell-features/transient-timer-stream-combinators.md): OSD needs merge plus restartable delayed hide behavior.
- [Brightness provider](../../missing-shell-features/brightness-provider.md): normalized screen brightness should be exposed as a typed provider.
- [Audio endpoint provider](../../missing-shell-features/audio-endpoint-provider.md): default speaker volume, mute state, and symbolic icon should be exposed without depending on AGS/Astal concepts.
- [Monitor list provider](../../missing-shell-features/monitor-list-provider.md): active connector-to-monitor resolution needs a typed source for current GTK monitor topology.

## Open Questions

- Should the migrated OSD suppress initial provider snapshots so it appears only after user-initiated brightness or volume changes?
- Should audio OSD react to mute-only changes even when the numeric volume does not change?
- Should brightness writes be part of the same provider family, or should the OSD remain display-only and let keybinding handlers own mutation?
