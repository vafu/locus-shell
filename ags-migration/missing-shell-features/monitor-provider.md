# Monitor Provider

## Needed By

- [App runtime](../migration/widgets/app-runtime.md)
- [OSD](../migration/widgets/osd.md)
- [Bar](../migration/widgets/bar.md)

## Gap

Monitor state is needed by per-monitor bars and active-monitor overlays.

## Direction

Same family as [Monitor list provider](monitor-list-provider.md). Prefer typed
`MonitorInfo`/`MonitorId` values over raw `Gdk.Monitor` in provider DTOs.

