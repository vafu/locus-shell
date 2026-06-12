# shell-core Refactor Notes

## Current Role

`shell-core` is the generic framework crate for GTK4/Relm4 shell widget processes. It currently owns process startup, stylesheet registration/watching, and raw GTK layer-shell window configuration.

This matches the repository boundary: no bar, OSD, notification, launcher, or workspace-switcher role constructors are exposed here.

## Public Surface

- `ShellApp` in `shell/core/src/app.rs` wraps Relm4 application startup and global stylesheet registration.
- `window` exports `WindowConfig`, `Anchors`, `SurfaceMargins`, `ExclusiveZone`, `Layer`, `Edge`, `create_layer_window`, and `apply_layer_shell_config`.
- `css` exports `CssPriority`, `Stylesheet`, `StylesheetSource`, `StylesheetError`, `load_stylesheet`, and `add_css_classes`.
- The crate re-exports `gtk`, `gtk4_layer_shell`, and `relm4` for consumers.

## Step-By-Step File Walkthrough

1. `shell/core/src/lib.rs` - crate-level module map and public re-exports. Read first to see the intended public surface.
2. `shell/core/src/app.rs` - process-level Relm4 application wrapper and stylesheet startup path. Read next because it owns the top-level runtime lifecycle.
3. `shell/core/src/window/config.rs` - generic layer-shell configuration types and builders. Read before `layer.rs` because it defines the consumer-facing vocabulary.
4. `shell/core/src/window/layer.rs` - translation from framework config into `gtk4-layer-shell` calls. Read after config to check whether each option maps cleanly to GTK behavior.
5. `shell/core/src/window/test.rs` - pure tests for the window config API. Read after the implementation to confirm intended behavior.
6. `shell/core/src/css/mod.rs` - CSS module public surface and small helper functions. Read before deeper CSS files to separate compatibility conveniences from the newer stylesheet objects.
7. `shell/core/src/css/source.rs` - CSS versus SCSS source loading and watch-root selection. Read before stylesheet loading because it decides what gets watched.
8. `shell/core/src/css/stylesheet.rs` - stylesheet provider lifecycle, installation, and polling watcher. Read as the core CSS runtime behavior.
9. `shell/core/src/css/fingerprint.rs` - development watcher change detection. Read after `stylesheet.rs` because it explains when reloads fire.
10. `shell/core/src/css/compiler.rs` - SCSS shell-out integration through `sass`. Read after source loading to inspect external process behavior and diagnostics.
11. `shell/core/src/css/error.rs` - typed stylesheet errors. Read with compiler/source code to check user-facing diagnostics.
12. `shell/core/src/css/test.rs` - stylesheet watch-root and fingerprint tests. Read last to compare tested behavior with expected development reload behavior.

## Internal Structure

- `app.rs` prepares stylesheets before `RelmApp::run`, then installs and optionally watches them during GTK startup.
- `window/config.rs` defines backend-neutral config values and builder-style methods.
- `window/layer.rs` maps local config enums to `gtk4-layer-shell` calls.
- `css/source.rs`, `css/compiler.rs`, `css/stylesheet.rs`, and `css/fingerprint.rs` implement CSS/SCSS loading and polling-based development reload.
- Tests currently cover config builders and stylesheet fingerprint behavior.

## Behavior Summary

`ShellApp::try_run` creates a `RelmApp`, loads configured CSS/SCSS into `Stylesheet` instances, registers a startup callback on the main GTK application, installs each stylesheet provider on startup, and starts a 250ms GLib timeout watcher when requested.

`create_layer_window` creates a plain `gtk::Window` and applies layer-shell configuration: layer, keyboard mode, optional namespace, anchors, surface margins, and exclusive zone.

SCSS compilation shells out to the `sass` executable. CSS loads directly from disk. SCSS watching fingerprints the source file's parent directory so changes in imported stylesheet files can trigger recompilation.

## User Notes

- For `shell/core/src/lib.rs`, explicit `gtk` and `relm4` usage/re-exports are expected and acceptable for this framework crate.
- The Rust guide should be tightened: public API traits and structs should live in `mod.rs` or `lib.rs`, not scattered through implementation files.
- For `shell/core/src/app.rs`, `ShellApp` is a good abstraction for a wrapper around binary lifecycle setup.
- Decision from review: `ShellApp` does not need the `PhantomData<M>` marker. The component input/message generic should move to `run` or otherwise live only where the component type is actually constrained.
- Question from review: for builder methods that differ only by defaults, consider whether Rust has an appropriate crate/pattern for single-definition overloads with defaults rather than maintaining method pairs such as `with_css` and `with_css_at_priority`.
- Action point from review: `try_run` can be removed; startup errors can be fatal through `run`.
- For `shell/core/src/window/config.rs`, same Rust guide note applies: public traits and structs should live in `mod.rs` or `lib.rs`; implementation files should not be the public API home.
- Question from review: `WindowConfig` may need a dynamic application/update path, especially for settings that can change at runtime such as keyboard interactivity.
- For `shell/core/src/window/layer.rs`, `WindowConfig` should be reconfigurable rather than only an initial setup value.
- Runtime layer-window reconfiguration should support Locus-driven functions, for example changing keyboard interactivity or related surface behavior from provider state.
- Test-only files are straightforward and can be skipped during interactive review unless they clarify a specific behavior or concern.
- Public domain/API for modules should live in `mod.rs`; continue checking `shell-core` for public definitions that should move to module roots.
- For `shell/core/src/css/mod.rs`, remove `load_stylesheet`; no real usage exists and it bypasses the newer `ShellApp`/`Stylesheet` error path.
- Remove `add_css_classes` if it remains unused; GTK/Relm exposes class APIs directly.
- Keep `CssPriority` public as typed framework vocabulary instead of exposing raw GTK `u32` priority constants.
- Make `CssPriority::gtk_priority` private or `pub(crate)` as an internal GTK conversion helper.
- For `shell/core/src/css/source.rs`, `StylesheetSource` should be internal. CSS versus SCSS should be inferred from the provided file extension/path rather than exposed as public source variants.
- Polling-based stylesheet watching is not acceptable for the shell's low-footprint goal. Replace it with event-driven file watching, likely using `notify`, with debouncing.
- For SCSS, event-driven watching can initially watch the inferred parent directory or configured roots; exact Sass dependency tracking can wait until needed.
- For `shell/core/src/css/stylesheet.rs`, `Stylesheet` appears internal. Public CSS API should be stylesheet path plus optional priority, not a public stylesheet runtime object.
- Split development watch behavior out of `stylesheet.rs` when replacing polling with event-driven watching.
- Simple logging is enough for dev stylesheet reload errors; no framework logging hook is needed now.
- For `shell/core/src/css/fingerprint.rs`, fingerprinting is unnecessary once stylesheet watching uses `notify`; event filtering plus debounce should be the initial design.
- Keep a stylesheet extension filter for notify events (`css`, `scss`, `sass`).
- Be deliberate about which `notify` event kinds trigger reloads; avoid responding to irrelevant metadata/noise where possible.
- If noisy reloads become a real issue, consider compile-output dedup later rather than keeping mtime fingerprinting.
- For `shell/core/src/css/compiler.rs`, the Sass executable should be environment-configurable with a default of `sass`.
- SCSS support is not only development-only; production builds may still load SCSS so users can easily edit styles.
- Sass load paths should be configurable, not limited to Sass defaults.
- For `shell/core/src/css/error.rs`, keep internal `Result<_, StylesheetError>` for contextual diagnostics, but make `StylesheetError` private/internal if no public fallible API returns it.
- Remove `StylesheetError` from public CSS exports when `try_run` and public stylesheet runtime APIs are removed.

## Findings

- Risk: `StylesheetFingerprint` only compares the newest modification time plus the set of stylesheet paths. Rapid edits within filesystem timestamp resolution, or edits that preserve/restore mtimes, may be missed by the development watcher. See `shell/core/src/css/fingerprint.rs`.
- Risk: `css::load_stylesheet` is a separate public path from the newer `Stylesheet`/`ShellApp` registration flow. It silently does nothing when there is no default display and does not report load errors, so consumers may choose a less debuggable API by accident.
- Gap: there are no tests around `ShellApp` stylesheet registration order or watcher startup behavior. This may be acceptable until a GTK test harness is introduced, but the public lifecycle wrapper is currently verified indirectly.

## Refactor Plan

1. Keep the window API generic and continue rejecting role-specific helpers in `shell-core`.
2. Consider replacing `StylesheetFingerprint` with a content hash plus path list for development watching, or document the current mtime limitation if keeping it intentionally lightweight.
3. Decide whether `css::load_stylesheet` remains a supported public convenience or should be deprecated/hidden behind the `Stylesheet` path before the API stabilizes.
4. Add pure tests for any fingerprint behavior change. Add GTK lifecycle tests only if the project later standardizes a headless GTK test setup.
5. At the end of the review pass, evaluate moving public API type definitions/re-exports into `lib.rs` or feature `mod.rs` files per the tightened Rust guide. For `shell-core`, this likely means checking whether `app.rs`, `window/config.rs`, and CSS implementation files expose public structs/enums that should instead be declared or re-exported from module roots.
6. Remove `ShellApp::try_run` if the project wants `ShellApp::run` to be the only public lifecycle entrypoint and considers stylesheet startup failures fatal.
7. Remove `ShellApp<M>`'s generic marker. If input constraints are still needed, put them on `run<C>` instead of the `ShellApp` type.
8. Review defaulted builder method pairs (`with_css` / `with_css_at_priority`, `with_scss` / `with_scss_at_priority`) and decide whether a local pattern or helper crate is worth it. Avoid adding a dependency unless it clearly improves API clarity.
9. Move `window` public API definitions toward `window/mod.rs` per the Rust guide tightening, leaving sibling files for implementation details.
10. Evaluate whether `WindowConfig` needs a dynamic update API for changing already-created layer windows, starting with keyboard interactivity.
11. Split one-time layer-shell initialization from repeatable layer-window reconfiguration if `apply_layer_shell_config` becomes a runtime update path.
12. Design reconfiguration APIs so Locus/provider-driven values can update window behavior without rebuilding the GTK window.
13. Remove unused CSS convenience helpers (`load_stylesheet`, likely `add_css_classes`) and keep `CssPriority` as the public priority type with non-public GTK conversion.
14. Make stylesheet source detection internal and infer CSS/SCSS from file extension/path.
15. Replace the 250ms polling stylesheet watcher with event-driven watching plus debouncing for development reloads.
16. Make `Stylesheet` internal unless a concrete consumer need appears for direct runtime stylesheet objects.
17. Split event-driven stylesheet watching into a dedicated watcher module.
18. Remove the mtime fingerprint module when notify-based watching is implemented, preserving only any needed stylesheet path filtering.
19. Add configuration for the Sass executable path/name, defaulting to `sass` and allowing environment override.
20. Add configurable Sass load paths and ensure stylesheet watching can cover those roots when enabled.
21. Make stylesheet errors internal unless a public fallible stylesheet API is reintroduced.

## Tests And Verification

- `cargo check` passes for the workspace.
- Existing `shell-core` tests cover config builders and fingerprint path behavior.

## Open Questions

- Should stylesheet watching favor correctness via hashing, or is low-overhead mtime polling sufficient for development-only reloads?
- Should `ShellApp::run` continue to panic on stylesheet errors, or should framework consumers be nudged toward `try_run` for better diagnostics in widget binaries?
