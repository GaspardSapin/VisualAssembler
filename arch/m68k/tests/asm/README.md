# arch/m68k/tests/asm

## Purpose
Test target `asm`: the tests of the assembler in `arch/m68k/src/asm/` (m2-assembler.md §10.1,
exit criterion E3). They will pin the line fields and operands, expressions, form selection and
encoding, the optimizations, branch relaxation, directives, macros and conditionals, the
S-record, binary and listing outputs, the golden fixtures and the coverage of every `asm-*` code.

**WP0 scaffold (wave 1b, commit `c500c74`); written in wave 2 by WP-B, WP-C and WP-D.** `main.rs`
declares every module; each module file holds only its `//!` line and
`#![allow(clippy::unwrap_used)]`, so the target compiles and runs no test yet.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `main.rs` | test root | Declares the 12 modules below. Stays as WP0 wrote it (m2 §1): each wave-2 package owns its module files and keeps its helpers there |
| `parser.rs` | test (empty) | WP-B: every operand form and rejection of §4.3 with its fix, the `<…>` lexer rows |
| `expr.rs` | test (empty) | WP-B: the §4.4 precedence examples, 32-bit wrap, `/` toward zero, `>>` sign, shifts by 32 or more, characters, `*`, `--2` |
| `encode.rs` | test (empty) | WP-C: the m1 §4.7 golden opwords assembled from text, the §4.10 selection examples, every `Place`, MOVEM mask reversal, selection error codes |
| `optimize.rs` | test (empty) | WP-C: every §4.10 rule on and off, with the forward-reference cases of §12.2 |
| `corpus.rs` | test (empty) | WP-C: the generated forms corpus (one line per entry, form, size and operand mode), assembled without diagnostics and decoded back; shared with the a68k comparison (§10.4) |
| `directives.rs` | test (empty) | WP-D: every row of the §4.5 table, alignment, labels and local blocks |
| `macros.rs` | test (empty) | WP-D: `\0`, `\@` numbering, NARG, `<…>`, MEXIT, IFxx blocks, the depth and statement limits with lowered `AsmLimits` |
| `relax.rs` | test (empty) | WP-D: the §12.2 relaxation probes, the CNOP regrowth rule, the sweep cap |
| `output.rs` | test (empty) | WP-D: the exact §12.1 S-record texts, CRLF, the empty image, `parse_srec` errors and round trip, binary base and gap fill |
| `listing.rs` | test (empty) | WP-D: listing layout, truncation and size cap |
| `fixtures.rs` | test (empty) | WP-D: the golden files of `arch/m68k/fixtures/{asm,asm-opt,asm-inc,asm-diag}/` (§10.2), rewritten with `VA_BLESS=1` |
| `diagnostics.rs` | test (empty) | WP-D: oversized, unreadable and too many includes, cancellation, severity and blocking per §7.2, and the check that the fixtures and this file produce every code of `ASM_DIAG_CODES` |

## Entry points
- `main.rs`: the module list and the owner of each module.

## Dependencies
- Internal: `va-arch-m68k` (`asm`, `isa`, `syntax`), `va-core` (`asm`, `diag`, `image`,
  `source`); the fixtures in `arch/m68k/fixtures/asm*/` once WP-D adds them.
- External: none.

## Tests / usage
```bash
cargo test -p va-arch-m68k --test asm
```
In the scaffold this compiles the target and runs 0 tests. The a68k comparison
(`arch/m68k/tests/a68k.rs`) and the throughput bench (`arch/m68k/tests/asm_bench.rs`) are separate
targets that WP-H adds, run by `make compare-a68k` and `make bench-asm`. Both make targets are
decided, not implemented yet: until WP-H they fail with cargo's "no test target named …" error
(m2 §15.3 G3).
