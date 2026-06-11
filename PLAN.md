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
- Graph values to subscribe to.
- Rendering and state transitions.
- CSS and visual design.
- Process boundaries and application lifecycle.

Framework crates own:

- Generic shell app lifecycle setup.
- Global CSS/SCSS registration and development-time stylesheet watching.
- GTK4 / Relm4 integration primitives.
- Layer-shell window creation.
- Future D-Bus subscription plumbing.
- Future macros that reduce Relm4 boilerplate.
- Shared contracts for Locus graph-driven UI state.

## Roadmap

### 1. Foundation: Workspace And Boundaries

- Keep flat root crates: `shell-core`, `dev-widgets`, `providers`, `locus-graph`, `macros`, `dbus`, and `standard-dbus`.
- `shell-core` only exposes generic framework primitives.
- `dev-widgets` remains internal and proves ergonomics.
- `providers` owns reusable provider traits, subscription handles, cancellation, and backend-neutral combinators.
- `locus-graph` owns generated Locus graph contracts plus the Locus-over-D-Bus provider implementation.
- `dbus` owns generic D-Bus object/property provider implementation.
- `standard-dbus` exposes feature-gated common service definitions and contains no watcher/runtime policy.
- Do not put user-facing bar, OSD, notification, launcher, or workspace switcher behavior in framework crates.

### 2. Shell Core V1

- Add `ShellApp` as the process-level owner for Relm4 app startup, global stylesheets, and long-lived development watchers.
- Finalize `WindowConfig`.
- Keep the API centered on `create_layer_window(config)`.
- Keep naming explicit: `SurfaceMargins`, `Anchors`, `Layer`, `Edge`, `ExclusiveZone`.
- Prefer `ExclusiveZone::Auto` for widgets whose reserved screen area should follow the GTK surface's computed size.
- Document compositor placement versus CSS layout.
- Add pure tests for config behavior.

### 3. Dev Widgets

- Build small primitive consumers:
  - top strip
  - transient overlay
  - notification-like popup
- Each dev widget defines its own role-specific config locally.
- Use external SCSS registered through `ShellApp`.
- Use these widgets to test whether `shell-core` stays generic and ergonomic.

### 4. D-Bus Integration Crates

- Add a root-level `locus-graph` crate for typed Locus graph bindings.
- Generate `locus-graph::{model, paths, binding}` from the Locus schema/codegen output.
- Wrap `io.github.Locus.Graph.Resolve` in `locus-graph`.
- Implement `providers::Provider<T>` for Locus graph field bindings in `locus-graph`, where `FieldBinding<T>` is owned.
- Add a root-level `dbus` crate for generic D-Bus bindings.
- Provide generic pure D-Bus property bindings:
  - `dbus::Object<Target>::session(...)`
  - `dbus::Object<Target>::system(...)`
  - `dbus::Property<Target, Value>::new(property)`
  - `object.bind(property)`
  - `dbus::watch_property(binding, on_value)`
- Implement `providers::Provider<T>` for pure D-Bus property bindings.
- Keep async D-Bus work off the GTK thread.

### 4a. Standard D-Bus Definitions

- Add a root-level `standard-dbus` crate for common typed D-Bus objects/properties.
- Keep modules feature-gated by service, starting with `upower`.
- Expose definitions such as `standard_dbus::upower::DISPLAY_DEVICE`.
- Keep runtime watchers and binding machinery in `dbus`.

### 5. Macro Crate

- Add a root-level `macros` crate after D-Bus contracts are known.
- Accept generated typed `FieldBinding<T>` expressions instead of raw string tuple paths.
- Integrate directly with `#[relm4::component]` instead of requiring side modules.
- Let consumers declare a typed state model with field-level source attributes:
  - `#[locus_macros::model]`
  - `#[locus(source = ...)]`
  - `#[locus_macros::component(model = BarLocus)]`
- Generate minimal Relm4 glue for graph-bound fields:
  - typed model cache
  - typed update messages
  - async watcher startup
- Dispatch binding expressions through `providers::Provider<T>` instead of backend-specific watcher functions.
- Let component views bind GTK setters with `#[locus(field)]`:
  - closure adapters such as `set_label: |title| title.as_str()`
  - function adapters such as `set_css_classes: window_title_classes`
  - generated Relm4 `#[track(...)]` guards so unrelated field changes do not redraw the setter
- Keep generated code understandable and debuggable with `cargo expand`.

### 6. Framework Integration Layer

- Connect macro output to provider subscriptions.
- Translate `ResolveChanged` into Relm4 input messages.
- Maintain cached model state for watched GTK properties.
- Avoid client-side polling or a separate reactive runtime.
- Support derived provider chains for summarized UI data, such as workspace status, window indicators, build status, agent state, and system indicators.

### 7. External User-Facing Widgets

- Create actual shell widgets outside this framework boundary.
- First likely consumer: bar.
- Then OSD.
- Then notifications.
- These crates depend on `shell-core`, `locus-graph`, `dbus`, `standard-dbus` as needed, and `macros`.

### 8. Hardening

- Add examples and docs once APIs settle.
- Add integration tests where possible.
- Add macro debugging guidance.
- Validate runtime behavior on a real Wayland compositor.

## Next Concrete Step

Decide whether macro ergonomics need a lighter syntax for common summaries, or whether explicit custom providers plus `combine_latest` are sufficient. The dev bar remains the proof target: selected-window title comes from `locus_graph::{paths, model}`, battery percentage comes from `standard-dbus`, and GTK setters bind with `#[locus(field)]`.
