# arch/m68k/locales/fr

## Purpose
The French Fluent resources of the 68000 (m1-core-isa-lexer.md §6.1): instruction documentation and
lexer diagnostics, shown with `--lang fr`, `VA_LANG=fr` or a French locale. `src/locales.rs` embeds
them with `include_str!` and `M68K.locale_resources("fr")` returns them. **French user-facing text
lives only in these `.ftl` files**: code, ids, comments and every other file stay English. The ids
are the English kebab-case ids of `../en/`.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `isa.ftl` | Fluent resource | The same 102 ids and attributes as `../en/isa.ftl`: 74 instruction entries, 16 conditions, 8 outcomes, 4 units |
| `diagnostics.ftl` | Fluent resource | The same 11 `diag-lex-<…>` messages as `../en/diagnostics.ftl`, with `$text` and, for `diag-lex-operands-in-comment`, `$cause` |

## Entry points
- `isa.ftl`, header comment: the French terminology used throughout (one term per notion) and the
  style rules.

Rules for every edit:
- **Own words, natural French.** The text is written in French, not translated word for word, and
  nothing is copied from the Motorola manuals or another program's help (m1 §4.6, exit criterion E9).
- **No cycle numbers that the tables give.** The timing tables in `arch/m68k/src/isa/` are the
  single source of clock-period counts (m1 §12.1 R12-3).
- **Typography** (m1 §12.2): regular spaces before `:` `;` `»` and after `«`, not U+202F, which
  terminals render unevenly. Typographic apostrophes (’) only.
- **Parity with `../en/`**: same ids, same attributes. A message or attribute missing here falls back
  to English at run time, but the locales test fails first.

## Dependencies
- Internal: embedded by `arch/m68k/src/locales.rs`; rendered by `crates/va-workspace/src/i18n/`.
- External: none (the format is Fluent).

## Tests / usage
```bash
cargo test -p va-arch-m68k --test locales
cargo run -p va -- --lang fr isa move.l
```
