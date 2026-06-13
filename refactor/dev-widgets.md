# dev-widgets Refactor Notes

## Current Role

`dev-widgets` is an internal consumer crate used to exercise framework APIs. It is not a user-facing shell implementation and should be allowed to define role-specific development widgets locally without moving those roles into `shell-core`.

## Public Surface

- Binary entrypoint in `main.rs` starts `ShellApp` with development SCSS and launches `Bar`.
- `primitives/` defines the development bar, source models, window title row component, provider helpers, and tests.
- `locus_schema` contains development-only generated schema markers, paths, relations, and extension traits produced by Locus codegen.
- `stylesheets/dev-widgets.scss` contains visual styling outside Rust.

## Step-By-Step File Walkthrough

1. `dev-widgets/src/main.rs` - binary startup, stylesheet registration, and selected component launch. Read first to see process-level consumer usage of `shell-core`.
2. `dev-widgets/src/primitives/mod.rs` - module facade and public `Bar` / `BarInit` re-export. Read first to see the local public surface.
3. `dev-widgets/src/primitives/bar.rs` - `Bar` source/component model, layer-shell config application, view bindings, and remaining parent update behavior. Read next because it shows the target macro authoring shape.
4. `dev-widgets/src/primitives/window_title.rs` - `WindowTitle` source/component model and row CSS class helpers. Read to see local typed bindings per repeated graph row without an intermediate source wrapper.
5. `dev-widgets/src/primitives/battery.rs` - battery provider and display helpers. Read to check provider ergonomics and local UI derivation.
6. `dev-widgets/src/locus_schema/generated.rs` - generated development schema markers, paths, relations, typed property wrappers, and semantic extension traits. Read to distinguish generated consumer schema code from framework code.
7. `dev-widgets/src/locus_schema/mod.rs` - thin generated-schema facade. Read to confirm no schema-specific helper behavior remains handwritten in the dev widget crate.
8. `dev-widgets/stylesheets/dev-widgets.scss` - external stylesheet. Read to confirm styling remains outside Rust.
9. `dev-widgets/src/primitives/test.rs` - provider and semantic-provider checks. Read last to see what ergonomics are currently verified.

## Internal Structure

- `Bar` combines typed Locus graph nodes, UPower D-Bus data, and local UI state in one provider-backed component model.
- Battery percentage is subscribed once; the progress bar derives directly from the source field. The label still uses a cached display string because current `#[bind]` output cannot pass an owned derived `String` into borrowed GTK setters.
- Window rows are ordinary Relm4 child components declared through `#[bind_list(..., row = WindowTitle)]`. `Bar` stores the resolved `Vec<NodeRef<Window>>` source value, while the annotated GTK list container selects the box list path and owns row controllers through `shell_core::list::ComponentListBoxExt`.
- Development schema code is local to the consumer crate, preserving the framework boundary.

## Behavior Summary

The binary launches a top-layer, top-anchored development bar with automatic exclusive zone. The bar subscribes to selected workspace windows and battery percentage, renders a progress bar and battery label, and uses `bind_list` to render a row component for each window node. Each row binds to its own node title and a derived selected-state provider.

## User Notes

None yet.

## Findings

- Good boundary: role-specific bar config, widget layout, schema helpers, and CSS all stay in `dev-widgets`; generic list reconciliation lives in `shell-core`.
- Risk: `WindowNodeExt::is_selected` subscribes each window row to `paths::SELECTED_WINDOW.target()`. With many rows, this can create repeated selected-window subscriptions. A shared selected-window provider or parent-owned selected id could reduce duplicated work.
- Completed: the old `WindowRows` adapter was replaced by macro-level `#[bind_list(..., row = WindowTitle)]`; `Bar` no longer stores a row renderer or list adapter field.
- Completed: `WorkspacePathExt::windows` is generated from schema collection metadata instead of handwritten relation/kind strings.
- Completed: `BarSources` and `WindowTitleSources` wrappers were removed; `Bar` and `WindowTitle` are provider-backed component models directly.
- Completed: generated schema collection providers now expose typed node refs, so `Bar.window_nodes` is `Vec<NodeRef<Window>>` instead of raw `Vec<String>`.
- Gap: `bind_list` V1 uses value equality for identity and the GTK box component-list path. Explicit key/sort hooks and GTK-native/custom add-remove adapters remain future work.
- Gap: tests verify provider composition and descriptor shapes, but not actual GTK row reconciliation, layer-shell behavior, stylesheet loading, or live Locus/UPower integration.

## Refactor Plan

1. Keep `dev-widgets` as the place for role-specific development surfaces; do not promote `Bar` or row-specific rendering policy into framework crates.
2. Completed: migrate repeated rows to macro-level `bind_list` with row controllers owned by the annotated GTK container.
3. Completed: move handwritten schema extension traits in `locus_schema/mod.rs` into generated schema output.
4. Completed: split `primitives.rs` into logical `primitives/` module files.
5. Completed: make `Bar` and `WindowTitle` provider-backed component models directly instead of wrapping `BarSources` / `WindowTitleSources`.
6. Completed: change generated collection providers and dev widget state from raw node ids to typed `NodeRef<Target>` lists.
7. Completed: add macro-level `bind_list` support and replace the hand-written `WindowRows` reconciler for ordinary collection rendering.
8. Consider sharing selected-window target state across rows if repeated subscriptions show up as real runtime overhead.
9. Add UI/runtime smoke testing only when the project has a practical Wayland/GTK test environment.

## Tests And Verification

- `cargo test -p dev-widgets` passes: 3 unit tests.

## Open Questions

- Should row selection be derived locally in each row for encapsulation, or lifted to the parent to reduce subscriptions?
- Should future repeated-item widgets use this same factory-wrapper pattern around macro-generated child components, or should shell macros grow a first-class factory integration?
