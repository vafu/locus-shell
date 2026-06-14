# Side Effect Runner

## Needed By

- [App runtime](../migration/widgets/app-runtime.md)

## Gap

Pomodoro and theme changes trigger commands and external effects that should be
deduplicated and observable.

## Direction

Implement an `rsynapse-shell` side-effect runner with typed commands, logging,
and retry policy where needed.

