# Work Log

## Current Refactor State

The provider removal and Observable-first refactor is complete for the current
workspace shape.

- `providers` is stream-native: `ObservableSource<T>` opens
  `Stream<Item = Result<T, E>>` values and uses
  `tokio_util::sync::CancellationToken` directly.
- `Subscription` and `SubscriptionGroup` own forwarding task lifecycle.
- `ShellApp` installs the Tokio-backed source task spawner; source core no
  longer creates a hidden runtime.
- Descriptor-keyed shared latest fanout is a future runtime sharing step for
  duplicated backend source functions.
- Composition now belongs in rxrust operators and
  `#[shell_macros::observable]` source functions, not source-runtime
  combinators.
- `rsynapse-shell` exists as an in-repository playground for migrating the local
  AGS shell while preserving the framework/consumer boundary.
- Locus nodes are represented as raw locusfs node path strings such as
  `window:1`.
- `dev-widgets` and `rsynapse-shell` use handwritten locusfs Observable source
  functions and component-backed rows.
- `shell-macros` treats all source expressions as Observable-compatible
  expressions through `shell_core::source`. The target user-facing source API is
  documented in `SOURCE_API.md`.

## Completed Cleanup

- Removed callback-style provider context/sender APIs.
- Removed generic provider map/combine/switch adapters from provider core.
- Removed Locus shell-side typed descriptor wrappers; consumer source functions
  own conversion from Locus wire values.
- Split `dev-widgets/src/primitives.rs` into logical module files.
- Made DBus and Locus descriptor fields private behind accessors.
- Moved provider task runtime ownership out of provider core and into
  ShellApp/framework setup.
- Updated roadmap, architecture, migration, and refactor notes to match the
  current crate boundaries.

## Current Next Step

Specify and test the Observable source macro API from `SOURCE_API.md`, including
compile-expanded Relm4 components that bind observable sources into plain model
fields.
