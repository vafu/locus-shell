# shell/macros and shell/rx-macros Review

## Scope

Unit reviewed:

- `shell/macros/src/**`
- `shell/rx-macros/src/**`
- Macro-generated subscription behavior in relation to `shell-core::source`
- Representative consumer usage in `rsynapse-shell` where it clarifies macro pressure points

Required context read first:

- `AGENTS.md`
- `PROJECT.md`
- `PLAN.md`
- `SOURCE_API.md`
- `shell/core/src/source/AGENTS.md`

Verification run:

- `CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test -p shell-macros -p shell-rx-macros`
- `CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test -p shell-core source::support::tests`
- `CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test --workspace`

All commands passed on 2026-06-26 01:06 PDT.

## Current Role

`shell-macros` owns Relm4/source binding generation. Its public proc macros are:

- `#[shell_macros::bindings(...)]`: legacy inline binding module generator.
- `#[shell_macros::model(...)]`: typed state model generator for field-level `#[source(...)]`, legacy `#[locus(source = ...)]`, and nested `#[model(source = ...)]`.
- `#[shell_macros::component(...)]`: Relm4 impl transformer that starts generated subscriptions, routes generated messages into model updates, rewrites `#[bind(...)]`/legacy `#[locus(...)]` view attributes into Relm4 `#[track(...)]`, and injects dirty-flag clearing after view updates.

`shell-rx-macros` owns a small `combine_latest!` declarative macro. It is correctly runtime-free: it expands to RxRust `combine_latest` chains and optional final `map`.

## File Walkthrough

`shell/macros/src/lib.rs`

- Thin proc-macro entrypoint. It exposes `bindings`, `component`, and `model`.
- Missing target API surface: `#[shell_macros::observable]`, `#[observe]`, and `#[inject]` are documented in `SOURCE_API.md` but not implemented.

`shell/macros/src/locus_bindings/config.rs`

- Parses component-level typed binding entries of the form `field: Type = source_expr`.
- Parses model field attributes:
  - `#[source(expr)]`
  - legacy `#[locus(source = expr)]`
  - nested `#[model(source = expr)]`
- Stores `source_expr` as arbitrary `syn::Expr`.
- Validates duplicate fields, duplicate generated variants, and more than 128 direct source bindings.

`shell/macros/src/locus_bindings/expand.rs`

- Generates source state modules, `Msg`, `Field`, dirty masks, runtime sidecar state, `start`/`start_async`, and update methods.
- Subscribes once per source-bound field by assigning the expression to `::shell_core::source::Observable<T, _>`, then calling `on_error(...).subscribe(...)`.
- Generates nested `SourceModel` restart behavior for single `Option<T>` context models.

`shell/macros/src/locus_bindings/component.rs`

- Detects sync vs async Relm4 component traits.
- Injects `model.<state>.set_subscriptions(...start...)` after the local `let model = ...`.
- Adds `update` if the component did not define one.
- Injects dirty clearing in `post_view`.

`shell/macros/src/locus_bindings/view.rs`

- Token-walks Relm4 `view!` bodies.
- Rewrites `#[bind(field)]` and legacy `#[locus(field)]` into tracked setter calls.
- Rewrites `#[bind_list(field, row = RowComponent)]` into `ComponentListUpdate` wiring.

`shell/macros/src/locus_bindings/test.rs`

- Unit tests assert parser behavior and generated token strings.
- Coverage is useful but mostly string-fragment based; there are no compile-fail or expanded-behavior tests outside unit tests.

`shell/rx-macros/src/lib.rs`

- Defines and tests `combine_latest!` up to nine sources.
- No runtime state, backend policy, subscriptions, or descriptor concepts are introduced here, which matches `PLAN.md`.

## Source Expression Parsing

Source expressions are intentionally generic today:

- Component attributes parse source expressions as `syn::Expr` after `field: Type =`.
- Model field `#[source(...)]` parses the attribute argument directly as `syn::Expr`.
- Parenthesized source expressions are accepted for legacy component config.
- The macro does not inspect whether an expression is a locusfs property, relation, D-Bus helper, derived source function, or user source.
- Type checking is delegated to generated Rust:

```rust
let source: ::shell_core::source::Observable<FieldType, _> = source_expr;
```

This is the right high-level API direction for authoring: widget authors see ordinary Rust expressions returning shell-owned `Observable<T>`. It also means the macro cannot safely infer descriptor identity from arbitrary source expressions. Token-string equality would miss equivalent expressions and can accidentally merge non-equivalent expressions when runtime context differs.

Decision: do not implement descriptor sharing by canonicalizing arbitrary `#[source(expr)]` token streams.

## Generated Subscription Behavior

For each source binding, generated code currently:

1. Clones the Relm4 sender twice.
2. Creates the observable from the source expression.
3. Calls `.on_error(...)` to send `Msg::Field(Err(error.to_string()))`.
4. Calls `.subscribe(...)` to send `Msg::Field(Ok(value))`.
5. Boxes the RxRust subscription.
6. Wraps it in `unsubscribe_when_dropped()`.
7. Pushes the guard into the generated subscription vector.

The same shape is generated in four places:

- Legacy inline modules.
- Typed model `start`.
- Typed model `start_async`.
- `SourceModel::start_for_source_context`.

Nested source models subscribe to a context observable, drop the previous inner subscription group on each context value, emit a context update message, then start subscriptions for the new context. This is effectively a generated `switch_map` at the subscription layer rather than an Rx operator chain.

## Descriptor Sharing Status

`shell-core::source` already implements primitive descriptor sharing:

- `watch`, `property`, `relation`, `node`, `children`, and `children_events` all call private `shared_source(kind, path, factory)`.
- `shared_source` keys by `{ kind, TypeId, PathBuf }`.
- `ShareReplayHub` starts upstream work on first active subscriber, replays the latest value to later subscribers, and disconnects upstream work when the last subscriber drops.
- Existing source support tests cover last-subscriber disconnect and pending-stream abort behavior.

This means repeated primitive calls such as:

```rust
path.observe_prop_or::<String>("title", String::new())
```

share the same underlying property watch/read work when the normalized path, kind, and output type match.

What is not covered:

- Equivalent derived source functions such as repeated `window_tile_vm(window.clone())`.
- Equivalent composed descriptors that share a semantic source but are not a single primitive path.
- Descriptor reuse across generated model fields before the source expression returns an already-shared observable.
- Cache eviction for many ephemeral paths. The global cache stores strong `Arc` hubs forever, so upstream work stops but descriptor entries accumulate.

## Findings

### API

- The target `#[shell_macros::observable]`, `#[observe(...)]`, and `#[inject]` API is absent. This is the main missing piece for descriptor-keyed sharing of derived source functions without burdening `#[source(expr)]` with descriptor syntax.
- Generated APIs expose RxRust subscription implementation details through `Vec<SubscriptionGuard<BoxedSubscriptionSend>>` in public-ish generated `start`/`set_subscriptions` signatures. A shell-owned alias such as `shell_core::model::SourceSubscriptions` would reduce generated-code leakage.
- Generated message variants carry `Result<T, String>` and generated error state is named `WatchError`. The source API is broader than file watches now, so `SourceError` or `BindingError` would fit better.
- `#[shell_macros::bindings]` and legacy `#[locus(...)]` remain useful compatibility shims but should be documented as transitional. They increase the surface area to maintain during the descriptor work.
- `SourceModel` is Relm4-sync-specific because it takes `ComponentSender<Component>`. Async components have separate generated direct binding support, but nested source-model behavior does not have an async equivalent.

### Redundancy

- Subscription quote blocks are duplicated across legacy modules, typed models, async typed models, and source-model context starts.
- `BindingConfig` and `NestedModelConfig` carry the same field/variant/type/source shape. They differ semantically, but the duplicated storage invites repeated validation and expansion logic.
- Generated `Changed`, `Field::bit`, `WatchError`, and subscription storage are repeated between legacy `Model` and typed-model `Runtime`.
- Consumer source modules repeat common D-Bus object property path patterns, but that is outside this unit. Macro changes should not encode those paths.

### Performance And Concurrency

- Primitive path sharing is in better shape than the macro layer: repeated low-level path observables already share upstream watch/read work.
- Derived observable graphs are still recreated per source expression and per component instance. Primitive leaves may share, but all intermediate `combine_latest`, `switch_map`, maps, allocations, and sender fanout remain per subscriber.
- The global primitive source cache is unbounded by descriptor cardinality. Dynamic rows such as windows, D-Bus objects, menus, and sessions can leave dead cache entries behind after upstream work stops.
- The generated source-model dirty mask validates only direct source bindings for the 128-bit limit. Direct bindings plus nested model fields can exceed 128 generated `Field` variants without an early macro error.
- `start_async` omits nested source-model watchers. Sync `start` includes `#(#nested_watchers)*`; async `start_async` only includes direct `#(#async_watchers)*`. Async components with nested source models would silently miss nested subscriptions.
- Locking in `ShareReplayHub` is short and not held across await points. That part is acceptable. The higher risk is cache growth, not lock contention.

### Tidiness

- Macro output is understandable enough for `cargo expand`, but the repeated subscription expansion makes fixes easy to miss in one path.
- Unit tests validate many parser and expansion cases, but they assert token substrings. Add compile tests for contract-level behavior and errors.
- `WatchError` naming and legacy D-Bus test naming lag behind the Observable-first source API.
- Manual token traversal in `view.rs` is reasonable for Relm4 `view!` rewriting, but errors should stay focused because the parser is not a full Rust AST for the view DSL.

### Best Practices

- Keep `shell-rx-macros` runtime-free. Descriptor sharing should not move into `combine_latest!`.
- Prefer Rx-native composition for derived source functions. Where generated nested `SourceModel` behavior is really dynamic dependency switching, the longer-term `#[observable]` implementation should generate or use `switch_map`-style observable composition rather than open-coded subscription restarts when practical.
- Do not add a public `ObservableSource<T>` replacement. If descriptor support needs public types for macro expansion, keep them shell-owned, minimal, and either normal source API concepts or `#[doc(hidden)]` macro support.
- Add `trybuild` tests for proc-macro diagnostics and generated API shape. Unit string assertions are not enough for descriptor-keyed sharing or async nested subscription correctness.

### Domain-Specific Path Layout

- The macro crates are mostly path-agnostic, which is correct. They should not know about `/dbus/<service>/objects`, `/methods`, `_absolute`, or legacy `@properties`/`@methods`.
- Existing consumer sources still include legacy D-Bus layouts such as `dbus/upower/object/.../@properties`, `dbus/networkmanager/object`, and `object.child("@methods").child(method).prop("call")`. Those should be handled in consumer/source-helper refactors, not by teaching macros legacy or new D-Bus path rules.
- Descriptor keys for locusfs primitives should use normalized `LocusPath`/`PathBuf` values after the latest path layout has been constructed. That keeps equivalent paths shareable without exposing D-Bus implementation detail in macro syntax.
- Macro tests named around D-Bus should be rewritten to assert generic observable expression handling, or updated to examples that reflect the new layout. The macro itself should remain backend-neutral.

## Concrete Refactor Plan

### 1. Consolidate Source Subscription Types

Add shell-owned aliases in `shell-core`, for example:

```rust
pub type SourceSubscription =
    rxrust::subscription::SubscriptionGuard<rxrust::prelude::BoxedSubscriptionSend>;
pub type SourceSubscriptions = Vec<SourceSubscription>;
```

Then generate `::shell_core::model::SourceSubscriptions` instead of spelling RxRust internals in every expanded module.

Also rename generated `WatchError` to `SourceBindingError` or `BindingError`. Keep a compatibility alias only if existing consumers refer to it directly.

### 2. Fix Macro Correctness Gaps Before Adding Sharing

- Validate the combined count of direct source bindings plus nested model fields against the `u128` dirty mask limit.
- Generate async nested source-model support or reject nested source models in async components with a clear compile error until support exists.
- Add tests for both cases.

### 3. Factor Subscription Expansion

Create internal quote helpers in `expand.rs` for:

- direct sync subscription block
- direct async subscription block
- source-model-context subscription block
- error-to-message mapping

This is not a runtime abstraction. It reduces maintenance risk before descriptor work touches every generated subscription path.

### 4. Replace Primitive Sharing Key With a Descriptor Type

In `shell-core::source`, introduce a source descriptor type that can represent both primitives and generated derived sources:

- descriptor namespace or kind
- output `TypeId`
- normalized path or argument parts
- optional function/module identity for generated derived sources

Use it internally to replace the current `{ kind, TypeId, PathBuf }` `SourceKey`.

Important implementation detail: avoid a permanent strong `Arc` in the global cache. Store weak hubs in the cache, make active observables/subscriptions hold the strong hub, and prune dead weak entries opportunistically. Otherwise dynamic paths still leak descriptor entries forever.

### 5. Add `share_latest_by_descriptor`

Expose a small shell-owned helper for generated code and source helpers:

```rust
pub fn share_latest_by_descriptor<T>(
    descriptor: SourceDescriptor,
    create: impl Fn() -> Observable<T> + Send + Sync + 'static,
) -> Observable<T>
where
    T: Clone + Send + 'static;
```

This helper should preserve the current v1 semantics:

- first subscriber starts upstream
- later subscribers receive latest value and share upstream
- last subscriber stops upstream
- later subscribers can restart upstream
- errors reset the active connection

### 6. Implement `#[shell_macros::observable]`

Add the source-definition macro documented in `SOURCE_API.md`.

Initial implementation target:

- Public function keeps only plain context parameters.
- `#[observe(expr)]` parameters are removed from the public signature and generated inside the wrapper.
- `#[inject]` parameters are resolved through a shell-owned facade when that facade exists. If DI is not ready, implement parsing and emit a clear unsupported diagnostic for `#[inject]`.
- The generated public wrapper builds a descriptor from `module_path!()`, function name, output type, and plain context args, then calls `share_latest_by_descriptor`.
- The user body moves to a private implementation function that receives context args, observed observables, and injected services.

Do not require model `#[source(...)]` to change. A field binding to an annotated observable function will receive an already-shared observable.

### 7. Descriptor Argument Encoding

Add a trait for descriptor arguments with implementations for expected source context values:

- `LocusPath`
- `PathBuf`
- `String`
- `&str` where usable in generated code
- numeric and bool primitives
- `Option<T>`
- small tuples if needed by real source functions

If a context argument cannot be encoded, the generated `#[observable]` wrapper should produce a compile error pointing at that argument and suggesting either an encodable key type or explicit unshared behavior.

### 8. Keep `#[source(expr)]` Generic

Do not group field bindings by source expression tokens. The model macro should keep accepting any expression that type-checks as `Observable<T>`.

Descriptor sharing should come from:

- primitive source helpers
- generated `#[observable]` wrappers
- explicit source helper use of `share_latest_by_descriptor` when needed

This keeps implementation details out of model field syntax.

### 9. Keep `shell-rx-macros` Focused

No descriptor or subscription behavior should be added to `shell-rx-macros`.

Possible cleanup only:

- Add `trybuild` coverage for the single-source compile error.
- Add tests for two-source and higher-arity expansion if desired.

## Test Plan

### Existing Baseline

The current workspace test suite passes:

- `rsynapse-shell`: 0 tests
- `shell-core`: 31 tests
- `shell-macros`: 25 tests
- `shell-rx-macros`: 3 unit tests plus 1 doctest

### Add Shell-Core Descriptor Tests

- Two subscribers to the same descriptor create one upstream subscription.
- Second subscriber receives latest value immediately.
- Last subscriber drop disconnects upstream.
- Later subscriber restarts upstream.
- Different output types do not share.
- Different descriptor args do not share.
- Error closes/reset behavior allows later restart.
- Weak-cache pruning removes dead dynamic descriptors or at least prevents strong retention.

### Add Macro Tests

- `#[observable]` removes `#[observe]` and `#[inject]` parameters from the public function signature.
- Generated wrapper builds observed dependency expressions in declaration order.
- Generated wrapper wraps body with `share_latest_by_descriptor`.
- Unsupported descriptor argument types produce a clear compile error.
- Async components with nested models either generate async nested subscriptions or fail with a clear diagnostic.
- Direct plus nested fields above 128 produce the dirty-mask limit diagnostic.

Use unit tests for parser shape and `trybuild` for compile-level contracts.

### Add Consumer Compile Coverage

After D-Bus path migration work lands in consumer sources, compile the workspace and add focused pure tests around path construction helpers for:

- `/dbus/<service>/objects/...` property paths
- `/dbus/<service>/methods/...` callable method paths
- `_absolute` outside ObjectManager paths

Macro tests should not hardcode these layouts except as examples of arbitrary source expressions.

## Important Decisions

- Descriptor sharing should live in `shell-core::source` and generated `#[observable]` wrappers, not in `shell-rx-macros`.
- `#[source(expr)]` should remain a generic Observable expression. Do not infer descriptors from source-expression tokens.
- Primitive source sharing is already present and should be generalized rather than replaced.
- The first macro changes should fix subscription correctness and reduce duplicate expansion code before adding derived descriptor sharing.
- D-Bus path layout changes belong in consumer/source helper code. The macro layer should stay backend-neutral.

## Open Questions For Validation

- Should `SourceDescriptor` be a documented public source API concept, or a `#[doc(hidden)]` macro-support type?
- Should generated descriptor sharing be mandatory for every `#[observable]` function, or should there be an opt-out for deliberately cold observables?
- What descriptor argument trait should be accepted for custom app context types: `Hash`, a shell-owned `SourceDescriptorArg`, or explicit `#[key(...)]` annotations?
- Should generated source errors remain `String` for v1 compatibility, or should this pass introduce a shell-owned error enum before descriptor work?
- Should nested source models continue to exist once `#[observable]` dynamic dependencies can generate Rx `switch_map` composition?
