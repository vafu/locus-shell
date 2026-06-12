# Work Log

## Provider Core

Created the `providers` crate as a small reusable core for typed asynchronous sources. It defines `Provider<T>`, `ProviderContext`, `ProviderSender<T>`, cancellation tokens, subscriptions, subscription groups, and a `run_provider` helper. This keeps provider mechanics independent of GTK, Relm4, and D-Bus.

## D-Bus And Locus Migration

Migrated current Locus graph field bindings and pure D-Bus property bindings to implement `providers::Provider<T>`. Existing `watch_field` and `watch_property` remain for compatibility, but macro output now targets the unified provider contract. Pure D-Bus property watching now emits the initial property value before listening for changes.

## Locus Graph Split

Split direct Locus graph support into `locus-provider`, which owns generated graph contracts, typed decoding, `watch_field`, and the provider implementation for `FieldBinding<T>`. The generic `dbus-provider` crate now only owns reusable D-Bus object/property bindings, which keeps optional end-user features separated by capability.

## Generated Schema Workflow

Added `scripts/locus-provider-schema` so generated graph contracts can be regenerated or checked against the adjacent `~/proj/locus` checkout. This keeps generated code vendored for normal builds while making drift explicit.

## Provider Output Validation

Added `providers::provider_for<T, _>(provider)` and made macro-generated watchers call it with the declared field type before running a provider. This gives generated code a focused compile-time check that a source expression really provides the model field type.

## Provider Composition

Added `ProviderExt::combine_latest` for deriving typed summary values from two provider sources. The combiner receives references to the latest left/right values and emits after both sides have produced at least one value, which matches shell summary use cases such as combining graph state with system state.

## Derived Chain Proof

Added a dev-widget unit test that combines two providers into a typed `BarSummary` without changing the visible bar. This proves the intended consumer-facing shape for summarized state while keeping the current GTK primitive minimal.

## Tokio Provider Runtime

Moved generated provider task spawning from `relm4::spawn` to `providers::spawn`, backed by a shared Tokio runtime. This aligns Locus and D-Bus providers with the Tokio-flavored Zbus dependencies and gives custom async providers a consistent runtime target.

## Provider Family Layout

Moved provider-family crates under `provider/`: `provider/core` for the `providers` crate, `provider/dbus` for generic D-Bus properties, and `provider/locus` for Locus graph bindings. Package names and user-facing imports stay stable while the directory layout now reflects responsibility areas.

## Common Providers Rename

Renamed the reusable common provider definitions crate from `standard-dbus` to `common-providers` and moved it under `provider/common`. This keeps UPower and future shared service definitions in the provider family without implying they are standards-layer code.

## Macro Provider Dispatch

Removed macro-side source classification. `#[locus(source = ...)]` now treats the source as a generic provider expression. This makes custom providers possible without teaching the macro about every backend.

## Subscription Lifecycle

Generated locus models now include a hidden `SubscriptionGroup`, and the component macro attaches the started subscriptions to `model.locus` during `init`. This gives provider tasks a component-owned lifecycle handle instead of a completely untracked fire-and-forget shape for both inline bindings and typed model bindings.

## Derived Provider Ergonomics

Added an initial `ProviderExt::map` combinator for simple derived values. More complex chains such as combining workspace, window, agent, and build sources should be modeled as explicit derived providers rather than embedded in `shell-core`.

## AGS Reference

Explored `/home/v47/.config/ags` via an agent and documented the bar’s data sources, domain models, update patterns, styling, and feature inventory in `AGS_REFERENCE.md`. The AGS code is treated only as a product/behavior reference.

## Verification Pass

After the provider migration, ran the full workspace formatter check, `cargo check --workspace`, and `cargo test --workspace --all-features`. Subagents also reviewed the provider, D-Bus, and macro slices with the Rust guidance in mind and made small fixes around cancellation, provider docs, and generated subscription storage.

## Architecture Review Fixes

Addressed the architecture review findings by making provider cancellation awaitable, letting subscriptions own spawned Tokio task handles, and selecting D-Bus/Locus watch loops against cancellation. Added provider-neutral macro spelling with `#[source(...)]` and `#[bind(field)]` while keeping `#[locus(...)]` as a compatibility path. Updated the roadmap, blueprint, migration notes, and PlantUML architecture source to reflect generic providers, the `provider/common` crate, and the remaining hardening work.

## Shared Sources And Macro State

Added shared/replay providers to `providers` so multiple fields can reuse one upstream source and late subscribers receive the latest value. Updated the macro model contract to default to a `sources` module, support explicit `state = sources`, and hide generated dirty/error/subscription state behind one private `__shell` runtime field. The dev bar now uses `BarSources` and shares one UPower battery percentage source between the progress bar and a derived text label.

## Tokio Stream Provider Core

Added `tokio-stream` to the provider core as an implementation-facing stream substrate. `stream_provider` adapts `Stream<Item = Result<T, E>>` into the existing `Provider<T>` contract, and `ProviderExt::switch_map` replaces active downstream subscriptions when an upstream key changes. This gives Locus collection work the primitive needed for selected-node-to-dependent-list flows without exposing a broad Rx-style runtime to widget authors.

## Locus Collection Providers

Added Locus target and node-list bindings in `locus-provider`. `Path<T>::target()` now provides the resolved target node id, `Path<T>::all()` materializes `SubscribeResolveAll` diffs into `Vec<NodeId>`, and relation descriptors can create `sources(...)` or `targets(...)` list providers. A dev-widget compile test proves the intended selected-workspace-to-window-list shape through `switch_map`; the next cleanup is moving relation descriptors into Locus Rust codegen.

## Semantic Workspace Windows

Added `Path<Workspace>::windows()` as the first consumer-facing semantic collection helper. It resolves the selected workspace, switches the live Locus subscription when selection changes, and hides the raw `sources("workspace")` direction from widget authors. The current implementation keeps no independent shell-side cache; repeated consumers can opt into `ProviderExt::shared()` today, while backend-wide subscription registries remain an optional later optimization.

## Wrapped Component Inputs

Relaxed typed model subscription startup so generated provider messages can feed any Relm4 component input that implements `From<sources::Msg> + Send`. This lets a component keep the convenient generated source model while still defining local messages for dynamic child widgets, factory updates, or imperative GTK reconciliation.

## Selected Workspace Window Rows

Added direct typed node property bindings through `locus_provider::node::<model::Window>(id).property(model::Window::TITLE)`, backed by the existing Locus `WatchNode` D-Bus path with an empty relation path. Updated the dev bar to subscribe to `paths::SELECTED_WORKSPACE.windows()`, reconcile one GTK label per window node, start an independent title subscription for each row, and style the selected row from `paths::SELECTED_WINDOW.target()`.

## Row-Local Window Bindings

Moved window row title and selection subscriptions into a dedicated `WindowTitle` Relm4 component. Typed source models can now start subscriptions from instance data, so a child model can hold `window: NodeRef<Window>` and declare `#[source(window.title())]` plus `#[source(window.is_selected())]`. Models with context fields get a generated constructor, for example `WindowTitleSources::new(window)`, instead of requiring invalid placeholder defaults. The bar only reconciles the list of child components, which removes the parent-level title message boilerplate and better matches the intended widget composition model.
