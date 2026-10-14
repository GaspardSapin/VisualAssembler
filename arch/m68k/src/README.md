# arch/m68k/src

## Purpose
Sources of `va-arch-m68k`, the Motorola 68000 support. In M0 the crate holds only its documentation
comment; code arrives in M1.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate-level `//!` doc: responsibility, allowed internal dependency (`va-core`), and the M0 skeleton note |

## Entry points
- `lib.rs`. Planned internal layout (architecture §2, m0-foundations.md §2.2):
  `isa/` (instruction tables), `syntax/` (lexer shared by the highlighter and the assembler),
  `asm/` (A68k-dialect assembler), `disasm/`, `emu/` (adapter over the `m68k` crate behind
  `Emulator`), `io/` (I/O profiles `easy68k`, the default, and `d68k-tp`).

## Dependencies
- Internal: none yet. Allowed: `va-core`.
- External: none in M0.

## Tests / usage
```bash
cargo test -p va-arch-m68k
```
Tests will live in `arch/m68k/tests/`, mirroring `src/`, with golden files in `tests/fixtures/`.
