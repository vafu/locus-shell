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
- 2026-06-26 01:18 PDT: Migrated D-Bus consumers to latest locusfs
  `/objects` and `/methods` paths.
- 2026-06-26 01:20 PDT: Full workspace test suite passed with
  `CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test --workspace`.
- 2026-06-26 01:21 PDT: Wrote arbitration and implementation decision records.

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
- Introduced `rsynapse-shell/src/locusfs_paths.rs` as a consumer-local helper
  for configured D-Bus service paths. This stays out of shell-core until reuse
  by another consumer proves it belongs there.
- Migrated method call paths to direct files under `/dbus/<service>/methods`.
  BlueZ rows now write to `.../methods/.../<Method>` instead of
  `@methods/<Method>/call`.

## Test Coverage

- Added `shell-core` test coverage proving two equal semantic descriptors share
  one active upstream source.
- Added rsynapse-shell tests for D-Bus ObjectManager path mapping:
  root object, relative object, root ObjectManager, outside-manager `_absolute`,
  and absent `/` for non-root managers.
- Updated source child filtering tests so `_absolute` is a visible current
  namespace and only private `@...` entries are hidden.

Verification commands run:

```text
CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test -p shell-core source::support::tests
CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test -p rsynapse-shell
CARGO_TARGET_DIR=/tmp/locus-shell-target cargo test --workspace
```

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
