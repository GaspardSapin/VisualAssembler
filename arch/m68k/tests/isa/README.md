# arch/m68k/tests/isa

## Purpose
Test target `isa`: the consistency tests of the 68000 instruction tables in `arch/m68k/src/isa/`
(m1-core-isa-lexer.md §4.7, exit criterion E4). They check the tables against their own rules,
against facts from the manuals, and against opwords and sizes observed with a68k 2.71.F3w. They are
not an emulator test: that cross-check against the `m68k` crate arrives in M3.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `main.rs` | test root | Declares the modules; helpers `all_forms`, `operands`, `form_of` |
| `spellings.rs` | test | `SPELLINGS` equals the §4.1 list; family expansion plus aliases give exactly it; every spelling resolves; alias and `also_accepts` targets exist; each condition appears once per family |
| `forms.rs` | test | Default size within the sizes, unsized forms, operand counts, addressing categories, the privileged set, flag spot checks (MOVE, ADD, ADDA, CMP, CLR, BTST, ABCD, CHK, LEA, Scc, ASR) |
| `encoding.rs` | test | Field layout; over all 65,536 opwords at most one (form, condition) matches, and every form matches one; golden opwords from a68k listings and negative ones (68010+ and line A/F); the pinned count `VALID_OPWORDS = 45_816`, also broken down per opword line and per entry |
| `timing.rs` | test | One timing row per (form, size, mode, outcome); even clocks and plausible bus counts; `EA_TIME` and MOVE-matrix spot checks; `cycles()` spot checks against the User's Manual §8 |
| `view.rs` | test | `M68kIsa` as `va_core::isa::InstructionSet`: case-insensitive `describe`, branch size mapping, `SizeNotAccepted`/`Unknown`, flag names, mode notation, agreement of the rendered timing rows with `cycles()` |

## Entry points
- `encoding.rs` for the opword uniqueness proof and the pinned count.
- `timing.rs` for the cycle facts.

The pinned count and the cycle values the tests check are **provisional until the M3 cross-check**
(m1 §12.3; list in `arch/m68k/src/isa/README.md`). When M3 changes one, the test is updated with the
reason recorded.

## Dependencies
- Internal: `va-arch-m68k` (`isa`), `va-core` (`isa`).
- External: none.

## Tests / usage
```bash
cargo test -p va-arch-m68k --test isa
cargo test -p va-arch-m68k --test isa -- --nocapture
```
The second form also shows the line `valid 68000 opwords: 45816` printed by `encoding.rs`.
