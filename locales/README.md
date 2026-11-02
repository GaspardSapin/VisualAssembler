# locales

## Purpose
The architecture-independent Fluent resources of the IDE (architecture §5, m4-editor-shell.md
§4.6): shell text, command titles and configuration problems, one folder per language. They own
the `ui-*`, `cmd-*` and `diag-cfg-*` ids and define no Fluent term. A CPU's own texts (its
diagnostics and instruction docs) live in its crate, e.g. `arch/m68k/locales/`. `va-workspace`
embeds these files with `include_str!` (`ui_resources`, M4 WP-D) and `Localizer::for_ui` loads
them together with the CPU's resources, with English as the per-pattern fallback.

**M4 wave 2 in progress.** `config.ftl` holds its 12 `diag-cfg-*` messages (WP-B, commit
`1cf9729`). `ui.ftl` and `commands.ftl` still hold only their header comment; WP-F writes their
messages. Nothing renders these texts yet: the GUI is wired by WP-G.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `en/` | folder | English resources, also the fallback language. See its README. |
| `fr/` | folder | French resources, written in our own words. See its README. |

## Entry points
- `en/ui.ftl`, header comment: the id scheme and the namespace rule.
- `en/config.ftl`, header comment: the arguments of the `diag-cfg-*` messages.

Rules for every edit:
- Same ids and attributes in `en/` and `fr/`; add or rename an id in both folders in the same
  change.
- Ids are kebab-case English: `ui-<area>-<name>`, `cmd-<group>-<action>` (from the command id,
  `.` and `-` both mapped to `-`), `diag-<code>` for each `cfg-*` code of `CONFIG_CODES`.
- French text in our own words, with the m1-core-isa-lexer.md §12.2 typography.
- User overrides in `<config>/locales/<lang>/` are deferred (architecture §5); M4 does not read
  them.

## Dependencies
- Internal: embedded by `crates/va-workspace` (`src/i18n/ui.rs`); referenced by `crates/va-ui`,
  `apps/visual-assembler` and `va_workspace::config` (`CONFIG_CODES`).
- External: none (Fluent format; `fluent-bundle` parses it in `va-workspace`).

## Tests / usage
```bash
cargo test -p va-workspace --test i18n_ui
cargo test -p va-ui --test locales
```
The first checks that every resource parses, keeps its namespace and defines no term, that `en`
and `fr` have the same ids and attributes, and that every `CONFIG_CODES` entry has its message;
`config_settings.rs` also formats the `diag-cfg-*` messages. The second checks that every
`ui-*`/`cmd-*` id the UI code references exists in both languages and none is unused (exit
criterion E6). Both pass on today's header-only `ui.ftl` and `commands.ftl` and tighten as they
fill. On 2026-11-02 they ran 24 and 7 tests, all passing.
