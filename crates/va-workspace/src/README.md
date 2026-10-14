# crates/va-workspace/src

## Purpose
Sources of `va-workspace`. In M0 the crate holds only its documentation comment; code arrives in M4.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate-level `//!` doc: responsibility, allowed internal dependencies (`va-core`), and the M0 skeleton note |

## Entry points
- `lib.rs`. Projected layout at about six months (m0-foundations.md §2.2): `src/{project,settings,keybindings,session,backup,i18n,watch}/`.

## Dependencies
- Internal: none yet. Allowed: `va-core`.
- External: none in M0.

## Tests / usage
```bash
cargo test -p va-workspace
```
Tests will live in `crates/va-workspace/tests/`, never in `src/` (architecture §8).
