# arch/m68k/locales/en

## Purpose
The English Fluent resources of the 68000 (m1-core-isa-lexer.md §6.1): instruction documentation and
lexer diagnostics. `src/locales.rs` embeds them with `include_str!`, `M68K.locale_resources("en")`
returns them, and the `va-workspace` localizer renders them. English is also the fallback: a
message or attribute missing in French is taken from here (m1 §6.2, §12.2). This folder holds text
only. Operand modes, flags and cycle counts come from the tables in `arch/m68k/src/isa/`.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `isa.ftl` | Fluent resource | 102 messages: the 74 entries `isa-m68k-<entry>` (one-line summary, `.details`, and `.flags` iff a form has an instruction-specific flag rule), 16 conditions `isa-m68k-cond-<name>`, 8 outcomes `isa-m68k-outcome-<id>`, 4 units `isa-m68k-unit-<id>` |
| `diagnostics.ftl` | Fluent resource | The 11 lexer messages `diag-lex-<…>`, each with `$text`. `diag-lex-operands-in-comment` also takes `$cause`, which selects its `.help`. |

## Entry points
- `isa.ftl`, header comment: the id scheme, the style rules and the vocabulary.
- `diagnostics.ftl` for the text `va lex` prints.

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
The test checks parsing, completeness against the real tables, `en`/`fr` parity, orphans,
formatting with sample arguments, and the style rules (m1 §6.3, exit criterion E5).
