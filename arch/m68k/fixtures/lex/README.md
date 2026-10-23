# arch/m68k/fixtures/lex

## Purpose
Golden files of the 68000 lexer (m1-core-isa-lexer.md §5.5). Each `<name>.asm` is an A68k source,
and `<name>.tokens` is the exact token list that `lex_line` must produce for it. The `va lex` tests
also read them. Fixtures sit in `arch/<cpu>/fixtures/<area>/` (depth 4), not under `tests/` (§2.1).
They are LF-only UTF-8. Byte-level cases (CRLF, CR, BOM, invalid UTF-8, NUL) are built in test code,
never stored here.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `basic.asm` / `basic.tokens` | fixture | A small complete program: every common field, addressing modes, directives |
| `labels.asm` / `labels.tokens` | fixture | Column-1, indented and local labels (`1$`, `\name`), `lab::` and `lab:::`, label errors |
| `literals.asm` / `literals.tokens` | fixture | Numbers in every base, strings with doubled quotes, registers, operators, `INCLUDE`/`INCBIN` paths |
| `comments.asm` / `comments.tokens` | fixture | Comment forms and every row of the §5.7 table (operand-like text after a space), tabs included |
| `macros.asm` / `macros.tokens` | fixture | Macro definitions, parameters (`\0`–`\9`, `\@`) and calls |
| `errors.asm` / `errors.tokens` | fixture | One line per lexer error code, then one 65,542-byte line (`lex-line-too-long`); the `.tokens` file is about 67 KB |

Token line format, whitespace tokens included:
`<line>:<start>..<end> <kind>[ <code>] <text as a JSON string>`, with 1-based lines and
line-relative byte offsets, e.g. `3:0..5 label "start"`.

## Entry points
- `basic.asm` to see the dialect; `comments.asm` for the §5.7 rule.
- The comparison lives in `arch/m68k/tests/syntax/fixtures.rs`.

## Dependencies
- Internal: read by `arch/m68k/tests/syntax/` (golden comparison and token-kind coverage) and
  `apps/va/tests/lex.rs`.
- External: none.

## Tests / usage
```bash
cargo test -p va-arch-m68k --test syntax
cargo run -p va -- lex arch/m68k/fixtures/lex/basic.asm
```
After a deliberate lexer change, `VA_BLESS=1 cargo test -p va-arch-m68k --test syntax` rewrites
the `.tokens` files (m1 §5.5). Review the diff in git before committing.
