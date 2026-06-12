# dev-widgets Refactor Notes

## Current Role

`dev-widgets` is an internal consumer crate used to exercise framework APIs. It is not a user-facing shell implementation and should be allowed to define role-specific development widgets locally without moving those roles into `shell-core`.

## Public Surface

- Binary entrypoint in `main.rs` starts `ShellApp` with development SCSS and launches `Bar`.
- `primitives.rs` defines the development bar, source models, window title row component, manual row reconciler, provider helpers, and tests.
- `locus_schema` contains development-only schema markers, paths, relations, and extension traits that should eventually come from Locus codegen.
- `stylesheets/dev-widgets.scss` contains visual styling outside Rust.

## Step-By-Step File Walkthrough

1. `dev-widgets/src/main.rs` - binary startup, stylesheet registration, and selected component launch. Read first to see process-level consumer usage of `shell-core`.
2. `dev-widgets/src/primitives.rs` lines 21-31 - `BarSources` typed provider model. Read next because it shows the target macro authoring shape.
3. `dev-widgets/src/primitives.rs` lines 38-105 - `Bar` Relm4 component. Read to see layer-shell config application, view bindings, and parent update behavior.
4. `dev-widgets/src/primitives.rs` lines 117-164 - `WindowTitleSources` and `WindowTitle` child component. Read to see local typed bindings per repeated graph row.
5. `dev-widgets/src/primitives.rs` lines 166-225 - `WindowRows` manual child reconciler. Read because this is explicitly called out in `PLAN.md` as an area to evaluate against Relm4 factories.
6. `dev-widgets/src/primitives.rs` lines 227-269 - CSS class helpers, shared battery provider, label derivation, and local bar window config. Read to check provider combinator ergonomics and local role policy.
7. `dev-widgets/src/locus_schema/generated.rs` - development schema markers, paths, and relations. Read to distinguish generated-schema-shaped code from framework code.
8. `dev-widgets/src/locus_schema/mod.rs` - handwritten semantic extension traits such as `WindowNodeExt` and `WorkspacePathExt`. Read because these are the intended future codegen target.
9. `dev-widgets/stylesheets/dev-widgets.scss` - external stylesheet. Read to confirm styling remains outside Rust.
10. `dev-widgets/src/primitives.rs` tests - provider combinator and semantic-provider checks. Read last to see what ergonomics are currently verified.

## Internal Structure

- `BarSources` combines Locus graph data and UPower D-Bus data through generic `providers::Provider<T>` sources.
- Battery percentage is shared through a `OnceLock<SharedProvider<...>>` so the label and progress bar reuse one upstream D-Bus source.
- Window rows are ordinary Relm4 child components launched and reconciled manually by node id.
- Development schema code is local to the consumer crate, preserving the framework boundary.

## Behavior Summary

The binary launches a top-layer, top-anchored development bar with automatic exclusive zone. The bar subscribes to selected workspace windows and battery percentage, renders a progress bar and battery label, and reconciles a row component for each window node. Each row binds to its own node title and a derived selected-state provider.

## User Notes

None yet.

## Findings

- Good boundary: role-specific bar config, widget layout, row reconciliation, schema helpers, and CSS all stay in `dev-widgets`, not `shell-core`.
- Risk: `WindowNodeExt::is_selected` subscribes each window row to `paths::SELECTED_WINDOW.target()`. With many rows, this can create repeated selected-window subscriptions. A shared selected-window provider or parent-owned selected id could reduce duplicated work.
- Risk: `WindowRows::render_order` removes and re-appends every child when order changes. This is simple and fine for a dev widget, but a Relm4 factory or more targeted reorder API may scale better.
- Risk: `WorkspacePathExt::windows` eventually resolves through hardcoded `"workspace"` and `"window"` strings. This is acceptable as development schema glue, but future generated schema extensions should use relation/kind descriptors rather than handwritten strings.
- Gap: tests verify provider composition and descriptor shapes, but not actual GTK row reconciliation, layer-shell behavior, stylesheet loading, or live Locus/UPower integration.

## Refactor Plan

1. Keep `dev-widgets` as the place for role-specific development surfaces; do not promote `Bar` or `WindowRows` into framework crates.
2. Evaluate `WindowRows` against a Relm4 factory pattern, as noted in `PLAN.md`, once the macro/provider API settles.
3. Move handwritten extension traits in `locus_schema/mod.rs` into generated schema output when Locus codegen supports semantic helpers.
4. Consider sharing selected-window target state across rows if repeated subscriptions show up as real runtime overhead.
5. Add UI/runtime smoke testing only when the project has a practical Wayland/GTK test environment.

## Tests And Verification

- `cargo test -p dev-widgets` passes: 4 unit tests.

## Open Questions

- Should row selection be derived locally in each row for encapsulation, or lifted to the parent to reduce subscriptions?
- Should the manual child reconciler become a Relm4 factory now, or stay ordinary consumer code until a larger repeated-item widget proves the need?
