# shell-macros Refactor Notes

## Current Role

`shell-macros` is the procedural macro crate for reducing Relm4/provider binding boilerplate. It parses binding declarations, generates provider state/messages/subscription startup code, and rewrites view binding attributes into Relm4 `#[track(...)]` setters.

The crate currently supports both legacy inline binding declarations and the newer typed `#[shell_macros::model]` field-source flow.

## Public Surface

- `#[shell_macros::bindings(...)]` expands an inline module containing provider state and startup glue.
- `#[shell_macros::component(...)]` transforms a Relm4 component impl, injects provider startup, adds a default update method when needed, and rewrites `#[bind(...)]` / compatibility `#[locus(...)]` view attributes.
- `#[shell_macros::model(...)]` transforms a source model struct by adding a private `__shell` runtime field and generating a companion module with messages, dirty tracking, errors, and subscription startup.

## Step-By-Step File Walkthrough

1. `shell/macros/src/lib.rs` - proc-macro entrypoints. Read first to see the public macro names and error conversion pattern.
2. `shell/macros/src/locus_bindings/mod.rs` - top-level macro dispatch for legacy bindings, component transforms, and typed model expansion. Read next because it connects parsing, model mutation, and code generation.
3. `shell/macros/src/locus_bindings/config.rs` - attribute parsing for bindings/components/models, field-source extraction, validation, and generated variant naming. Read before expansion because it defines accepted syntax and macro diagnostics.
4. `shell/macros/src/locus_bindings/component.rs` - Relm4 impl transformation: view rewrite, subscription injection, `post_view` dirty-clear injection, and fallback `update` method generation. Read before generated module code to understand where macro output is inserted.
5. `shell/macros/src/locus_bindings/expand.rs` - code generation for legacy modules and typed model sidecar modules. Read as the primary generated runtime shape.
6. `shell/macros/src/locus_bindings/view.rs` - token-level view macro rewrite from `#[bind(field)]` / `#[locus(field)]` into tracked setter calls. Read after expansion because it depends on generated `Field`/`changed` APIs.
7. `shell/macros/src/locus_bindings/test.rs` - parser and token-shape tests for all current macro modes. Read last to see which generated patterns are locked down.

## Internal Structure

- Parsing is isolated in `config.rs` using `syn::Parse`.
- Component-level transformation is separated from codegen, but both legacy and typed-model output generate similar `Field`, `Msg`, `WatchError`, dirty mask, and subscription logic.
- View rewriting is manual token-tree traversal instead of parsing Relm4's `view!` DSL as Rust syntax.
- Tests mostly inspect generated token strings rather than compiling expanded macro output against a real component.

## Behavior Summary

Legacy component bindings generate a local `sources`-style module with a `Model`, result-carrying `Msg` variants, dirty tracking, last-error state, and `start(sender)` function. The component macro inserts `model.sources.set_subscriptions(sources::start(sender.clone()))` after a local `let model = ...` in `init`.

Typed models keep the user struct as the source state object, remove `#[source]`/legacy `#[locus(source = ...)]` field attributes, append `__shell: sources::Runtime`, generate `new`, optional `Default`, `update`, `start`, `changed`, `clear_changed`, and `last_error` helpers, and let `#[shell_macros::component(model = ..., state = ...)]` wire startup into the Relm4 component.

View attributes rewrite a setter into `#[track(model.state.changed(sources::Field::FieldVariant))]` plus an adapter that receives `&model.state.field`.

## User Notes

None yet.

## Findings

- Risk: model-mode view bindings do not validate that `#[bind(field)]` refers to a generated source field. A typo generates `sources::Field::Typo` / `model.sources.typo` and fails later as a Rust compile error instead of a macro diagnostic.
- Risk: component injection is tightly coupled to `init` containing a simple local binding named `model`. Other valid-looking component shapes fail with a macro error or require restructuring.
- Risk: if `#[shell_macros::component(model = ...)]` is used with a wrapped input enum and no handwritten `update`, the generated fallback update calls `self.state.update(msg)` with `Self::Input`, which only works when `Self::Input` is exactly the generated message type.
- Risk: generated watcher errors are currently converted into `providers::ProviderError` by display string because arbitrary provider error types are not nameable in generated message enums.
- Gap: tests assert generated token substrings but do not compile expanded code through realistic Relm4 components or check negative cases for view binding typos, unsupported init shapes, wrapped-input fallback updates, or generated `post_view` signatures.
- Refactor smell: legacy module generation and typed-model generation duplicate dirty tracking, field enums, watch errors, messages, subscription startup patterns, and update handling.

## Refactor Plan

1. In model mode, validate `#[bind(field)]` against fields discovered from the referenced typed model if practical. If the macro cannot inspect the external type, document that model-mode typos are compiler errors and add trybuild coverage.
2. Add compile-fail/compile-pass tests with `trybuild` or an equivalent harness for generated component shapes, especially typed models, wrapped input enums, and view setter rewrites.
3. Consider requiring explicit `update` when `Self::Input` is not the generated message type, or add a config knob for the wrapper variant so the macro can generate correct fallback code.
4. Factor shared generated runtime pieces between `expand_locus_module` and `expand_model_impl` to reduce drift before adding more provider combinator support.
5. Preserve richer provider errors if `providers::ProviderError` is hardened to keep error sources.

## Tests And Verification

- `cargo test -p shell-macros` passes: 18 unit tests, 0 doctests.

## Open Questions

- How much should the macro constrain component shape versus relying on clear documentation and compiler errors?
- Is the legacy `#[bindings]` / inline binding mode still needed after typed `#[model]` stabilizes, or should it become a compatibility-only path with fewer new features?
