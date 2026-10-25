# crates/va-editor/src

## Purpose
Sources of `va-editor` (m4-editor-shell.md §3). **WP0 scaffold (commit `b4e7aae`); implemented in
wave 2 by WP-A.** Every module is inline in `lib.rs` and holds the frozen signatures of §3 with
`todo!()` bodies, each under `#![expect(unused_variables, clippy::todo, …, reason = "WP0 stub: …")]`;
`history` holds only constants and `EditKind`, which are final.
WP-A moves each module into its own file (`text.rs`, `encoding.rs`, …) as it implements it, so
that `lib.rs` ends with the crate doc and seven `pub mod` lines; the module paths
(`va_editor::text::Text`, …) never change (m4 §15.1 M1). A listed lint that stops firing raises
`unfulfilled_lint_expectations`, which CI denies: each lint goes in the commit that makes it stop
firing, and the attribute with the last `todo!()` of its module (m4 §15.1 M3).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate doc and seven inline modules: `text` (§3.1: `Text` over a `ropey::Rope` whose breaks are LF, CRLF and lone CR, `MAX_DOCUMENT_BYTES` = 16 MiB, display columns), `encoding` (§3.1: BOM, UTF-8 and Windows-1252 decoding, line endings), `selection` (§3.2: carets and selections), `edit` (§3.2: changes, edit commands, motions), `history` (§3.3: undo grouping limits and edit kinds), `document` (§3.3: text, selections, history, dirty state, snapshots), `search` (§3.4: literal and regex find/replace, line by line, with a match cap) |

## Entry points
- `lib.rs`: `document::Document` and `search`.

## Dependencies
- Internal: `va-core` (`source::MAX_SOURCE_BYTES`; `LineIndex` is the reference line count).
- External: `ropey` (`text`), `regex` (`search`).

## Tests / usage
```bash
cargo test -p va-editor
```
Tests will live in `crates/va-editor/tests/`, never in `src/` (architecture §8).
