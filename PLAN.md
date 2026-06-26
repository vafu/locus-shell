# Locus Shell Roadmap

## Core Idea

`locus-shell` is the Rust framework for building shell widgets, not the shell itself.

User-facing widgets such as bars, OSDs, notification surfaces, launchers, and workspace switchers should live in separate consumer crates. This repository provides reusable framework pieces that make those widgets small, fast, and consistent.

The framework should make this authoring model possible:

```rust
let window = shell_core::window::create_layer_window(config);
```

Consumer crates own:

- Widget role: bar, OSD, notification, launcher, and similar shell surfaces.
- Placement policy: anchors, layer, exclusive zone, surface margins, namespace.
- Graph and system source values to subscribe to.
- Rendering and state transitions.
- CSS and visual design.
- Process boundaries, application lifecycle, and whether multiple surfaces live
  in one process or separate binaries.

Framework crates own:

- Generic shell app lifecycle setup.
- Global CSS/SCSS registration and development-time stylesheet watching.
- GTK4 / Relm4 integration primitives.
- Layer-shell window creation.
- Future D-Bus/source subscription plumbing.
- Future macros that reduce Relm4 boilerplate.
- Shared contracts for Locus graph-driven UI state.

## Roadmap

### 1. Foundation: Workspace And Boundaries

- Current workspace crates are `shell/core`, `shell/macros`,
  `shell/rx-macros`, and `rsynapse-shell`.
- `shell-core` exposes generic framework primitives plus the small Observable
  source facade used by generated code and handwritten sources.
- `shell-macros` subscribes to Observable-compatible source expressions through `shell_core::source`.
- `shell-rx-macros` exposes lightweight declarative macros that expand to
  ordinary RxRust operators for source composition ergonomics.
- `rsynapse-shell` is the active in-repository consumer and AGS migration target.
- The old `provider/*` workspace family has been removed. Do not reintroduce Provider, ObservableSource, custom subscription runtime, or D-Bus graph compatibility layers.
- The user-facing source API is Observable-first, described in `SOURCE_API.md`.
- Do not put user-facing bar, OSD, notification, launcher, or workspace switcher behavior in framework crates.

### 2. Shell Core V1

- Add `ShellApp` as the process-level owner for Relm4 app startup, global stylesheets, and long-lived development watchers.
- Finalize `WindowConfig`.
- Keep the API centered on `create_layer_window(config)`.
- Keep naming explicit: `SurfaceMargins`, `Anchors`, `Layer`, `Edge`, `ExclusiveZone`.
- Prefer `ExclusiveZone::Auto` for widgets whose reserved screen area should follow the GTK surface's computed size.
- Document compositor placement versus CSS layout.
- Add pure tests for config behavior.

### 3. In-Repository Consumer

- Use `rsynapse-shell` as the in-repository consumer for framework ergonomics.
- Keep widget policy, source composition, CSS, and AGS migration behavior in
  `rsynapse-shell`, not in framework crates.
- `rsynapse-shell` currently runs the bar and OSD as multiple layer-shell
  windows in one binary; do not split OSD back into a separate binary unless
  that consumer policy changes again.

### 4. Locusfs Source Integration

- Use `locusfs-client` as the current graph transport for reads and watches.
- Consumer crates should represent Locus paths with `LocusPath`.
- Do not reintroduce schema-specific marker structs, `NodeRef`, `Property`,
  `Relation`, `Path`, or generated graph extension traits in this workspace.
- Locus source helpers should return shell-owned `Observable<T>` values.
- D-Bus can be proxied into locusfs outside this workspace; do not add D-Bus provider crates back here.
- Keep blocking locusfs watch/read work off the GTK UI thread through RxRust-backed source expressions.

### 5. Macro Crate

- Keep the `shell/macros` crate as the Relm4 source binding proc-macro crate.
- Accept generated typed source expressions instead of raw string tuple paths.
- Integrate directly with `#[relm4::component]` instead of requiring side modules.
- Let consumers declare a typed state model with field-level source attributes:
  - `#[shell_macros::model]`
  - `#[source(...)]`
  - `#[shell_macros::component(model = Bar)]`
- Keep `#[source(...)]` only for model fields. Derived source function
  dependencies use `#[observe(...)]`; stable service dependencies use
  `#[inject]`.
- Add `#[shell_macros::observable]` for user-authored derived source functions
  that return shell-owned `Observable<T>` values.
- Default generated binding modules to `sources`, with `state = ...` available when the component field name needs to be explicit.
- Keep generated runtime internals in one private `__shell` sidecar field on typed models.
- Generate minimal Relm4 glue for source-bound fields:
  - typed model cache
  - typed update messages
  - async watcher startup
- Dispatch binding expressions through Observable sources instead of
  backend-specific watcher functions.
- Generate source messages from result-carrying observable items and keep task
  handles owned by subscriptions.
- Let component views bind GTK setters with `#[bind(field)]`:
  - closure adapters such as `set_label: |title| title.as_str()`
  - function adapters such as `set_css_classes: window_title_classes`
  - generated Relm4 `#[track(...)]` guards so unrelated field changes do not redraw the setter
- Let repeated child regions bind collection fields with `#[bind_list(...)]`.
  The concrete list path is inferred from the annotated widget type. The first
  supported path hosts Relm4 row component controllers on a GTK container;
  GTK-native and Adwaita list adapters should remain optional integrations.
- Keep generated code understandable and debuggable with `cargo expand`.

### 5a. Rx Macro Crate

- Keep `shell/rx-macros` as a small declarative macro crate for RxRust operator
  ergonomics.
- Macros in this crate must expand to existing RxRust operators and must not
  introduce source runtimes, subscriptions, watchers, backend clients, or UI
  policy.
- Use it for concise fixed-arity composition such as `combine_latest!` where
  Rust's heterogeneous observable types make `Vec<Observable<_>>` unsuitable.

### 6. Framework Integration Layer

- Connect macro output to source subscriptions.
- Translate source updates, including Locus `ResolveChanged` updates, into
  Relm4 input messages.
- Maintain cached model state for watched GTK properties.
- Avoid client-side polling or a separate reactive runtime.
- Use shared latest observable sources when multiple model fields derive from
  the same upstream descriptor.
- Keep selected graph node -> dependent collection flows in Observable source
  functions rather than component lifecycle code.
- Prefer semantic source functions such as `selected_workspace_windows()` over
  raw graph traversal at widget call sites.
- Prefer dynamic child components with local source bindings for repeated graph
  items, such as `WindowTitle` taking a `String` node path and binding
  `window_title(window.clone())` internally.
- Let Relm4 components wrap generated source messages in a richer input enum when they need local events, dynamic child rows, or factory messages.
- Support derived observable source functions for summarized UI data, such as
  workspace status, window indicators, build status, agent state, and system
  indicators.

### 7. External User-Facing Widgets

- Create actual shell widgets outside this framework boundary.
- Use `rsynapse-shell` as the in-repository AGS migration playground and
  framework stress test, while keeping product policy out of framework crates.
- Current in-repository consumer: `rsynapse-shell`, with bar and OSD windows in
  one process.
- Then notifications.
- These crates depend on `shell-core` and `shell-macros`; graph data should come
  from Observable source functions over locusfs.

### 8. Hardening

- Add examples and docs once APIs settle.
- Add integration tests where possible.
- Add macro debugging guidance.
- Validate runtime behavior on a real Wayland compositor.

### 9. Observable Source API Migration

- Completed: removed the provider task runtime and `provider/*` crates.
- Completed: replaced `ObservableSource<T>` with the shell-owned `Observable<T>` alias/re-export backed by `rxrust`.
- Keep model `#[source(...)]` bindings as plain value fields and subscribe to
  Observable sources in generated Relm4 glue.
- Add `#[shell_macros::observable]` derived source functions with explicit
  `#[observe(...)]` observable dependencies and `#[inject]` DI service
  dependencies.
- Use `nject` behind a small shell facade for stable services. Reactive graph
  values remain Observable dependencies, not DI services.
- Keep context-dependent source factory behavior in macros/codegen. Shell
  authors should see ordinary Rust functions returning `Observable<T>`, not
  custom source traits.
- Future service helpers should return observables or proxy through locusfs.
- Completed: primitive locusfs sources share by normalized backend path.
- Completed: `source::shared_by_key` provides descriptor-keyed sharing for
  handwritten derived sources where reuse is expected, so widget authors do not
  need local `OnceLock` caches or manual `.shared()` calls.
- Completed in `rsynapse-shell`: generic D-Bus consumers use the current
  `/dbus/<service>/objects` and `/dbus/<service>/methods` layout instead of
  legacy `object`, `@properties`, `@methods`, and method `/call` paths.
- Replace ad hoc consumer source code with user-authored observable source functions where it improves ergonomics.

## Next Concrete Step

Measure real shell runtime source fanout with `SHELL_CORE_SOURCE_TRACE`, then
finish the macro-layer correctness work identified in `refactor/agent-macros.md`
before adding generated `#[shell_macros::observable]` descriptor support.
