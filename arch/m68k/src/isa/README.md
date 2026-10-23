# arch/m68k/src/isa

## Purpose
The 68000 instruction tables (m1-core-isa-lexer.md §4): every accepted spelling, its forms
(operands, sizes, addressing modes, flag effects, privilege), opword encodings and cycle counts,
plus the CPU-agnostic view `M68kIsa` that `va isa` (M1) and hover (M5) read. The tables are
hand-written `static`/`const` Rust data checked by `arch/m68k/tests/isa/` (§4.2). This folder holds
no prose: documentation text lives in `arch/m68k/locales/`. It does not lex, assemble or execute.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | Catalogue: `INSTRUCTIONS` (74 entries), `SPELLINGS` (126, the A68k 2.71.F3w opcode table), `spelling_info` and `instruction_of` (ASCII case-insensitive, no allocation) |
| `model.rs` | module | Frozen data model (§4.3): `Size`, `SizeSet`, `Operand`, `QuickRange`, `Flags`, `Form`, `Family`, `Instruction`, `SpellingInfo` |
| `modes.rs` | module | `Mode` (12 effective-address modes, Motorola order) and `ModeSet` with the Motorola categories (data, memory, control, alterable) |
| `conditions.rs` | module | The 16 condition codes of Bcc, DBcc and Scc, with their flag tests |
| `encoding.rs` | module | `Encoding` (fixed bits, size and EA fields, extra rule) and `Form::matches` over a 16-bit opword |
| `timing.rs` | module | `TimingRow`, `Outcome`, `Unit`, `EA_TIME`, the MOVE matrices `MOVE_BW`/`MOVE_L`, and `cycles()` |
| `view.rs` | module | `M68kIsa`, the `va_core::isa::InstructionSet` implementation (`describe`, `spellings`, `ea_timing`) |
| `table_arith.rs` | table | ADD, SUB, CMP families, MULS/MULU, DIVS/DIVU, NEG/NEGX, CLR, EXT, TST, ABCD/SBCD/NBCD, CHK |
| `table_logic.rs` | table | AND, OR, EOR families (with CCR and SR forms), NOT, shifts and rotates, bit operations, TAS, Scc |
| `table_move.rs` | table | MOVE (with CCR, SR and USP forms), MOVEA, MOVEM, MOVEP, MOVEQ, EXG, SWAP, LEA, PEA, LINK, UNLK |
| `table_flow.rs` | table | Bcc, BRA, BSR, DBcc, JMP, JSR, RTE, RTR, RTS, TRAP, TRAPV, NOP, RESET, STOP, ILLEGAL |

## Entry points
- `mod.rs`: `INSTRUCTIONS`, `SPELLINGS`, `spelling_info` (used by the lexer for `takes_operands`).
- `view.rs`: `M68kIsa::describe(spelling, size)`, what `va isa` prints.
- `timing.rs`: `cycles(form, size, modes, outcome, n)`.

**Provisional values (m1 §12.3).** The cycle counts and the pinned count of valid opwords,
`VALID_OPWORDS = 45_816` in `arch/m68k/tests/isa/encoding.rs`, were typed without the manual at
hand. They stay as they are and are **provisional until the M3 cross-check**. This applies in
particular to TRAPV trap 34(5/3), CHK trap 40(4/3)+ea, TAS memory 10(1/1)+ea, ANDI.L #,Dn 14(3/0),
ANDI.L #,memory 20(3/2) (some printings show 20(3/1)), RESET 132(1/0) and the MOVEM per-mode
bases. In M3, `cycles()` and the valid-opword set are checked against the `m68k` crate and the
SingleStepTests 680x0 vectors. Each disagreement is then settled from the manual or the vectors,
and the pinned count is updated with a recorded reason. These values are the **only** source of
cycle numbers: the `.ftl` documentation never repeats a count that these tables give (§12.1 R12-3).

**Factual sources and legal rule (m1 §4.6).**

| Source | Used for |
|---|---|
| Motorola, *M68000 Family Programmer's Reference Manual* (M68000PM/AD rev. 1, 1992) | Instruction list, operand sizes, addressing-mode categories, condition codes, X N Z V C effects, opword bit patterns |
| Motorola, *MC68000 8-/16-/32-Bit Microprocessors User's Manual* (M68000UM/AD rev. 8), §8 | Clock periods and bus read/write counts (tables 8-1 to 8-14) |
| J. Cwik, *68000 Yacht* (cycle analysis) | Cross-check of the timing footnotes only |
| `a68k.exe` 2.71.F3w, run locally on 2026-10-17 (read-only, outputs in a scratch folder) | Spellings, directives, aliases, size defaults, dialect rules, the golden opwords of the tests |

Numbers, bit patterns and names are facts and may be reproduced. Every description (doc text,
comments) is written in our own words. No sentence, table caption or example is copied from the
manuals, from EASy68K's help or from another program's tables. Table values are typed from the
manuals, never copied from other software.

## Dependencies
- Internal: `va-core` (`isa` view types `Cycles`, `Bound`, `FlagEffect`, `InstructionSet`).
- External: none.

## Tests / usage
```bash
cargo test -p va-arch-m68k --test isa
cargo run -p va -- isa move.l
```
The first runs the consistency tests of §4.7 (exit criterion E4); see `arch/m68k/tests/isa/README.md`.
