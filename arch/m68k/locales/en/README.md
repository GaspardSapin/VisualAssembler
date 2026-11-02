# arch/m68k/locales/en

## Purpose
The English Fluent resources of the 68000 (m1-core-isa-lexer.md §6.1, m2-assembler.md §7.1):
instruction documentation, the lexer and assembler diagnostics, and the directive documentation.
`src/locales.rs` embeds the three files with `include_str!` (in the order `isa.ftl`,
`diagnostics.ftl`, `directives.ftl`), `M68K.locale_resources("en")` returns them, and the
`va-workspace` localizer renders them. English is also the fallback: a
message or attribute missing in French is taken from here (m1 §6.2, §12.2). This folder holds text
only. Operand modes, flags and cycle counts come from the tables in `arch/m68k/src/isa/`.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `isa.ftl` | Fluent resource | 102 messages: the 74 entries `isa-m68k-<entry>` (one-line summary, `.details`, and `.flags` iff a form has an instruction-specific flag rule), 16 conditions `isa-m68k-cond-<name>`, 8 outcomes `isa-m68k-outcome-<id>`, 4 units `isa-m68k-unit-<id>` |
| `diagnostics.ftl` | Fluent resource | 61 messages `diag-<code>`, one per code of `M68K.diagnostic_codes()`. The 11 lexer messages `diag-lex-<…>` (M1), each with `$text`; `diag-lex-operands-in-comment` also takes `$cause`, which selects its `.help`. Then the 50 assembler messages `diag-asm-<…>` (M2 WP-F, commit `525d619`): 45 diagnostics, lowercase without a final period, most with a `.help` saying how to fix the problem and some with a `.note`, and the five `diag-asm-related-*` notes, without attribute. The variables each assembler message takes, and the values of every selector, are documented on its constant in `arch/m68k/src/asm/codes.rs` |
| `directives.ftl` | Fluent resource | 49 messages `isa-m68k-dir-<name>` (M2 WP-F, commit `6553939`), one per directive spelling of the dialect (`=` is `isa-m68k-dir-equals`): a one-line summary and `.details` (2 to 6 sentences) saying what VisualAssembler does with the directive, including the ones it refuses as `a68k -s` does. They are the doc ids that `SyntaxProvider::keywords()` gives the directive keywords |

## Entry points
- `isa.ftl`, header comment: the id scheme, the style rules and the vocabulary.
- `diagnostics.ftl` for the text `va lex` prints (and, once WP-D and WP-G land, `va asm`).
- `directives.ftl`, header comment: what the directive docs cover.

Rules for every edit:
- **Own words.** No sentence, caption or example is copied from the Motorola manuals or from another
  program's help (m1 §4.6, exit criterion E9). Facts (numbers, bit patterns, names) are fine.
- **No cycle numbers that the tables give.** The timing tables are the single source of clock-period
  counts. `.details` explains timing qualitatively and never repeats a count (m1 §12.1 R12-3). The
  table values are themselves provisional until M3 (`arch/m68k/src/isa/README.md`).
- **Fluent style** (tested): no `{` except placeables, no continuation line starting with `.`, `*`
  or `[`, typographic apostrophes (’) only. `.details` may span lines: a single newline is a soft
  wrap, a blank line a paragraph break (m1 §7.4).
- **Same ids and attributes in `en` and `fr`**, and no unreferenced id. Add or rename an id in both
  folders in the same change.

## Dependencies
- Internal: embedded by `arch/m68k/src/locales.rs`; rendered by `crates/va-workspace/src/i18n/`.
- External: none (the format is Fluent; `fluent-bundle` parses it in tests and in `va-workspace`).

## Tests / usage
```bash
cargo test -p va-arch-m68k --test locales
cargo run -p va -- --lang en isa move.l
```
The test checks parsing, completeness against the real tables, codes and keyword list, `en`/`fr`
parity, orphans, formatting of every diagnostic with the arguments of its code (each one used), and
the style rules (m1 §6.3 and m2 §15.2 W1, exit criterion E5 of each).
