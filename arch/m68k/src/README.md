# arch/m68k/src

## Purpose
Sources of `va-arch-m68k`, the Motorola 68000 support. In M1: the instruction tables, the
A68k-dialect lexer, and the glue that presents them, the register file, the diagnostic codes and
the embedded documentation as one `va_core::arch::Architecture`.

**M2 (in progress):** the WP0 scaffold of the assembler (`asm/`) and the disassembler (`disasm/`)
is in place (commit `c500c74`): the modules exist with their frozen signatures and `todo!()`
bodies, and are implemented in wave 2 by WP-B to WP-E (m2-assembler.md §1). The emulator adapter
and the I/O profiles are not implemented yet (M3).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate doc; declares the public `asm`, `disasm`, `isa` and `syntax` modules; re-exports `M68k` and its instance `M68K` |
| `arch.rs` | module | `M68k`, the `Architecture` implementation: id `m68k`, `syntax()`, `instruction_set()`, `registers()`, `diagnostic_codes()` (the lexer codes, then `asm::codes::ASM_DIAG_CODES`, joined at compile time), `locale_resources("en" \| "fr")`, `assembler()` and `disassembler()` (M2, `Some` of the scaffolded types) |
| `registers.rs` | module | The register file as `RegisterSpec`s in panel order (D0–D7, A0–A7, USP, SSP, PC, SR), stable `RegId`s, the SR fields |
| `locales.rs` | module | `include_str!` of `../locales/{en,fr}/{isa,diagnostics}.ftl` (allowed by m0 §17.4 S8); never parses them |
| `isa/` | folder | Instruction tables, encodings, cycle counts and the `M68kIsa` view. See its README. |
| `syntax/` | folder | The A68k-dialect lexer and its diagnostic codes. See its README. |
| `asm/` | folder | The strict-A68k assembler (M2): front end, encoder and driver. WP0 scaffold; implemented in wave 2 by WP-B, WP-C and WP-D. See its README. |
| `disasm/` | folder | The disassembler (M2). WP0 scaffold; implemented in wave 2 by WP-E. See its README. |

## Entry points
- `lib.rs` and `arch.rs`: `M68K` is what the apps register (`apps/va/src/arch_registry.rs`).
- `syntax::lex_line` (the lexer) and `isa::M68kIsa::describe` (what `va isa` prints).
- `asm::assemble` and `disasm::M68kDisassembler::decode` (M2; `todo!()` stubs until wave 2).

Planned layout (m1 §2.2): `emu/` and `io/` (I/O profiles `easy68k`, the default, and `d68k-tp`)
in M3.

## Dependencies
- Internal: `va-core`.
- External: none.

## Tests / usage
```bash
cargo test -p va-arch-m68k
```
Tests live in `arch/m68k/tests/`, mirroring this folder, and golden files in
`arch/m68k/fixtures/lex/` (the M2 fixture folders arrive in wave 2).
