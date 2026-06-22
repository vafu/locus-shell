# Bar Migration Proposal

## Source Files Reviewed

- `/home/v47/.config/ags/app.ts`
- `/home/v47/.config/ags/widgets/bar/*`
- `/home/v47/.config/ags/services/locus.ts`
- `/home/v47/.config/ags/services/locus.generated.ts`
- `/home/v47/.config/ags/services/workspace-status-provider.ts`
- `/home/v47/.config/ags/services/bzbus.ts`
- `/home/v47/.config/ags/services/agent*.ts`
- `/home/v47/.config/ags/services/bluetooth/*`
- `/home/v47/.config/ags/services/brightness.tsx`
- `/home/v47/.config/ags/style/bar.scss`
- `/home/v47/.config/ags/widgets/materialicon.tsx`
- `/home/v47/.config/ags/widgets/circularstatus.ts`

## Native Structure

Implement this outside the framework boundary as a `locus-bar` or
`rsynapse-shell` consumer binary. `shell-core` should only create the generic
layer window from local bar config: bottom/left/right anchors, exclusive auto
zone, no keyboard focus, namespace `bar`, and per-monitor lifecycle ownership.

Suggested component tree:

- `BarApp`: owns monitor enumeration and one `BarWindow` controller per output.
- `BarWindow`: layer-shell window and centerbox layout.
- `WorkspaceStrip`: left workspace/project buttons for one connector.
- `WorkspaceButton`: collapsed icon, title reveal, hint badge, and optional
  agent child buttons.
- `ActiveWindowStrip`: center window tiles for the active workspace.
- `WindowTile`: plain or agent-specific window indicator.
- `BuildStatus`: bzbus/build invocation summary.
- `MediaStatus`: MPRIS metadata plus audio route entry point.
- `SystemCluster`: tray, power profile, Bluetooth, audio, network, battery.
- `ClockButton`: clock/date and notification-center toggle.
- Shared UI: `PanelButtonGroup`, `BadgeOverlay`, `MaterialIcon`, `LevelMeter`.

Initial implementation status:

- `rsynapse-shell` now creates a bottom layer-shell bar using `shell-core`
  layer-window and stylesheet primitives.
- The workspace/project strip is implemented with widget-local observable
  sources, project labels, selected/urgent state, and agent
  attention/working/complete styling.
- The selected-workspace window strip is implemented with sorted window tiles,
  desktop icon lookup, active/urgent state, agent tile state, context meter, and
  subagent badge.
- The right-side essentials currently implemented are clock/date, CPU/RAM,
  battery, NetworkManager wired/Wi-Fi, PipeWire default sink, and Bluetooth
  device groups.
- Still missing from the AGS bar shape: per-output/per-monitor lifecycle,
  PowerProfiles, StatusNotifier tray, MPRIS, build/BzBus, full audio route
  popover/actions, and exact visual parity polish.

## Target Models

Use derived DTO providers for list-heavy regions. Bind those DTOs through one
field per component rather than wiring every graph property directly into the
view. The current handwritten implementation already follows this direction by
exposing widget-local `ViewModel` structs from observable source functions; the
macro sketch below is the target authoring shape once source macros are ready.

```rust
#[shell_macros::model]
pub struct BarWindowModel {
    #[source(bar_providers::workspaces_for_output(connector.clone()))]
    pub workspaces: Vec<WorkspaceModel>,

    #[source(bar_providers::active_workspace_for_output(connector.clone()))]
    pub active_workspace: Option<WorkspaceModel>,

    #[source(bar_providers::media_view())]
    pub media: MediaView,

    #[source(bar_providers::build_status())]
    pub build: BuildStatusView,

    #[source(bar_providers::system_status())]
    pub system: SystemStatusView,

    #[source(bar_providers::clock())]
    pub clock: ClockView,
}

#[shell_macros::model]
pub struct WorkspaceButtonModel {
    #[source(bar_providers::workspace_button(workspace.clone()))]
    pub workspace: WorkspaceModel,
}

#[shell_macros::model]
pub struct WindowTileModel {
    #[source(bar_providers::window_tile(window.clone()))]
    pub window: WorkspaceWindowIndicatorModel,
}

#[shell_macros::model]
pub struct AgentButtonModel {
    #[source(agent_providers::session_status(session_id.clone()))]
    pub status: AgentStatus,

    #[source(agent_providers::session_project_branch(session_id.clone()))]
    pub branch: String,

    #[source(agent_providers::subagent_count(session_id.clone()))]
    pub subagent_count: u32,

    #[source(selected_agent_session())]
    pub selected_agent_session: Option<LocusPath>,
}
```

Candidate DTOs: `WorkspaceModel`, `WorkspaceWindowIndicatorModel`,
`BuildStatusView`, `MediaView`, `BluetoothStatusView`, `AudioRouteView`,
`NetworkStatusView`, `BatteryStatusView`, `PowerProfileView`, `ClockView`, and
`SystemStatusView`.

## Essential Panel Phase

Before the full AGS bar is complete, port the right-side essentials as a focused
system cluster phase. The concrete plan lives in
[System Indicators Migration Plan](system-indicators.md).

Completed in the Rust bar:

- clock/date button, including the `swaync-client -t` click action.
- CPU/memory dual level bar, matching the AGS `DualIndicator` shape and
  `memory` Material icon.
- battery indicator through locusfs UPower BAT1, including AGS-compatible
  symbolic icons derived from status and percentage.
- NetworkManager Wi-Fi indicator with SSID tooltip and wired Ethernet
  indicator.
- PipeWire default output indicator with volume/mute icon and output
  description tooltip.
- PipeWire audio route popover in the right cluster, backed by locusfs sink
  nodes and a narrow `pactl set-default-sink` action until locusfs exposes an
  action path.
- MPRIS metadata, album art, playback state, and previous/play-pause/next
  controls through live `/mpris/player/*` locusfs player nodes.
- Bluetooth status and grouped keyboard/audio/pointer device indicators through
  locusfs BlueZ/UPower data.

Remaining right-side bar work:

- PowerProfiles active profile indicator and profile cycling once the
  method/command path is available.
- StatusNotifier tray and DBusMenu.
- PipeWire route grouping metadata and locusfs-backed default-sink action.
- final AGS sizing/spacing parity.

## Providers And Stream Composition

Create consumer-owned provider modules rather than adding bar policy to
`shell-core`.

- `bar_providers::workspaces_for_output(connector)`: Locus output source list,
  workspace properties, workspace project relation/properties, selected
  workspace, workspace windows, and agent session state. This is the native
  replacement for `WorkspaceStatusProvider`.
- `bar_providers::active_workspace_for_output(connector)`: derived from
  `workspaces_for_output`; used by `ActiveWindowStrip`.
- `bar_providers::workspace_windows(workspace)`: workspace window list,
  window icon lookup, selected window/column, urgency, and optional
  window-agent-session data.
- `agent_providers`: AgentDBus ObjectManager bootstrap, session property
  updates, elicitation signals, response methods, subagent count from Locus.
- `build_providers::bzbus_status`: Locus `build-invocation` subject discovery,
  property hydration, property change updates, sorting, and status formatting.
- `desktop_providers`: MPRIS, StatusNotifier tray, PowerProfiles, NetworkManager,
  UPower battery, BlueZ/Bluetooth battery, WirePlumber/PipeWire, and clock.
- `process_providers`: narrow command providers for `scripts/sysstats.sh`,
  `pw-dump`, `swaync-client -t`, and any future shell commands.

The current implementation keeps these providers as widget-local source modules
under `rsynapse-shell/src/widgets/bar`. New bar work should continue that shape:
public files define view models and observable source functions, while GTK
components consume already-shaped values.

Use Observable source functions for composition such as selected workspace plus
monitor workspace list, status plus project branch, and default speaker plus
icon metadata. Dynamic collections where the upstream list changes and each
item needs hydration should also be expressed as observable source functions
once the macro API from `../../../SOURCE_API.md` exists. Those functions should
read as typed data composition and hide watcher loops, switch/restart plumbing,
subscription wiring, and fanout policy.

Use shared latest sources for expensive or fanout-heavy inputs: AgentDBus
sessions map, Locus selected nodes, per-output workspace list, per-window agent
data, app-id-to-icon lookup, Bluetooth device list, PipeWire sink props, bzbus
properties store, and clock ticks.

Observable merging should live in consumer source functions. Relm4 components
should receive already-shaped DTOs and handle only view-local interactions such
as hover reveal, popover open state, selecting an audio sink, toggling
Bluetooth, cycling power profile, responding to elicitation, and toggling
notifications.

## D-Bus And Locus Dependencies

Locus graph dependencies:

- output -> workspace sources by connector.
- selected workspace, selected window, and selected agent session paths.
- workspace properties: `index`, `name`, `urgent`.
- workspace -> project targets and project display properties.
- workspace -> window sources.
- window properties: `app-id`, `column`, `row`, `tile-width`, `tile-height`,
  `urgent`.
- window -> app-instance -> agent-session path.
- app-instance `icon` property.
- agent-session -> workspace/project and direct project paths.
- agent-session -> subagent-session targets.
- build-invocation discovery and properties.

D-Bus/provider dependencies:

- `io.github.AgentDBus`: sessions, status properties, elicitation signals,
  response methods, and stats if later surfaced.
- `org.freedesktop.UPower`, BlueZ, and Bluetooth GATT battery paths.
- NetworkManager or equivalent for wired and Wi-Fi state.
- PowerProfiles daemon for active profile and cycling.
- MPRIS players for artist/title/playback state.
- StatusNotifier/AppIndicator tray with DBusMenu support.
- WirePlumber/PipeWire for default output, volume icon, route list, and setting
  default sink.
- Local process commands for sysstats, PipeWire dump fallback, and
  notification-center toggle.

## Missing Shell Features

- [Per-monitor layer surface lifecycle](../../missing-shell-features/per-monitor-layer-surface-lifecycle.md)
- [Dynamic provider collections](../../missing-shell-features/dynamic-provider-collections.md)
- [Typed Locus collection hydration](../../missing-shell-features/typed-locus-collection-hydration.md)
- [Shared source fanout keys](../../missing-shell-features/shared-source-fanout-keys.md)
- [Observable source composition](../../missing-shell-features/observable-source-composition.md)
- [StatusNotifier tray provider](../../missing-shell-features/statusnotifier-tray-provider.md)
- [MPRIS provider](../../missing-shell-features/mpris-provider.md)
- [WirePlumber audio provider](../../missing-shell-features/wireplumber-audio-provider.md)
- [Bluetooth and BlueZ battery providers](../../missing-shell-features/bluetooth-bluez-battery-providers.md)
- [Network and power profile providers](../../missing-shell-features/network-power-profile-providers.md)
- [Material icon asset provider](../../missing-shell-features/material-icon-asset-provider.md)
- [Custom level meter widget](../../missing-shell-features/custom-level-meter-widget.md)
- [Popover and hover reveal bindings](../../missing-shell-features/popover-hover-reveal-bindings.md)
- [Command action provider](../../missing-shell-features/command-action-provider.md)

## Open Questions

- Whether monitor identity should use connector strings directly or a typed
  output id provider shared with Locus output nodes.
- Whether build invocation status should remain Locus-backed only or gain a
  dedicated bzbus provider crate once the data contract stabilizes.
- Whether Material Symbols should be vendored, generated at build time, or
  resolved through a runtime icon cache.
