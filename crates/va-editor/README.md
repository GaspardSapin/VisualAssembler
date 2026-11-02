# crates/va-editor

## Purpose
Package `va-editor`. Text buffer (ropey), selections, grouped undo/redo, search and replace
(m4-editor-shell.md §3). No UI, no threads, no I/O beyond `std::io::Write` sinks. Offsets are bytes
into the document text, lines are 0-based, and time is passed in, so tests control it.

**M4 (implemented in wave 2 by WP-A, commit `7324608`).** The frozen types and signatures of
m4 §3 (WP0 scaffold, commit `b4e7aae`) now have their bodies, one file per module; no `todo!()`
is left. Nothing calls the crate yet: the editor view and the shell of `va-ui` arrive with WP-E
and WP-F.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Dependencies `ropey`, `regex`, `va-core` (all `workspace = true`, m4 §8.2) |
| `src/` | folder | Crate sources, one file per module. See its README. |
| `tests/` | folder | One test file per module (m4 §10.1, exit criterion E3). See its README. |

## Entry points
- `src/document.rs`: `document::Document`, which the editor view (`va-ui`) and the shell edit.
- `src/search.rs`: `search::find_all`, `find_from` and `replace_all`, meant for the shell's
  `va-search` worker, never the UI thread (m4 §3.4).

## Dependencies
- Internal (allowed): `va-core`.
- External: `ropey` 1.6.1 (features `simd`, `cr_lines`) for the rope and its line index, `regex`
  1.13.1 (linear-time engine, a reduced set of Unicode tables) for find and replace. Both approved
  by the user on 2026-10-24 (m4 §8.3); see `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va-editor
```
On 2026-11-02 this ran 102 tests in seven targets, all passing (see `tests/README.md`).
