# crates/va-editor/src

## Purpose
Sources of `va-editor`. In M0 the crate holds only its documentation comment; code arrives in M4.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate-level `//!` doc: responsibility, allowed internal dependencies (`va-core`), and the M0 skeleton note |

## Entry points
- `lib.rs`. Projected layout at about six months (m0-foundations.md §2.2): `src/{buffer,selection,history,search}/`: rope buffer, multi-cursor, undo groups, regex search.

## Dependencies
- Internal: none yet. Allowed: `va-core`.
- External: none in M0.

## Tests / usage
```bash
cargo test -p va-editor
```
Tests will live in `crates/va-editor/tests/`, never in `src/` (architecture §8).
