# arch/m68k/src/disasm

## Purpose
The 68000 disassembler (m2-assembler.md §5), built on the same instruction tables as the
assembler: it decodes the instruction at an address into a `va_core::disasm::DecodedInsn` (its
length, canonical text, lexer tokens, flow kind and target), or reports `Truncated` or `Illegal`.
The text reassembles to the same bytes with `optimize = false` (§5.2). `M68K.disassembler()`
returns an `M68kDisassembler`; `va disasm` (M2) and the debugger (M3, M6) use it.

**WP0 scaffold (wave 1b, commit `c500c74`); implemented in wave 2 by WP-E.** `decode` has a
`todo!()` body and panics when called; `flow_kind` is final (it returns `insn.flow`).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | `M68kDisassembler`, the `va_core::disasm::Disassembler` implementation: `decode` (stub) and `flow_kind` |
| `decode.rs` | module | The lazily built 65,536-entry opword table over `Form::matches`, the extension words read through `isa::fields::places`, flow kind and target (§5.1). Doc only |
| `format.rs` | module | The canonical text (lowercase spelling and size, operands joined by `,`, the §5.2 number notation) and its tokens. Doc only |

## Entry points
- `mod.rs`: `M68kDisassembler::decode(bytes, address)`.

`mod.rs` carries `#![expect(unused_variables, clippy::todo, reason = "WP0 stub: …")]`. A listed lint
that stops firing raises `unfulfilled_lint_expectations`, which CI denies, so WP-E removes each lint
in the commit that makes it stop firing and the attribute with the last `todo!()`, never replacing
it by `allow`; new stub files carry their own expectation (m2 §15.1 W6).
The golden files arrive with WP-E in `arch/m68k/fixtures/disasm/` and the round trip in
`arch/m68k/tests/roundtrip.rs` (m2 §10.2, §10.3); neither exists yet.

## Dependencies
- Internal: `va-core` (`disasm`, `syntax`); `crate::isa` (forms, encodings, `isa::fields`),
  `crate::syntax` (tokens of the text).
- External: none.

## Tests / usage
```bash
cargo test -p va-arch-m68k --test disasm
```
The test target lives in `arch/m68k/tests/disasm/` (see its README). In the scaffold it compiles
and runs 0 tests.
