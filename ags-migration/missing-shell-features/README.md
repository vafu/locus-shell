# Missing Shell Features

Feature notes in this directory describe repeated gaps in `locus-shell` that
block or complicate migration from AGS.

Each note should include:

- Widgets that need the feature.
- Current workaround, if any.
- Proposed locus-shell API direction.
- Whether the gap belongs in `shell-core`, `shell-macros`, `providers`, a
  provider backend, or `rsynapse-shell`.

## Current Migration Status

Partially addressed in the current `rsynapse-shell` bar:

- Observable source composition for widget-local source modules.
- Dynamic child lists for workspace/window rows.
- Locusfs-backed D-Bus object projections for UPower, NetworkManager, PipeWire,
  and BlueZ.
- Material icon asset lookup.
- Custom level-meter drawing for CPU/RAM.
- Bluetooth/BlueZ battery display.
- Network and PipeWire display state.

Still high-priority gaps:

- Consumer request service and `rsynapsectl` replacement for `ags request`.
- OSD event stream and active-monitor overlay lifecycle.
- Per-monitor/per-output bar lifecycle.
- Agent approval overlay, approval response methods, and auto-open runtime
  policy.
- StatusNotifier tray and DBusMenu.
- MPRIS.
- PowerProfiles display/actions.
- Audio route popover/actions.
- Brightness provider/actions.
- Pomodoro provider and side-effect runner.
- Keyboard hints bridge.
