# Locus Shell Review Arbitration

Timestamp: 2026-06-26 01:21 PDT

Coordinator scope: review and refactor `../locus-shell` with the same
top-down rubric as the locusfs pass, prioritizing observable path sharing and
compatibility with the latest locusfs D-Bus filesystem layout.

## Inputs

- `refactor/agent-source-core.md`
- `refactor/agent-macros.md`
- `refactor/agent-rsynapse-consumer.md`
- `refactor/agent-docs-paths.md`
- Adjacent locusfs source of truth: `../locusfs/plugins/dbus/src/state.rs` and
  `../locusfs/plugins/dbus/src/state/test.rs`
- Local docs: `SOURCE_API.md`, `PLAN.md`, `AGENTS.md`

## Decisions

### 1. Sharing Belongs In Source APIs, Not Macro Token Deduplication

Decision: keep `#[source(expr)]` generic and runtime-free. Do not attempt to
deduplicate arbitrary source expressions by comparing macro token streams.

Reasoning: the macro report found this would merge the wrong expressions and
miss equivalent runtime descriptors. Source functions are the place where
stable semantic identity is known.

Implementation: added `shell_core::source::shared_by_key(kind, key, create)`.

### 2. Primitive Sharing Stays Transparent

Decision: preserve the existing primitive `watch`, `property`, `relation`,
`node`, `children`, and `children_events` API. Normalize primitive paths before
cache lookup so equivalent `LocusPath`/raw `PathBuf` callers are more likely to
share.

Implementation: `shared_source` now normalizes through `LocusPath::new`.

### 3. Avoid Strong Process-Lifetime Retention For New Semantic Keys

Decision: the source registry should not hold strong references to every
transient descriptor forever.

Implementation: source cache entries now store weak hubs. Active observables
and subscriptions keep the hub alive; dead descriptors can be recreated on
later lookup.

### 4. Fix The Real Observable Explosion First

Decision: prioritize row-amplified derived graphs over primitive watches.

Implementation:

- Added shared `window_snapshots()` for aggregate window placement/app-id data.
- Added shared `agent_sessions()` for AgentDBus session data.
- Reused those sources from project labels, selected workspace windows, and
  window tile agent indicators.
- Shared BlueZ/UPower detail scans across Bluetooth popover groups.
- Shared systray and dbusmenu item list sources.

### 5. Latest Locusfs D-Bus Layout Is Current Contract

Decision: rsynapse-shell should consume the generic locusfs D-Bus projection:

```text
/dbus/<service>/objects/<relative-object-path>/<Property>
/dbus/<service>/methods/<relative-object-path>/<Method>
```

Legacy `object`, `@properties`, `@methods`, `@absolute`, and method `/call`
paths are removed from source code.

Implementation:

- Added `rsynapse-shell/src/locusfs_paths.rs`.
- Migrated Battery, PowerProfiles, NetworkManager, BlueZ, and AgentDBus
  consumers.
- Added tests for ObjectManager-relative, root-manager, and `_absolute` object
  path mapping.

### 6. D-Bus Helper Stays In rsynapse-shell For Now

Decision: do not move the D-Bus layout helper into shell-core yet.

Reasoning: shell-core should stay a generic locusfs/Rx source layer. The helper
currently encodes rsynapse-shell service local IDs and ObjectManager roots. If a
second consumer needs the same service-aware construction, promote a generic
helper deliberately.

### 7. Macro Correctness Findings Are Deferred

Decision: do not mix macro correctness work into this observable-path pass.

Deferred items from `agent-macros.md`:

- async nested source models are not started;
- dirty-mask validation should count direct plus nested source fields;
- subscription expansion should be factored before generated descriptor support.

## Residual Risks

- `shared_by_key` still depends on hand-authored keys. Bad keys can under-share
  or incorrectly share unrelated semantic graphs.
- The share/replay hub still has a known subscribe ordering race called out in
  `agent-source-core.md`; this pass did not replace the subject-based hub.
- The window snapshot source observes `app-id` for all windows because project
  label fallback needs it. This is a deliberate tradeoff: fewer repeated graphs
  at the cost of one broader shared aggregate.
- Real runtime fanout should be measured with `SHELL_CORE_SOURCE_TRACE`; tests
  validate mechanics but not live descriptor counts under a compositor.
