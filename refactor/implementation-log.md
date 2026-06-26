# Locus Shell Implementation Log

## Milestones

- 2026-06-26 00:50 PDT: Began non-interactive locus-shell review/refactor pass
  and wrote the current prompt into `AGENTS.md`.
- 2026-06-26 01:02 PDT: Docs/path subagent completed
  `refactor/agent-docs-paths.md`.
- 2026-06-26 01:05 PDT: Source-core subagent completed
  `refactor/agent-source-core.md`.
- 2026-06-26 01:06 PDT: Macro subagent completed
  `refactor/agent-macros.md`.
- 2026-06-26 01:07 PDT: Consumer subagent completed
  `refactor/agent-rsynapse-consumer.md`.
- 2026-06-26 01:10 PDT: Implemented `source::shared_by_key`, normalized
  primitive source cache paths, and converted cache entries to weak hubs.
- 2026-06-26 01:14 PDT: Added rsynapse-shell D-Bus path helper and shared
  window/session semantic sources.
- 2026-06-26 01:18 PDT: Migrated D-Bus consumers to the then-current interim
  locusfs `/objects` and `/methods` paths.
- 2026-06-26 01:20 PDT: Full workspace test suite passed with
  `CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test --workspace`.
- 2026-06-26 01:21 PDT: Wrote arbitration and implementation decision records.
- 2026-06-26 10:14 PDT: Updated rsynapse-shell from the interim
  `/objects`/`/methods` D-Bus layout to the bus-native locusfs layout.
- 2026-06-26 10:19 PDT: Rebuilt the release shell binary, installed it to
  `/home/v47/.local/bin/rsynapse-shell`, and restarted the running shell
  process.

## Important Implementation Decisions

- Added `shell_core::source::shared_by_key` instead of changing macro syntax.
  Stable semantic identity is known inside source functions, not by generic
  `#[source(expr)]` token analysis.
- Kept primitive source APIs unchanged. Existing callers continue to use
  `LocusPath::as_property`, `as_children`, `observe_prop`, and similar helpers.
- Normalized path-backed source keys through `LocusPath::new` at the source
  cache boundary.
- Changed source cache entries from strong hubs to weak hubs. Active
  subscriptions keep the hub alive; stale descriptors do not stay strongly held
  forever.
- Introduced `rsynapse-shell/src/widgets/bar/window_source.rs` for shared
  window aggregate data used by workspaces and project labels.
- Introduced `rsynapse-shell/src/widgets/bar/agent_sessions.rs` for shared
  AgentDBus sessions used by project labels and window tiles.
- Kept `rsynapse-shell/src/locusfs_paths.rs` as a consumer-local helper, but
  changed it from configured service/ObjectManager roots to `DBUS_SYSTEM` and
  `DBUS_SESSION` bus roots.
- Migrated method call paths to `.call` files beside D-Bus object properties.
  BlueZ rows now write to `/dbus/system/org/bluez/.../<Method>.call`.

## Test Coverage

- Added `shell-core` test coverage proving two equal semantic descriptors share
  one active upstream source.
- Added rsynapse-shell tests for D-Bus bus-native path mapping: system/session
  roots, root object mapping, non-absolute rejection, and `.call` method files.
- Source child filtering still hides private `@...` entries and treats ordinary
  names such as `_absolute` as visible if a provider exposes them.

Verification commands run:

```text
CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test -p shell-core source::support::tests
CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test -p rsynapse-shell
CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test --workspace
CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test -p rsynapse-shell
CARGO_TARGET_DIR=/tmp/locus-shell-target cargo build --release -p rsynapse-shell
```

Installed binary verification: `pgrep -af rsynapse-shell` reported
`/home/v47/.local/bin/rsynapse-shell` running after restart.

All passed.

## Validation Questions

- Is it acceptable that shared window snapshots observe `app-id` for all
  windows to remove repeated per-workspace fallback scans?
- Should the rsynapse-shell D-Bus helper be promoted to shell-core once another
  consumer needs service-aware object/method path construction?
- Do you want a follow-up pass to fix the two macro correctness issues from
  `refactor/agent-macros.md` before adding generated `#[observable]` support?
- Should we add a runtime fanout check that launches the shell with
  `SHELL_CORE_SOURCE_TRACE=1` and asserts expected descriptor counts for a
  representative window/session setup?
- Should the share/replay hub subscribe-ordering race called out in
  `refactor/agent-source-core.md` be fixed immediately, or only after trace data
  shows real missed emissions?
