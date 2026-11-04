# arch/m68k/src/asm

## Purpose
The strict-A68k 68000 assembler (m2-assembler.md §4): it turns a source and its INCLUDE/INCBIN
files into a `va_core::image::ProgramImage` byte-identical to `a68k -s` where a68k succeeds, plus
the diagnostics, the symbol table, the source map, the listing and the S-record or binary output.
`M68K.assembler()` returns an `M68kAssembler`, the `va_core::asm::Assembler` of this folder.

**M2 wave 2 in progress.** Implemented: the front end (WP-B, commit `8064d41`: `parser`,
`operand`, `expr`), the encoder (WP-C, commit `50ad40d`: `encode`, `optimize`, with the
placement table `isa::fields`) and the 50 diagnostic codes of `codes.rs` (WP-F, commit
`525d619`). **The driver (WP-D) is in progress** over the WP0 scaffold (wave 1b, commit
`c500c74`): `srec.rs` and `listing.rs` are implemented (commit `2151d08`, on the final signatures
of m2 §15.2 W3 and W5, commit `d0c68da`). Still the scaffold: `mod.rs` holds `assemble` with a
`todo!()` body, and `symbols`, `directives`, `macros` and `layout` hold only their `//!` doc.
Calling `assemble` (or `M68K.assembler()`'s `assemble`) panics until WP-D lands.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | `M68kAssembler` (the `Assembler` implementation) and `assemble(input, cancel)`, the pipeline of §4.1: pass 1 expands and places, relaxation shortens branches, pass 2 encodes. Stub, WP-D |
| `codes.rs` | module | One `DiagCode` constant per `asm-*` code of §7.2: 45 diagnostic codes and the five `asm-related-*` note codes, each documenting the Fluent arguments its message takes; `ASM_DIAG_CODES` (50), appended after the 11 lexer codes by `Architecture::diagnostic_codes` (§7.1). Their `diag-asm-*` messages are in `arch/m68k/locales/{en,fr}/diagnostics.ftl` (WP-F, m2 §15.2 W1). Severity and blocking are decided by the emitter (WP-D) |
| `parser.rs` | module | Front end (frozen §4.2): `parse_line` reads the label, operation and operand field of one line from the lexer tokens, without re-scanning the text; `OpField`, `Line`, and `Problem`, the line-relative problem that the front end and the encoder report to the driver. Lexer errors are not repeated; macro parameters outside the operand field give `asm-macro-param-outside` |
| `operand.rs` | module | Front end (frozen §4.2, §4.3): `Ea` (the 12 addressing modes as written), `Xn`, `Operand` (EA, register list, `CCR`/`SR`/`USP`, a lone string as bytes), `Spanned`, `parse_operand`; the rejections `asm-missing-displacement` and `asm-absolute-size-syntax` carry a fix |
| `expr.rs` | module | Front end (frozen §4.2, §4.4): `Expr`, `SymRef`, `UnOp`, `BinOp`, `Scope`, `Lookup`, `Value`, `parse_expr` and `eval`: 32-bit wrapping values, the a68k precedence (unary, then shifts, then `&` `!`, then `*` `/`, then `+` `-`), at most 32 nested parentheses and 256 tree levels |
| `encode.rs` | module | Encoder (frozen §4.2, §4.10): `Choice`, `select` (form selection, with the closest failure reported: `asm-size-not-allowed`, `asm-mode-not-allowed`, `asm-no-matching-form`, `asm-operand-count`), `length` (from the operand shapes only), `encode` (words through `isa::fields::places` and the M1 EA fields, with the §4.10 value ranges) |
| `optimize.rs` | module | Encoder (frozen §4.2, §4.10): `optimize`, the a68k rewrites `moveq`, `addq`, `subq`, `lea`, `addq-lea`, `subq-lea`, `zero-displacement`, `pc-relative` and `movem-move`, applied to the generic spelling only and returning the rule id; branch shortening (`short-branch`) is `layout`'s relaxation |
| `symbols.rs` | module | Driver (WP-D): the symbol table, local blocks, pass values and references (§4.5, §4.13). Doc only |
| `directives.rs` | module | Driver (WP-D): ORG, EQU, SET, DC, DS, DCB, EVEN, CNOP, INCLUDE, INCBIN, END and the refused directives (§4.5). Doc only |
| `macros.rs` | module | Driver (WP-D): MACRO/ENDM, the `\0`–`\9`, `\@` and NARG substitutions, MEXIT, IFxx/ENDC (§4.6, §4.7). Doc only |
| `layout.rs` | module | Driver (WP-D): location counter, alignment and the branch relaxation sweeps (§4.5, §4.10). Doc only |
| `listing.rs` | module | Driver (WP-D): `render_listing(listing, symbols, options, name)`, our own listing format (§4.12, LF): the header, one row per line with continuation rows up to 64 bytes, `----` file markers, the symbols and the counts, capped by `AsmLimits::max_listing_bytes` with a `listing truncated` marker (§15.2 W5) |
| `srec.rs` | module | Driver (WP-D): `to_srec` (byte-identical to `a68k -s`: S0, S2 records of 16 bytes, S8, CRLF, addresses masked to 24 bits), `parse_srec` (also read by the disassembler golden tests and, from WP-G, `va disasm`), `to_binary` (lowest address, gaps filled with zeros), `SrecError { line, reason: SrecReason }`, 9 reasons with kebab-case ids (§4.12, §15.2 W3) |

## Entry points
- `mod.rs`: `assemble`, reached by `M68K.assembler()` through `va_core::arch::Architecture`
  (WP-D).
- The frozen interfaces between the three groups (§4.2): `parser::parse_line`,
  `operand::parse_operand`, `expr::{parse_expr, eval}` (WP-B), `encode::{select, length, encode}`
  and `optimize::optimize` (WP-C), all implemented. WP-D drives them; none of them may be renamed
  or removed.

Rules for the wave-2 work packages:
- Each package owns its files only (m2 §1). The interfaces above are frozen; a change goes back
  to the Architect.
- The remaining stub module (`mod.rs`) starts with
  `#![expect(unused_variables, clippy::todo, reason = "WP0 stub: …")]`, listing exactly the lints
  its stubs trigger. A listed lint that stops firing raises `unfulfilled_lint_expectations`, which
  CI denies: the owner removes each lint in the commit that makes it stop firing, and the attribute
  with the last `todo!()`, never replacing it by `allow` (m2 §15.1 W6). A new file holding a stub
  carries its own expectation: the one of `mod.rs` would absorb its `todo!()`.
- No merged test may call another package's `todo!()` (m2 §15.3 G1): a test that needs another
  package's function merges after it. The WP-B and WP-C tests therefore build their inputs
  through the §4.2 types, and the WP-D output and listing tests build their images and listings
  by hand; none calls `assemble`.
- The golden fixtures arrive with WP-D in `arch/m68k/fixtures/{asm,asm-opt,asm-inc,asm-diag}/`
  (m2 §10.2); none exists yet.

## Dependencies
- Internal: `va-core` (`asm`, `diag`, `source`, `syntax`, `image`); `crate::isa` (the tables and
  `isa::fields`), `crate::syntax` (the M1 lexer, shared with the editor).
- External: none (m2 §9).

## Tests / usage
```bash
cargo test -p va-arch-m68k --test asm
```
The test target lives in `arch/m68k/tests/asm/` (see its README). On 2026-11-04 it ran 101 tests
(`parser`, `expr`, `encode`, `optimize`, `corpus`, `output`, `listing`), all passing; the other
five WP-D modules are still empty.
