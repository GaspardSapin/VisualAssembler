# arch/m68k/tests/disasm

## Purpose
Test target `disasm`: the tests of the disassembler in `arch/m68k/src/disasm/`
(m2-assembler.md §10.1, exit criterion E4): golden decodes per mode and family, `Truncated` and
`Illegal`, all 65,536 opwords (no panic, valid exactly when some form matches), the canonical
text and its tokens, and the flow kinds and targets.

**Written in wave 2 by WP-E (commit `29e7fea`)** on the WP0 scaffold (wave 1b, commit `c500c74`):
`main.rs` declares the modules, and the three module files hold the tests and their shared
helpers.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `main.rs` | test root | Declares `decode`, `flow` and `format`. Stays as WP0 wrote it (m2 §1): WP-E owns the module files and keeps its helpers there |
| `decode.rs` | test | Golden decodes per mode and family, `Truncated` and `Illegal`, non-canonical words, far addresses, all 65,536 words (§5.1), the golden files `modes` and `illegal`; the shared helpers (`decode_words`, `candidates`/`matching`, the table oracle, `render` and `check_golden`, which defines the `.dis` format) |
| `format.rs` | test | Spellings and sizes, every condition and `dbra`, number notation, register lists, the 120 immediate opwords of §16.2, `tokens` equal to the lexer tokens of `text`, and every text reassembling to its words through the front end and the encoder, never `assemble` (rule G1; §5.2) |
| `flow.rs` | test | Flow kinds of every opword (branches, jumps, calls with their return address, returns, traps), the branch, jump, call, LEA/PEA and PC-relative targets (§5.1), and the golden file `flow` |

## Entry points
- `main.rs`: the module list.

The golden files are in `arch/m68k/fixtures/disasm/` (see its README); `VA_BLESS=1` rewrites
their `.dis` files. Not written yet: the asm → bin → disasm → asm round trip
(`arch/m68k/tests/roundtrip.rs`, m2 §10.3), which needs the driver `asm::assemble` (WP-D).

## Dependencies
- Internal: `va-arch-m68k` (`asm`: `parse_srec`, the front end and the encoder; `disasm`, `isa`,
  `syntax`), `va-core` (`disasm`, `image`, `syntax`).
- External: none.

## Tests / usage
```bash
cargo test -p va-arch-m68k --test disasm
```
On 2026-11-04 it ran 24 tests, all passing: `decode` 10, `flow` 7, `format` 7.
