# locales/en

## Purpose
The English UI resources of the IDE (m4-editor-shell.md §4.6): shell text, command titles and the
configuration problems. English is also the fallback: a message or attribute missing in French
is taken from here (m1 §6.2 per-pattern fallback).

**M4 scaffold (wave 1, commit `b4e7aae`).** Each file holds only its header comment, which states
its id scheme; the messages arrive in wave 2 (owners below). Nothing embeds these files yet: WP-D
adds `ui_resources` in `crates/va-workspace/src/i18n/ui.rs`.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `ui.ftl` | Fluent resource | `ui-<area>-<name>`: labels, notices and prompts of the shell (m4 §5). Header only; WP-F writes the messages |
| `commands.ftl` | Fluent resource | `cmd-<group>-<action>`: one title per command id of the registry (m4 §5.12), e.g. `cmd-edit-toggle-comment`. Header only; WP-F |
| `config.ftl` | Fluent resource | `diag-cfg-<…>`: one message per `cfg-*` code of `CONFIG_CODES` (m4 §4.2), with `.help` where the fix is not obvious. Header only; WP-B |

## Entry points
- The header comment of each file.

Rules: same ids, attributes and variables as `locales/fr/`; written in our own words; labels short
and imperative.

## Dependencies
- Internal: embedded by `crates/va-workspace` (from WP-D); the ids are referenced by `crates/va-ui`,
  `apps/visual-assembler` and `va_workspace::config` (`CONFIG_CODES`).
- External: none.

## Tests / usage
No test reads these files yet. In wave 2, `crates/va-workspace/tests/i18n_ui.rs` (WP-D) and
`crates/va-ui/tests/locales.rs` (WP-D) check them (m4 §4.6, exit criterion E6); see
`locales/README.md`.
