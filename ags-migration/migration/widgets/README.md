# Widget Migration Proposals

Each widget proposal should include:

- AGS source files reviewed.
- Native `rsynapse-shell` component/model outline.
- Initial `#[shell_macros::model]` shape.
- Provider and stream dependencies.
- Links to missing shell features in `../../missing-shell-features/`.
- Open questions.

Current proposals:

- [App runtime](app-runtime.md)
- [Bar](bar.md)
- [System indicators](system-indicators.md)
- [Agent approvals](agent-approvals.md)
- [OSD](osd.md)

Current implementation status:

- Bar is partially implemented in `rsynapse-shell`.
- System indicators currently implemented: clock, CPU/RAM, battery,
  NetworkManager wired/Wi-Fi, PipeWire default sink, PipeWire route popover,
  MPRIS, PowerProfiles, StatusNotifier tray, DBusMenu popovers, and Bluetooth
  groups.
- Initial OSD window is implemented inside the main `rsynapse-shell` process;
  active-monitor rebinding and a proper brightness provider remain.
- Runtime/request infrastructure, agent approvals, per-monitor lifecycle,
  build/BzBus, locusfs-native audio route actions, normalized Bluetooth battery
  data, and exact visual parity remain.
