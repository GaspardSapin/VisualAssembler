# crates/va-editor

## Purpose
Package `va-editor`. Text buffer (ropey), selections, grouped undo/redo, search and replace
(m4-editor-shell.md §3). No UI, no threads, no I/O beyond `std::io::Write` sinks. Offsets are bytes
into the document text, lines are 0-based, and time is passed in, so tests control it.

**M4 WP0 scaffold (wave 1, commit `b4e7aae`); implemented in wave 2 by WP-A.** `src/lib.rs` holds
the frozen types and signatures of m4 §3 as inline modules with `todo!()` bodies: any call panics.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Dependencies `ropey`, `regex`, `va-core` (all `workspace = true`, m4 §8.2) |
| `src/` | folder | Crate sources. See its README. |

The `tests/` folder arrives with WP-A: one file per module (`text.rs`, `encoding.rs`,
`selection.rs`, `edit.rs`, `history.rs`, `search.rs`, `document.rs`; m4 §10.1).

## Entry points
- `src/lib.rs`: `document::Document`, which the editor view (`va-ui`) and the shell edit.

## Dependencies
- Internal (allowed): `va-core`.
- External: `ropey` 1.6.1 (features `simd`, `cr_lines`) for the rope and its line index, `regex`
  1.13.1 (linear-time engine, a reduced set of Unicode tables) for find and replace. Both approved
  by the user on 2026-10-24 (m4 §8.3); see `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va-editor
```
In the scaffold this builds the crate and runs no test.
