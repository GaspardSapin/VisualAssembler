# locales

## Purpose
The architecture-independent Fluent resources of the IDE (architecture §5, m4-editor-shell.md
§4.6): shell text, command titles and configuration problems, one folder per language. They own
the `ui-*`, `cmd-*` and `diag-cfg-*` ids and define no Fluent term. A CPU's own texts (its
diagnostics and instruction docs) live in its crate, e.g. `arch/m68k/locales/`. `va-workspace`
will embed these files with `include_str!` (`ui_resources`, WP-D) and `Localizer::for_ui` will load
them together with the CPU's resources, with English as the per-pattern fallback.

**M4 scaffold (wave 1, commit `b4e7aae`).** Each `.ftl` file holds only its header comment; the
messages are written in wave 2 (WP-B: `config.ftl`; WP-F: `ui.ftl`, `commands.ftl`), and nothing
embeds the files yet (WP-D).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `en/` | folder | English resources, also the fallback language. See its README. |
| `fr/` | folder | French resources, written in our own words. See its README. |

## Entry points
- `en/ui.ftl`, header comment: the id scheme and the namespace rule.

Rules for every edit:
- Same ids and attributes in `en/` and `fr/`; add or rename an id in both folders in the same
  change.
- Ids are kebab-case English: `ui-<area>-<name>`, `cmd-<group>-<action>` (from the command id,
  `.` and `-` both mapped to `-`), `diag-<code>` for each `cfg-*` code of `CONFIG_CODES`.
- French text in our own words, with the m1-core-isa-lexer.md §12.2 typography.
- User overrides in `<config>/locales/<lang>/` are deferred (architecture §5); M4 does not read
  them.

## Dependencies
- Internal: embedded by `crates/va-workspace` (from WP-D); referenced by `crates/va-ui` and
  `apps/visual-assembler`.
- External: none (Fluent format; `fluent-bundle` parses it in `va-workspace`).

## Tests / usage
The completeness tests arrive in wave 2: `crates/va-workspace/tests/i18n_ui.rs` (every resource
parses, equal `en`/`fr` id sets, every `CONFIG_CODES` entry covered) and
`crates/va-ui/tests/locales.rs` (every id the UI code references exists in both languages, none is
unused). Neither exists yet, so there is no command to run here.
