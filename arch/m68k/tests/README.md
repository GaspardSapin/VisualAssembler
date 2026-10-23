# arch/m68k/tests

## Purpose
Integration tests of `va-arch-m68k`, mirroring `src/` (architecture §8): one file or folder per
source area. No test lives in `src/`. Each test target uses only the public API of the crate and of
`va-core`, plus `fluent-bundle` (a dev-dependency) for the locale checks.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `isa/` | folder | Test target `isa`: consistency of the instruction tables (m1 §4.7, exit criterion E4). See its README. |
| `syntax/` | folder | Test target `syntax`: lexer kinds, field rules, the §5.7 table, coverage and robustness properties, golden fixtures (exit criterion E3). See its README. |
| `arch.rs` | test | `M68K` through `va_core::arch::Architecture`: id `m68k`, every keyword and every spelling (with each accepted suffix) lexes as its kind, `diagnostic_codes() == DIAG_CODES`, the register file and the SR fields |
| `locales.rs` | test | Both languages parse, document every id the tables can reference with the required attributes, have identical id and attribute sets, hold no orphan, format with sample arguments without resolver error, and keep the §6.1 style rules (m1 §6.3, exit criterion E5) |
| `syntax_bench.rs` | test (ignored) | Lexer throughput on 100,000 generated lines, release build, one `va.bench.lexer/1` JSON line (m1 §5.6, exit criterion E7) |

The golden files live outside this folder, in `arch/m68k/fixtures/lex/` (m1 §2.1).

## Entry points
- `isa/main.rs` and `syntax/main.rs`: the shared helpers of the two multi-file targets.

## Dependencies
- Internal: `va-arch-m68k`, `va-core`.
- External: `fluent-bundle` (dev-dependency, `locales.rs` only). See `DEPENDANCE.md`.

## Tests / usage
```bash
cargo test -p va-arch-m68k
cargo test -p va-arch-m68k --test arch
cargo test -p va-arch-m68k --test locales
make bench-lexer
```
`make bench-lexer` runs the ignored `syntax_bench.rs` in release
(`cargo test -p va-arch-m68k --release --test syntax_bench -- --ignored --nocapture`). The
Performance report `docs/perf/2026-10-23-m1-lexer.md` gives the reference result: median 7.157 ms
for 100,000 lines on the reference machine, on mains with "Best performance", against a budget of
≤ 50 ms. Measure on mains power: battery slows every bench.
