# crates/va-lang/src

## Purpose
Sources of `va-lang`. In M0 the crate holds only its documentation comment; code arrives in M5.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate-level `//!` doc: responsibility, allowed internal dependencies (`va-core`, `va-editor`), and the M0 skeleton note |

## Entry points
- `lib.rs`. Projected layout at about six months (m0-foundations.md §2.2): `src/{analysis,symbols,navigation,completion,hover,outline}/`.

## Dependencies
- Internal: none yet. Allowed: `va-core`, `va-editor`.
- External: none in M0.

## Tests / usage
```bash
cargo test -p va-lang
```
Tests will live in `crates/va-lang/tests/`, never in `src/` (architecture §8).
