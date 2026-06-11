# Locus Shell Project Blueprint

## Objective

Locus Shell is a Rust/Relm4 desktop shell for high-performance, low-footprint widgets such as bars, OSDs, and notifications. It replaces heavier GJS/AGS-style shell code with native GTK4 binaries and server-driven reactivity from the `io.github.Locus` D-Bus service.

The shell should provide a concise authoring model for widgets while preserving the runtime characteristics of compiled Rust and GTK4.

## Core Constraints

- No JavaScript engine, embedded interpreter, or client-side reactive runtime.
- Each major widget is a standalone binary, for example `locus-bar` and `locus-osd`.
- Widget failures must be isolated to their own process.
- UI state is driven by `io.github.Locus.Graph.Resolve`.
- Widgets subscribe through `SubscribeResolve` and redraw only when `ResolveChanged` is emitted.
- Relm4 boilerplate should be hidden behind procedural macros where practical.
- D-Bus work must run asynchronously outside the GTK UI thread.
- Styling belongs in external CSS files, not hardcoded Rust widget properties.

## Target Workspace Layout

```text
locus/
├── Cargo.toml
├── shell-core/
├── dev-widgets/
├── provider/
│   ├── core/        # package: providers
│   ├── locus/       # package: locus-graph
│   └── dbus/        # package: dbus
├── macros/
└── standard-dbus/
```

This repository is the framework workspace. User-facing shell implementations should live in separate crates or repositories that consume these framework crates.

## Runtime Architecture

Locus Shell widgets are thin UI processes. They subscribe to graph resolutions from `locusd`, receive server-side changes over D-Bus, translate those changes into Relm4 messages, and let Relm4 update watched GTK properties.

```text
+---------------+                    +------------------+                    +-----------------+
|   locusd      | --ResolveChanged-->|  Relm4 Command   | --Injected Msg---->|   Relm4 Model   |
| (D-Bus Graph) |                    |  Async Worker    |                    | (State Mutate)  |
+---------------+                    +------------------+                    +-----------------+
                                                                                      |
                                                                               Triggers #[watch]
                                                                                      |
                                                                             +--------v--------+
                                                                             |  GTK4 Widget    |
                                                                             +-----------------+
```

The client should not poll, diff large graph payloads, or maintain an independent reactivity engine. `locusd` owns graph resolution and invalidation.

## Crates

### `shell-core`

Common UI support crate for GTK4/Relm4 widgets.

Responsibilities:

- Provide a generic process-level app wrapper for GTK/Relm4 shell widget binaries.
- Register global CSS/SCSS stylesheets and optional development-time stylesheet watchers.
- Create GTK windows with explicit layer-shell options.
- Encapsulate setup for GTK4 layer-shell integration.
- Offer small abstractions for raw layer-shell configuration: anchors, surface margins, exclusive zones, layers, namespace, and keyboard mode.
- Support fixed and automatic exclusive zones; automatic exclusivity reserves compositor space from the layer surface's computed size.
- Avoid consumer roles such as panel, bar, overlay widget, notification, or OSD. Those roles belong to consuming crates.

Non-responsibilities:

- No product-specific shell widgets.
- No panel/bar/OSD constructors.
- No product-specific application policy beyond generic lifecycle setup.
- No D-Bus subscription policy.
- No visual styling content; consumers provide stylesheets.

Initial dependencies:

- `gtk4`
- `relm4`
- `gtk4-layer-shell`, or another GTK4-compatible Wayland layer shell binding

### `macros`

Procedural macro crate that reduces Relm4 widget boilerplate and binds UI state to Locus graph resolutions.

Initial dependencies:

- `syn`
- `quote`
- `proc-macro2`

Responsibilities:

- Parse `#[locus_macros::component(...)]` attributes stacked with `#[relm4::component]`.
- Parse typed state models annotated with `#[locus_macros::model]`.
- Extract typed generated field bindings from model fields of the form:

```rust
#[locus_macros::model]
pub struct BarLocus {
    #[locus(
        source = locus_graph::paths::SELECTED_WINDOW
            .property(locus_graph::model::Window::TITLE)
    )]
    pub selected_window_title: String,
}
```

- Keep `#[locus_macros::component(model = BarLocus)]` focused on Relm4 lifecycle wiring and view tracking.
- Preserve legacy component-level bindings during the transition:

```rust
selected_window_title: String = locus_graph::paths::SELECTED_WINDOW
    .property(locus_graph::model::Window::TITLE)
```

- Generate model state for resolved values.
- Generate message handling for field updates.
- Generate async subscription setup that forwards provider updates into Relm4 input messages.
- Support binding providers:
  - Locus graph field bindings through generated `FieldBinding<T>` expressions.
  - Pure D-Bus property bindings through typed `dbus::Object<Target>` and `dbus::Property<Target, Value>` pairs.
  - Consumer-defined providers that implement `providers::Provider<T>`.
- Rewrite `#[locus(field)]` view setters into Relm4 `#[track(...)]` updates so only widgets bound to the changed field redraw.

Target authoring shape:

```rust
#[locus_macros::model]
pub struct BarLocus {
    #[locus(
        source = locus_graph::paths::SELECTED_WINDOW
            .property(locus_graph::model::Window::TITLE)
    )]
    pub selected_window_title: String,
    #[locus(source = DISPLAY_DEVICE.bind(DisplayDevice::PERCENTAGE))]
    pub battery_percent: f64,
}

pub struct Bar {
    locus: BarLocus,
}

#[locus_macros::component(model = BarLocus)]
#[relm4::component(pub)]
impl SimpleComponent for Bar {
    type Input = locus::Msg;

    view! {
        gtk::Window {
            gtk::Label {
                #[locus(selected_window_title)]
                set_label: |title| title.as_str(),

                #[locus(selected_window_title)]
                set_css_classes: window_title_classes,
            }
        }
    }
}
```

Generated concepts:

- A `locus` module scoped beside the component.
- A user-authored state struct containing one field per typed binding.
- Unified update message with one generated variant per field.
- An initialization hook that starts provider subscriptions and emits Relm4 messages.
- Per-field dirty tracking, cleared after Relm4 updates the view.
- View setter adapters that receive typed references to generated model fields.
- Dynamic styling through normal GTK setters such as `set_css_classes`; CSS contents still live in external stylesheets.

### `providers`

Small reusable provider contract crate.

Responsibilities:

- Define `Provider<T>` for typed asynchronous value sources.
- Provide `ProviderContext`, `ProviderSender<T>`, `Subscription`, and `SubscriptionGroup`.
- Provide small provider combinators such as `ProviderExt::map`.
- Stay independent of GTK, Relm4, D-Bus, and product-specific shell behavior.

Non-responsibilities:

- No D-Bus transport implementation.
- No GTK widget or shell-window policy.
- No standard service definitions.

### `locus-graph`

Generated Locus graph contracts plus the direct Locus-over-D-Bus provider implementation.

Responsibilities:

- Vendor generated Rust contracts from `~/proj/locus/locus-codegen`.
- Expose generated `binding`, `model`, and `paths` modules.
- Own `FieldBinding<T>` so the crate can implement `providers::Provider<T>` without violating Rust coherence rules.
- Decode Locus wire values into typed Rust values.
- Watch `io.github.Locus.Graph.Resolve` through `locus-dbus`.
- Keep async Locus D-Bus work off the GTK thread.

Non-responsibilities:

- No generic D-Bus object/property model.
- No standard service definitions such as UPower.
- No GTK or Relm4 widget policy.

### `dbus`

Generic typed D-Bus object/property provider crate.

Responsibilities:

- Expose `dbus::Object<T>`, `dbus::Property<T, V>`, and `dbus::PropertyBinding<V>`.
- Support session and system bus objects.
- Implement `providers::Provider<T>` for pure D-Bus property bindings.
- Emit initial property values before subscribing to property changes.

Non-responsibilities:

- No Locus graph schema or `FieldBinding<T>`.
- No generated graph contracts.
- No standard service definitions.

### `dev-widgets`

Internal development crate for primitive widgets used to exercise framework APIs.

Responsibilities:

- Depend on `shell-core` as a consumer.
- Define role-specific window configs for development widgets, such as a test panel or OSD-like primitive.
- Keep dev widgets out of the framework API and out of later user-facing shell implementations.
- Keep styling in external CSS files.

### `standard-dbus`

Feature-gated typed definitions for common D-Bus services.

Responsibilities:

- Expose common service objects and properties as `dbus::Object<T>` and `dbus::Property<T, V>`.
- Keep runtime watching/provider implementation in the `dbus` crate.
- Keep each standard service behind an opt-in feature such as `upower`.
- Provide definitions consumers can import directly, for example `standard_dbus::upower::DISPLAY_DEVICE`.

### Future user-facing widget crates

Standalone shell widget binaries such as bars, OSDs, and notifications.

Responsibilities:

- Live outside the core framework boundary.
- Start their own Relm4 applications.
- Load their own CSS.
- Decide their own shell roles, placement, exclusive zones, and behavior.
- Use `shell-core` only for generic layer-shell setup.

## Implementation Phases

### Phase 1: Shell Core

1. Convert the repository into a Cargo workspace if needed.
2. Add root-level `shell-core`.
3. Add GTK4, Relm4, and layer-shell dependencies.
4. Implement generic layer-shell window creation.
5. Keep helpers small and explicit around `gtk::Window`.
6. Add root-level `dev-widgets` as a consumer crate for development-only primitive widgets.

### Phase 2: Procedural Macros

1. Add root-level `macros` as a `proc-macro = true` crate.
2. Implement attribute parsing for field-to-resolution bindings.
3. Preserve the wrapped Relm4 component implementation.
4. Generate cache state and update messages.
5. Add async subscription scaffolding for D-Bus updates.
6. Integrate with `locus-dbus` once the local proxy API is available.

### Phase 3: First Widget

1. Add the first user-facing widget crate outside the core framework boundary.
2. Create a top panel with external CSS.
3. Build the panel's layer-shell config in the consuming crate.
4. Apply that config through `shell-core`.
5. Bind visible labels through `macros`.
6. Verify that widget startup, CSS loading, and layer placement work independently.

### Phase 4: OSD Widget

1. Add a user-facing OSD widget crate outside the core framework boundary.
2. Build OSD placement and transient behavior in the consuming crate.
3. Subscribe to volume and brightness graph paths.
4. Implement transient display behavior without blocking the GTK thread.

## Engineering Guardrails

- Do not block the GTK UI thread with D-Bus work.
- Use Relm4 command spawning, Tokio, or GLib async facilities for subscriptions.
- Avoid allocations in render/watch paths where practical.
- Prefer `as_str()` and precomputed model state over `format!` inside `#[watch]`.
- Keep CSS in stylesheet files and attach classes from Rust.
- Avoid hardcoded visual styling in Rust.
- Keep each widget binary independently runnable.
- Keep macro output understandable enough to debug with `cargo expand`.
- Add tests around macro parsing before expanding generated behavior.

## Open Integration Questions

- Exact shape of the existing `locus-dbus` Resolve proxy API.
- Whether the final workspace root is this repository or a parent `locus` workspace.
- Which GTK4 layer-shell crate is actively maintained and compatible with the target platform.
- Whether async subscriptions should standardize on Tokio, GLib, or a thin compatibility layer.
- The final D-Bus payload format for `ResolveChanged` values.
