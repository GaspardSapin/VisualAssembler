# arch/m68k

## Purpose
Package `va-arch-m68k`: Motorola 68000 support. It implements the `va-core` traits and is registered
by the apps (`va` in M1); nothing in `crates/` may depend on it (rule R2).

**M1 (implemented):** the 68000 instruction tables (126 spellings, 74 entries, forms, flags,
encodings, cycle counts), the A68k-dialect line lexer shared by the M4 highlighter and the M2
assembler, the register file, the 11 lexer diagnostic codes, and the English and French instruction
documentation (m1-core-isa-lexer.md §3.6, §4, §5, §6). **Not implemented yet:** the assembler and
disassembler (M2), the emulator adapter over the `m68k` crate and the I/O profiles (M3).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Depends on `va-core`; dev-dependency `fluent-bundle` for the locale tests (m1 §8) |
| `src/` | folder | Crate sources: `isa/`, `syntax/`, the `M68K` architecture, registers, embedded locales. See its README. |
| `locales/en/`, `locales/fr/` | folders | Fluent resources: instruction docs (`isa.ftl`) and lexer diagnostics (`diagnostics.ftl`), embedded at compile time. See their READMEs. |
| `fixtures/lex/` | folder | Golden `.asm`/`.tokens` pairs of the lexer. See its README. |
| `tests/` | folder | Integration tests mirroring `src/`, and the ignored lexer bench. See its README. |

Projected (m1 §2.2): `src/asm/`, `src/disasm/` and `fixtures/{asm,disasm}/` in M2; `src/emu/`,
`src/io/` and `tests/{emu,io}/` in M3; `benches/` in M7.

## Entry points
- `src/lib.rs`: the `M68K` instance (`va_core::arch::Architecture`), and the public modules `isa`
  and `syntax`.

## Dependencies
- Internal (allowed): `va-core`.
- External: none at run time. Dev-dependency `fluent-bundle` (`tests/locales.rs`). Planned
  (m0-foundations.md §13.3): `m68k` (MIT), M3, without its `jit` feature.

The external `a68k.exe` and `d68k.exe` tools are never shipped or redistributed (no licence). The
user configures their paths; nothing hardcodes them. The M1 dialect facts were observed by running
a68k locally (m1 §4.6). An automated comparison with `a68k` will run only when `VA_A68K_PATH` is
set (M2, not implemented), never in CI.

## Tests / usage
```bash
cargo test -p va-arch-m68k
make bench-lexer
```
The first runs the `arch`, `isa`, `locales` and `syntax` test targets (exit criteria E3 to E5).
`make bench-lexer` runs the ignored throughput bench in release; the measured median is 7.157 ms
for 100,000 lines against a 50 ms budget (`docs/perf/2026-10-23-m1-lexer.md`).
