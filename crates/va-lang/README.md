# crates/va-lang

## Purpose
Package `va-lang`. Language services over the assembler output: symbols, definition and references, outline, completion, hover, plus the background-assembly service (architecture §4, rule T3: debounce, cancellation, generation numbers). No UI.

**M0 skeleton:** no logic yet; the first content arrives in M5.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Package template of m0-foundations.md §3.6: workspace version, edition, licence and lints; no dependency in M0 |
| `src/` | folder | Crate sources. See its README. |

## Entry points
- `src/lib.rs`: the crate documentation, which states the responsibility and the allowed internal dependencies.

## Dependencies
- Internal (allowed): `va-core`, `va-editor`. Declared only when first used (m0-foundations.md §3.6).
- External: none in M0. Planned: none listed yet. Every addition goes through the Architect, an exact
  pin in the root `[workspace.dependencies]`, `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va-lang
```
In M0 this runs no test. The `tests/` folder appears with the first test (M5).
