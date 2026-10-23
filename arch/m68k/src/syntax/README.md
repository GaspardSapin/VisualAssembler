# arch/m68k/src/syntax

## Purpose
The A68k-dialect line lexer (m1-core-isa-lexer.md §5), shared by the M4 highlighter and the M2
assembler. `lex_line` is the single implementation. `M68kSyntax` exposes it as a
`va_core::syntax::SyntaxProvider` and always returns `LineState::INITIAL`, because the dialect is
line-oriented. This folder does not parse statements, evaluate expressions or open `INCLUDE` paths.
Those are M2's job.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | `lex_line`, `M68kSyntax` (`keywords`, and `line_diagnostics`, which completes the §5.7 diagnostic), the re-exported codes |
| `lexer.rs` | module | One forward pass over the line: label field, operation field, operand field (or path, or nothing, by the operation's policy) and trailing comment |
| `literals.rs` | module | Character classes and scanners: identifiers, numbers, quoted strings, backslash sequences, runs of rejected characters |
| `keywords.rs` | module | The 49 directives with their operand policy, the 21 register names, the 4 size suffixes, and the keyword list. Mnemonics come from `../isa/`. |
| `codes.rs` | module | The 11 diagnostic codes and `DIAG_CODES` |
| `spacing.rs` | module | §5.7: the R1/R2/R3 rules that turn operand-like comment text into a `SuspectComment`, its `cause` argument and its quick fix |

## Entry points
- `mod.rs`: `lex_line(line, &mut out)` and `M68kSyntax`.
- `spacing.rs` for the operand-spacing rule; `lexer.rs` for the field rules of §5.2.

**The dialect is strict A68k 2.71.F3w** (§5.1). A label sits in column 1, so `nop` in column 1 is
the label `nop`. `;` outside quotes starts a comment anywhere, and `*` in column 1 starts a comment
line. **The operand field ends at the first unquoted space or tab, and the rest of the line is a
comment**, for the lexer, for the M2 assembler and for the bytes produced. `dc.b 1, 2, 3` therefore
produces one byte, as in a68k, and `move.l d0, d1` is an a68k error.

**Operand-like text after a space** (§5.7, user decision of 2026-10-17). Only the presentation
changes. When the trailing text continues the operands, it becomes one
`SuspectComment(lex-operands-in-comment)` token instead of a `Comment`. It is greyed like a comment
and underlined as an error. Its diagnostic is a **non-blocking error**: it counts as an error
(`va lex` exits 1) but never prevents the program image. The rule fires when:
- R1: the operands end with `,`, `#`, `(` or an operator, or open more `(` than they close;
- R2: the text starts with `,` or with `+ / & ! | << >>` (not `-` or `*`, which often start free text);
- R3: the first word of the text is itself an operand list (`dc.b 1,2 3,4`).

R1 and R2 carry a quick fix that removes the spaces touching `,`, `#`, `(`, `)` or an operator
(`dc.b 1, 2, 3` gives `dc.b 1,2,3`). The fix is attached only when it changes the text, and R3 has
none. A comment that starts with `;` never fires, nor does a column-1 `*` line, nor the text after
a keyword without operands (`rts d0, d1`). In the trailing text, a quote opens a string only if the
line closes it. A real `; comment` therefore stays a plain comment.

**Labels.** A run of `:` after a label is one `Punctuation` token: `lab:`, `lab::`, `lab:::` and the
indented `  other:: nop` are accepted, because a68k assembles them to the same object as `lab:`
(§12.1 R12-2). Text glued after the colon (`lab:nop`) is `lex-label-junk`.

**Deliberately stricter than a68k** (§5.4), because a68k's silent reading is almost always a mistake:
a number prefix without digits (`#$`), text glued after a label colon, and lines over 64 KiB
(`MAX_LINE_BYTES`, one `lex-line-too-long` token).

Error codes: `lex-bad-label`, `lex-label-junk`, `lex-bad-operation`, `lex-bad-size-suffix`,
`lex-bad-number`, `lex-unterminated-string`, `lex-bad-backslash`, `lex-unexpected-char`,
`lex-non-ascii`, `lex-line-too-long` (blocking errors), and `lex-operands-in-comment`
(non-blocking). Their text is the Fluent message `diag-<code>` in
`arch/m68k/locales/{en,fr}/diagnostics.ftl`.

Contract (`va_core::syntax`, §3.4): the tokens cover the line exactly, in order, on char
boundaries. `lex_line` never panics and runs in linear time. It allocates nothing besides growing
`out`; in M1 that is checked by code review only (§12.4). Throughput: median 7.157 ms for 100,000
lines against a 50 ms budget (`docs/perf/2026-10-23-m1-lexer.md`).

## Dependencies
- Internal: `va-core` (`source`, `syntax`, `diag`), and `../isa/` for the mnemonic spellings and
  `takes_operands`.
- External: none.

## Tests / usage
```bash
cargo test -p va-arch-m68k --test syntax
cargo run -p va -- lex arch/m68k/fixtures/lex/basic.asm
```
See `arch/m68k/tests/syntax/README.md` and `arch/m68k/fixtures/lex/README.md`.
