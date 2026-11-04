# crates/va-ui/tests/support

## Purpose
Shared support of the `va-ui` kittest suite (m4-editor-shell.md §10): a fake CPU, so that no test
needs a CPU crate (rule R2), small document builders, and harness helpers. It is a module folder,
not a test target: cargo builds only `tests/*.rs` and `tests/*/main.rs`, and each test file
includes this module with `mod support;`.

**Written in WP0 (wave 1, commit `b4e7aae`)**, with no `todo!()`, and extended by WP-E (commit
`2cc6e9e`) with the editor helpers. WP-E owns the file after WP0; the shell tests (WP-F) keep their
own helpers in their `shell_*.rs` files and ask WP-E for a change here (m4 §1).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | test support | `FakeSyntax` (a stateless lexer honouring the `SyntaxProvider` contract that counts its calls), `FakeIsa`, `FakeArch` and the shared `FAKE_ARCH`, `fake_arch_support` (line comment `;`); `numbered_text` and `document`; `HARNESS_SIZE` (800×600), `WAIT_TIMEOUT` (5 s), `harness`, `repaint_delay`, `set_time`, `wait_until`; `temp_dir` (`<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>`). WP-E: `settle`, `click_at`, `EDITOR_LABEL`, `CODE_FONT_SIZE`, `OTHER_WIDGET_LABEL`, `editor_style`, `EditorState` (document, view and frame inputs; `text`, `rect`, `char_pos`) and `editor_harness` |

## Entry points
- `harness(app, state)` for every UI test, `editor_harness(EditorState::new(text))` for the
  editor tests; `FakeSyntax::calls` for the re-lex counts of `editor_cache.rs`.

Notes:
- `FAKE_ARCH` is shared by every test of a binary, and those run in parallel: a test that counts
  lexer calls builds its own `FakeSyntax`.
- The file starts with `#![allow(clippy::unwrap_used)]` and allows `dead_code`, since each test file
  uses a different subset of the helpers.

## Dependencies
- Internal: `va-ui` (`ArchSupport`, `editor`, `fonts`, `theme`), `va-core` (`arch`, `diag`, `isa`,
  `regs`, `source`, `syntax`), `va-editor` (`document`, `edit`, `encoding`, `text`).
- External: `egui_kittest` (dev-dependency), `egui`.

## Tests / usage
```bash
cargo test -p va-ui
```
`editor_view.rs`, `editor_input.rs`, `editor_cache.rs` and `theme.rs` include the module, so it
is built with each of them.
