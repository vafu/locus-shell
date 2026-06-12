# providers Refactor Notes

## Current Role

`providers` is the backend-neutral contract crate for asynchronous typed value sources. It intentionally stays independent of GTK, Relm4, D-Bus transport details, and shell widget policy.

The crate now owns the stream-native provider contract, direct Tokio cancellation, subscription task ownership, shared latest fanout, and a transitional runtime spawn helper used by current framework code.

## Public Surface

- `Provider<T>`, `provider_for`, and `run_provider` define and execute typed asynchronous value sources.
- `CancellationToken` is re-exported from `tokio_util` and is passed directly to provider streams and subscription tasks.
- `Subscription` and `SubscriptionGroup` define lifecycle ownership and cancel plus abort owned tasks.
- `ProviderExt` currently only exposes `shared`.
- Any `tokio_stream::Stream<Item = Result<T, E>>` is a provider through the blanket `Provider<T>` implementation.
- `spawn` exposes the crate-owned Tokio runtime.
- `ProviderError` remains as a small display-level error type; provider stream errors are otherwise structured through each provider's `Error` associated type.

## Step-By-Step File Walkthrough

1. `provider/core/src/lib.rs` - public module map, `Provider<T>` trait, blanket stream implementation, `run_provider`, and `ProviderExt::shared`.
2. `provider/core/src/subscription.rs` - subscription and subscription group ownership. Read after the trait to understand direct `CancellationToken` lifecycle and task abort semantics.
3. `provider/core/src/shared.rs` - shared latest provider wrapper. Read after subscription because it coordinates one upstream task across active stream subscribers.
4. `provider/core/src/error.rs` - small display-level provider error type.
5. `provider/core/src/runtime.rs` - shared Tokio runtime and transitional `spawn` entrypoint.
6. `provider/core/src/test.rs` - contract, lifecycle, stream, and shared fanout tests.

## Internal Structure

- The contract layer is centered on `Provider<T>::stream(self, CancellationToken) -> Stream<Item = Result<T, E>>`.
- Lifecycle state is split between cooperative Tokio cancellation and RAII subscription ownership (`Subscription`, `SubscriptionGroup`).
- `runtime.rs` owns one global Tokio multi-thread runtime used by provider helpers and combinators that need to spawn background work.
- Custom map/combine/switch operators were removed from provider core; ordinary stream adapters should be used until concrete framework needs justify thin helpers.

## Behavior Summary

`run_provider` drives one provider stream and forwards every `Result<T, E>` item into a callback. Cancellation is cooperative through direct `CancellationToken`, while `Subscription` also aborts its owned Tokio task on drop or explicit cancel.

`shared` uses `tokio::sync::watch` to expose latest state. It starts upstream on the first active subscriber, cancels upstream when the last subscriber drops, and restarts on a later subscriber.

## User Notes

- Provider direction from review: avoid adopting Rx/RxRust for now; use standard Rust async pieces where possible.
- Sharing upstream values is mandatory for app performance. Duplicate D-Bus/Locus subscriptions should be treated as a design bug unless explicitly requested.
- Explore simplifying value flow around `Stream`/`StreamExt`, with `tokio::sync::watch` as the default shared latest-state primitive.
- Keep explicit lifecycle ownership (`Subscription` / `SubscriptionGroup` or equivalent), because streams alone do not own spawned task lifetime or async remote cleanup.
- Keep the `Provider` name/domain concept for consumer and macro-facing APIs, even if internals move toward streams.
- Keep a type-checking helper like `provider_for::<T, _>(source)` so generated code has a focused compile-time assertion for source value types.
- Provider errors may be valid UI model state. Avoid flattening errors too early; preserve structured/typed errors until the display/log boundary where possible.
- Consumers should not have to think about value sharing for common reusable sources. If a provider source might be reused, it should be shared automatically by construction or runtime policy.
- For `provider/core/src/context.rs`, avoid growing `ProviderContext` into a bag of runtime services. The contract likely only needs cancellation token semantics.
- Avoid Rx vocabulary such as `Disposable`; keep Rust/domain naming.
- Combinators may cancel sibling/upstream/downstream work on terminal errors, but cancellation should remain cooperative and documented.
- Automatic shared sources should use refcounting semantics: start on first subscriber, stop when the last subscriber drops.
- Provider direction from review: commit to Tokio-native primitives rather than maintaining local equivalents or adopting Rx.
- Replace the local cancellation token with `tokio_util::sync::CancellationToken`.
- Keep `Provider<T>` as the macro-facing domain trait, but move its internals toward a stream-producing shape using `tokio_stream::Stream<Item = Result<T, E>>`.
- Use `tokio::sync::watch` for shared latest-value fanout; use `broadcast` only for true event streams if needed.
- `Subscription` is still useful in a Tokio-stream design as an RAII owner for component forwarding tasks.
- `Subscription::cancel` should stop the subscription: request cooperative cancellation and abort the task. If request-only cancellation is needed, name it separately.
- Avoid half-initialized subscriptions and remove `set_task`; construct subscriptions with their task already present or provide a spawn helper.
- `SubscriptionGroup::cancel` should drain/drop/cancel subscriptions immediately if `cancel` means stop now.
- Separate component lifecycle from shared upstream lifecycle: component subscriptions own forwarding tasks, while refcounted shared-source state owns upstream tasks.
- In a stream-producing provider design, `ProviderSender` should be removed entirely from the core contract.
- Provider error state should be structured, not string-only.
- Prefer generated source messages shaped as one variant per field carrying `Result<T, E>` instead of separate success and failure variants.
- The macro/provider contract needs a way to make each provider's error type nameable so generated models can store structured per-field errors.
- Provider runtime ownership should move out of `providers`; `ShellApp` should own provider async runtime setup as part of process lifecycle.
- Macro-generated code should not depend on a hidden global runtime owned by `providers`. Prefer a framework spawn path initialized/owned by `ShellApp`, while keeping macro ergonomics simple.
- `StreamProvider` should disappear if `Provider<T>` becomes stream-native; streams should be the core provider value-flow shape rather than an adapter.
- Provider stream items should be `Result<T, E>` universally so generated source messages can carry `Result<T, E>` per field.
- Shrink `ProviderExt` to only direct consumer-facing helpers that remain justified.
- In a stream-native provider design, rely on `tokio_stream::Stream`/`StreamExt` mapping instead of custom `ProviderExt::map`.
- Move sharing out of a manual generic combinator mindset and into source/runtime policy for reusable descriptors.
- Remove `combine_latest` for now; reintroduce it only if real widget code needs it, preferably as a thin stream helper.
- Keep switch-map behavior as an internal capability if needed, but remove it from generic consumer-facing `ProviderExt`.
- Push selected graph node -> dependent collection flows into Locus/schema provider helpers instead of exposing generic client-side reactive composition.
- Reducing public switch/path composition may allow `locus_provider::Path` to become internal/generated descriptor data rather than a user-facing abstraction.
- Shared latest sources should be keyed/cached by descriptor identity, e.g. D-Bus bus/service/path/interface/property or Locus source/relations/property/query.
- Shared source lifecycle should start upstream on first active subscriber, stop on last subscriber drop, and restart on a later subscriber.
- Shared source state should track active stream subscribers explicitly, not only cloned handles.
- Use a small Rust/domain enum for latest shared state, for example pending/value/error, rather than Rx vocabulary. `Result<Option<T>, E>` is acceptable but less explicit.

## Findings

- Completed: `Provider<T>` is stream-native, `ProviderContext`/`ProviderSender`/custom cancellation/combine/switch/map/stream adapter modules are removed, and `Subscription` owns tasks at construction.
- Completed: `SharedProvider` has explicit active-subscriber refcounting with stop-on-last-drop and restart-on-later-subscriber behavior.
- Direction: `rxrust` should not be adopted now. Locus should own graph reactivity server-side, and Rust/Tokio primitives cover current provider-core needs.
- Direction: automatic descriptor-keyed sharing still belongs in D-Bus/Locus descriptor/runtime policy rather than as a manual generic combinator requirement.
- Gap: downstream crates (`provider/dbus`, `provider/locus`, `shell/macros`, `dev-widgets`) still need migration from the removed callback provider API.
- Gap: `providers::spawn` is still a transitional global runtime helper; long-term runtime ownership should move toward ShellApp/framework initialization.

## Refactor Plan

1. Migrate `dbus-provider` to implement `Provider<T>::stream(self, CancellationToken)` and produce `Stream<Item = Result<T, WatchError>>`.
2. Migrate `locus-provider` to the same stream-native provider contract.
3. Update macro-generated forwarding code to drive provider streams and generate `Msg::Field(Result<T, E>)`-style messages with nameable error types.
4. Replace dev-widget examples that rely on removed generic `combine_latest`/`switch_map` with direct stream composition or semantic schema helpers.
5. Design automatic descriptor-keyed sharing for reusable D-Bus/Locus sources so repeated source use does not create duplicate upstream subscriptions.
6. Move provider runtime/spawn ownership from `providers` into `shell-core::ShellApp` or a ShellApp-initialized runtime module.
7. Reintroduce richer stream helpers only when concrete widget code proves they are needed.

## Tests And Verification

- `cargo test -p providers` passes: 12 unit tests, 0 doctests.
- `cargo check` for the full workspace is expected to fail until downstream crates are migrated from the removed callback provider API.

## Open Questions

- Should the global provider runtime remain the default long-term API, or should the roadmap's possible `ProviderSpawner` / `providers-tokio` split happen before macro output depends heavily on `providers::spawn`?
- Should `ProviderError` preserve structured error sources instead of flattening all provider runner failures to strings?
