# crates/va-debug

## Purpose
Package `va-debug`. Debug session: a worker thread that owns a `dyn Emulator`, command and event channels, line-to-address breakpoints, step over and out, run budget, undo, change tracking (architecture §4, rule T4). No UI.

**M0 skeleton:** no logic yet; the first content arrives in M3.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Package template of m0-foundations.md §3.6: workspace version, edition, licence and lints; no dependency in M0 |
| `src/` | folder | Crate sources. See its README. |

## Entry points
- `src/lib.rs`: the crate documentation, which states the responsibility and the allowed internal dependencies.

## Dependencies
- Internal (allowed): `va-core`. Declared only when first used (m0-foundations.md §3.6).
- External: none in M0. Planned: none listed yet. Every addition goes through the Architect, an exact
  pin in the root `[workspace.dependencies]`, `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va-debug
```
In M0 this runs no test. The `tests/` folder appears with the first test (M3).
