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

### Provider Core

- Keep `combine_latest2_stream` / `combine_latest2` as the first derived
  provider primitive.
- Add only concrete stream helpers demanded by migration code:
  - restartable delayed hide for OSD.
  - possibly `combine_latest3` for dense bar DTOs.
- Preserve custom streams as the escape hatch for dynamic collection hydration.

### Shell Macros

- Add multi-field `#[bind(a, b)]` view setters so composed DTO fields are not
  required for every small formatting case.
- Continue improving `#[bind_list]`:
  - keyed row identity.
  - GTK-native or custom list backends when approval or bar lists require
    selection, paging, or custom layout.
  - stable row component ownership for dynamic collections.
- Keep generated code understandable and avoid hiding dependency graphs inside
  component-local derived-field magic.

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

### Provider Backends And Common Providers

- Add or prototype typed providers/clients for:
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
  typed service definitions into `common-providers`.

### Locus Schema And Collection Helpers

- Generate or hand-write consumer schema helpers for semantic collections:
  - workspace rows for an output.
  - active workspace for an output.
  - workspace windows with hydrated window/project/agent data.
  - selected workspace agent sessions for approval auto-open.
  - build invocation summaries.
- Keep graph traversal and hydration in provider/schema helpers, not in view
  components.

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
  typed Rust providers/helpers.

## First Migration Order

1. Finalize provider-core `combine_latest2` and commit it.
2. Keep `rsynapse-shell` compiling as an empty playground.
3. Port CSS files into `rsynapse-shell` and verify stylesheet loading.
4. Build the request bridge and `rsynapsectl` because approvals, hints, and
   theme commands depend on it.
5. Port monitor provider/lifecycle helpers for per-monitor bars and
   active-monitor overlays.
6. Port the OSD first: small UI surface, clear provider gaps, good test of
   transient streams.
7. Port agent approvals: exercises ObjectManager, Locus joins, dynamic lists,
   keyboard mode, and GtkSourceView.
8. Port the bar incrementally by cluster: clock/battery, workspace strip,
   window indicators, build status, audio/network/Bluetooth/tray/media.
