# Source Module Instructions

Each source file owns one public view model shape for direct Relm4 consumption.

- Declare the public `*Vm`/view model struct near the top of the file.
- Expose source provider functions that return `Observable<ViewModel>` or
  `Observable<Option<ViewModel>>` for component `#[source(...)]` bindings.
- Keep helper structs, source composition, path construction, decoding policy,
  and pure formatting helpers private in the same file unless another source
  file actually needs them.
- Prefer composing `shell_core::source` observables with Rx operators directly.
  Do not introduce local mini source APIs around `property`, `relation`, or
  `children`; source-level behavior belongs in `shell_core::source`.
- Use `LocusPath` for locusfs path composition and returned path values.
- Prefer Rx-native operators and shell-exposed helpers/types over custom
  implementations. If a source needs a truly custom observable, parser,
  watcher, cache, or adapter, document the reason in the file near that code:
  explain why existing Rx operators, `shell_core::source` primitives,
  `LocusPath`, or other shell services cannot express it cleanly.
