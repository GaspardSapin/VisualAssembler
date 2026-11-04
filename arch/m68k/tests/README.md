# arch/m68k/tests

## Purpose
Integration tests of `va-arch-m68k`, mirroring `src/` (architecture §8): one file or folder per
source area. No test lives in `src/`. Each test target uses only the public API of the crate and of
`va-core`, plus `fluent-bundle` (a dev-dependency) for the locale checks.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `isa/` | folder | Test target `isa`: consistency of the instruction tables (m1 §4.7, exit criterion E4) and of the M2 operand placement table (`isa/fields.rs`, WP-C). See its README. |
| `syntax/` | folder | Test target `syntax`: lexer kinds, field rules (with the bracketed macro arguments of M2 WP-B), the §5.7 table, coverage and robustness properties, golden fixtures (exit criterion E3). See its README. |
| `asm/` | folder | Test target `asm` (M2, m2-assembler.md §10.1, exit criterion E3), 12 modules. Written: `parser`, `expr` (WP-B), `encode`, `optimize`, `corpus` (WP-C), `output`, `listing` (WP-D); the other 5 WP-D modules are still empty. See its README. |
| `disasm/` | folder | Test target `disasm` (M2, exit criterion E4), written by WP-E: `decode`, `format`, `flow` and the golden files. See its README. |
| `arch.rs` | test | `M68K` through `va_core::arch::Architecture`: id `m68k`, the crate's lexer and tables, every keyword and every spelling (with each accepted suffix) lexes as its kind, mnemonics and directives carry their doc ids and nothing else does, `diagnostic_codes()` is the lexer codes then `ASM_DIAG_CODES`, `assembler()` and `disassembler()` are provided (not called), English and French resources exist, the register file and the SR fields |
| `locales.rs` | test | The three resources of both languages parse; they document every id the 68000 can reference (the tables, the 61 diagnostic codes, the 49 directive keywords) with the required attributes, have identical id and attribute sets and no orphan; every diagnostic formats without resolver error with the arguments of its code, each printed argument appears and each selector value gives its own text; summaries, details and messages keep their length and the §6.1 style rules, and French is not a copy of English (m1 §6.3, m2 §15.2 W1, exit criterion E5 of both) |
| `syntax_bench.rs` | test (ignored) | Lexer throughput on 100,000 generated lines, release build, one `va.bench.lexer/1` JSON line (m1 §5.6, exit criterion E7) |

The golden files live outside this folder, in `arch/m68k/fixtures/lex/` and
`arch/m68k/fixtures/disasm/` (m1 §2.1).

Planned for M2 wave 2 (m2 §2, §10; none exists yet): `roundtrip.rs` (asm → bin → disasm → asm,
WP-E), `a68k.rs` (the ignored local comparison with a68k, run by `make compare-a68k`, WP-H) and
`asm_bench.rs` (the ignored throughput bench, run by `make bench-asm`, WP-H). Both make targets are decided, not implemented yet: until WP-H they stop
with cargo's "no test target named …" error (m2 §15.3 G3).

## Entry points
- `isa/main.rs` and `syntax/main.rs`: the shared helpers of the two multi-file targets.
- `asm/main.rs` and `disasm/main.rs`: module lists that stay as WP0 wrote them (m2 §1).

## Dependencies
- Internal: `va-arch-m68k`, `va-core`.
- External: `fluent-bundle` (dev-dependency, `locales.rs` only). See `DEPENDANCE.md`.

## Tests / usage
```bash
cargo test -p va-arch-m68k
cargo test -p va-arch-m68k --test arch
cargo test -p va-arch-m68k --test locales
cargo test -p va-arch-m68k --test asm
cargo test -p va-arch-m68k --test disasm
make bench-lexer
```
On 2026-11-04 the first command ran 268 tests, all passing: `arch` 13, `asm` 101, `disasm` 24,
`isa` 49, `locales` 18, `syntax` 63; the bench is ignored.
`make bench-lexer` runs the ignored `syntax_bench.rs` in release
(`cargo test -p va-arch-m68k --release --test syntax_bench -- --ignored --nocapture`). The
Performance report `docs/perf/2026-10-23-m1-lexer.md` gives the reference result: median 7.157 ms
for 100,000 lines on the reference machine, on mains with "Best performance", against a budget of
≤ 50 ms. Measure on mains power: battery slows every bench.
