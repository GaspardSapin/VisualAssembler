# crates/va-ui/tests

## Purpose
The headless kittest suite of `va-ui` (m4-editor-shell.md §10.2, exit criteria E5 and E6). Tests
drive egui through `egui_kittest::Harness` and read the AccessKit tree: no window, no GPU. They use
a fake CPU instead of `arch/*` (rule R2 forbids that edge, dev-dependencies included).

**M4 scaffold (wave 1, commit `b4e7aae`).** Only the shared `support/` module exists; there is no
test target yet. WP-E adds `editor_view.rs`, `editor_input.rs`, `editor_cache.rs`, `theme.rs` and
`fonts.rs`, WP-F the `shell_*.rs` files (tabs, explorer, palette, find, status, hot exit), and
WP-D `locales.rs` (m4 §2, §10.2).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `support/` | folder | Shared test code: the fake lexer and architecture, documents and harness helpers. See its README. |

## Entry points
- `support/mod.rs`, included by each test file with `mod support;`.

## Dependencies
- Internal: `va-ui`, `va-core`, `va-editor`.
- External: `egui_kittest` (dev-dependency), `egui`.

## Tests / usage
```bash
cargo test -p va-ui
```
In the scaffold this builds the crate and runs no test.
