# locales/fr

## Purpose
The French UI resources of the IDE (m4-editor-shell.md §4.6), with the same ids, attributes and
variables as `locales/en/`. Written in our own words, in natural French rather than a
word-for-word translation, with the m1-core-isa-lexer.md §12.2 typography (regular spaces before
`:`, `;` and the closing quote). A message missing here falls back to English.
`ui_resources(Lang::Fr)` in `crates/va-workspace/src/i18n/ui.rs` embeds the three files.

**M4 wave 2 in progress.** `config.ftl` is written (WP-B, commit `1cf9729`); `ui.ftl` and
`commands.ftl` hold only their header comment until WP-F writes their messages.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `ui.ftl` | Fluent resource | `ui-<area>-<name>`: shell text. Header only; WP-F |
| `commands.ftl` | Fluent resource | `cmd-<group>-<action>`: command titles. Header only; WP-F |
| `config.ftl` | Fluent resource | The same 12 `diag-cfg-<…>` messages, `.help` attributes and variables as `../en/config.ftl` |

## Entry points
- The header comment of each file.

## Dependencies
- Internal: embedded by `crates/va-workspace` (`src/i18n/ui.rs`), selected when the UI language is
  French (`--lang fr`, `VA_LANG`, `ui.language`, or `auto`; m4 §4.6).
- External: none.

## Tests / usage
```bash
cargo test -p va-workspace --test i18n_ui
```
It checks that `fr` has the same ids and attributes as `en`; the other completeness tests are
listed in `locales/README.md`.
