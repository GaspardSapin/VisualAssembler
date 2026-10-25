# arch/m68k

## Purpose
Package `va-arch-m68k`: Motorola 68000 support. It implements the `va-core` traits and is registered
by the apps (`va` in M1); nothing in `crates/` may depend on it (rule R2).

**M1 (implemented):** the 68000 instruction tables (126 spellings, 74 entries, forms, flags,
encodings, cycle counts), the A68k-dialect line lexer shared by the M4 highlighter and the M2
assembler, the register file, the 11 lexer diagnostic codes, and the English and French instruction
documentation (m1-core-isa-lexer.md §3.6, §4, §5, §6). **M2 (in progress):** the assembler and
the disassembler are scaffolded (`src/asm/`, `src/disasm/`, `src/isa/fields.rs`, the `asm` and
`disasm` test targets; commit `c500c74`) with `todo!()` bodies, implemented in wave 2 by WP-B to
WP-H (m2-assembler.md §1). `M68K.assembler()` and `M68K.disassembler()` already return them, so
calling either panics until then. **Not implemented yet:** the emulator adapter over the `m68k`
crate and the I/O profiles (M3).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Depends on `va-core`; dev-dependency `fluent-bundle` for the locale tests (m1 §8) |
| `src/` | folder | Crate sources: `isa/`, `syntax/`, the M2 scaffolds `asm/` and `disasm/`, the `M68K` architecture, registers, embedded locales. See its README. |
| `locales/en/`, `locales/fr/` | folders | Fluent resources: instruction docs (`isa.ftl`) and lexer diagnostics (`diagnostics.ftl`), embedded at compile time. See their READMEs. |
| `fixtures/lex/` | folder | Golden `.asm`/`.tokens` pairs of the lexer. See its README. |
| `tests/` | folder | Integration tests mirroring `src/`, the M2 `asm` and `disasm` targets (scaffold), and the ignored lexer bench. See its README. |

Projected: `fixtures/{asm,asm-opt,asm-inc,asm-diag,disasm}/` in M2 wave 2 (m2 §10.2); `src/emu/`,
`src/io/` and `tests/{emu,io}/` in M3; `benches/` in M7 (m1 §2.2).

## Entry points
- `src/lib.rs`: the `M68K` instance (`va_core::arch::Architecture`), and the public modules `isa`,
  `syntax`, `asm` and `disasm`.

## Dependencies
- Internal (allowed): `va-core`.
- External: none at run time. Dev-dependency `fluent-bundle` (`tests/locales.rs`). Planned
  (m0-foundations.md §13.3): `m68k` (MIT), M3, without its `jit` feature.

The external `a68k.exe` and `d68k.exe` tools are never shipped or redistributed (no licence). The
user configures their paths; nothing hardcodes them. The M1 dialect facts were observed by running
a68k locally (m1 §4.6), and the M2 facts by probe runs on 2026-10-24 (m2 §12). An automated
comparison with `a68k` will run only when `VA_A68K_PATH` is set (`make compare-a68k`, never in
CI). It is decided, not implemented yet: the target fails until M2 WP-H adds `tests/a68k.rs`
(m2 §15.3 G3).

## Tests / usage
```bash
cargo test -p va-arch-m68k
make bench-lexer
```
The first runs the `arch`, `isa`, `locales` and `syntax` test targets (exit criteria E3 to E5),
and the M2 `asm` and `disasm` targets, which hold no test yet.
`make bench-lexer` runs the ignored throughput bench in release; the measured median is 7.157 ms
for 100,000 lines against a 50 ms budget (`docs/perf/2026-10-23-m1-lexer.md`).
