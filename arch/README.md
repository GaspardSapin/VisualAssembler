# arch

## Purpose
One crate per CPU, each named `va-arch-<cpu>` and depending only on `va-core`. A new CPU is a new
folder here plus one registration line per app; nothing in `crates/` changes (architecture §10.1).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `m68k/` | folder | `va-arch-m68k`, Motorola 68000 support. In M1: instruction tables, A68k-dialect lexer, registers, EN/FR docs. See its README. |

Candidate future folders: `z80/`, `mos6502/`, `riscv/`, `arm/`, `x86/`.

## Entry points
- `m68k/src/lib.rs` (the `M68K` architecture).

Adding a CPU (architecture §10.1):
1. Create `arch/<cpu>/` (package `va-arch-<cpu>`, lower-case folder) implementing `Architecture` and
   its traits, with resources in `arch/<cpu>/locales/`, golden files in `arch/<cpu>/fixtures/` and
   tests in `arch/<cpu>/tests/`.
2. Add it to the root `[workspace].members`. The layering rule already covers every `va-arch-*`.
3. Add an optional `arch-<cpu>` feature and one line in each app's `src/arch_registry.rs`
   (`apps/va/src/arch_registry.rs` exists since M1; the GUI's arrives in M4).
4. Never special-case a CPU id in core or UI code; a missing capability is a `va-core` change
   through an Architect spec.

## Dependencies
- Internal: `va-core` only (rule R2 forbids the reverse edge).
- External: per CPU; see `DEPENDANCE.md`.

## Tests / usage
```bash
cargo test -p va-arch-m68k
```
