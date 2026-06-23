# AGS Migration

## Scope

Migrate the local AGS shell configuration from `/home/v47/.config/ags` into the
Rust `rsynapse-shell` crate.

## Principles

- Treat AGS as behavior and visual reference, not as the architecture to copy.
- Preserve CSS/SCSS visual styling as closely as practical during migration.
- Keep widget responsibilities documented before porting implementation.
- Track missing framework/provider features once, then link widgets to those
  feature notes instead of duplicating gap descriptions.

## Source

- AGS root: `/home/v47/.config/ags`
- Rust target crate: `rsynapse-shell`

## Directories

- `docs/widgets/`: factual widget responsibilities and visual/design notes.
- `migration/widgets/`: locus-shell-native widget migration proposals.
- `missing-shell-features/`: repeated framework/provider gaps discovered during
  widget analysis.
- `widget-graph.md`: actual reachable AGS widget graph and excluded dead
  islands.

## Top-Level Surfaces

- `bar`: per-monitor top bar and status modules.
- `osd`: monitor-bound on-screen display overlay.
- `agent-approvals`: approval overlay and request UI.
- `app-runtime`: cross-widget setup such as monitor window lifecycle, pomodoro
  DND side effects, request handling, command binding, and theme preparation.

Excluded: the AGS `widgets/rsynapse` launcher/search surface is unused and is
not part of the Rust port scope.

## Migration Status

- [x] Inventory reachable AGS widgets and services.
- [x] Write factual widget docs under `docs/widgets/`.
- [x] Write locus-shell-native proposals under `migration/widgets/`.
- [x] Track repeated framework gaps under `missing-shell-features/`.
- [x] Summarize required locus-shell framework updates.

## Rust Implementation Snapshot

Current `rsynapse-shell` implementation:

- Bottom layer-shell bar using `shell-core` window/style primitives.
- Bar-local observable source modules under `rsynapse-shell/src/widgets/bar`;
  the old top-level `rsynapse-shell/src/sources` layer has been removed.
- Workspace/project strip with project labels, selected/urgent state, and
  agent attention/working/complete styling.
- Selected-workspace window tile strip, including app icon lookup, active/urgent
  styling, agent tile state, context meter, and subagent badge.
- Right-side essentials:
  - clock/date button with `swaync-client -t`.
  - CPU/RAM sysstats `DualIndicator`-style widget.
  - battery via locusfs UPower BAT1.
  - NetworkManager wired and Wi-Fi indicators via locusfs.
  - PipeWire default sink indicator via locusfs.
  - BlueZ/UPower Bluetooth status and keyboard/audio/pointer device groups.
- Material icon asset lookup and AGS-derived bar SCSS are in place.
- Initial OSD window is in place in the main `rsynapse-shell` process; it
  handles PipeWire default sink volume events and local backlight brightness
  changes.
- MPRIS playback controls and album art are backed by
  `../locusfs/plugins/mpris` and `/mpris/player/*` nodes.
- PipeWire audio route selection is available from the volume popover; route
  selection still uses a narrow `pactl set-default-sink` bridge until locusfs
  exposes a write/action node for the default sink.
- PowerProfiles is backed by the locusfs D-Bus projection and a writable
  `ActiveProfile` property; the control is integrated into the CPU/RAM block.
- StatusNotifier tray items are backed by locusfs StatusNotifier and DBusMenu
  plugins; tray menus open in GTK popovers and item activation writes the
  DBusMenu `activate` node.

Known current gaps:

- Per-output/per-monitor bar lifecycle is not restored yet; the current bar is
  still a single-window shape.
- OSD active-monitor rebinding is not implemented yet; the initial Rust OSD
  uses the compositor's default layer-shell monitor placement.
- `ags request` replacement is not implemented. A typed request service plus
  `rsynapsectl` is needed for theme switching, hints mode, approval opening,
  and other cross-widget actions.
- Agent approval overlay is not implemented.
- Build/BzBus widget, locusfs-backed audio route actions, normalized Bluetooth
  dual-battery data, Pomodoro/DND side effects, brightness actions/provider,
  and triggerhappy hints bridge remain to port or normalize.
