# locus-provider Refactor Notes

## Current Role

`locus-provider` owns generic Locus graph binding primitives plus the Locus-over-D-Bus provider implementation. It should not contain schema-specific model markers, path constants, relation constants, or convenience extension traits; those belong in consuming crates such as `dev-widgets` until codegen emits them.

## Public Surface

- `Property<Model, Value>`, `Path<Target>`, and `FieldBinding<T>` describe typed graph field bindings.
- `NodeRef<Model>`, `NodePropertyBinding<Model, Value>`, and `node` support direct node property providers.
- `Relation<Source, Target>`, `TargetBinding<Target>`, `NodeListBinding<Target>`, and `KindFilteredNodeListBinding<Target>` describe target and node-list subscriptions.
- `watch_field`, `watch_target`, and `watch_node_list` expose callback-based watcher helpers.
- `Provider` implementations let field, node property, target, and node-list bindings plug into the neutral `providers` crate.
- `DecodeLocusValue`, `DecodeError`, `ListError`, and `WatchError` define decode and watch diagnostics.

## Step-By-Step File Walkthrough

1. `provider/locus/src/lib.rs` - crate-level boundary statement and public re-exports. Read first to confirm generic versus schema-specific ownership.
2. `provider/locus/src/binding.rs` - core field descriptor types: properties, paths, and field bindings. Read before watchers because it defines the typed authoring surface.
3. `provider/locus/src/node.rs` - direct `NodeRef` property bindings and provider implementation. Read after path descriptors to understand node-specific property access.
4. `provider/locus/src/decode.rs` - wire string decoding into Rust primitives and `NONE_STRING` default handling. Read before watch code because all emitted values pass through this layer.
5. `provider/locus/src/error.rs` - decode, list, and D-Bus watch errors. Read with decode/watch code to understand diagnostics and source preservation.
6. `provider/locus/src/watch.rs` - field watcher implementation over `GraphResolveProxy::watch_node`, `GraphReadProxy::get_property`, and `PropertiesUpdated`. Read as the main field subscription runtime.
7. `provider/locus/src/collection/mod.rs` - collection module exports. Read before submodules to see the public collection surface.
8. `provider/locus/src/collection/binding.rs` - target, relation, node-list, and kind-filtered binding descriptors. Read before collection watcher code.
9. `provider/locus/src/collection/diff.rs` - node-list diff command decoding and application. Read before `collection/watch.rs` because list watchers maintain local node lists through these commands.
10. `provider/locus/src/collection/watch.rs` - target and node-list D-Bus subscriptions, signal filtering, kind filtering, and provider implementations. Read as the collection runtime behavior.
11. `provider/locus/src/test.rs` - field descriptor, primitive decode, node property, and cancellation tests.
12. `provider/locus/src/collection/test.rs` - collection descriptor, diff, provider wiring, and cancellation tests.

## Internal Structure

- Descriptor modules are generic and use phantom model types to preserve type relationships without carrying schema code.
- Field and direct-node property providers share `watch_node_path_property_with_context`.
- Collection providers are split between descriptor construction, diff application, and D-Bus watcher loops.
- Watchers check cancellation before D-Bus setup and while waiting for signal streams.

## Behavior Summary

Field bindings resolve a source/path to a watch object through `GraphResolveProxy::watch_node`, emit the current target property value, then listen for `PropertiesUpdated` signals. Removed or missing values become `T::default()` through the Locus `NONE_STRING` convention.

Target bindings call `SubscribeResolve`, emit the current target node id, then listen for matching `ResolveChanged` signals. Node-list bindings call `SubscribeResolveAll`, `SubscribeSources`, or `SubscribeTargets`, apply Locus diff commands into a local `Vec<NodeId>`, optionally filter by each node's `kind` property, and emit the visible node list.

## User Notes

- Public generic path composition may be unnecessary if generated schema helpers expose semantic providers directly.
- Selected graph node -> dependent collection flows should be pushed into Locus/schema provider helpers where possible, rather than exposed as generic client-side `switch_map` composition.

## Findings

- Risk: descriptor structs expose raw string fields publicly (`source`, `relations`, `property`, `kind`, etc.). This is convenient for tests and codegen experiments, but it lets consumers construct or mutate around the intended typed API. `PLAN.md` already calls out making raw descriptor fields private with accessors before stabilization.
- Risk: `Path::name` and `Path::many` are carried in descriptors but are not used by `property`, `target`, or `all`. If generated schema code accidentally uses a many path where a scalar binding is expected, this crate will not catch it.
- Risk: field watchers always attempt `Close` after streaming; if the local provider exits because of cancellation but the close call fails during bus/service teardown, the provider returns an error. Cancellation may need to treat close failure as best-effort cleanup.
- Risk: kind-filtered node lists call `GetProperty(kind)` once per node on every list emission. That is simple and generic, but it can become N+1 D-Bus work for large or frequently changing lists.
- Gap: unit tests cover descriptors, decode behavior, diff application, and pre-cancelled providers. They do not cover signal body parsing, close behavior, live subscription cleanup, kind filtering, or D-Bus error paths.

## Refactor Plan

1. Keep schema-specific helpers out of `locus-provider`; generated `Window::TITLE`, `SELECTED_WORKSPACE.windows()`, and similar helpers should stay consumer-local per `PLAN.md`.
2. Add accessor methods for descriptor fields, then make raw descriptor fields private once consuming code and macro output no longer require direct access.
3. Decide how `Path::many` should constrain APIs. Options include splitting scalar and collection path descriptor types or adding debug assertions/tests around generated schema output.
4. Treat field-watch `Close` failures after cancellation as best-effort if cancellation should be a clean provider shutdown path.
5. Add tests around collection diff edge cases, kind filtering behavior, and watch-loop error/cancellation behavior with mocked or synthetic D-Bus messages if practical.
6. Revisit kind filtering when row hydration/codegen exists; prefer server-side filtering or batched property reads if Locus supports it.
7. Evaluate making `Path` and related generic path composition internal/generated descriptor data, with user-facing APIs centered on semantic generated helpers such as selected workspace windows.

## Tests And Verification

- `cargo test -p locus-provider` passes: 25 unit tests, 0 doctests.

## Open Questions

- Does `SubscribeResolve` / `SubscribeResolveAll` require an unsubscribe or close operation, or are subscriptions scoped implicitly by connection/service semantics?
- Should target bindings emit raw `NodeId` strings, `NodeRef<Target>`, or an optional node representation for `NONE_STRING`?
- Should `NONE_STRING` mapping to `Default` remain the universal behavior, or should missing values be representable separately for some property types?
