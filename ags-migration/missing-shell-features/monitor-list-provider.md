# Monitor List Provider

## Needed By

- [OSD](../migration/widgets/osd.md)
- [App runtime](../migration/widgets/app-runtime.md)

## Gap

Runtime needs typed monitor topology changes and connector lookup.

## Direction

Add a consumer provider around GTK/GDK monitor state. Consider a generic
`shell-core` helper only if it can stay independent of widget policy.

