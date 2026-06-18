# Migration Summary

## Current State

- `rsynapse-shell` exists as a compileable Rust playground crate.
- AGS widget and service inventory is captured in `inventory.md`.
- The reachable widget graph is captured in `widget-graph.md`.
- Per-widget docs and native migration proposals are captured under
  `docs/widgets/` and `migration/widgets/`.
- The unused AGS `widgets/rsynapse` launcher/search surface is excluded from
  the port.

## Expected Locus-Shell Update Areas

### Observable Source API

- Adopt the Observable source API in `../SOURCE_API.md` as the target authoring
  model for derived widget data.
- Use `#[shell_macros::observable]` functions for workspace status, window
  indicators, agent status, build status, and system summaries.
- Keep current provider helpers only as a migration bridge until generated
  Locus/D-Bus/common sources are observable-native.

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

- Add or prototype typed sources/clients for:
  - D-Bus ObjectManager collections.
  - D-Bus method calls.
  - MPRIS.
  - StatusNotifier/AppIndicator tray plus DBusMenu.
  - WirePlumber/PipeWire audio endpoint state and actions.
  - NetworkManager and PowerProfiles.
  - BlueZ and UPower-backed Bluetooth battery state.
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

- Copy AGS SCSS into `rsynapse-shell` stylesheets with minimal visual changes.
- Implement binaries or modules for:
  - bar.
  - OSD.
  - agent approvals.
  - optional runtime/request coordinator.
- Replace `ags request` with `rsynapsectl` over a typed session D-Bus request
  service.
- Keep scripts where low risk, then replace stats and side-effect scripts with
  typed Rust sources/helpers.

## First Migration Order

1. Keep `rsynapse-shell` compiling while the Observable source API is introduced.
2. Port CSS files into `rsynapse-shell` and verify stylesheet loading.
3. Build the request bridge and `rsynapsectl` because approvals, hints, and
   theme commands depend on it.
4. Port monitor source/lifecycle helpers for per-monitor bars and
   active-monitor overlays.
5. Port the OSD first: small UI surface, clear source gaps, good test of
   transient streams.
6. Port agent approvals: exercises ObjectManager, Locus joins, dynamic lists,
   keyboard mode, and GtkSourceView.
7. Port the bar incrementally by cluster: clock/battery, workspace strip,
   window indicators, build status, audio/network/Bluetooth/tray/media.
