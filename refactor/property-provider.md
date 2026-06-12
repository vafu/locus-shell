# property-provider Refactor Notes

## Current Role

`property-provider` is the shared property contract crate for provider backends
that expose typed property descriptors. It keeps property vocabulary out of
individual DBus and Locus implementations while staying independent of GTK,
Relm4, shell widget policy, and backend transport details.

## Public Surface

- `Property<Target, Value>` describes a static typed property key on a target
  marker type.
- `PropertyBinding<T>` extends `providers::Provider<T>` for inspectable
  property-backed providers and exposes the typed target, backend key, property
  descriptor, and stable backend key.
- `providers` is re-exported so backend crates can name the shared provider
  contract through the property crate when useful.

## Step-By-Step File Walkthrough

1. `provider/property/src/lib.rs` - public API for typed property descriptors
   and property-backed provider bindings. Read first because this entire crate
   is the shared contract surface.
2. `provider/property/src/test.rs` - minimal descriptor and trait-shape tests.
   Read last to see what the contract currently guarantees.

## Internal Structure

- `Property<Target, Value>` stores only a static key. Phantom marker fields
  preserve the target/value type relation without runtime data.
- `PropertyBinding<T>` requires implementors to also implement
  `providers::Provider<T>`, keeping macros and consumers provider-based.
- Backend-specific identity remains in each backend's `Key` type, for example
  DBus object/interface/property keys or Locus source/path/property keys.

## Behavior Summary

The crate does not perform runtime watching. It only defines the typed property
descriptor and the trait shape that DBus and Locus bindings implement. Error
handling, decoding, cancellation, and subscription lifecycles stay in the
backend provider crates.

## User Notes

- Type safety for properties is important.
- A property-based system is a provider family, and DBus/Locus are backends for
  that family.
- Macros should continue to depend on `providers::Provider<T>`; property
  inspection is additional backend/runtime information, not the macro's primary
  contract.

## Findings

- Completed: shared `Property<Target, Value>` and `PropertyBinding<T>` live in
  `provider/property`.
- Completed: DBus and Locus property-backed bindings implement the shared
  property trait.
- Good boundary: the crate contains no backend transport, schema decoding, GTK,
  Relm4, or widget policy.
- Gap: automatic descriptor-keyed sharing is not implemented here. That should
  be a backend/runtime concern using each binding's stable key.

## Refactor Plan

1. Completed: add the crate to the workspace and workspace dependencies.
2. Completed: move shared typed property descriptors out of DBus/Locus-specific
   code.
3. Completed: add minimal tests for descriptor keys and trait shape.
4. Keep this crate as contract-only; do not add D-Bus, Locus, or UI runtime
   behavior here.
5. Use backend `Key` types later for descriptor-keyed sharing registries.

## Tests And Verification

- `cargo test -p property-provider` passes.

## Open Questions

- Should property helper wrappers such as `with_default` live in this crate, or
  should they stay generic provider/stream adapters until repeated use proves a
  property-specific need?
