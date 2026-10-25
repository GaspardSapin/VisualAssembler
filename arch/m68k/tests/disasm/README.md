# arch/m68k/tests/disasm

## Purpose
Test target `disasm`: the tests of the disassembler in `arch/m68k/src/disasm/`
(m2-assembler.md §10.1, exit criterion E4): golden decodes per mode and family, `Truncated` and
`Illegal`, all 65,536 opwords (no panic, valid exactly when some form matches), the canonical
text and its tokens, and the flow kinds and targets.

**WP0 scaffold (wave 1b, commit `c500c74`); written in wave 2 by WP-E.** `main.rs` declares the
modules; each module file holds only its `//!` line and `#![allow(clippy::unwrap_used)]`, so the
target compiles and runs no test yet.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `main.rs` | test root | Declares `decode`, `flow` and `format`. Stays as WP0 wrote it (m2 §1): WP-E owns the module files and keeps its helpers there |
| `decode.rs` | test (empty) | Golden decodes per mode and family, `Truncated` and `Illegal`, all 65,536 words (§5.1) |
| `format.rs` | test (empty) | Spellings, sizes, number notation, register lists, and `tokens` covering `text` (§5.2) |
| `flow.rs` | test (empty) | Flow kinds (branches, jumps, calls with their return address, returns, traps) and the branch, jump, call and PC-relative targets (§5.1) |

## Entry points
- `main.rs`: the module list.

The golden files (`arch/m68k/fixtures/disasm/`) and the asm → bin → disasm → asm round trip
(`arch/m68k/tests/roundtrip.rs`, m2 §10.3) arrive with WP-E; neither exists yet.

## Dependencies
- Internal: `va-arch-m68k` (`disasm`, `isa`), `va-core` (`disasm`, `syntax`).
- External: none.

## Tests / usage
```bash
cargo test -p va-arch-m68k --test disasm
```
In the scaffold this compiles the target and runs 0 tests.
