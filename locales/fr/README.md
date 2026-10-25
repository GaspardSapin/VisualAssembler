# locales/fr

## Purpose
The French UI resources of the IDE (m4-editor-shell.md §4.6), with the same ids, attributes and
variables as `locales/en/`. Written in our own words, in natural French rather than a
word-for-word translation, with the m1-core-isa-lexer.md §12.2 typography (regular spaces before
`:`, `;` and the closing quote). A message missing here falls back to English.

**M4 scaffold (wave 1, commit `b4e7aae`).** Each file holds only its header comment; the messages
arrive in wave 2 (WP-F: `ui.ftl`, `commands.ftl`; WP-B: `config.ftl`). Nothing embeds these files
yet (WP-D).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `ui.ftl` | Fluent resource | `ui-<area>-<name>`: shell text. Header only; WP-F |
| `commands.ftl` | Fluent resource | `cmd-<group>-<action>`: command titles. Header only; WP-F |
| `config.ftl` | Fluent resource | `diag-cfg-<…>`: configuration problems. Header only; WP-B |

## Entry points
- The header comment of each file.

## Dependencies
- Internal: embedded by `crates/va-workspace` (from WP-D), selected when the UI language is French
  (`--lang fr`, `VA_LANG`, `ui.language`, or `auto`; m4 §4.6).
- External: none.

## Tests / usage
No test reads these files yet; the wave-2 completeness tests are listed in `locales/README.md`.
