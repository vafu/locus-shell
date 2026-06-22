# MPRIS Provider

## Needed By

- [Bar](../migration/widgets/bar.md)

## Status

Implemented in `../locusfs/plugins/mpris` and consumed by the Rust bar.

## Projection

The `rsynapse-shell` bar consumes:

```text
/mpris/player/<id>/artist
/mpris/player/<id>/title
/mpris/player/<id>/album
/mpris/player/<id>/art-url
/mpris/player/<id>/playback-status
/mpris/player/<id>/can-play
/mpris/player/<id>/can-pause
/mpris/player/<id>/can-go-next
/mpris/player/<id>/can-go-previous
/mpris/player/<id>/playerctl-name
```

The dedicated plugin watches session-bus `org.mpris.MediaPlayer2.*` services and
subscribes to MPRIS property changes; the generic D-Bus projection remains
object-oriented and is not used for media player state.
