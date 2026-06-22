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

When proposing or changing architecture, cross-reference `PLAN.md` and keep it updated if the roadmap changes.
