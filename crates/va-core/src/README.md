# crates/va-core/src

## Purpose
Sources of `va-core`. In M0 the crate holds only its documentation comment; code arrives in M1.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate-level `//!` doc: responsibility, allowed internal dependencies (none), and the M0 skeleton note |

## Entry points
- `lib.rs`. Projected layout at about six months (m0-foundations.md §2.2): `src/{arch,syntax,asm,emu,diag,image,source}/`: traits and types grouped by contract.

## Dependencies
- Internal: none yet. Allowed: none.
- External: none in M0.

## Tests / usage
```bash
cargo test -p va-core
```
Tests will live in `crates/va-core/tests/`, never in `src/` (architecture §8).
