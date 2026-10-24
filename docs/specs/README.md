# docs/specs

## Purpose
Normative specifications, written by the Architect role. Developers implement what these files say.
A change to a spec is an Architect decision, never a side edit by another role.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `architecture.md` | spec | Durable architecture: goals and non-goals, crate map and rules R1–R8, core contracts (with the provided-method rule for trait additions), threading (T1–T6), i18n and id ownership, settings files and atomic writes, performance definitions, testing strategy, platforms and support tiers, offline and unsafe policies with the tripwire threat model (§9.3, §9.4), INCLUDE path policy (§9.5), fonts (§9.6), licence (§9.7), renderer policy (§9.8, with the renderer setting in §6), extension paths; amended on 2026-10-24 by m0 §20, `m2-assembler.md` and `m4-editor-shell.md` (list in its header) |
| `m0-foundations.md` | spec | File-level spec of milestone M0: workspace, toolchain, GUI spike, CLI, xtask, Makefile, CI, cargo-deny, tests, renderer protocol, dependencies, fragile points; amended by §17 and §18, then by §19 (the user's renderer decision of 2026-10-14, with §19.9 ratifying its implementation), then by §20 (CI runner labels, 2026-10-24) |
| `m1-core-isa-lexer.md` | spec | File-level spec of milestone M1: frozen `va-core` contracts (and the M2/M3 signatures), 68000 ISA tables (spellings, forms, flags, cycles, encodings), the A68k-dialect lexer (with the §5.7 operand-spacing error of 2026-10-17), EN/FR instruction docs with Fluent, `va lex` and `va isa`, work packages, tests, lexer budget; amended by §12 (ratified implementation per wave, fix-now items R12-1…R12-4, QA round 1 in §12.8 with the approved advisory ignores). Its §3.7 M2 row is superseded by m2 §3 (`Option` return types, provided methods) |
| `m2-assembler.md` | spec | File-level spec of milestone M2: `va-core` additions (assembler, disassembler, `SourceProvider`, `CancelToken`), strict-A68k assembler (directives, macros, conditionals, INCLUDE/INCBIN, optimizations and branch relaxation byte-identical to `a68k -s`), disassembler on the same tables, S-record/binary/listing/source map, localized `asm-*` diagnostics, limits and the INCLUDE path policy, `va asm` and `va disasm`, tests, a68k comparison, budget, a68k observations; user decisions Q1–Q2 of 2026-10-24 in §13.3; coordination with M4 in §14 |
| `m4-editor-shell.md` | spec | File-level spec of milestone M4: `va-editor` (rope buffer, multi-cursor, grouped undo, regex search), `va-workspace` (std-only paths, portable mode, TOML settings/keybindings/project, atomic writes, session lock, hot exit and restore, UI localization), `va-ui` (virtualized editor, themes, font, tabs, explorer, palette, find bar, status bar), GUI integration (renderer setting, startup error box, native dialogs), bench v3 (open/scroll/type), dependencies; user decisions of 2026-10-24 in §8.3, §8.4 and §13; coordination with M2 in §14 |

## Entry points
- Read `architecture.md` first; milestone specs refine it and must not contradict it.
- In `m0-foundations.md` the precedence is **§20 > §19 > §18 > §17 > body (§0–§16)**.
  - §18.8 indexes every body passage that §17 or §18 supersedes; read that index before trusting a
    body section. The Architect folds §17 and §18 into the body once, when M0 closes (§18.8 CR1-09).
  - §19 replaces the renderer decision rule of §12.3/§12.4/§17.13 and outcome W. The §12 gates stay
    as measurements only. Its fix waves are §19.8 and §19.9. §19.9 ratifies the landed
    implementation (same precedence as §19) and adds rows 19-8…19-11. §18.8 also indexes the body
    passages superseded by §19, §19.9 and §20; §19.9.7 also lists the §19 passages that §20
    supersedes.

- `m1-core-isa-lexer.md` refines architecture §3, §5, §7 and §8; each refined passage there carries
  a cross-reference to it. Its §12 has the same precedence as the body it amends and wins where they differ.
- `m2-assembler.md` and `m4-editor-shell.md` run in parallel. Read §14 of either first: the merged
  wave order (no two concurrent work packages edit the same file) and the contracts between them.
  Both sections say the same thing; architecture.md carries the amendments they needed.

Naming: one file per milestone, `mN-<topic>.md` (`m3-…` and `m5-…` next). The approved Phase 0 plan stays
outside the repository; `architecture.md` restates everything developers need from it.

## Dependencies
- Internal: referenced by every README, `CLAUDE.md`, `SECURITY.md` and the code comments
  (`m0-foundations.md §N`).
- External: none.

## Tests / usage
Not executable. The checks that enforce these specs are listed in `xtask/README.md`,
`scripts/README.md` and `SECURITY.md`.
