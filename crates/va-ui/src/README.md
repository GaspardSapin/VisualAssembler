# crates/va-ui/src

## Purpose
Sources of `va-ui`. In M0 the crate holds only its documentation comment; code arrives in M4.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate-level `//!` doc: responsibility, allowed internal dependencies (`va-core`, `va-editor`, `va-lang`, `va-debug`, `va-workspace`), and the M0 skeleton note |

## Entry points
- `lib.rs`. Projected layout at about six months (m0-foundations.md §2.2): `src/{editor,gutter,tabs,explorer,palette,problems,debug,theme}/`.

## Dependencies
- Internal: none yet. Allowed: `va-core`, `va-editor`, `va-lang`, `va-debug`, `va-workspace`.
- External: none in M0.

## Tests / usage
```bash
cargo test -p va-ui
```
Tests will live in `crates/va-ui/tests/`, never in `src/` (architecture §8).
