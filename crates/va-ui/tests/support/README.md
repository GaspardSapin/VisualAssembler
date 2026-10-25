# crates/va-ui/tests/support

## Purpose
Shared support of the `va-ui` kittest suite (m4-editor-shell.md §10): a fake CPU, so that no test
needs a CPU crate (rule R2), small document builders, and harness helpers. It is a module folder,
not a test target: cargo builds only `tests/*.rs` and `tests/*/main.rs`, and each test file
includes this module with `mod support;`.

**Written in full in WP0 (wave 1, commit `b4e7aae`)**, with no `todo!()`. `document` calls the
`va_editor` constructors, which stay `todo!()` stubs until WP-A implements them in wave 2. WP-E
owns the file after WP0; the shell tests (WP-F) keep their own helpers in their `shell_*.rs` files
and ask WP-E for a change here (m4 §1).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | test support | `FakeSyntax` (a stateless lexer honouring the `SyntaxProvider` contract that counts its calls), `FakeIsa`, `FakeArch` and the shared `FAKE_ARCH`, `fake_arch_support` (line comment `;`); `numbered_text` and `document`; `HARNESS_SIZE` (800×600), `WAIT_TIMEOUT` (5 s), `harness`, `repaint_delay`, `set_time`, `wait_until`; `temp_dir` (`<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>`) |

## Entry points
- `harness(app, state)` for every UI test; `FakeSyntax::calls` for the re-lex counts of
  `editor_cache.rs`.

Notes:
- `FAKE_ARCH` is shared by every test of a binary, and those run in parallel: a test that counts
  lexer calls builds its own `FakeSyntax`.
- The file starts with `#![allow(clippy::unwrap_used)]` and allows `dead_code`, since each test file
  uses a different subset of the helpers.

## Dependencies
- Internal: `va-ui` (`ArchSupport`), `va-core` (`arch`, `diag`, `isa`, `regs`, `source`,
  `syntax`), `va-editor` (`document`, `encoding`, `text`).
- External: `egui_kittest` (dev-dependency), `egui`.

## Tests / usage
```bash
cargo test -p va-ui
```
No test file includes the module yet, so cargo does not compile it in the scaffold; it is first
built with the WP-E tests.
