# crates/va-ui/src/editor

## Purpose
The editor widget (m4-editor-shell.md §5.2–§5.6): a custom, virtualized view of a
`va_editor::document::Document`, coloured by the architecture's lexer through
`va_core::syntax::SyntaxProvider`, never an egui `TextEdit`. Every per-frame cost is O(visible
lines): only the lines in the viewport, plus one above and one below, are fetched, lexed through a
per-line cache and painted.

**Implemented in wave 2 by WP-E (commit `2cc6e9e`)** over the WP0 scaffold (wave 1, commit
`b4e7aae`): the items moved into seven private files, re-exported by name (`pub use`, no glob) from
`mod.rs`, so the paths `va_ui::editor::*` never move (m4 §15.1 M2); no `todo!()` is left. The shell
that hosts the view is still a WP-F stub.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | Module doc and the `pub use` by name of the items below |
| `view.rs` | module | `EditorView` (`new`, `show`, `stats`, `metrics`, `cached_lines`, `top_line`, `scroll_to_line`, `request_focus`) and the frozen types of §5.2: `EditorStyle`, `DecorationKind`, `Decoration`, `EditorContext`, `EditorResponse`, `RenderStats`; the per-frame paint order |
| `layout.rs` | module | `ViewMetrics`: fixed-height rows, the visible line range, the capped line fetch (`MAX_DRAWN_CHARS`), the display↔byte map of tab-expanded lines |
| `cache.rs` | module | Colouring through `SyntaxProvider::lex_line` and the per-line cache keyed by the drawn text (`LINE_CACHE_CAPACITY`), cleared on a style or font-atlas change (§5.3) |
| `input.rs` | module | Keys, clipboard, IME and mouse after the shell's keymap (§5.6) |
| `caret.rs` | module | Caret blink (`BLINK_HALF_PERIOD`, `BLINK_TIMEOUT`: no wake-up after 10 s idle) and caret painting |
| `gutter.rs` | module | Line numbers and the M5 marker column (`gutter_digits`, `gutter_columns`) |
| `plain.rs` | module | `PlainSyntax`, the lexer of a build without any CPU |

## Entry points
- `EditorView::show(ui, doc, ctx)`: draws one frame of a document and applies its input.

## Dependencies
- Internal: `va-core` (`source`, `syntax`), `va-editor` (`document`, `edit`, `selection`, `text`),
  `crate::theme`, `crate::fonts`.
- External: `egui`.

## Tests / usage
```bash
cargo test -p va-ui
```
The editor tests are `crates/va-ui/tests/editor_view.rs`, `editor_input.rs` and `editor_cache.rs`
(m4 §10.2: on a 100,000-line document only visible lines are laid out; one keystroke re-lexes one
line; no repaint at rest).
