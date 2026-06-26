# Locus Shell Agent Guide

This repository is the Rust/Relm4 shell framework plus the in-repository
`rsynapse-shell` consumer. It consumes LocusFS; it does not own the LocusFS
filesystem layout or D-Bus plugin implementation.

## First Steps

Before planning or editing, read:

- `PROJECT.md` for the project blueprint and constraints.
- `PLAN.md` for the current roadmap and crate boundaries.
- `SOURCE_API.md` when work touches source bindings, observable APIs, LocusFS
  paths, D-Bus path helpers, or macro ergonomics.
- `shell/core/src/source/AGENTS.md` before changing shell-core source
  primitives.
- `rsynapse-shell/src/widgets/AGENTS.md` before changing concrete widgets or
  widget-local source providers.

Use `$locus-shell` and `rust-guide` for Rust, GTK/Relm4, source, and
framework-boundary work.

## Workspace Boundaries

- `shell/core` owns generic GTK/layer-shell app setup, window primitives,
  stylesheet loading, `LocusPath`, and reusable Observable source primitives.
- `shell/macros` owns Relm4/model/source binding macros.
- `shell/rx-macros` owns small RxRust composition macros only.
- `rsynapse-shell` owns concrete bar, OSD, request CLI/server, AGS migration
  behavior, styling, widget-specific view models, and product policy.
- Do not reintroduce removed `provider/*` crates, `ObservableSource<T>`, a
  custom provider task runtime, or a D-Bus provider layer in this repo.
- D-Bus service implementation belongs in `../locusfs` or `../claude-dbus`.
  Shell code only consumes their public surfaces.

## Observable Source Contract

The source API is Observable-first.

- Widget model fields are plain values.
- Source expressions return `shell_core::source::Observable<T>`.
- Macro-generated glue subscribes to observables and updates Relm4 model state.
- Use Rx-native operators such as `map`, `filter_map`, `combine_latest`,
  `merge`, `switch_map`, `start_with`, and `distinct_until_changed`.
- Use `shell_rx_macros::combine_latest!` for fixed-arity heterogeneous source
  composition when plain RxRust chains are awkward.
- Keep handwritten async/watch loops isolated inside small shell-core
  primitives that bridge external APIs into Observable form.

Sharing rules:

- Primitive LocusFS sources already share by normalized backend path, primitive
  kind, and emitted type.
- Derived semantic sources that many widgets/rows can request should use
  `source::shared_by_key(kind, key, || ...)`.
- Shared sources must replay the latest value to new subscribers, start upstream
  work on the first active subscriber, and stop upstream work when the last
  subscriber drops.
- Do not add local `OnceLock` caches or manual `.shared()` wrappers in widgets
  unless `shared_by_key` cannot express the descriptor.
- Do not use debounce, sleeps, or timeouts to hide source ordering bugs, list
  churn, or lifecycle problems. Time-based coalescing is only acceptable for
  inherently noisy external systems such as stylesheet reloads, and it must be
  named as such.

Consumer source rules:

- Consumers compose `shell_core::source` observables; they must not depend on
  the `locusfs-watch` client API.
- Do not call `locusfs_watch::read_*`, `Watch::open`, or custom Future/Stream
  factories from `rsynapse-shell`.
- Add a shell-core observable primitive when a backend capability is generally
  reusable.
- Keep concrete widget view models and display policy in `rsynapse-shell`.

## Current LocusFS D-Bus Layout

The current generic D-Bus path layout in LocusFS is bus-native:

```text
/dbus/system/<actual/dbus/object/path>/<Property>
/dbus/system/<actual/dbus/object/path>/<Method>.call
/dbus/session/<actual/dbus/object/path>/<Property>
/dbus/session/<actual/dbus/object/path>/<Method>.call
```

Shell path rules:

- Use `rsynapse-shell/src/locusfs_paths.rs` for local D-Bus path construction.
- Use `DBUS_SYSTEM.object("/org/...")` and
  `DBUS_SESSION.object("/io/...")` with absolute native D-Bus object paths.
- Use `method_for_object(..., "MethodName")` to append `.call` method files.
- Reject non-absolute D-Bus object paths at helper boundaries.
- There is no service-local public root, no ObjectManager-relative stripping,
  no `_absolute`, no `objects` directory, no `methods` directory, no
  `@properties`, no `@methods`, and no method `/call` child.
- Do not duplicate the service name in shell paths for readability. For
  example, UPower is `/dbus/system/org/freedesktop/UPower/...`, not
  `/dbus/system/org.freedesktop.UPower/org/freedesktop/UPower/...`.
- When listing children under D-Bus objects, remember that method `.call` files
  may appear beside property files and object directories. Filter `.call`
  entries when enumerating object/session children, as the AgentDBus session
  source does.

## Widget Rules

- Keep each widget module self-contained. Source providers live beside the
  widget that consumes them.
- Do not add a top-level `src/sources` module.
- A provider should expose `Observable<ViewModel>` or
  `Observable<Option<ViewModel>>` for `#[source(...)]` bindings.
- Use `LocusPath` for LocusFS path composition and returned path values.
- Prefer enum-shaped view models when they simplify Relm4 view matching; use
  subcomponents when enum unpacking makes the view awkward.
- Keep helper structs, parsing, formatting, and path construction private unless
  another widget actually needs them.
- Do not hardcode widget heights in Rust or CSS. The bar height itself may set
  vertical size; child widgets should use padding, alignment, and natural size.
- Do not solve visual behavior by adding ad hoc graph traversal inside GTK
  component lifecycle methods.

## Request CLI/Server

- The Unix-socket request bridge is `rsynapse-shell` product behavior, not a
  `shell-core` framework feature.
- Keep command names and policies such as `scheme-toggle` and `hints
  active|show|hide|toggle` in `rsynapse-shell` unless another consumer needs the
  same transport contract.
- Direct `.config/ags` runtime usages should be migrated to `rsynapse-shell`
  commands when the behavior now lives in Rust.

## AgentDBus Consumption

- The shell consumes AgentDBus through LocusFS's generic D-Bus projection at
  `/dbus/session/io/github/AgentDBus/...`.
- Agent session rows should represent current visible state, not every stale
  historical session object. Prefer the latest session for a window when
  multiple session objects share the same `WindowId`.
- Do not let method call files such as `RespondToElicitation.call` enter the
  session list.
- If status appears stuck in `thinking`, verify AgentDBus `State` directly over
  D-Bus and inspect the source aggregation before adding timing workarounds.

## Do

- Keep framework code generic and consumer policy in `rsynapse-shell`.
- Prefer existing `shell_core::source` primitives and Rx operators over custom
  source runtimes.
- Add focused tests for path helpers, source filtering, parsing, and
  request-command behavior.
- Update `PLAN.md` or relevant refactor docs when architecture changes.
- Preserve nested AGENTS guidance; top-level rules are broad, nested rules win
  for their directories.

## Don't

- Do not add public one-shot read helpers or imperative LocusFS clients to
  `shell-core::source`.
- Do not expose `locusfs-watch` client usage to consumer crates.
- Do not reintroduce schema-specific marker structs, generated-style path
  extension traits, `NodeRef`, `Property`, `Relation`, or a provider facade in
  this workspace.
- Do not hand-write generated schema APIs. If a graph concept needs generated
  helpers, change schema/codegen in the owning repository and run codegen.
- Do not patch generated files manually.
- Do not use timing hacks to make UI updates appear stable.
- Do not put UI cards inside cards or hardcode child widget heights.

## Verification

Useful commands:

```sh
env CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test --workspace
cargo fmt --check
```

Narrow checks:

```sh
env CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test -p shell-core source::support::tests
env CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test -p rsynapse-shell locusfs_paths
env CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test -p rsynapse-shell agent_sessions
env CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test -p rsynapse-shell request
```

Install/restart when the running shell should reflect changes:

```sh
env CARGO_TARGET_DIR=/tmp/locus-shell-target cargo install --path rsynapse-shell --locked --force --root /home/v47/.local
systemctl --user restart rsynapse-shell.service
systemctl --user status rsynapse-shell.service --no-pager
```

Live checks that often catch integration mistakes:

```sh
find /run/user/1000/locusfs/dbus/session/io/github/AgentDBus/sessions/codex -maxdepth 1 -mindepth 1 -printf '%f\n' | sort
rsynapse-shell request hints toggle
rsynapse-shell request scheme-toggle
```
