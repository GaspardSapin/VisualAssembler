# crates/va-editor

## Purpose
Package `va-editor`. Text buffer (ropey), selections, grouped undo/redo, search and replace. No UI.

**M0 skeleton:** no logic yet; the first content arrives in M4.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Package template of m0-foundations.md §3.6: workspace version, edition, licence and lints; no dependency in M0 |
| `src/` | folder | Crate sources. See its README. |

## Entry points
- `src/lib.rs`: the crate documentation, which states the responsibility and the allowed internal dependencies.

## Dependencies
- Internal (allowed): `va-core`. Declared only when first used (m0-foundations.md §3.6).
- External: none in M0. Planned (m0-foundations.md §13.3): `ropey`, `regex`. Every addition goes through the Architect, an exact
  pin in the root `[workspace.dependencies]`, `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va-editor
```
In M0 this runs no test. The `tests/` folder appears with the first test (M4).
