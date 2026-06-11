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
