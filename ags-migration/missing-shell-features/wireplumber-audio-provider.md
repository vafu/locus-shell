# WirePlumber Audio Provider

## Needed By

- [Bar](../migration/widgets/bar.md)
- [OSD](../migration/widgets/osd.md)

## Gap

Audio route and volume UI need PipeWire/WirePlumber state and actions.

## Direction

Default sink, sink list, volume, mute, and icons now come from the locusfs
PipeWire projection.

Remaining provider work:

- expose route grouping metadata equivalent to AGS' `pw-dump` device id and
  `priority.session` data.
- expose a write/action path for setting the default sink so
  `rsynapse-shell` can stop calling `pactl set-default-sink` directly.
