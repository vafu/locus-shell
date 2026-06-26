# Agent Instructions

Before planning or implementing work in this repository, read:

- `PROJECT.md` for the project blueprint and constraints.
- `PLAN.md` for the current roadmap and responsibility boundaries.
- `SOURCE_API.md` when work touches source bindings, observable APIs, or macro
  ergonomics.

Use `$locus-shell` whenever working with shell widgets, GTK/Relm4 components,
AGS migration code, Locus graph source functions, providers, or framework
boundary decisions in this repository.

Preserve the framework boundary:

- `shell/core` creates generic layer-shell GTK windows and exposes framework primitives only.
- Consumer crates own widget roles such as bars, OSDs, notifications, launchers, and workspace switchers.
- `rsynapse-shell` is the active in-repository shell consumer and AGS migration target.

Source implementation rule:

- Prefer Rx-native operators for source behavior whenever possible. Model source
  functions as observable dataflow using operators such as `map`, `filter_map`,
  `combine_latest`, `merge`, `switch_map`, `start_with`, and
  `distinct_until_changed`; keep handwritten async/watch loops isolated inside
  small reusable observable primitives only when an external API must be bridged
  into Rx.
- Do not use timing hacks for correctness or UI update behavior. Avoid
  debounce/throttle/sleep/timeouts to mask partial source updates, list churn,
  ordering bugs, or lifecycle issues; fix the source semantics, event shape, or
  reconciliation model instead. Time-based coalescing is only acceptable for
  inherently noisy external systems such as filesystem stylesheet reloads, and
  must be named as such.

UI layout rule:

- Do not hardcode widget heights in shell UI, including `min-height`,
  `height-request`, or fixed-height CSS/GTK setters. The bar height itself is
  the only place allowed to define vertical size; child widgets must fit that
  height through padding, alignment, or natural sizing.

When proposing or changing architecture, cross-reference `PLAN.md` and keep it updated if the roadmap changes.

# Current Review And Refactor Prompt

Run a non-interactive top-down review and implementation pass similar to the locusfs pass. The coordinator is the decision maker for this pass; user feedback comes at the end.

Primary implementation goal: optimize observable paths. Too many duplicate observables are currently being created for equivalent locusfs paths/source descriptors. Design and implement a smart sharing strategy so equivalent source paths share upstream watch/read work, replay latest values, and stop upstream work when the last subscriber drops.

Compatibility goal: adapt all shell source consumers to the latest locusfs generic D-Bus path layout:

- Replace legacy `object`, `@properties`, `@methods`, and `@absolute` D-Bus path assumptions.
- Use `/dbus/<service>/objects/...` for object property files.
- Use `/dbus/<service>/methods/...` for callable method files.
- Use `_absolute` for outside ObjectManager paths.

Review grounds for each unit:

- API: source API clarity, observable descriptor surface, replacement flexibility, and whether implementation details leak.
- Redundancy: duplicated observable/source helpers, repeated path construction, repeated subscriptions/watch loops, and unnecessary abstraction layers.
- Performance: duplicated watch/read work, allocation churn, hot-path cloning, lock contention, async/threading behavior, and fanout sharing.
- Tidiness: docs, repo boundary alignment, and rust-guide fit.
- Best practices: whether RxRust/shared observable primitives already cover the need before custom runtime code.
- Domain-specific: locusfs paths should be readable, maintainable, and consistent with latest locusfs.

For each review unit, write a detailed report/plan to `refactor/<unit>.md`.

After reports are written, arbitrate and record decisions in `refactor/arbitration.md`. Execute the refactor, keep `refactor/implementation-log.md` with timestamps, decisions, validation commands, and final questions for user validation.

Everything should be covered with focused tests and the whole project test suite should pass.
