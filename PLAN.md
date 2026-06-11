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

- Keep related crates grouped by family: `shell/core`, `shell/macros`, `provider/core`, `provider/locus`, `provider/dbus`, and `provider/common`.
- `shell-core` only exposes generic framework primitives.
- `dev-widgets` remains internal and proves ergonomics.
- `provider/core` publishes the `providers` crate with reusable provider traits, subscription handles, cancellation, runtime spawning, and backend-neutral combinators.
- Provider internals may use `tokio-stream` for stream adaptation and dynamic subscription switching, but widget-facing APIs should remain centered on `Provider<T>`.
- `provider/locus` publishes the `locus-provider` crate with generated Locus graph contracts plus the Locus-over-D-Bus provider implementation.
- `provider/dbus` publishes the `dbus-provider` crate with generic D-Bus object/property provider implementation.
- `provider/common` publishes `common-providers` with feature-gated common service definitions and contains no watcher/runtime policy.
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

- Keep the `locus-provider` crate under `provider/locus` for typed Locus graph bindings.
- Generate `locus-provider::{model, paths, binding}` from the Locus schema/codegen output.
- Wrap `io.github.Locus.Graph.Resolve` in `locus-provider`.
- Implement `providers::Provider<T>` for Locus graph field bindings in `locus-provider`, where `FieldBinding<T>` is owned.
- Keep the `dbus-provider` crate under `provider/dbus` for generic D-Bus bindings.
- Provide generic pure D-Bus property bindings:
  - `dbus_provider::Object<Target>::session(...)`
  - `dbus_provider::Object<Target>::system(...)`
  - `dbus_provider::Property<Target, Value>::new(property)`
  - `object.bind(property)`
  - `dbus_provider::watch_property(binding, on_value)`
- Implement `providers::Provider<T>` for pure D-Bus property bindings.
- Keep async D-Bus work off the GTK thread.

### 4a. Common Provider Definitions

- Keep the `common-providers` crate under `provider/common` for common typed D-Bus objects/properties.
- Keep modules feature-gated by service, starting with `upower`.
- Expose definitions such as `common_providers::upower::DISPLAY_DEVICE`.
- Keep runtime watchers and binding machinery in `dbus-provider`.

### 5. Macro Crate

- Keep the `shell/macros` crate as the Relm4/provider binding proc-macro crate.
- Accept generated typed `FieldBinding<T>` expressions instead of raw string tuple paths.
- Integrate directly with `#[relm4::component]` instead of requiring side modules.
- Let consumers declare a typed state model with field-level source attributes:
  - `#[shell_macros::model]`
  - `#[source(...)]`
  - `#[shell_macros::component(model = BarSources, state = sources)]`
- Default generated binding modules to `sources`, with `state = ...` available when the component field name needs to be explicit.
- Keep generated runtime internals in one private `__shell` sidecar field on typed models.
- Generate minimal Relm4 glue for graph-bound fields:
  - typed model cache
  - typed update messages
  - async watcher startup
- Dispatch binding expressions through `providers::Provider<T>` instead of backend-specific watcher functions.
- Spawn generated provider tasks through `providers::spawn`, backed by the provider Tokio runtime, and keep task handles owned by subscriptions.
- Let component views bind GTK setters with `#[bind(field)]`:
  - closure adapters such as `set_label: |title| title.as_str()`
  - function adapters such as `set_css_classes: window_title_classes`
  - generated Relm4 `#[track(...)]` guards so unrelated field changes do not redraw the setter
- Keep generated code understandable and debuggable with `cargo expand`.

### 6. Framework Integration Layer

- Connect macro output to provider subscriptions.
- Translate provider updates, including Locus `ResolveChanged` updates, into Relm4 input messages.
- Maintain cached model state for watched GTK properties.
- Avoid client-side polling or a separate reactive runtime.
- Use shared/replay providers when multiple model fields derive from the same upstream source.
- Use `ProviderExt::switch_map` when one provider value selects another long-lived provider, such as selected workspace -> windows in that workspace.
- Support derived provider chains for summarized UI data, such as workspace status, window indicators, build status, agent state, and system indicators.

### 7. External User-Facing Widgets

- Create actual shell widgets outside this framework boundary.
- First likely consumer: bar.
- Then OSD.
- Then notifications.
- These crates depend on `shell-core`, `shell-macros`, `locus-provider`, `dbus-provider`, and `common-providers` as needed.

### 8. Hardening

- Add examples and docs once APIs settle.
- Add integration tests where possible.
- Add macro debugging guidance.
- Validate runtime behavior on a real Wayland compositor.

### 9. Provider API Hardening

- Decide whether the neutral provider contract should split runtime spawning into a follow-up crate such as `providers-tokio` or a configurable `ProviderSpawner`.
- Keep the current `providers::spawn` runtime as the transitional default used by macro output.
- Add richer provider combinators only when concrete widget requirements justify them:
  - `distinct`
  - `filter_map`
  - `fallible_map`
  - `combine_latest3`
- Keep `stream_provider` available as the adapter from `tokio_stream::Stream<Item = Result<T, E>>` into `Provider<T>` for custom network, socket, timer, and service integrations.
- Keep `switch_map` available for dynamic provider replacement; this is the core primitive for selected graph node -> dependent collection subscription flows.
- Keep shared/replay providers available for connection and subscription reuse.
- Add collection providers for Locus paths that resolve many nodes, with stable IDs for workspace lists, window lists, tray items, media players, and agent sessions.
- Keep descriptor constructors and typed bind APIs public, but consider making raw string descriptor fields private with accessors before the API stabilizes.

## Next Concrete Step

Next, add collection providers for Locus paths that resolve many nodes. Use `tokio-stream` internally where it simplifies D-Bus signal/list-diff streams, expose the result as `Provider<Vec<T>>` or typed diff providers, and compose dynamic selections with `ProviderExt::switch_map`. The dev bar now proves scalar Locus fields, shared UPower-derived fields, explicit `sources` state, and GTK setters bound with `#[bind(field)]`; collections are the next missing primitive for workspaces, windows, tray items, and media players.
