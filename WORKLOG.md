# Work Log

## Current Refactor State

The provider and schema-facing refactor is complete for the current workspace
shape.

- `providers` is stream-native: `Provider<T>` opens
  `Stream<Item = Result<T, E>>` values and uses
  `tokio_util::sync::CancellationToken` directly.
- `Subscription` and `SubscriptionGroup` own forwarding task lifecycle.
- `ShellApp` installs the Tokio-backed provider task spawner; provider core no
  longer creates a hidden runtime.
- `SharedProvider` provides refcounted latest-value fanout and restarts upstream
  on later subscribers.
- `combine_latest2_stream` and `combine_latest2` provide the first narrow
  derived-provider primitive for composing two typed sources.
- `rsynapse-shell` exists as an in-repository playground for migrating the local
  AGS shell while preserving the framework/consumer boundary.
- `provider/property` owns shared typed property descriptors and
  property-binding traits.
- `dbus-provider` and `locus-provider` implement the stream-native provider
  contract and the shared property-binding contract where applicable.
- Locus schema-specific models, properties, paths, relations, typed decoding,
  and semantic helpers live in generated consumer schema code.
- `dev-widgets` carries generated development schema code and uses
  component-backed window rows.
- `shell-macros` treats DBus, Locus, and custom sources as generic
  `providers::Provider<T>` expressions.

## Completed Cleanup

- Removed callback-style provider context/sender APIs.
- Removed generic provider map/combine/switch adapters from provider core.
- Removed Locus shell-side typed decode traits; generated schema wrappers own
  conversion from Locus wire values.
- Split `dev-widgets/src/primitives.rs` into logical module files.
- Made DBus and Locus descriptor fields private behind accessors.
- Moved provider task runtime ownership out of provider core and into
  ShellApp/framework setup.
- Updated roadmap, architecture, migration, and refactor notes to match the
  current crate boundaries.

## Current Next Step

Add compile-expanded macro tests for realistic Relm4 components so generated
provider subscriptions and view binding rewrites are validated beyond token
shape assertions.
