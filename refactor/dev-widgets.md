# dev-widgets Refactor Notes

## Current Role

`dev-widgets` is an internal consumer crate used to exercise framework APIs. It is not a user-facing shell implementation and should be allowed to define role-specific development widgets locally without moving those roles into `shell-core`.

## Public Surface

- Binary entrypoint in `main.rs` starts `ShellApp` with development SCSS and launches `Bar`.
- `primitives/` defines the development bar, source models, window title row component, Relm4 factory-backed row list, provider helpers, and tests.
- `locus_schema` contains development-only generated schema markers, paths, relations, and extension traits produced by Locus codegen.
- `stylesheets/dev-widgets.scss` contains visual styling outside Rust.

## Step-By-Step File Walkthrough

1. `dev-widgets/src/main.rs` - binary startup, stylesheet registration, and selected component launch. Read first to see process-level consumer usage of `shell-core`.
2. `dev-widgets/src/primitives/mod.rs` - module facade and public `Bar` / `BarInit` re-export. Read first to see the local public surface.
3. `dev-widgets/src/primitives/bar.rs` - `BarSources`, `Bar`, layer-shell config application, view bindings, and parent update behavior. Read next because it shows the target macro authoring shape.
4. `dev-widgets/src/primitives/window_title.rs` - `WindowTitleSources`, `WindowTitle`, and row CSS class helpers. Read to see local typed bindings per repeated graph row.
5. `dev-widgets/src/primitives/window_rows.rs` - `WindowTitleRow` factory item and `WindowRows` factory-backed reconciler. Read because this is the repeated-row pattern for development widgets.
6. `dev-widgets/src/primitives/battery.rs` - battery provider and display helpers. Read to check provider ergonomics and local UI derivation.
7. `dev-widgets/src/locus_schema/generated.rs` - generated development schema markers, paths, relations, typed property wrappers, and semantic extension traits. Read to distinguish generated consumer schema code from framework code.
8. `dev-widgets/src/locus_schema/mod.rs` - thin generated-schema facade. Read to confirm no schema-specific helper behavior remains handwritten in the dev widget crate.
9. `dev-widgets/stylesheets/dev-widgets.scss` - external stylesheet. Read to confirm styling remains outside Rust.
10. `dev-widgets/src/primitives/test.rs` - provider and semantic-provider checks. Read last to see what ergonomics are currently verified.

## Internal Structure

- `BarSources` combines Locus graph data and UPower D-Bus data through generic `providers::Provider<T>` sources.
- Battery percentage is subscribed once; the progress bar and label both derive from the same model field so widget code does not manage provider sharing.
- Window rows are ordinary Relm4 child components hosted by a `FactoryVecDeque` and reconciled by node id.
- Development schema code is local to the consumer crate, preserving the framework boundary.

## Behavior Summary

The binary launches a top-layer, top-anchored development bar with automatic exclusive zone. The bar subscribes to selected workspace windows and battery percentage, renders a progress bar and battery label, and reconciles a row component for each window node. Each row binds to its own node title and a derived selected-state provider.

## User Notes

None yet.

## Findings

- Good boundary: role-specific bar config, widget layout, row reconciliation, schema helpers, and CSS all stay in `dev-widgets`, not `shell-core`.
- Risk: `WindowNodeExt::is_selected` subscribes each window row to `paths::SELECTED_WINDOW.target()`. With many rows, this can create repeated selected-window subscriptions. A shared selected-window provider or parent-owned selected id could reduce duplicated work.
- Completed: `WindowRows` now uses a Relm4 `FactoryVecDeque` so row add/remove/reorder operations are managed by the factory instead of manual child removal and re-append.
- Completed: `WorkspacePathExt::windows` is generated from schema collection metadata instead of handwritten relation/kind strings.
- Gap: tests verify provider composition and descriptor shapes, but not actual GTK row reconciliation, layer-shell behavior, stylesheet loading, or live Locus/UPower integration.

## Refactor Plan

1. Keep `dev-widgets` as the place for role-specific development surfaces; do not promote `Bar` or `WindowRows` into framework crates.
2. Completed: migrate `WindowRows` to a Relm4 factory pattern.
3. Completed: move handwritten schema extension traits in `locus_schema/mod.rs` into generated schema output.
4. Completed: split `primitives.rs` into logical `primitives/` module files.
5. Consider sharing selected-window target state across rows if repeated subscriptions show up as real runtime overhead.
6. Add UI/runtime smoke testing only when the project has a practical Wayland/GTK test environment.

## Tests And Verification

- `cargo test -p dev-widgets` passes: 3 unit tests.

## Open Questions

- Should row selection be derived locally in each row for encapsulation, or lifted to the parent to reduce subscriptions?
- Should future repeated-item widgets use this same factory-wrapper pattern around macro-generated child components, or should shell macros grow a first-class factory integration?
