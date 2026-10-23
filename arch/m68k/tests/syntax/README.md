# arch/m68k/tests/syntax

## Purpose
Test target `syntax`: the tests of the A68k-dialect lexer in `arch/m68k/src/syntax/`
(m1-core-isa-lexer.md §5, §9.1, exit criterion E3). They pin the field and token rules, the §5.7
operand-spacing table, the coverage contract of `va_core::syntax`, robustness on hostile input, and
the golden fixtures. The throughput bench is separate (`../syntax_bench.rs`).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `main.rs` | test root | Declares the modules; shared helpers and short names for token kinds |
| `kinds.rs` | test | A table of lines with their expected kinds: every `TokenKind` and every `lex-*` code is produced, over the table and the six fixtures; `DIAG_CODES` itself |
| `fields.rs` | test | §5.1–§5.3: column-1 and indented labels, `lab::`, the operation field and its suffixes, operand policies, every operand rule, and each row of the §5.7 table with its `cause` and quick fix (including the no-op-fix and quote cases of §12.8) |
| `coverage.rs` | test | Through `check_line_tokens`: every ASCII string of length ≤ 3 over 24 interesting chars, 100,000 deterministic pseudo-random lines, shuffled-order statelessness, `LineState::INITIAL`; the diagnostics of every line |
| `robustness.rs` | test | 40 pathological 64 KiB lines in debug mode under 30 s (the CI guard against quadratic time), 64 KiB + 1 gives one `lex-line-too-long`, `\r`, `\n` and NUL inside a line, 4-byte UTF-8 chars, every keyword in mixed case |
| `fixtures.rs` | test | Each `arch/m68k/fixtures/lex/<name>.asm` lexes to its `<name>.tokens`, byte for byte |

## Entry points
- `fields.rs` for the dialect rules; `fixtures.rs` for the golden comparison.

## Dependencies
- Internal: `va-arch-m68k` (`syntax`), `va-core` (`syntax`, `diag`, `source`), the fixtures in
  `arch/m68k/fixtures/lex/`.
- External: none.

## Tests / usage
```bash
cargo test -p va-arch-m68k --test syntax
```
A golden mismatch prints the first differing token line. After a deliberate lexer change,
`VA_BLESS=1 cargo test -p va-arch-m68k --test syntax` rewrites the `.tokens` files
(m1 §5.5); review the diff in git.
