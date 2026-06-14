# Locus Shell Project Blueprint

## Objective

Locus Shell is a Rust/Relm4 desktop shell for high-performance, low-footprint widgets such as bars, OSDs, and notifications. It replaces heavier GJS/AGS-style shell code with native GTK4 binaries and server-driven reactivity from the `io.github.Locus` D-Bus service.

The shell should provide a concise authoring model for widgets while preserving the runtime characteristics of compiled Rust and GTK4.

## Core Constraints

- No JavaScript engine, embedded interpreter, or client-side reactive runtime.
- Each major widget is a standalone binary, for example `locus-bar` and `locus-osd`.
- Widget failures must be isolated to their own process.
- Locus graph-derived UI state is driven by `io.github.Locus.Graph.Resolve`.
- Locus graph providers subscribe through `SubscribeResolve` and redraw only when `ResolveChanged` is emitted.
- Non-Locus sources such as UPower, time, weather, media, and custom user logic enter the UI through typed `providers::Provider<T>` implementations.
- Relm4 boilerplate should be hidden behind procedural macros where practical.
- D-Bus work must run asynchronously outside the GTK UI thread.
- Styling belongs in external CSS files, not hardcoded Rust widget properties.

## Target Workspace Layout

```text
locus/
├── Cargo.toml
├── shell/
│   ├── core/        # package: shell-core
│   └── macros/      # package: shell-macros
├── dev-widgets/
├── provider/
│   ├── core/        # package: providers
│   ├── locus/       # package: locus-provider
│   ├── dbus/        # package: dbus-provider
│   ├── property/    # package: property-provider
│   └── common/      # package: common-providers
├── rsynapse-shell/  # package: rsynapse-shell, AGS migration playground
```

This repository is the framework workspace. User-facing shell implementations should live in separate crates or repositories that consume these framework crates.
`rsynapse-shell` is an in-repository migration playground used to exercise the
framework while porting the local AGS configuration; framework crates must not
take product-specific policy from it.

## Runtime Architecture

Locus Shell widgets are thin UI processes. They subscribe to typed provider sources, translate provider changes into Relm4 messages, and let Relm4 update watched GTK properties. Locus graph-derived providers receive server-side changes from `locusd` over D-Bus; pure D-Bus and custom providers use the same UI binding path.

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
- Install the process-level provider task runtime used by generated provider subscriptions.
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

### `shell-macros`

Procedural macro crate that reduces Relm4 widget boilerplate and binds UI state to typed providers.

Initial dependencies:

- `syn`
- `quote`
- `proc-macro2`

Responsibilities:

- Parse `#[shell_macros::component(...)]` attributes stacked with `#[relm4::component]`.
- Parse typed state models annotated with `#[shell_macros::model]`.
- Extract typed provider sources from model fields of the form:

```rust
#[shell_macros::model]
pub struct Bar {
    #[source(schema::paths::SELECTED_WINDOW
        .property(schema::model::Window::TITLE))]
    pub selected_window_title: String,
}
```

- Keep `#[shell_macros::component(model = Bar)]` focused on Relm4 lifecycle wiring and view tracking.
- Preserve legacy component-level bindings during the transition:

```rust
selected_window_title: String = schema::paths::SELECTED_WINDOW
    .property(schema::model::Window::TITLE)
```

- Generate model state for resolved values.
- Generate message handling for field updates.
- Generate async subscription setup that forwards provider updates into Relm4 input messages.
- Support binding providers:
  - Locus graph property bindings through generated schema expressions backed by
    raw `locus_provider::LocusPropertyBinding<Target>` streams and generated
    typed schema wrappers.
  - Pure D-Bus property bindings through typed `dbus_provider::Object<Target>` and shared `property_provider::Property<Target, Value>` descriptors.
  - Consumer-defined providers that implement `providers::Provider<T>`.
- Rewrite `#[bind(field)]` view setters into Relm4 `#[track(...)]` updates so only widgets bound to the changed field redraw. `#[locus(...)]` remains a compatibility spelling during the transition.

Target authoring shape:

```rust
#[shell_macros::model]
pub struct Bar {
    #[source(paths::SELECTED_WINDOW.property(model::Window::TITLE))]
    pub selected_window_title: String,
    #[source(DISPLAY_DEVICE.bind(DisplayDevice::PERCENTAGE))]
    pub battery_percent: f64,
}

#[shell_macros::component(model = Bar)]
#[relm4::component(pub)]
impl SimpleComponent for Bar {
    type Input = sources::Msg;

    view! {
        gtk::Window {
            gtk::Label {
                #[bind(selected_window_title)]
                set_label: |title| title.as_str(),

                #[bind(selected_window_title)]
                set_css_classes: window_title_classes,
            }
        }
    }
}
```

Generated concepts:

- A `sources` module scoped beside the component by default.
- A user-authored state struct containing one field per typed binding.
- One private generated `__shell` runtime field for dirty tracking, last errors, and subscription ownership.
- Unified update message with one generated variant per field.
- An initialization hook that starts provider subscriptions and emits Relm4 messages.
- Per-field dirty tracking, cleared after Relm4 updates the view.
- View setter adapters that receive typed references to generated model fields.
- Dynamic styling through normal GTK setters such as `set_css_classes`; CSS contents still live in external stylesheets.

### `providers`

Small reusable provider contract crate.

Responsibilities:

- Define `Provider<T>` for typed asynchronous value sources backed by `tokio_stream::Stream<Item = Result<T, E>>`.
- Use `tokio_util::sync::CancellationToken` directly for cooperative cancellation.
- Provide `Subscription` and `SubscriptionGroup` for lifecycle ownership.
- Provide shared latest providers for reusing one upstream source across multiple derived fields.
- Use a framework-installed task spawner for subscription tasks; the provider
  contract crate does not create its own Tokio runtime.
- Prefer `tokio_stream::StreamExt` and ordinary Tokio primitives over custom reactive combinator layers until concrete widget requirements justify them.
- Stay independent of GTK, Relm4, D-Bus, and product-specific shell behavior.

Non-responsibilities:

- No D-Bus transport implementation.
- No GTK widget or shell-window policy.
- No common service definitions.

### `locus-provider`

Generic Locus graph binding primitives plus the direct Locus-over-D-Bus provider implementation.

Responsibilities:

- Expose generic `Property<Model, Value>`, `Path<Target>`,
  raw `LocusPropertyBinding<Target>`, `NodeRef<Model>`, relation, target, and
  node-list binding primitives.
- Own generic binding types so the crate can implement raw Locus
  `providers::Provider<String>` subscriptions without violating Rust coherence
  rules.
- Watch `io.github.Locus.Graph.Resolve` through `locus-dbus`.
- Expose typed node-id providers for resolved targets and node-list providers backed by `SubscribeResolveAll`, `SubscribeSources`, and `SubscribeTargets`.
- Keep async Locus D-Bus work off the GTK thread.

Non-responsibilities:

- No generated schema markers, paths, relations, or schema-specific convenience
  helpers such as `Window::TITLE` or `SELECTED_WORKSPACE.windows()`.
- No schema-specific typed decoding. Generated consumer schema modules adapt
  raw Locus wire strings into typed property providers.
- No generic D-Bus object/property model.
- No common service definitions such as UPower.
- No GTK or Relm4 widget policy.

Generated schema code belongs to the consuming crate. In this workspace,
`dev-widgets` carries its own development schema module to prove ergonomics.

### `dbus-provider`

Generic typed D-Bus object/property provider crate.

Responsibilities:

- Expose `dbus_provider::Object<T>`, shared `dbus_provider::Property<T, V>`, and `dbus_provider::PropertyBinding<T, V>`.
- Support session and system bus objects.
- Implement `providers::Provider<T>` for pure D-Bus property bindings.
- Emit initial property values before subscribing to property changes.

Non-responsibilities:

- No Locus graph schema or Locus graph property resolution.
- No generated graph contracts.
- No common service definitions.

### `dev-widgets`

Internal development crate for primitive widgets used to exercise framework APIs.

Responsibilities:

- Depend on `shell-core` as a consumer.
- Define role-specific window configs for development widgets, such as a test panel or OSD-like primitive.
- Keep dev widgets out of the framework API and out of later user-facing shell implementations.
- Keep styling in external CSS files.

### `common-providers`

Feature-gated typed definitions for common D-Bus services.

Responsibilities:

- Expose common service objects and properties as `dbus_provider::Object<T>` and `dbus_provider::Property<T, V>`.
- Keep runtime watching/provider implementation in the `dbus-provider` crate.
- Keep each common service behind an opt-in feature such as `upower`.
- Provide definitions consumers can import directly, for example `common_providers::upower::DISPLAY_DEVICE`.

### Future user-facing widget crates

Standalone shell widget binaries such as bars, OSDs, and notifications.

Responsibilities:

- Live outside the core framework boundary.
- Start their own Relm4 applications.
- Load their own CSS.
- Decide their own shell roles, placement, exclusive zones, and behavior.
- Use `shell-core` only for generic layer-shell setup.

## Implementation Status

`PLAN.md` is the live roadmap. At a high level, the current workspace already has:

- `shell/core` for app startup, CSS/SCSS loading, and generic layer-shell windows.
- `shell/macros` for Relm4 provider-model bindings.
- `provider/core`, `provider/locus`, `provider/dbus`, and `provider/common` for typed provider contracts and implementations.
- `dev-widgets` as a framework ergonomics target, not a user-facing shell.

Future user-facing widgets such as bars and OSDs should be created outside this framework workspace and consume these crates.

## Engineering Guardrails

- Do not block the GTK UI thread with D-Bus work.
- Use provider subscriptions for async work; the current default runtime is Tokio-backed and should remain off the GTK thread.
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
- Whether async subscriptions should settle on Tokio, GLib, or a thin compatibility layer.
- The final D-Bus payload format for `ResolveChanged` values.
