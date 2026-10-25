# crates/va-ui/src/editor

## Purpose
The editor widget (m4-editor-shell.md §5.2–§5.6): a custom, virtualized view of a
`va_editor::document::Document`, coloured by the architecture's lexer through
`va_core::syntax::SyntaxProvider`, never an egui `TextEdit`. Every per-frame cost is O(visible
lines): only the lines in the viewport, plus one above and one below, are fetched, lexed through a
per-line cache and painted.

**WP0 scaffold (wave 1, commit `b4e7aae`); implemented in wave 2 by WP-E.** `mod.rs` holds the
frozen types and signatures of §5.2 with `todo!()` bodies (it panics when used); only
`PlainSyntax::keywords` is final.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | `EditorStyle`, `DecorationKind`, `Decoration`, `EditorContext`, `EditorResponse`, `RenderStats`, `EditorView` (`new`, `show`, `stats`, `top_line`, `scroll_to_line`, `request_focus`), and `PlainSyntax`, the lexer of a build without any CPU |

WP-E splits the items into the private modules `view.rs`, `layout.rs`, `cache.rs`, `input.rs`,
`caret.rs`, `gutter.rs` and `plain.rs` (m4 §2) by the end of its package, and re-exports each frozen
item by name from `mod.rs` (`pub use`, no glob), so the paths `va_ui::editor::*` never move
(m4 §15.1 M2).

## Entry points
- `EditorView::show(ui, doc, ctx)`: draws one frame of a document and applies its input.

`mod.rs` carries `#![expect(unused_variables, dead_code, clippy::todo, reason = "WP0 stub: …")]`;
a listed lint that stops firing raises `unfulfilled_lint_expectations`, which CI denies. WP-E
removes each lint in the commit that makes it stop firing and the attribute with the last
`todo!()`, never replacing it by `allow`; a new stub file carries its own expectation (m4 §15.1 M3).

## Dependencies
- Internal: `va-core` (`syntax`), `va-editor` (`document`, `edit`), `crate::theme`.
- External: `egui`.

## Tests / usage
```bash
cargo test -p va-ui
```
In the scaffold this runs no test. WP-E adds `crates/va-ui/tests/editor_view.rs`,
`editor_input.rs` and `editor_cache.rs` (m4 §10.2: on a 100,000-line document only visible lines
are laid out; one keystroke re-lexes one line; no repaint at rest).
