# arch/m68k/src/disasm

## Purpose
The 68000 disassembler (m2-assembler.md §5), built on the same instruction tables as the
assembler: it decodes the instruction at an address into a `va_core::disasm::DecodedInsn` (its
length, canonical text, lexer tokens, flow kind and target), or reports `Truncated` or `Illegal`.
The text reassembles to the same bytes with `optimize = false` (§5.2). `M68K.disassembler()`
returns an `M68kDisassembler`; `va disasm` (M2) and the debugger (M3, M6) use it.

**Implemented in wave 2 by WP-E (commit `29e7fea`, completed by m2 §16.2)** over the WP0 scaffold
(wave 1b, commit `c500c74`); no `todo!()` is left. `decode` never panics; `flow_kind` returns
`insn.flow`. `va disasm`, its first caller, is still a WP0 shell until WP-G.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | `M68kDisassembler`, the `va_core::disasm::Disassembler` implementation: `decode` (delegates to `decode.rs`) and `flow_kind` |
| `decode.rs` | module | The lazily built 65,536-entry opword table over `Form::matches` (`form_of`), lengths from the opword alone, the extension words read through `isa::fields::places`, `Truncated`/`Illegal`, flow kind and target (§5.1) |
| `format.rs` | module | The canonical text (lowercase spelling and size, operands joined by `,`, the §5.2 number notation, canonical register lists, `dc.w` for an empty MOVEM mask) and its lexer tokens |

## Entry points
- `mod.rs`: `M68kDisassembler::decode(bytes, address)`.

The text reassembles to the same bytes except for the §16.2 cases: the 120 immediate opwords of
ADD, SUB, CMP, AND and OR `<ea>,Dn` (they reassemble to ADDI, SUBI, CMPI, ANDI and ORI) and targets
outside `$000000`–`$FFFFFF`. The golden files are in `arch/m68k/fixtures/disasm/` (m2 §10.2). Not
written yet: the round trip `arch/m68k/tests/roundtrip.rs` (m2 §10.3), which needs the driver
`asm::assemble` (WP-D).

## Dependencies
- Internal: `va-core` (`disasm`, `syntax`); `crate::isa` (forms, encodings, `isa::fields`),
  `crate::syntax` (tokens of the text).
- External: none.

## Tests / usage
```bash
cargo test -p va-arch-m68k --test disasm
```
The test target lives in `arch/m68k/tests/disasm/` (see its README). On 2026-11-04 it ran 24
tests, all passing.
