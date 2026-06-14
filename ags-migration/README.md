# AGS Migration

## Scope

Migrate the local AGS shell configuration from `/home/v47/.config/ags` into the
Rust `rsynapse-shell` crate.

## Principles

- Treat AGS as behavior and visual reference, not as the architecture to copy.
- Preserve CSS/SCSS visual styling as closely as practical during migration.
- Keep widget responsibilities documented before porting implementation.
- Track missing framework/provider features once, then link widgets to those
  feature notes instead of duplicating gap descriptions.

## Source

- AGS root: `/home/v47/.config/ags`
- Rust target crate: `rsynapse-shell`

## Directories

- `docs/widgets/`: factual widget responsibilities and visual/design notes.
- `migration/widgets/`: locus-shell-native widget migration proposals.
- `missing-shell-features/`: repeated framework/provider gaps discovered during
  widget analysis.
- `widget-graph.md`: actual reachable AGS widget graph and excluded dead
  islands.

## Top-Level Surfaces

- `bar`: per-monitor top bar and status modules.
- `osd`: monitor-bound on-screen display overlay.
- `agent-approvals`: approval overlay and request UI.
- `app-runtime`: cross-widget setup such as monitor window lifecycle, pomodoro
  DND side effects, request handling, command binding, and theme preparation.

Excluded: the AGS `widgets/rsynapse` launcher/search surface is unused and is
not part of the Rust port scope.

## Migration Status

- [x] Inventory reachable AGS widgets and services.
- [x] Write factual widget docs under `docs/widgets/`.
- [x] Write locus-shell-native proposals under `migration/widgets/`.
- [x] Track repeated framework gaps under `missing-shell-features/`.
- [x] Summarize required locus-shell framework updates.
