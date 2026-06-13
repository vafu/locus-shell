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
- `provider/core` publishes the `providers` crate with reusable provider traits, stream-oriented subscription handles, cancellation, shared latest fanout, and minimal runtime spawning.
- Provider APIs are centered on `Provider<T>` as a typed Tokio stream source: `Stream<Item = Result<T, E>>`. Prefer `tokio_stream::StreamExt` and ordinary Rust/Tokio primitives over custom reactive combinator layers.
- `provider/locus` publishes the `locus-provider` crate with generic Locus graph binding primitives plus the Locus-over-D-Bus provider implementation.
- `provider/dbus` publishes the `dbus-provider` crate with generic D-Bus object/property provider implementation.
- `provider/common` publishes `common-providers` with feature-gated common service definitions and contains no watcher/runtime policy.
- Do not put user-facing bar, OSD, notification, launcher, or workspace switcher behavior in framework crates.

### 2. Shell Core V1

- Add `ShellApp` as the process-level owner for Relm4 app startup, global stylesheets, and long-lived development watchers.
- Let `ShellApp` install the provider task runtime used by generated provider subscriptions.
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
- Carry development-only generated Locus schema code locally, including marker
  structs, path constants, relation constants, and schema extension traits.
- Use these widgets to test whether `shell-core` stays generic and ergonomic.

### 4. D-Bus Integration Crates

- Keep the `locus-provider` crate under `provider/locus` for generic Locus graph bindings.
- Generate schema-specific `model`, `paths`, relation constants, and extension
  traits inside consuming crates, not inside `locus-provider`.
- Wrap `io.github.Locus.Graph.Resolve` in `locus-provider`.
- Implement raw `providers::Provider<String>` subscriptions for Locus graph
  property bindings in `locus-provider`; generated schema wrappers adapt those
  raw wire values into typed `Provider<T>` bindings.
- Keep the `dbus-provider` crate under `provider/dbus` for generic D-Bus bindings.
- Keep the `property-provider` crate under `provider/property` for shared property descriptors and property-backed provider traits.
- Provide generic pure D-Bus property bindings:
  - `dbus_provider::Object<Target>::session(...)`
  - `dbus_provider::Object<Target>::system(...)`
  - `dbus_provider::Property<Target, Value>::new(property)`
  - `object.bind(property)`
  - `dbus_provider::watch_property(binding, cancellation)`
- Implement `providers::Provider<T>` for pure D-Bus property bindings.
- Keep async D-Bus work off the GTK thread.

### 4a. Common Provider Definitions

- Keep the `common-providers` crate under `provider/common` for common typed D-Bus objects/properties.
- Keep modules feature-gated by service, starting with `upower`.
- Expose definitions such as `common_providers::upower::DISPLAY_DEVICE`.
- Keep runtime watchers and binding machinery in `dbus-provider`.

### 5. Macro Crate

- Keep the `shell/macros` crate as the Relm4/provider binding proc-macro crate.
- Accept generated typed provider expressions instead of raw string tuple paths.
- Integrate directly with `#[relm4::component]` instead of requiring side modules.
- Let consumers declare a typed state model with field-level source attributes:
  - `#[shell_macros::model]`
  - `#[source(...)]`
  - `#[shell_macros::component(model = Bar)]`
- Default generated binding modules to `sources`, with `state = ...` available when the component field name needs to be explicit.
- Keep generated runtime internals in one private `__shell` sidecar field on typed models.
- Generate minimal Relm4 glue for graph-bound fields:
  - typed model cache
  - typed update messages
  - async watcher startup
- Dispatch binding expressions through `providers::Provider<T>` instead of backend-specific watcher functions.
- Generate provider messages from `Result<T, E>` stream items and keep task handles owned by subscriptions.
- Let component views bind GTK setters with `#[bind(field)]`:
  - closure adapters such as `set_label: |title| title.as_str()`
  - function adapters such as `set_css_classes: window_title_classes`
  - generated Relm4 `#[track(...)]` guards so unrelated field changes do not redraw the setter
- Let repeated child regions bind collection fields with `#[bind_list(...)]`.
  The concrete list path is inferred from the annotated widget type. The first
  supported path hosts Relm4 row component controllers on a GTK container;
  GTK-native and Adwaita list adapters should remain optional integrations.
- Keep generated code understandable and debuggable with `cargo expand`.

### 6. Framework Integration Layer

- Connect macro output to provider subscriptions.
- Translate provider updates, including Locus `ResolveChanged` updates, into Relm4 input messages.
- Maintain cached model state for watched GTK properties.
- Avoid client-side polling or a separate reactive runtime.
- Use shared latest providers when multiple model fields derive from the same upstream source.
- Push selected graph node -> dependent collection flows into Locus/schema helpers where possible before adding custom switch/combine provider APIs.
- Prefer semantic collection helpers such as `paths::SELECTED_WORKSPACE.windows()` over raw graph direction at widget call sites.
- Prefer dynamic child components with local typed bindings for repeated graph
  items, such as `WindowTitle` taking a `NodeRef<Window>` and binding
  `window.title()` internally.
- Let Relm4 components wrap generated source messages in a richer input enum when they need local events, dynamic child rows, or factory messages.
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

- Completed: provider task spawning is owned by framework setup. `ShellApp`
  installs a Tokio-backed task spawner, while provider core only stores and uses
  the installed spawner for subscription tasks.
- Add richer provider combinators only when concrete widget requirements justify them:
  - `distinct`
  - `filter_map`
  - `fallible_map`
  - `combine_latest3`
- Treat any `tokio_stream::Stream<Item = Result<T, E>>` as a provider directly through the blanket `Provider<T>` implementation.
- Reintroduce `switch_map`/`combine_latest` only as thin stream helpers if real widget requirements prove they are needed.
- Keep shared latest providers available as the provider-core primitive for
  connection and subscription reuse. Backend or generated providers should
  apply sharing from stable provider keys so widget authors do not need local
  `OnceLock` caches or manual `.shared()` calls.
- Keep collection providers for Locus paths and reverse relation lookups available as stable node-id lists for workspace lists, window lists, tray items, media players, and agent sessions.
- Move hand-written development schema descriptors and extension traits into generated Rust schema output once the Locus codegen contract is updated. Basic node property helpers, selected-node helpers, relation constants, and filtered reverse collection helpers are generated.
- Add typed row hydration helpers for collection results so consumers can request summaries such as window id/title, workspace name/focus state, and project display fields without manually wiring one provider per property.
- Keep descriptor constructors and typed bind APIs public, but consider making raw string descriptor fields private with accessors before the API stabilizes.

## Next Concrete Step

Next, add compile-expanded macro tests for realistic Relm4 components so the
generated provider subscription and view-binding contracts are validated beyond
token-shape assertions.
