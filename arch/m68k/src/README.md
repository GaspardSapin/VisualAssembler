# arch/m68k/src

## Purpose
Sources of `va-arch-m68k`, the Motorola 68000 support. In M1: the instruction tables, the
A68k-dialect lexer, and the glue that presents them, the register file, the diagnostic codes and
the embedded documentation as one `va_core::arch::Architecture`.

**M2 (in progress):** the WP0 scaffold of the assembler (`asm/`) and the disassembler (`disasm/`)
(commit `c500c74`) is being implemented in wave 2 (m2-assembler.md §1). Done: the assembler's
front end (WP-B), its encoder and the placement table `isa/fields.rs` (WP-C), its 50 diagnostic
codes with their messages and the directive documentation (WP-F), the S-record, binary and
listing outputs (`asm/srec.rs`, `asm/listing.rs`, WP-D) and the disassembler (WP-E). Still a
`todo!()` stub: the rest of the assembler's driver (`asm/mod.rs` and the doc-only modules, WP-D).
The emulator adapter and the I/O profiles are not implemented yet (M3).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate doc; declares the public `asm`, `disasm`, `isa` and `syntax` modules; re-exports `M68k` and its instance `M68K` |
| `arch.rs` | module | `M68k`, the `Architecture` implementation: id `m68k`, `syntax()`, `instruction_set()`, `registers()`, `diagnostic_codes()` (the 11 lexer codes, then the 50 of `asm::codes::ASM_DIAG_CODES`, joined at compile time), `locale_resources("en" \| "fr")` (three resources per language), `assembler()` and `disassembler()` (M2, `Some` of the types of `asm/` and `disasm/`; `assemble` still panics) |
| `registers.rs` | module | The register file as `RegisterSpec`s in panel order (D0–D7, A0–A7, USP, SSP, PC, SR), stable `RegId`s, the SR fields |
| `locales.rs` | module | `include_str!` of `../locales/{en,fr}/{isa,diagnostics,directives}.ftl`, in that order (allowed by m0 §17.4 S8); never parses them |
| `isa/` | folder | Instruction tables, encodings, cycle counts and the `M68kIsa` view. See its README. |
| `syntax/` | folder | The A68k-dialect lexer, its keyword list (with the directives' doc ids) and its diagnostic codes. See its README. |
| `asm/` | folder | The strict-A68k assembler (M2): front end and encoder implemented (WP-B, WP-C), diagnostic codes (WP-F), S-record and listing outputs (WP-D); the rest of the driver is a WP0 scaffold, implemented in wave 2 by WP-D. See its README. |
| `disasm/` | folder | The disassembler (M2, implemented by WP-E): opword table, decoding, canonical text, flow and targets. See its README. |

## Entry points
- `lib.rs` and `arch.rs`: `M68K` is what the apps register (`apps/va/src/arch_registry.rs`).
- `syntax::lex_line` (the lexer) and `isa::M68kIsa::describe` (what `va isa` prints).
- `disasm::M68kDisassembler::decode` (M2, WP-E) and `asm::assemble` (M2; a `todo!()` stub until
  WP-D). The frozen front-end and encoder functions and the outputs under `asm/` already work on
  their own.

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
