# crates/va-core

## Purpose
Package `va-core`. CPU-agnostic traits and data types (architecture §3): `Architecture`, `SyntaxProvider`, `Assembler`, `Disassembler`, `EmulatorFactory`, `Emulator`, `IoProfile`, diagnostics and source mapping. No I/O, no threads, no UI, no user-facing text.

**M0 skeleton:** no logic yet; the first content arrives in M1.

Every library and arch crate will depend on it, so its API changes go through an Architect spec.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Package template of m0-foundations.md §3.6: workspace version, edition, licence and lints; no dependency in M0 |
| `src/` | folder | Crate sources. See its README. |

## Entry points
- `src/lib.rs`: the crate documentation, which states the responsibility and the allowed internal dependencies.

## Dependencies
- Internal (allowed): none. Declared only when first used (m0-foundations.md §3.6).
- External: none in M0. Planned: none listed yet. Every addition goes through the Architect, an exact
  pin in the root `[workspace.dependencies]`, `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va-core
```
In M0 this runs no test. The `tests/` folder appears with the first test (M1).
