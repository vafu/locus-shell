# Audio Endpoint Provider

## Needed By

- [OSD](../migration/widgets/osd.md)
- [Bar](../migration/widgets/bar.md)

## Status

Default output volume, mute state, symbolic icon, endpoint list, and route
selection are available through the locusfs PipeWire projection and Rust bar
sources.

## Direction

Remaining follow-up:

- expose a locusfs write/action node for default-sink changes so the Rust bar
  no longer shells out to `pactl set-default-sink`.
- expose route grouping metadata if the AGS grouped route layout should be
  matched exactly.
