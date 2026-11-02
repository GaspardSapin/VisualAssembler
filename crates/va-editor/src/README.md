# crates/va-editor/src

## Purpose
Sources of `va-editor` (m4-editor-shell.md §3). **Implemented in wave 2 by WP-A (commit
`7324608`).** Each module of the WP0 scaffold (commit `b4e7aae`) moved out of `lib.rs` into its
own file with its bodies, and the stub expectations went with the last `todo!()`. `lib.rs` now
holds the crate doc and seven `pub mod` lines; the module paths (`va_editor::text::Text`, …)
never changed (m4 §15.1 M1).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate doc (offsets in bytes, 0-based lines, time passed in) and the seven `pub mod` declarations |
| `text.rs` | module | §3.1: `Text` over a `ropey::Rope` built with `cr_lines` only, so its breaks are LF, CRLF and lone CR and its line count equals `va_core::source::LineIndex`; `MAX_DOCUMENT_BYTES` = 16 MiB; line queries, slices, chunks, `write_to`, `widest_line_chars`; `display_col` and `byte_at_display_col` (tabs) |
| `encoding.rs` | module | §3.1: `Encoding` (UTF-8, UTF-8 with BOM, Windows-1252 when forced, through a hand-written table of the bytes 0x80–0x9F), `LineEnding`, `DecodeError`, `decode`, `first_unencodable`, `encode_to`, `detect_line_ending` |
| `selection.rs` | module | §3.2: `Selection` and `Selections`, never empty, sorted and merged, with a primary selection (the last added caret) and the `MAX_CARETS` cap; no box selection |
| `edit.rs` | module | §3.2: `Change`, `ChangeSet`, `ChangeError`, `Assoc`, `EditCommand`, `Motion`, `EditOptions`, `is_word_char`; a command is planned as one sorted list of non-overlapping changes plus the landing place of every selection |
| `history.rs` | module | §3.3: `UNDO_GROUP_MS`, `MAX_UNDO_GROUPS`, `MAX_UNDO_BYTES`, `EditKind`, and the crate-private undo history: groups of change sets with their inverses, merged by kind, caret position and time |
| `document.rs` | module | §3.3: `Document` (text, selections, history, revision, dirty state against the saved content, encoding and line ending, widest-line hint, the selection commands) and `Snapshot`, an O(1) copy for workers |
| `search.rs` | module | §3.4: `Query`, `QueryError`, `Matcher`, `Matches`, `find_all` (cancellable, capped at `MAX_MATCHES`), `find_from` (one wrap), `expand_replacement`, `replace_all`; line by line, linear-time regex bounded by `REGEX_SIZE_LIMIT` and `REGEX_NEST_LIMIT` |

## Entry points
- `document.rs`: `Document::new`, `edit`, `move_carets`, `undo`, `redo`, `snapshot`.
- `search.rs`: `Matcher` and the find and replace functions.

## Dependencies
- Internal: `va-core` (`source::MAX_SOURCE_BYTES`; `LineIndex` is the reference line count).
- External: `ropey` (`text`), `regex` (`search`).

## Tests / usage
```bash
cargo test -p va-editor
```
Tests live in `crates/va-editor/tests/`, never in `src/` (architecture §8); see its README.
