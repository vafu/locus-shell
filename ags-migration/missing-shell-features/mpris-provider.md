# MPRIS Provider

## Needed By

- [Bar](../migration/widgets/bar.md)

## Gap

The bar needs media playback metadata and status from MPRIS players.

## Direction

Add a live locusfs MPRIS projection. The `rsynapse-shell` bar now expects:

```text
/mpris/player/<id>/artist
/mpris/player/<id>/title
/mpris/player/<id>/playback-status
/mpris/player/<id>/can-play
```

The generic locusfs D-Bus plugin is not enough yet because it snapshots service
objects on owner changes and does not subscribe to MPRIS property changes.
