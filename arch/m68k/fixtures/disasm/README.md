# arch/m68k/fixtures/disasm

## Purpose
Golden files of the 68000 disassembler (m2-assembler.md §10.2). Each `<name>.s68` is a Motorola
S-record input, read with `asm::srec::parse_srec`, and `<name>.dis` is the exact decode that
`M68K.disassembler()` must give for it. Every segment is decoded from its start, as `va disasm`
will (§8.3; the command is still a WP0 shell): an `Illegal` or `Truncated` word is written
`dc.w $XXXX` and decoding resumes 2 bytes later, and a trailing odd byte is `dc.b $XX`. Fixtures sit
in `arch/<cpu>/fixtures/<area>/` (depth 4), not under `tests/` (m1 §2.1). They are LF-only ASCII.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `modes.s68` / `modes.dis` | fixture | One 542-byte segment at `$1000`: every addressing mode as a source and as a destination, then the instruction families with their sizes, in the canonical text of §5.2 |
| `flow.s68` / `flow.dis` | fixture | One 228-byte segment at `$1000`: every flow kind (branches, jumps, calls with their return address, returns, traps) and the targets of branches, DBcc, JMP, JSR, LEA, PEA and PC-relative operands |
| `illegal.s68` / `illegal.dis` | fixture | Three segments: targets that wrap below `$000000`, unmatched opwords, empty MOVEM masks written as their words, non-canonical words read as the 68000 reads them, the immediate ADD/SUB/CMP/AND/OR forms of §16.2, a target above `$FFFFFF`, and instructions cut by the end of a segment |

`.dis` line format: one `segment $AAAAAA <n> bytes` line per segment, then one line per item,
`AAAAAA  <words, padded to 24>  <text, padded to 32>  ; <note>`. The note is the flow id
(`sequential`, `branch`, `jump`, `call`, `return`, `trap`), with ` return=$XXXXXXXX` for a call and
` target=$XXXXXXXX` when there is a target, or `illegal`, or `truncated needed=<n>`. The format is
defined in the module docs of `arch/m68k/tests/disasm/decode.rs`. Two lines of `flow.dis`:
```text
segment $001000 228 bytes
001008  6102                      bsr.s $0000100C                   ; call return=$0000100A target=$0000100C
```

## Entry points
- `modes.dis` for the canonical text; `illegal.dis` for the error and edge cases.
- The comparison is `check_golden` in `arch/m68k/tests/disasm/decode.rs`.

## Dependencies
- Internal: read by `arch/m68k/tests/disasm/` (`decode.rs` for `modes` and `illegal`, `flow.rs`
  for `flow`).
- External: none.

## Tests / usage
```bash
cargo test -p va-arch-m68k --test disasm
```
After a deliberate decoder or format change, `VA_BLESS=1 cargo test -p va-arch-m68k --test disasm`
rewrites the `.dis` files (never the `.s68` inputs). Review the diff in git before committing.
