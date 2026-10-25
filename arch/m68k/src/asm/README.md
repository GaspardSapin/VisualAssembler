# arch/m68k/src/asm

## Purpose
The strict-A68k 68000 assembler (m2-assembler.md §4): it turns a source and its INCLUDE/INCBIN
files into a `va_core::image::ProgramImage` byte-identical to `a68k -s` where a68k succeeds, plus
the diagnostics, the symbol table, the source map, the listing and the S-record or binary output.
`M68K.assembler()` returns an `M68kAssembler`, the `va_core::asm::Assembler` of this folder.

**WP0 scaffold (wave 1b, commit `c500c74`); implemented in wave 2 by WP-B, WP-C, WP-D and WP-F.**
Every module exists: those whose signatures §4.2 or §4.12 freeze hold them with `todo!()` bodies,
the others hold only their `//!` doc. Calling `assemble` panics until WP-D lands. Only the data is
final: the types of `parser`, `operand`, `expr` and `encode`, and the diagnostic codes of
`codes.rs`, whose list `ASM_DIAG_CODES` stays empty until WP-F's codes commit (m2 §15.2 W1).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | `M68kAssembler` (the `Assembler` implementation) and `assemble(input, cancel)`, the pipeline of §4.1: pass 1 expands and places, relaxation shortens branches, pass 2 encodes. Stub, WP-D |
| `codes.rs` | module | One `DiagCode` constant per `asm-*` code of §7.2: 45 diagnostic codes and the five `asm-related-*` note codes; `ASM_DIAG_CODES`, appended after the lexer codes by `Architecture::diagnostic_codes` (§7.1). Constants final. Owned by WP-F in wave 2 (m2 §15.2 W1): one commit fills the array with the 50 codes and adds their `diag-asm-*` messages in `en` and `fr`; WP-D only reads the file |
| `parser.rs` | module | Front end (WP-B, frozen §4.2): `parse_line` reads the label, operation and operand field of one line from the lexer tokens; `OpField`, `Line`, and `Problem`, the line-relative problem that the front end and the encoder report to the driver. Stub |
| `operand.rs` | module | Front end (WP-B, frozen §4.2): `Ea` (the 12 addressing modes as written), `Xn`, `Operand`, `Spanned`, `parse_operand` (§4.3). Stub |
| `expr.rs` | module | Front end (WP-B, frozen §4.2): `Expr`, `SymRef`, `UnOp`, `BinOp`, `Scope`, `Lookup`, `Value`, `parse_expr` and `eval` (32-bit values, the a68k precedence of §4.4). Stub |
| `encode.rs` | module | Encoder (WP-C, frozen §4.2): `Choice`, `select` (form selection), `length`, `encode` (words through `isa::fields::places`), §4.10. Stub |
| `optimize.rs` | module | Encoder (WP-C, frozen §4.2): `optimize`, the a68k rewrites of §4.10 (`addq`, `moveq`, `lea`, `pc-relative`, …), applied to the generic spelling only. Stub |
| `symbols.rs` | module | Driver (WP-D): the symbol table, local blocks, pass values and references (§4.5, §4.13). Doc only |
| `directives.rs` | module | Driver (WP-D): ORG, EQU, SET, DC, DS, DCB, EVEN, CNOP, INCLUDE, INCBIN, END and the refused directives (§4.5). Doc only |
| `macros.rs` | module | Driver (WP-D): MACRO/ENDM, the `\0`–`\9`, `\@` and NARG substitutions, MEXIT, IFxx/ENDC (§4.6, §4.7). Doc only |
| `layout.rs` | module | Driver (WP-D): location counter, alignment and the branch relaxation sweeps (§4.5, §4.10). Doc only |
| `listing.rs` | module | Driver (WP-D): `render_listing`, our own listing format (§4.12). Stub. Decided, not implemented yet (m2 §15.2 W5): the signature gains `options: &AsmOptions` (header and size cap) |
| `srec.rs` | module | Driver (WP-D): `to_srec` (byte-identical to `a68k -s`, CRLF), `parse_srec` (also used by `va disasm`), `to_binary`, `SrecError` (§4.12). Stub. Decided, not implemented yet (m2 §15.2 W3): `SrecError { line, reason: SrecReason }`, an enum of 9 reasons with kebab-case ids, replacing the `&'static str` |

## Entry points
- `mod.rs`: `assemble`, reached by `M68K.assembler()` through `va_core::arch::Architecture`.
- The frozen interfaces between the three groups (§4.2): `parser::parse_line`,
  `operand::parse_operand`, `expr::{parse_expr, eval}` (WP-B), `encode::{select, length, encode}`
  and `optimize::optimize` (WP-C). WP-D drives them; none of them may be renamed or removed.

Rules for the wave-2 work packages:
- Each package owns its files only (m2 §1). The interfaces above are frozen; a change goes back
  to the Architect.
- Stub modules start with `#![expect(unused_variables, clippy::todo, reason = "WP0 stub: …")]`
  (`encode` also has `#[expect(clippy::ptr_arg)]`), listing exactly the lints the stubs trigger.
  A listed lint that stops firing raises `unfulfilled_lint_expectations`, which CI denies: the
  owner removes each lint in the commit that makes it stop firing, and the attribute with the last
  `todo!()`, never replacing it by `allow` (m2 §15.1 W6). A new file holding a stub carries its own
  expectation: the one of `mod.rs` would absorb its `todo!()`.
- No merged test may call another package's `todo!()` (m2 §15.3 G1): a test that needs another
  package's function merges after it.
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
The test target lives in `arch/m68k/tests/asm/` (see its README). In the scaffold it compiles and
runs 0 tests.
