# docs

## Purpose
Project documentation that is longer than a README: normative specifications and measurement
reports. User-facing guides (French and English) arrive in M8 under `user/`.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `specs/` | folder | Architecture and milestone specifications (Architect role). See its README. |
| `perf/` | folder | Measurement reports; every performance claim cites one. See its README. |

## Entry points
- `specs/architecture.md`, then the milestone specs: `specs/m0-foundations.md` (M0),
  `specs/m1-core-isa-lexer.md` (M1: core contracts, 68000 ISA tables, lexer, `va lex`/`va isa`),
  and the two parallel milestones specified on 2026-10-24, not implemented yet:
  `specs/m2-assembler.md` (M2: assembler, disassembler, `va asm`/`va disasm`) and
  `specs/m4-editor-shell.md` (M4: editor, workspace, GUI shell, bench). Read §14 of either first.
- `perf/M0-backend.md` for the measured renderer numbers (§11: the shipped binary). The renderer
  decision itself is the user's: `specs/m0-foundations.md` §19 and `specs/architecture.md` §9.8.
- `perf/2026-10-23-m1-lexer.md` for the M1 lexer throughput (exit criterion E7).

## Dependencies
- Internal: none.
- External: none.

## Tests / usage
Not executable. `make docs` checks the README cascade of this folder like every other one.
