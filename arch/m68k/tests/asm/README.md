# arch/m68k/tests/asm

## Purpose
Test target `asm`: the tests of the assembler in `arch/m68k/src/asm/` (m2-assembler.md §10.1,
exit criterion E3). They pin the line fields and operands, expressions, form selection and
encoding, the optimizations and the generated forms corpus; once WP-D lands, also branch
relaxation, directives, macros and conditionals, the S-record, binary and listing outputs, the
golden fixtures and the coverage of every `asm-*` code.

**M2 wave 2 in progress.** `main.rs` (WP0 scaffold, wave 1b, commit `c500c74`) declares every
module. Written: `parser.rs` and `expr.rs` (WP-B, commit `8064d41`), `encode.rs`, `optimize.rs`
and `corpus.rs` (WP-C, commit `50ad40d`). Still empty (WP-D): each of the other seven files holds
only its `//!` line and `#![allow(clippy::unwrap_used)]`. Following rule G1 (m2 §15.3), no test
here calls another package's `todo!()`: the WP-C tests build their operands through the §4.2
types instead of the front end and never call `assemble`.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `main.rs` | test root | Declares the 12 modules below. Stays as WP0 wrote it (m2 §1): each wave-2 package owns its module files and keeps its helpers there |
| `parser.rs` | test | WP-B, 24 tests: line fields (`src/asm/parser.rs`) and every operand form and rejection of §4.3 with its fix (`src/asm/operand.rs`), and the `<…>` lexer rows |
| `expr.rs` | test | WP-B, 18 tests: the §4.4 precedence examples, 32-bit wrap, `/` toward zero, `>>` sign, shifts by 32 or more, characters, `*`, `--2` |
| `encode.rs` | test | WP-C, 11 tests: the m1 §4.7 golden opwords encoded from operands built through the §4.2 types, the §4.10 selection order and examples, every `Place`, MOVEM mask reversal, branch problems and odd targets, value ranges, symbols through a `Scope`, `length` from shapes only, the closest selection failure |
| `optimize.rs` | test | WP-C, 10 tests: every §4.10 rule on (generic spelling, then `encode`) and off (explicit spelling, or no `optimize`), with the forward-reference cases of §12.2 (pass 1 as `Lookup::Unknown`, pass 2 as `Lookup::Later`) |
| `corpus.rs` | test | WP-C, 4 tests: the generated forms corpus (one line per entry, form, condition, size and operand mode, from `org $1000`) lexes without error, covers every form, and every line selects a form and encodes to words that form matches, at the length `length` gives; the generic variant with optimizations. Its `lines(generic)` is shared with the a68k comparison (§10.4, §15.3 G2). Assembling through the driver and decoding through the disassembler join when WP-D and WP-E land |
| `directives.rs` | test (empty) | WP-D: every row of the §4.5 table, alignment, labels and local blocks |
| `macros.rs` | test (empty) | WP-D: `\0`, `\@` numbering, NARG, `<…>`, MEXIT, IFxx blocks, the depth and statement limits with lowered `AsmLimits` |
| `relax.rs` | test (empty) | WP-D: the §12.2 relaxation probes, the CNOP regrowth rule, the sweep cap |
| `output.rs` | test (empty) | WP-D: the exact §12.1 S-record texts, CRLF, the empty image, `parse_srec` errors and round trip, binary base and gap fill |
| `listing.rs` | test (empty) | WP-D: listing layout, truncation and size cap |
| `fixtures.rs` | test (empty) | WP-D: the golden files of `arch/m68k/fixtures/{asm,asm-opt,asm-inc,asm-diag}/` (§10.2), rewritten with `VA_BLESS=1` |
| `diagnostics.rs` | test (empty) | WP-D: oversized, unreadable and too many includes, cancellation, severity and blocking per §7.2, and the check that the fixtures and this file produce every code of `ASM_DIAG_CODES` |

## Entry points
- `main.rs`: the module list and the owner of each module.
- `corpus.rs`: `lines(generic)`, the corpus source that `tests/a68k.rs` (WP-H) will include with
  `#[path = "asm/corpus.rs"] mod corpus;`.

## Dependencies
- Internal: `va-arch-m68k` (`asm`, `isa`, `syntax`), `va-core` (`asm`, `diag`, `image`,
  `source`, `syntax`); the fixtures in `arch/m68k/fixtures/asm*/` once WP-D adds them.
- External: none.

## Tests / usage
```bash
cargo test -p va-arch-m68k --test asm
cargo test -p va-arch-m68k --test asm corpus
```
On 2026-11-02 the target ran 67 tests, all passing. The a68k comparison
(`arch/m68k/tests/a68k.rs`) and the throughput bench (`arch/m68k/tests/asm_bench.rs`) are separate
targets that WP-H adds, run by `make compare-a68k` and `make bench-asm`. Both make targets are
decided, not implemented yet: until WP-H they fail with cargo's "no test target named …" error
(m2 §15.3 G3).
