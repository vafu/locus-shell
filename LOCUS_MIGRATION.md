# Typed Locus Schema And GTK Binding Migration

## Current Shape

Locus schema typing and Rust codegen live in the adjacent `~/proj/locus`
workspace. `locus-shell` consumes generated schema modules from consumer crates;
it does not own schema-specific models, path constants, or decoding rules inside
`locus-provider`.

The current bridge is:

- YAML schema defines graph models, property types, paths, relations, and
  collection metadata.
- `locus-codegen` emits Rust model markers, property descriptors, path
  constants, relation constants, typed property wrappers, and semantic extension
  traits.
- `provider/property` defines shared typed `Property<Target, Value>` and
  inspectable `PropertyBinding<T>` contracts.
- `provider/locus` owns generic Locus graph descriptors and D-Bus-backed raw
  watch mechanics.
- Generated schema wrappers convert raw Locus wire strings into expected Rust
  property values.
- `shell-macros` currently accepts `providers::ObservableSource<T>` source expressions.
  The target source contract is Observable-first, as defined in
  `SOURCE_API.md`.

## Generated API Direction

Generated consumer schema should expose semantic typed sources, for example:

```rust
#[source(paths::SELECTED_WORKSPACE.windows())]
pub windows: Vec<locus_provider::NodeRef<model::Window>>,

#[source(window.title())]
pub title: String,

#[source(window.is_selected())]
pub selected: bool,
```

The widget author should not manually wire raw graph directions such as
`sources`, `targets`, `SubscribeSources`, or `SubscribeTargets` unless they are
working at an explicitly low-level escape hatch.

## Locus Source Responsibility

`locus-provider` remains generic and schema-free. It owns:

- `Path<Target>` and raw resolved-path property bindings.
- `NodeRef<Model>` and direct-node property bindings.
- `Relation<Source, Target>`, target bindings, and node-list bindings.
- Stream-native watcher implementations that can be adapted into shell-owned
  observables.
- Collection diff application for UI-facing node id lists.

It does not own:

- schema-specific models such as `Window` or `Workspace`;
- schema property constants such as `Window::TITLE`;
- typed decoding from Locus wire strings into schema value types;
- widget placement, GTK, Relm4, or product behavior.

## DBus And Property Contract

DBus and Locus property-backed sources share the property vocabulary through
`property-provider`, but each backend keeps its own runtime key type and watcher
implementation.

Base source streams pass backend errors as `Err(E)`. Defaulting or swallowing
errors should be explicit wrapper behavior, not part of the base watcher.

## Macro Contract

Macros should stay backend-agnostic:

- `#[source(...)]` on model fields binds a plain model value from an observable
  source expression.
- Current `ObservableSource<T>` expressions may be accepted through a bridge while
  migrating existing crates.
- Derived user sources use `#[shell_macros::observable]` functions with
  `#[observe(...)]` reactive dependencies and explicit `#[inject]` DI services.
- Generated messages carry result-carrying source updates per field.
- Per-field dirty tracking drives `#[bind(field)]` view updates.
- Legacy `#[locus(...)]` remains compatibility syntax only.

## Remaining Migration Work

- Add compile-expanded macro tests for realistic Relm4 components.
- Implement the Observable source API from `SOURCE_API.md`.
- Add row hydration helpers for collection results when real widgets need
  summary models instead of one property subscription per child row.
- Design backend descriptor-keyed sharing registries so repeated DBus/Locus
  observable sources reuse one upstream watch automatically.

## Assumptions

- YAML remains the source schema format.
- Locus wire property values may remain strings until locus-core changes; if
  typed D-Bus values are introduced later, generated schema conversion changes
  first, not widget code.
- Consumer crates own generated schema modules and role-specific widgets.
