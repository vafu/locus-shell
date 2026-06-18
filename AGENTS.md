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
- `dev-widgets` is internal development code for testing framework ergonomics, not a user-facing implementation.

When proposing or changing architecture, cross-reference `PLAN.md` and keep it updated if the roadmap changes.
