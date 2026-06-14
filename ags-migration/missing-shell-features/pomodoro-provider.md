# Pomodoro Provider

## Needed By

- [App runtime](../migration/widgets/app-runtime.md)

## Gap

Pomodoro state currently drives DND and dynamic theme side effects through AGS.

## Direction

Add a typed provider/client for `org.gnome.Pomodoro` in `rsynapse-shell`;
promote to `common-providers` if it proves generally useful.

