# docs/specs

## Purpose
Normative specifications, written by the Architect role. Developers implement what these files say.
A change to a spec is an Architect decision, never a side edit by another role.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `architecture.md` | spec | Durable architecture: goals and non-goals, crate map and rules R1–R8, core contracts, threading (T1–T6), i18n, settings files, performance definitions, testing strategy, platforms and support tiers, offline and unsafe policies with the tripwire threat model (§9.3, §9.4), licence (§9.7), renderer policy (§9.8, with the renderer setting in §6), extension paths |
| `m0-foundations.md` | spec | File-level spec of milestone M0: workspace, toolchain, GUI spike, CLI, xtask, Makefile, CI, cargo-deny, tests, renderer protocol, dependencies, fragile points; amended by §17 and §18, then by §19 (the user's renderer decision of 2026-10-14, with §19.9 ratifying its implementation) |
| `m1-core-isa-lexer.md` | spec | File-level spec of milestone M1: frozen `va-core` contracts (and the M2/M3 signatures), 68000 ISA tables (spellings, forms, flags, cycles, encodings), the A68k-dialect lexer (with the §5.7 operand-spacing error of 2026-10-17), EN/FR instruction docs with Fluent, `va lex` and `va isa`, work packages, tests, lexer budget |

## Entry points
- Read `architecture.md` first; milestone specs refine it and must not contradict it.
- In `m0-foundations.md` the precedence is **§19 > §18 > §17 > body (§0–§16)**.
  - §18.8 indexes every body passage that §17 or §18 supersedes; read that index before trusting a
    body section. The Architect folds §17 and §18 into the body once, when M0 closes (§18.8 CR1-09).
  - §19 replaces the renderer decision rule of §12.3/§12.4/§17.13 and outcome W. The §12 gates stay
    as measurements only. Its fix waves are §19.8 and §19.9. §19.9 ratifies the landed
    implementation (same precedence as §19) and adds rows 19-8…19-11. §18.8 also indexes the body
    passages superseded by §19 and §19.9.

- `m1-core-isa-lexer.md` refines architecture §3, §5, §7 and §8; each refined passage there carries
  a cross-reference to it.

Naming: one file per milestone, `mN-<topic>.md` (`m2-…` next). The approved Phase 0 plan stays
outside the repository; `architecture.md` restates everything developers need from it.

## Dependencies
- Internal: referenced by every README, `CLAUDE.md`, `SECURITY.md` and the code comments
  (`m0-foundations.md §N`).
- External: none.

## Tests / usage
Not executable. The checks that enforce these specs are listed in `xtask/README.md`,
`scripts/README.md` and `SECURITY.md`.
