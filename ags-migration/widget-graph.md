# AGS Widget Graph

## Reachable Roots

The port scope is based on what is reachable from `/home/v47/.config/ags/app.ts`
after intentionally excluding the unused Rsynapse launcher surface.

```text
app.ts
├── Bar per monitor
│   ├── WorkspacesWidget
│   │   ├── services/workspace-status-provider.ts
│   │   ├── services/locus.ts
│   │   ├── services/locus.generated.ts
│   │   ├── services/hints.ts
│   │   └── widgets/materialicon.tsx
│   ├── WorkspaceWindowIndicators
│   │   ├── services/workspace-status-provider.ts
│   │   ├── widgets/circularstatus.ts
│   │   └── widgets/materialicon.tsx
│   ├── BzBusWidget
│   │   ├── services/bzbus.ts
│   │   └── widgets/materialicon.tsx
│   ├── MPRISWidget
│   │   └── widgets/materialicon.tsx
│   ├── SysStats
│   │   └── scripts/sysstats.sh
│   ├── Tray
│   ├── PowerProfilesIndicator
│   ├── BluetoothStatus
│   │   ├── services/bluetooth/index.ts
│   │   ├── services/bluetooth/devicetype.ts
│   │   └── services/bluetooth/dbus-battery.ts
│   ├── AudioVolumeIndicator
│   │   └── widgets/bar/audio_route.tsx
│   ├── EthIndicator
│   ├── WifiIndicator
│   ├── BatteryIndicator
│   └── DateTime
├── OSD singleton
│   ├── widgets/osd/index.tsx
│   ├── widgets/osd/OSD.tsx
│   ├── services/brightness.tsx
│   └── WirePlumber/AstalWp default speaker state
├── AgentApprovalOverlay singleton
│   ├── widgets/agent-approvals/index.ts
│   ├── widgets/agent-approvals/overlay.tsx
│   ├── services/agent.ts
│   ├── services/locus.generated.ts
│   └── services/requests.ts
├── App runtime side effects
│   ├── services/requests.ts
│   ├── services/pomodoro.ts
│   ├── services/locus.ts
│   ├── style/style.ts
│   ├── style/theming.ts
│   ├── scripts/dnd.sh
│   ├── scripts/super-hints-trigger
│   └── triggerhappy/systemd files
└── Shared helpers
    ├── rxbinding/*
    ├── commons/rx.ts
    ├── widgets/materialicon.tsx
    ├── widgets/circularstatus.ts
    └── widgets/index.ts
```

## Excluded From Port

- `widgets/rsynapse/*`, `services/rsynapse.ts`, and `style/rsynapse.scss`.
  The widget is constructed in `app.ts`, but the user confirmed the launcher is
  unused and should be removed from migration scope.
- `services/remarked.ts`. No reachable import from `app.ts`.
- `services/agent-stats.ts`. No reachable import from `app.ts`.
- `services/agent-session-window.ts`. It registers a request handler when
  imported, but no reachable module imports it.
- `commands.ts`. `app.ts` calls `bindCommands()`, but the function is empty.

## Port Scope

- `bar`
- `osd`
- `agent-approvals`
- `app-runtime`
