# locales/en

## Purpose
The English UI resources of the IDE (m4-editor-shell.md §4.6): shell text, command titles and the
configuration problems. English is also the fallback: a message or attribute missing in French
is taken from here (m1 §6.2 per-pattern fallback). `ui_resources(Lang::En)` in
`crates/va-workspace/src/i18n/ui.rs` embeds the three files, in the order `ui.ftl`,
`commands.ftl`, `config.ftl`.

**M4 wave 2 in progress.** `config.ftl` is written (WP-B, commit `1cf9729`); `ui.ftl` and
`commands.ftl` hold only their header comment, which states their id scheme, until WP-F writes
their messages.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `ui.ftl` | Fluent resource | `ui-<area>-<name>`: labels, notices and prompts of the shell (m4 §5). Header only; WP-F writes the messages |
| `commands.ftl` | Fluent resource | `cmd-<group>-<action>`: one title per command id of the registry (m4 §5.12), e.g. `cmd-edit-toggle-comment`. Header only; WP-F |
| `config.ftl` | Fluent resource | 12 messages `diag-cfg-<…>`, one per `cfg-*` code of `CONFIG_CODES` (m4 §4.2), each with a `.help`, receiving the arguments of the §4.2 table (`$key`, `$value`, `$expected`, `$min`, `$max`, …). `$error` and `$detail` stay English in every language |

## Entry points
- The header comment of each file.

Rules: same ids, attributes and variables as `locales/fr/`; written in our own words; labels short
and imperative.

## Dependencies
- Internal: embedded by `crates/va-workspace` (`src/i18n/ui.rs`); the ids are referenced by
  `crates/va-ui`, `apps/visual-assembler` and `va_workspace::config` (`CONFIG_CODES`).
- External: none.

## Tests / usage
```bash
cargo test -p va-workspace --test i18n_ui
cargo test -p va-workspace --test config_settings
```
`crates/va-workspace/tests/i18n_ui.rs` and `crates/va-ui/tests/locales.rs` check these files
against `locales/fr/` and the code (m4 §4.6, exit criterion E6); `config_settings.rs` formats the
`diag-cfg-*` messages. See `locales/README.md`.
