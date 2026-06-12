# locus-provider Refactor Notes

## Current Role

`locus-provider` owns generic Locus graph binding primitives plus the Locus-over-D-Bus provider implementation. It should not contain schema-specific model markers, path constants, relation constants, or convenience extension traits; those belong in consuming crates such as `dev-widgets` until codegen emits them.

## Public Surface

- `Property<Model, Value>` and `Path<Target>` describe typed graph property descriptors.
- Raw `LocusPropertyBinding<Target>`, `NodeRef<Model>`, `NodePropertyBinding<Model>`, and `node` support direct Locus wire-string property providers; generated schema wrappers adapt them into typed providers.
- `Relation<Source, Target>`, `TargetBinding<Target>`, `NodeListBinding<Target>`, and `KindFilteredNodeListBinding<Target>` describe target and node-list subscriptions.
- `watch_field`, `watch_target`, and `watch_node_list` expose stream-native watcher helpers.
- Locus property, target, direct-node, and node-list bindings implement the completed stream-native provider-core contract.
- Generated property wrappers carry decode functions for Locus wire values; `DecodeError`, `ListError`, and `WatchError` define diagnostics.

## Step-By-Step File Walkthrough

1. `provider/locus/src/lib.rs` - crate-level boundary statement and public re-exports. Read first to confirm generic versus schema-specific ownership.
2. `provider/locus/src/binding.rs` - core property descriptor types: properties, paths, and Locus property bindings. Read before watchers because it defines the typed authoring surface.
3. `provider/locus/src/node.rs` - direct `NodeRef` property bindings and provider implementation. Read after path descriptors to understand node-specific property access.
4. `provider/locus/src/error.rs` - decode, list, and D-Bus watch errors. Read with watch code to understand diagnostics and source preservation.
5. `provider/locus/src/watch.rs` - field watcher implementation over `GraphResolveProxy::watch_node`, `GraphReadProxy::get_property`, and `PropertiesUpdated`. Read as the main field subscription runtime.
6. `provider/locus/src/collection/mod.rs` - collection module exports. Read before submodules to see the public collection surface.
7. `provider/locus/src/collection/binding.rs` - target, relation, node-list, and kind-filtered binding descriptors. Read before collection watcher code.
8. `provider/locus/src/collection/diff.rs` - node-list diff command decoding and application. Read before `collection/watch.rs` because list watchers maintain local node lists through these commands.
9. `provider/locus/src/collection/watch.rs` - target and node-list D-Bus subscriptions, signal filtering, kind filtering, and provider implementations. Read as the collection runtime behavior.
10. `provider/locus/src/test.rs` - property descriptor, generated-style decode, node property, and cancellation tests.
11. `provider/locus/src/collection/test.rs` - collection descriptor, diff, provider wiring, and cancellation tests.

## Internal Structure

- Descriptor modules are generic and use phantom model types to preserve type relationships without carrying schema code.
- Resolved-path and direct-node property providers share the property watch runtime in `watch.rs`.
- Collection providers are split between descriptor construction, diff application, and D-Bus watcher loops.
- Watchers check cancellation before D-Bus setup and while waiting for signal streams.

## Behavior Summary

Raw Locus property bindings resolve a source/path to a watch object through `GraphResolveProxy::watch_node`, emit the current target property string, then listen for `PropertiesUpdated` signals. Removed or missing values are emitted as the Locus `NONE_STRING` wire value. Generated schema wrappers turn that wire value into `Ok(None)` for optional properties or `Err(DecodeError::MissingValue)` for non-optional properties.

Target bindings call `SubscribeResolve`, emit the current target node id, then listen for matching `ResolveChanged` signals. Node-list bindings call `SubscribeResolveAll`, `SubscribeSources`, or `SubscribeTargets`, apply Locus diff commands into a local `Vec<NodeId>`, optionally filter by each node's `kind` property, and emit the visible node list.

## User Notes

- Public generic path composition may be unnecessary if generated schema helpers expose semantic providers directly.
- Selected graph node -> dependent collection flows should be pushed into Locus/schema provider helpers where possible, rather than exposed as generic client-side `switch_map` composition.
- Property-backed providers are a provider family. `locus-provider` uses `provider/property` for shared `Property<Target, Value>` and property-binding traits.
- Locus `FieldBinding<T>` has been replaced by raw `LocusPropertyBinding<Target>` because it is a resolved graph property binding over Locus wire values.
- Macro integration should treat Locus and DBus source expressions identically as `providers::Provider<T>` values; no macro code should depend on Locus-specific binding types.
- Base property streams should pass backend errors directly as `Err(E)`; explicit wrappers such as `with_default` can swallow errors and emit fallback values.
- Shared property crate should define `PropertyBinding<T>: providers::Provider<T>` for inspectable property-backed providers.
- Locus exposes concrete raw `LocusPropertyBinding<Target>` implementing both `providers::Provider<String>` and the shared property crate's `PropertyBinding<String>` trait. Generated schema wrappers adapt raw Locus strings into typed `Provider<Value>` bindings.
- Missing values should be represented by schema choice. `Property<Target, Option<Value>>` can emit `Ok(None)` for missing values; non-optional `Property<Target, Value>` should treat missing as an error unless explicitly wrapped with a default.
- Field-watch `Close` failures after cancellation should be best-effort cleanup, not property stream errors.
- Public watch helpers are stream-native.
- Do not merge resolved path property bindings and direct node property bindings into one enum unless a concrete need appears. Keep separate concrete backend types implementing the shared property binding trait.
- `NodeRef` should not be the primary public authoring surface long-term. Generated APIs should mostly expose `Property` and property binding providers.
- Locus wire decoding is schema/codegen-owned. Shared `Property<Target, Value>` stays key-only; generated schema wrappers own conversion functions and typed provider adaptation. If locus-core later changes to typed internal D-Bus values, codegen should change while locus-shell provider implementations remain stable.
- Completed: `DecodeLocusValue` and `provider/locus/src/decode.rs` were removed after generated schema wrappers started carrying conversion functions.
- Missing values are represented through `Result<T, E>` where `T` may itself be `Option<Value>` when the schema says missing is valid.
- Collection diffing for UI-facing node lists remains a `locus-provider` responsibility.
- From the shell/schema perspective, Locus graph target/list queries should be modeled as typed property-valued providers, just like DBus list-valued properties. `PropertyBinding<T>` can cover scalar, optional, and iterable `T`.
- `Relation`, `TargetBinding`, `NodeListBinding`, and kind-filter binding concepts are Locus implementation details or generated descriptor internals, not primary public shell authoring abstractions.
- Scalar/list/optional value shape should be resolved at descriptor/codegen level before a watch is created.
- Target resolution should follow the same optional semantics as other properties: `Option<T>` when missing is valid, otherwise backend missing state is an error.
- Kind filtering should move out of the primary public API and become generated/internal implementation detail when needed. Completed for generated semantic collection helpers such as selected workspace windows.

## Findings

- Completed: descriptor structs now hide raw string fields such as `source`, `relations`, `property`, and `kind` behind accessors. This keeps generated code and tests inspectable without exposing struct layout.
- Risk: `Path::name` and `Path::many` are carried in descriptors but are not used by `property`, `target`, or `all`. If generated schema code accidentally uses a many path where a scalar binding is expected, this crate will not catch it.
- Risk: field watchers always attempt `Close` after streaming; if the local provider exits because of cancellation but the close call fails during bus/service teardown, the provider returns an error. Cancellation may need to treat close failure as best-effort cleanup.
- Risk: kind-filtered node lists call `GetProperty(kind)` once per node on every list emission. That is simple and generic, but it can become N+1 D-Bus work for large or frequently changing lists.
- Gap: unit tests cover descriptors, decode behavior, diff application, and pre-cancelled providers. They do not cover signal body parsing, close behavior, live subscription cleanup, kind filtering, or D-Bus error paths.
- Completed: this crate now uses direct `CancellationToken` values and stream-native `Provider<T>` implementations.

## Refactor Plan

1. Completed: keep schema-specific helpers out of `locus-provider`; generated `Window::TITLE`, `SELECTED_WORKSPACE.windows()`, and similar helpers stay consumer-local per `PLAN.md`.
2. Completed: add `provider/property` crate and move shared typed `Property<Target, Value>` there.
3. Completed: replace `FieldBinding<T>` with concrete raw `LocusPropertyBinding<Target>`.
4. Completed: make raw `LocusPropertyBinding<Target>` implement both `providers::Provider<String>` and the shared property crate's `PropertyBinding<String>` trait; generated schema wrappers implement typed `Provider<Value>`.
5. Completed: migrate Locus property, target, direct-node, and list providers to `Provider<T>::stream(self, CancellationToken)` and `Stream<Item = Result<T, WatchError>>`.
6. Completed: replace universal `T::default()` missing-value behavior with schema-driven `Option<T>` semantics and non-optional missing errors.
7. Completed: keep direct node property and resolved path property bindings as separate concrete types that both implement the shared property binding trait.
8. Completed: add accessor methods for descriptor fields and make raw descriptor fields private.
9. Decide how `Path::many` should constrain APIs. Options include splitting scalar and collection path descriptor types or adding debug assertions/tests around generated schema output.
10. Treat field-watch `Close` failures after cancellation as best-effort if cancellation should be a clean provider shutdown path.
11. Move `NodeRef` out of the primary public authoring surface where generated property APIs can hide it.
12. Completed: update the Locus codegen layer so generated property wrappers carry the required conversion from Locus wire strings to expected Rust property types.
13. Completed: remove `DecodeLocusValue` from locus-shell provider code after generated schema wrappers own conversion.
14. Keep collection diffing in `locus-provider` as the UI-facing node-list state maintenance layer.
15. Fold public target/list relation query concepts into the property-backed provider model where generated schema can expose them as scalar/optional/iterable property bindings.
16. Move `Relation`, `TargetBinding`, `NodeListBinding`, and kind-filter binding types out of the primary public authoring surface; keep graph query machinery internal or generated descriptor data.
17. Ensure generated schema wrappers decide scalar/list/optional value shape before watch runtime starts.
18. Make target resolution use the same `Option<T>` versus error semantics as other properties.
19. Add tests around collection diff edge cases, kind filtering behavior, and watch-loop error/cancellation behavior with mocked or synthetic D-Bus messages if practical.
20. Revisit kind-filter implementation cost when row hydration/codegen exists; generated semantic collection helpers now hide the kind-filter detail, but the runtime still uses per-node kind reads.
21. Evaluate making `Path` and related generic path composition internal/generated descriptor data, with user-facing APIs centered on semantic generated helpers such as selected workspace windows.

## Tests And Verification

- `cargo test -p locus-provider` passes after the stream-native provider migration.
- Detailed test review was skipped for this pass; update tests mechanically during migration and add focused coverage for the new property-backed descriptor semantics.

## Open Questions

- Does `SubscribeResolve` / `SubscribeResolveAll` require an unsubscribe or close operation, or are subscriptions scoped implicitly by connection/service semantics?
- Should target bindings emit raw `NodeId` strings, `NodeRef<Target>`, or an optional node representation for `NONE_STRING`?
- Should `NONE_STRING` mapping to `Default` remain the universal behavior, or should missing values be representable separately for some property types?
