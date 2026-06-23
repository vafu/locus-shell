# Migration Summary

## Current State

- `rsynapse-shell` exists as a compileable Rust playground crate.
- AGS widget and service inventory is captured in `inventory.md`.
- The reachable widget graph is captured in `widget-graph.md`.
- Per-widget docs and native migration proposals are captured under
  `docs/widgets/` and `migration/widgets/`.
- The unused AGS `widgets/rsynapse` launcher/search surface is excluded from
  the port.
- The bar migration is partially implemented in Rust:
  - bottom layer-shell bar and AGS-derived SCSS.
  - workspace/project strip and selected-workspace window tiles.
  - project labels with agent attention/working/complete state.
  - plain/agent/neovim window tile states.
  - clock, battery, NetworkManager wired/Wi-Fi, PipeWire default sink,
    CPU/RAM sysstats, and BlueZ/UPower Bluetooth groups.
  - PipeWire audio route popover with route selection through a temporary
    `pactl set-default-sink` bridge.
  - MPRIS player controls, metadata, and album art through locusfs
    `/mpris/player/*` nodes.
  - PowerProfiles display and cycling through the locusfs D-Bus projection.
  - StatusNotifier tray icons and DBusMenu popovers through locusfs plugins.
  - source modules live beside their widgets and compose
    `shell_core::source::Observable` values.
- The current bar is not yet the final AGS shape:
  - no per-monitor/per-output window lifecycle.
  - no build/BzBus widget.
  - audio route selection still shells out to `pactl` instead of writing a
    locusfs action/property node.
  - Bluetooth dual battery data is still a shell-side approximation; locusfs
    should expose a normalized device battery model.
  - some visual parity work remains for exact AGS sizing/spacing.

## Expected Locus-Shell Update Areas

### Observable Source API

- `shell_core::source` now exposes the Observable-first Locus source layer used
  by the migrated bar widgets.
- Bar widget sources now live beside their widgets and compose
  `shell_core::source::Observable` values instead of depending on a top-level
  `rsynapse-shell/src/sources` layer.
- Still pending:
  - `#[shell_macros::observable]`, `#[observe(...)]`, and `#[inject]` for
    ergonomic derived sources.
  - descriptor-keyed sharing for fanout-heavy sources where repeated
    subscriptions are expected.
  - replacing repeated handwritten source composition with macro-authored
    observable functions once the macro surface is ready.

### Shell Macros

- Add multi-field `#[bind(a, b)]` view setters so composed DTO fields are not
  required for every small formatting case.
- Add `#[shell_macros::observable]`, `#[observe(...)]`, and `#[inject]` support
  so custom data composition does not require custom source structs.
- Continue improving `#[bind_list]`:
  - keyed row identity.
  - GTK-native or custom list backends when approval or bar lists require
    selection, paging, or custom layout.
  - stable row component ownership for dynamic collections.
- Keep generated code understandable and avoid hiding dependency graphs inside
  component-local manual wiring.

### Shell Core

- Keep `shell-core` generic; do not add bar, OSD, approval, or product-specific
  shell policy.
- Candidate generic primitives after migration pressure is proven:
  - monitor topology provider or helper types.
  - layer-shell monitor rebinding/reconfiguration helpers.
  - dynamic keyboard mode update helper.
  - reusable window lifecycle handles for consumer-managed multi-monitor
    windows.
- CSS registration and watcher APIs are already close to the needed direction;
  migration-specific SCSS preprocessing should stay in `rsynapse-shell`.

### Source Backends And Common Sources

- Implemented through locusfs-backed or local observable sources in
  `rsynapse-shell`:
  - UPower BAT1 battery state and AGS-compatible icon mapping.
  - NetworkManager wired and Wi-Fi indicators.
  - PipeWire default sink display state.
  - PipeWire sink list and route popover.
  - PowerProfiles active profile and cycling through a writable D-Bus property.
  - MPRIS metadata, album art, playback state, and playback commands.
  - StatusNotifier tray item discovery plus DBusMenu menu rendering and item
    activation.
  - BlueZ/UPower Bluetooth status and device groups.
  - CPU/RAM sysstats from local system data.
- Still pending:
  - locusfs write/action path for PipeWire default-sink changes.
  - full PipeWire/WirePlumber route grouping metadata.
  - normalized Bluetooth HID/GATT dual battery projection.
  - Pomodoro.
  - brightness/backlight.
- Keep service-specific display policy in `rsynapse-shell`; promote only stable
  typed service definitions into common source crates.

### Locus Schema And Collection Helpers

- Generate consumer schema helpers for semantic collections:
  - workspace rows for an output.
  - active workspace for an output.
  - workspace windows with hydrated window/project/agent data.
  - selected workspace agent sessions for approval auto-open.
  - build invocation summaries.
- Keep graph traversal and hydration in generated schema/source helpers, not in
  view components.

### Shell Consumer Work

- AGS SCSS has been copied into `rsynapse-shell` stylesheets and the bar is
  partially implemented.
- Still implement modules for:
  - agent approvals.
  - optional runtime/request coordinator.
  - per-monitor/per-output bar lifecycle.
  - build/BzBus status.
- Replace `ags request` with `rsynapsectl` over a typed session D-Bus request
  service.
- Keep scripts where low risk, then replace stats and side-effect scripts with
  typed Rust sources/helpers.

## Updated Migration Order

Completed:

1. Kept `rsynapse-shell` compiling while the Observable source API was
   introduced.
2. Ported CSS files into `rsynapse-shell` and verified stylesheet loading.
3. Built the first bar pass: workspace/project strip, selected-window strip,
   clock, battery, NetworkManager, PipeWire, CPU/RAM, and Bluetooth.
4. Added OSD in the main shell process.
5. Added MPRIS, audio route popover, PowerProfiles, StatusNotifier tray, and
   DBusMenu activation through locusfs-backed sources/plugins.

Next:

1. Build the request bridge and `rsynapsectl` because approvals, hints, and
   theme commands depend on it.
2. Port monitor source/lifecycle helpers for per-monitor bars and
   active-monitor overlays.
3. Port agent approvals: exercises ObjectManager, Locus joins, dynamic lists,
   keyboard mode, and GtkSourceView.
4. Finish the remaining bar gaps: build/BzBus, locusfs-native audio route
   actions, normalized Bluetooth batteries, and exact visual parity.
5. Replace local brightness/backlight and remaining side-effect bridges with
   locusfs or typed request/action sources where appropriate.
