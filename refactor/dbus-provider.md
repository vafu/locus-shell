# dbus-provider Refactor Notes

## Current Role

`dbus-provider` is the generic typed D-Bus object/property provider crate. It owns D-Bus transport watching for ordinary object properties and intentionally avoids Locus graph schema, generated graph contracts, common service definitions, GTK, Relm4, or shell widget policy.

## Public Surface

- `DbusBus` selects the session or system bus.
- `Object<Target>` describes a typed D-Bus object by bus, service, path, and interface.
- `Property<Target, Value>` describes a typed property on an object marker type.
- `PropertyBinding<Value>` combines object and property descriptors and should implement the stream-native `providers::Provider<Value>` contract after migration.
- `watch_property` exposes a callback helper for direct property watching.
- `WatchError` preserves `zbus` and `zbus::fdo` error sources.

## Step-By-Step File Walkthrough

1. `provider/dbus/src/lib.rs` - public module map and re-exports. Read first to see the crate's intended API.
2. `provider/dbus/src/property.rs` - typed object, property, and property-binding descriptors. Read before watcher code because this is the consumer-facing authoring surface.
3. `provider/dbus/src/watch.rs` - session/system connection selection, proxy construction, initial value read, property change stream, cancellation checks, and `Provider` implementation. Read as the runtime behavior.
4. `provider/dbus/src/error.rs` - typed watch errors and source preservation. Read with `watch.rs` to understand diagnostics.
5. `provider/dbus/src/test.rs` - descriptor construction, provider wiring, bus selection, and pre-cancelled-provider tests. Read last to compare intended behavior with coverage.

## Internal Structure

- Descriptors are pure typed metadata with phantom marker types.
- Runtime watching is currently centralized in the old callback-oriented `watch_property_with_context`.
- The provider implementation still uses the removed `ProviderContext` / `ProviderSender<T>` API and must be migrated to the completed stream-native provider core.
- Errors remain backend-specific rather than being flattened inside this crate.

## Behavior Summary

A `PropertyBinding<T>` selects the session or system bus, builds a `zbus::Proxy` for the configured service/path/interface, emits the current property value, then listens to `PropertiesChanged` updates for that property. Values are decoded through `zbus`'s `TryFrom<OwnedValue>` path.

Cancellation is currently checked through the old `ProviderContext` wrapper before D-Bus setup, after the initial property emission, while waiting for update events, and while resolving each update value. This should become direct `CancellationToken` use during migration.

## User Notes

None yet.

## Findings

- Risk: `watch_property_with_context` reads the initial property value before subscribing to property changes. A property update between `get_property` and `receive_property_changed` can be missed. If initial-plus-updates needs a strict no-gap handoff, subscribe first or use a zbus pattern that guarantees cached state and signal ordering.
- Risk: cancellation is not selected against connection creation, proxy building, or the initial `get_property` call. A cancellation request during those awaits is only observed afterward.
- Risk: descriptor structs expose raw bus/service/path/interface/property fields publicly. This mirrors the current Locus descriptors, but it leaves less room to validate or evolve descriptors before API stabilization.
- Gap: tests cover descriptor wiring and pre-cancelled exits, but not live property-change delivery, initial/update ordering, conversion failures, session versus system connection behavior, or D-Bus error propagation.
- Migration blocker: this crate does not currently compile against the completed provider-core refactor because it imports `ProviderContext` and `ProviderSender` and implements the removed callback-style provider API.

## Refactor Plan

1. Migrate `watch.rs` to a stream-producing implementation over `tokio_stream::Stream<Item = Result<T, WatchError>>`.
2. Replace `ProviderContext` with direct `CancellationToken` and remove `ProviderSender` from the provider implementation.
3. Decide whether property watching requires a no-missed-update guarantee between initial read and signal subscription. If yes, restructure the watcher and add a regression test with a fake or local D-Bus service.
4. Wrap longer D-Bus setup awaits in cancellation-aware `tokio::select!` blocks if prompt cancellation matters for generated provider tasks.
5. Add accessor methods for descriptors and make raw fields private before external API stabilization.
6. Add tests for value conversion failures and update delivery once there is a practical D-Bus test harness.

## Tests And Verification

- Previously `cargo test -p dbus-provider` passed under the callback provider API.
- After the completed provider-core refactor, `dbus-provider` is expected to fail until migrated to stream-native providers.

## Open Questions

- Should `watch_property` remain as a public callback helper, or should consumers be steered exclusively through the `Provider` implementation?
- Should descriptor constructors support owned strings for dynamically discovered services/paths, or should this crate intentionally keep only static definitions for generated/common providers?
