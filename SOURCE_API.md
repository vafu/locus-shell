# Observable Source API Design

## Summary

The long-term user-facing source API should be Observable-first.

Widget models store plain values. Source expressions create typed observables.
Macros subscribe those observables and write emitted values into Relm4 model
fields.

```text
locusfs source function / D-Bus / user source function
    -> Observable<Result<T, E>>
    -> shell macro subscription
    -> Relm4 Msg
    -> plain model field: T
```

`ObservableSource<T>` is not part of the target design. The migration replaces
it with Observable sources instead of keeping both abstractions alive.

## Authoring Model

Model fields keep the existing binding syntax:

```rust
#[shell_macros::model]
pub struct ProjectLabel {
    pub workspace: String,

    #[source(project_label(workspace.clone()))]
    pub label: ProjectLabelView,
}
```

The field type is the cached value type, not an observable type. The generated
sidecar module owns subscription lifecycle, error tracking, and dirty-field
updates.

Derived sources are ordinary Rust functions annotated with a source-definition
macro:

```rust
#[shell_macros::observable]
fn project_label(
    workspace: String,

    #[observe(workspace_name(workspace.clone()))]
    workspace_name: Observable<String>,

    #[observe(workspace_project(workspace.clone()))]
    project: Observable<Option<String>>,

    #[observe(project_display_main(project.clone()))]
    project_name: Observable<Option<String>>,

    #[inject]
    theme: Arc<ThemeConfig>,
) -> Observable<ProjectLabelView> {
    Observable::combine_latest3(workspace_name, project, project_name)
        .map(move |(workspace_name, project, project_name)| ProjectLabelView {
            primary: project_name.unwrap_or(workspace_name),
            has_project: project.is_some(),
            accent: theme.project_accent(),
        })
}
```

The generated public function keeps only explicit call-time context parameters:

```rust
fn project_label(workspace: String) -> Observable<ProjectLabelView>
```

Parameters are classified explicitly:

- Plain parameters are caller-provided context, such as a row's locusfs node path string.
- `#[observe(expr)]` parameters are reactive observable dependencies created by
  generated code.
- `#[inject]` parameters are real DI services resolved from the configured
  application dependency injector.

Do not use `#[source]` on derived-source function arguments. Reserve
`#[source(...)]` for model fields so the same word always means "bind this
model value from this source expression."

## Macro Responsibilities

`#[shell_macros::model]`:

- Parses model-field `#[source(expr)]` attributes.
- Type-checks `expr` as an observable source for the field value type.
- Generates Relm4 message variants, dirty tracking, subscription startup, error
  storage, and cancellation ownership.
- Keeps the user model as plain state.

`#[shell_macros::observable]`:

- Parses a user function returning `Observable<T>`.
- Removes `#[observe]` and `#[inject]` parameters from the public call
  signature.
- Builds observed dependency expressions from the explicit context arguments
  and previously declared observed values.
- Resolves explicit DI services.
- Calls the user body with all parameters and returns the resulting observable.

`#[observe(expr)]`:

- Describes an observable dependency for a derived source function.
- May reference plain context parameters and earlier observed parameters.
- Supports dynamic dependencies such as `project_display_main(project.clone())`
  where `project` is itself `Observable<Option<String>>`; generated code should
  implement this as switch/restart behavior over the inner observable.

`#[inject]`:

- Resolves stable services from a DI layer, for example clients, config,
  loggers, theme policy, caches, or runtime handles.
- Does not inject reactive graph values. Reactive values use `#[observe]`.

## Observable Contract

The shell owns the public `Observable<T>` alias/re-export, backed by `rxrust`.
Consumer code should import the shell-owned name, but authoring should use
normal `rxrust` operators rather than shell-specific operator wrappers.

Required v1 semantics:

- Items carry `Result<T, E>` internally so source failures remain observable.
- Infallible `Observable<T>` authoring should be ergonomic; fallible sources
  should still be expressible when UI state wants to model errors.
- Backend sources are shared and replay latest values by descriptor key where
  reuse is expected.
- Upstream work starts on first active subscription, stops when the last
  subscriber drops, and restarts for later subscribers.
- Cancellation must be cooperative and owned by generated subscription handles.

Current implementation status:

- Shell-core uses RxRust's error channel for source failures; macro-generated
  subscriptions turn those terminal errors into model messages.
- Primitive locusfs sources share by normalized backend path, primitive kind,
  and emitted type.
- Derived semantic sources that would otherwise rebuild the same graph for many
  callers should use `source::shared_by_key(kind, key, || ...)`.
- Descriptor cache entries are weak while no observable/subscription holds the
  shared source alive, so dynamic semantic keys do not need process-lifetime
  strong retention.

Useful operators for shell authors:

- `map`
- `filter_map`
- `distinct_until_changed`
- RxRust's binary `combine_latest`, plus `shell_rx_macros::combine_latest!`
  when fixed-arity heterogeneous composition would otherwise require repeated
  tuple mapper functions
- `switch_map` or equivalent dynamic dependency support
- `debounce` and throttling where widget behavior needs transient timing
- `Observable::create` for low-level custom sources

## DI Boundary

The Observable source API is DI-inspired, but it is not a general DI container.

Use DI for stable services:

- Locus and D-Bus clients
- configuration and theme state
- caches and registries
- logging and metrics
- service-specific command clients

Use Observable source construction for dynamic values:

- Locus graph properties and relations
- D-Bus properties and object collections
- timers, file watches, and process output
- derived UI DTOs from multiple sources

Use `nject` behind a small shell facade for stable services. The macro should
target shell-owned facade APIs, not `nject` internals, so service construction
can evolve without changing user source functions.

DI should not be responsible for reactive Locus graph resolution, switch-map
behavior, subscription sharing, or Relm4 model updates.

If a context-dependent factory is needed, keep it in macro/codegen internals.
The public authoring surface should still be a Rust function returning
`Observable<T>`, not a trait users implement.

## Locusfs Source Direction

Locusfs source helpers should return observables directly in the target API:

```rust
workspace_name(workspace.clone()) -> Observable<String>
workspace_project(workspace.clone()) -> Observable<Option<String>>
project_display_main(project.clone()) -> Observable<Option<String>>
selected_workspace_windows() -> Observable<Vec<LocusPath>>
```

Path-local observable creation is also supported for primitive graph reads:

```rust
source::root()
    .child("context/selected/workspace")
    .as_relation()
```

Consumer source modules own semantic collection helpers and UI view-model
composition. Direct locusfs reads and watches stay behind shell-core Observable
primitives. Nodes and relation targets are represented as `LocusPath`; this
workspace no longer exposes schema descriptors or `NodeRef`.

Current generic D-Bus paths come from locusfs and should be consumed through
normal `LocusPath` composition:

```text
/dbus/<service>/objects/<relative-object-path>/<Property>
/dbus/<service>/methods/<relative-object-path>/<Method>
```

If the object equals the service ObjectManager path, its properties live
directly under `/dbus/<service>/objects`. Services whose ObjectManager path is
`/` expose object paths relative to `objects`. Objects outside the configured
ObjectManager root live under `_absolute`. Legacy `object`, `@properties`,
`@methods`, and method `/call` suffix paths are not part of the current layout.

## Migration Notes

The current repository no longer has a stream-native `ObservableSource<T>` contract.
The remaining migration work is:

1. Add shared helper modules only where handwritten locusfs source functions
   become duplicated across consumers.
2. Add `#[shell_macros::observable]`, `#[observe(...)]`, and `#[inject]` for user-authored derived sources.
3. Convert future D-Bus/common service helpers to emit observables or proxy through locusfs.
4. Replace remaining ad hoc consumer source composition with annotated observable functions where that improves ergonomics.

During migration, do not add public `ObservableSource`-style composition APIs.
Composition belongs in `rxrust` operators and `#[shell_macros::observable]`
functions.
