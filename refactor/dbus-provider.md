# dbus-provider Refactor Notes

## Current Role

`dbus-provider` is the generic typed D-Bus object/property provider crate. It owns D-Bus transport watching for ordinary object properties and intentionally avoids Locus graph schema, generated graph contracts, common service definitions, GTK, Relm4, or shell widget policy.

## Public Surface

- `DbusBus` selects the session or system bus.
- `Object<Target>` describes a typed D-Bus object by bus, service, path, and interface.
- `Property<Target, Value>` is re-exported from `property-provider` and describes a typed property on an object marker type.
- `PropertyBinding<Target, Value>` combines object and property descriptors and implements both `providers::Provider<Value>` and `property_provider::PropertyBinding<Value>`.
- `watch_property` exposes a stream-native direct property watcher.
- `WatchError` preserves `zbus` and `zbus::fdo` error sources and is cloneable for shared providers.

## Step-By-Step File Walkthrough

1. `provider/dbus/src/lib.rs` - public module map and re-exports. Read first to see the crate's intended API.
2. `provider/dbus/src/property.rs` - typed object, property, and property-binding descriptors. Read before watcher code because this is the consumer-facing authoring surface.
3. `provider/dbus/src/watch.rs` - session/system connection selection, proxy construction, initial value read, property change stream, cancellation checks, and `Provider` implementation. Read as the runtime behavior.
4. `provider/dbus/src/error.rs` - typed watch errors and source preservation. Read with `watch.rs` to understand diagnostics.
5. `provider/dbus/src/test.rs` - descriptor construction, provider wiring, bus selection, and pre-cancelled-provider tests. Read last to compare intended behavior with coverage.

## Internal Structure

- Descriptors are pure typed metadata with phantom marker types.
- Runtime watching is centralized in `watch_property`, which returns `Stream<Item = Result<T, WatchError>>`.
- The provider implementation uses direct `CancellationToken` values from provider core.
- Errors remain backend-specific rather than being flattened inside this crate.

## Behavior Summary

A `PropertyBinding<T>` selects the session or system bus, builds a `zbus::Proxy` for the configured service/path/interface, emits the current property value, then listens to `PropertiesChanged` updates for that property. Values are decoded through `zbus`'s `TryFrom<OwnedValue>` path.

Cancellation is checked through direct `CancellationToken` use before D-Bus setup, after the initial property emission, while waiting for update events, and while resolving each update value.

## User Notes

- `lib.rs` is trivial today, but public parts of this crate should live in `lib.rs` per the Rust guide direction. Implementation files should become implementation detail modules rather than owning public structs/enums.
- Type safety for properties is important. Keep a typed `Property<Target, Value>` abstraction.
- Move shared property vocabulary to `property_provider::Property<Target, Value>` rather than duplicating DBus and Locus property structs.
- `Property` should use private fields and accessors.
- Schema/generated code drives descriptor construction; static descriptors are the right default.
- Macro integration should be backend-agnostic and provider-based. DBus and Locus source expressions should look identical to macros as `providers::Provider<T>` values.
- Rename macro internals away from Locus-specific wording toward provider/source terminology.
- Steer new macro usage to `#[source(...)]`; keep legacy `#[locus(...)]` only as compatibility spelling.
- Generate source messages as `Msg::Field(Result<T, E>)` rather than separate success and `WatchFailed` variants.
- Property-backed providers are a provider family. Add a `provider/property` crate to own shared property-related code, starting with typed `Property<Target, Value>` and property-binding traits. `dbus-provider` and `locus-provider` should implement those traits for their backend-specific bindings.
- Shared property crate should define `PropertyBinding<T>: providers::Provider<T>` for inspectable property-backed providers. DBus should implement it for its concrete DBus property binding type.
- Base property streams should pass backend errors directly as `Err(E)`. Error swallowing or defaulting belongs in explicit wrappers such as `with_default`, not inside the base watcher.
- Watcher control flow should decide internally which backend errors are recoverable and whether the stream continues or ends.
- Skip detailed test review for this pass.

## Findings

- Risk: `watch_property_with_context` reads the initial property value before subscribing to property changes. A property update between `get_property` and `receive_property_changed` can be missed. If initial-plus-updates needs a strict no-gap handoff, subscribe first or use a zbus pattern that guarantees cached state and signal ordering.
- Risk: cancellation is not selected against connection creation, proxy building, or the initial `get_property` call. A cancellation request during those awaits is only observed afterward.
- Completed: descriptor structs now hide raw bus/service/path/interface/property fields behind accessors, leaving room to validate or evolve descriptor internals before API stabilization.
- Gap: tests cover descriptor wiring and pre-cancelled exits, but not live property-change delivery, initial/update ordering, conversion failures, session versus system connection behavior, or D-Bus error propagation.
- Completed: this crate now compiles against the stream-native provider core, emits backend errors as stream items, and implements the shared property-binding trait.

## Refactor Plan

1. Completed: migrate `watch.rs` to a stream-producing implementation over `tokio_stream::Stream<Item = Result<T, WatchError>>`.
2. Completed: replace `ProviderContext` with direct `CancellationToken` and remove `ProviderSender`.
3. Completed: add `provider/property` with shared `Property<Target, Value>` and `PropertyBinding<T>` APIs.
4. Completed: make `dbus-provider` implement the property crate trait for `PropertyBinding<Target, Value>`.
5. Completed: keep a public stream-native `watch_property` API.
6. Treat per-update/decode errors as recoverable where the underlying watch stream can continue.
7. Add explicit property/provider wrappers such as `with_default` if consumers need to swallow errors and replace them with fallback values.
8. Consider shared watch-stream helper(s) only after DBus and Locus migrations reveal stable duplication.
9. Decide whether property watching requires a no-missed-update guarantee between initial read and signal subscription. If yes, restructure the watcher and add a regression test with a fake or local D-Bus service.
10. Wrap longer D-Bus setup awaits in cancellation-aware `tokio::select!` blocks if prompt cancellation matters for generated provider tasks.
11. Completed: add accessor methods for DBus descriptors and make raw fields private.
12. Add tests for value conversion failures and update delivery once there is a practical D-Bus test harness.

## Tests And Verification

- `cargo test -p dbus-provider` passes after the stream-native provider migration.
- Detailed DBus test review was skipped by request. Existing tests should be updated mechanically during migration, with deeper live DBus harness work deferred.

## Open Questions

- Should `watch_property` remain as a public callback helper, or should consumers be steered exclusively through the `Provider` implementation?
- Should descriptor constructors support owned strings for dynamically discovered services/paths, or should this crate intentionally keep only static definitions for generated/common providers?
