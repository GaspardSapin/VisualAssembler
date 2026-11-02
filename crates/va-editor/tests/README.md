# crates/va-editor/tests

## Purpose
Tests of `va-editor` (m4-editor-shell.md §10.1, exit criterion E3), one file per source module
(`src/text.rs` → `text.rs`; architecture §8). They use only the public API and run with no UI, no
file and no thread of their own: time is passed in as a `Duration`, and the random cases use a
small seeded generator written in the test file, so every run is the same.

**M4 WP-A (wave 2, commit `7324608`).**

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `text.rs` | test | Line count and line ranges equal `va_core::source::LineIndex` on every terminator kind and on 10,000 random texts; clamped line queries, slices, chunks and `write_to`; cheap clones; `display_col` and `byte_at_display_col` with tabs and multi-byte chars; the widest line; texts spanning several rope chunks |
| `encoding.rs` | test | UTF-8 detection and BOM strip; a NUL byte in the first 8 KiB is binary; the offset of the first invalid UTF-8 byte; 16 MiB + 1 is too large even when forced; all 256 bytes round-trip through Windows-1252 (the WHATWG C1 table); `first_unencodable`; `encode_to` per encoding and on a failing sink; line-ending detection |
| `selection.rs` | test | Normalization, merging (overlapping selections merge, touching ones do not; a caret merges with what it touches), the primary selection and its direction, `collapse_to_primary`, the `MAX_CARETS` cap |
| `edit.rs` | test | Every command of §3.2 with one and three carets, through `Document::edit` and `Document::move_carets`: insert, paste distribution, new line with indentation, tab, indent and outdent, Backspace and Delete, word deletions, line delete, duplicate and move, toggle comment both ways; change-set checks and offset mapping; every motion; random multi-caret edits never overlap and match a string model; CRLF and LF terminators are kept |
| `history.rs` | test | Undo grouping with injected time (a word typed quickly is one group; a 1 s pause, a line break, a caret jump or a change of kind splits it); undo restores the selections; new edits clear redo; undo to the save point is clean; the group and byte limits drop the oldest groups; `replace_text` is one group; undo and redo round trips over random edits |
| `search.rs` | test | Literal, case-insensitive (Unicode folding) and whole-word queries (`.`, `_`, `@` are word chars); regex queries line by line; empty matches skipped; invalid regexes and the size and nesting limits; `MAX_MATCHES`; cancellation; `find_from` both ways with one wrap; `$1`, `${name}` and `$$` templates; `replace_all` counts, limits and terminators |
| `document.rs` | test | A new document is clean; `Send + Sync`; revisions; no-op edits return false; O(1) snapshots that share storage; the widest-line hint; the dirty state with encoding and line-ending changes and with edits made during a save; the size limit; `set_selections`; the selection commands (Ctrl+D, Alt+click, Ctrl+Alt+Up/Down, word and line selection) |

## Entry points
- `edit.rs` and `history.rs`: the multi-caret and undo rules the editor view and the shell rely on.

## Dependencies
- Internal: `va-editor`, `va-core` (`source::LineIndex`, the reference line count).
- External: none beyond the crate's own (`ropey`, `regex`); no dev-dependency.

## Tests / usage
```bash
cargo test -p va-editor
cargo test -p va-editor --test edit
```
On 2026-11-02 the seven targets ran 102 tests, all passing.
