# Work Log

## Provider Core

Created the `providers` crate as a small reusable core for typed asynchronous sources. It defines `Provider<T>`, `ProviderContext`, `ProviderSender<T>`, cancellation tokens, subscriptions, subscription groups, and a `run_provider` helper. This keeps provider mechanics independent of GTK, Relm4, and D-Bus.

## D-Bus And Locus Migration

Migrated current Locus graph field bindings and pure D-Bus property bindings to implement `providers::Provider<T>`. Existing `watch_field` and `watch_property` remain for compatibility, but macro output now targets the unified provider contract. Pure D-Bus property watching now emits the initial property value before listening for changes.

## Locus Graph Split

Split direct Locus graph support into `locus-graph`, which owns generated graph contracts, typed decoding, `watch_field`, and the provider implementation for `FieldBinding<T>`. The generic `dbus` crate now only owns reusable D-Bus object/property bindings, which keeps optional end-user features separated by capability.

## Generated Schema Workflow

Added `scripts/locus-graph-schema` so generated graph contracts can be regenerated or checked against the adjacent `~/proj/locus` checkout. This keeps generated code vendored for normal builds while making drift explicit.

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
