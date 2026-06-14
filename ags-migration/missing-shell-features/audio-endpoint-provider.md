# Audio Endpoint Provider

## Needed By

- [OSD](../migration/widgets/osd.md)
- [Bar](../migration/widgets/bar.md)

## Gap

The shell needs default output volume, mute state, symbolic icon, endpoint list,
and route selection without depending on AGS/Astal.

## Direction

Add a consumer/provider module for WirePlumber/PipeWire first. Promote reusable
typed definitions later if they stabilize.

