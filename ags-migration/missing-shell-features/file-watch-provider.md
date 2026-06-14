# File Watch Provider

## Needed By

- [App runtime](../migration/widgets/app-runtime.md)

## Gap

Some AGS helpers convert files into observable values.

## Direction

Add consumer-local file-watch providers using `notify` or existing
`shell-core` stylesheet watcher patterns when non-CSS file state becomes
necessary.

