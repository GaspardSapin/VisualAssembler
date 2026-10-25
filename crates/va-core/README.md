# crates/va-core

## Purpose
Package `va-core`. The CPU-agnostic contracts every other crate builds on (architecture §3,
finalized by m1-core-isa-lexer.md §3): source texts and line splitting, diagnostics, the lexer
contract, the instruction view, register descriptions, program image and source map, and the
`Architecture` trait. No I/O, no threads, no UI, no dependency, **no user-facing text**: error enums
have no `Display`, and `name()`/`id()` methods return identifiers, never prose.

**M1 (implemented, frozen):** `source`, `diag`, `syntax`, `isa`, `regs`, `image` and `arch` with its
M1 methods. Frozen means implementations may add items but never remove or rename a public one; a
change goes back to the Architect. **M2 (implemented, frozen; WP-A, commit `e5e9221`):** `asm` (the
`Assembler` trait, `SourceProvider`, options, limits, output, `CancelToken`, `MemorySources`),
`disasm` (the `Disassembler` trait, `DecodedInsn`, `FlowKind`) and the provided methods
`Architecture::assembler`/`disassembler`, which default to `None` (m2-assembler.md §3).
**Not defined yet:** the M3 contracts of m1 §3.7 (emulator, I/O profiles).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Package template of m0-foundations.md §3.6; no dependency |
| `src/` | folder | One module per contract. See its README. |
| `tests/` | folder | One test file per module. See its README. |

## Entry points
- `src/arch.rs` (`Architecture`), then `src/syntax.rs` (`SyntaxProvider`) and `src/isa.rs`
  (`InstructionSet`): the three traits a CPU crate implements in M1.
- `src/asm.rs` (`Assembler`, `SourceProvider`) and `src/disasm.rs` (`Disassembler`): what a CPU
  crate offers from M2, through `Architecture::assembler`/`disassembler`.

## Dependencies
- Internal (allowed): none.
- External: none. Every addition goes through the Architect, an exact pin in the root
  `[workspace.dependencies]`, `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va-core
```
