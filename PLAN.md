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

- Keep flat root crates: `shell-core`, `dev-widgets`, `macros`, `dbus`, and `standard-dbus`.
- `shell-core` only exposes generic framework primitives.
- `dev-widgets` remains internal and proves ergonomics.
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

### 4. D-Bus Integration Crate

- Add a root-level `dbus` crate only after the core window API feels stable.
- Wrap `io.github.Locus.Graph.Resolve`.
- Provide generic pure D-Bus property bindings:
  - `dbus::Object<Target>::session(...)`
  - `dbus::Object<Target>::system(...)`
  - `dbus::Property<Target, Value>::new(property)`
  - `object.bind(property)`
  - `dbus::watch_property(binding, on_value)`
- Provide typed subscription primitives:
  - resolve once
  - subscribe resolve
  - stream changed values
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
- Dispatch binding expressions through providers:
  - Locus graph bindings call `dbus::watch_field`
  - pure D-Bus property bindings call `dbus::watch_property`
- Let component views bind GTK setters with `#[locus(field)]`:
  - closure adapters such as `set_label: |title| title.as_str()`
  - function adapters such as `set_css_classes: window_title_classes`
  - generated Relm4 `#[track(...)]` guards so unrelated field changes do not redraw the setter
- Keep generated code understandable and debuggable with `cargo expand`.

### 6. Framework Integration Layer

- Connect macro output to D-Bus subscriptions.
- Translate `ResolveChanged` into Relm4 input messages.
- Maintain cached model state for watched GTK properties.
- Avoid client-side polling or a separate reactive runtime.

### 7. External User-Facing Widgets

- Create actual shell widgets outside this framework boundary.
- First likely consumer: bar.
- Then OSD.
- Then notifications.
- These crates depend on `shell-core`, `dbus`, and `macros`.

### 8. Hardening

- Add examples and docs once APIs settle.
- Add integration tests where possible.
- Add macro debugging guidance.
- Validate runtime behavior on a real Wayland compositor.

## Next Concrete Step

Move the source expressions inside `#[locus(source = ...)]` from hand-written `dbus::schema::paths::...property(...)` calls toward generated typed path accessors from Locus schema/codegen. The dev bar remains the proof target: selected-window title and battery percentage are fields on `BarLocus`, and GTK setters bind with `#[locus(field)]` without repeating source paths in the component macro.
