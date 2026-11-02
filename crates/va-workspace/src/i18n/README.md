# crates/va-workspace/src/i18n

## Purpose
Language selection and Fluent rendering (m1-core-isa-lexer.md §6.2, m4-editor-shell.md §4.6,
architecture §5), shared by the CLI and the GUI. It is the single renderer of diagnostics,
instruction documentation and, from M4, the UI text. The CPU's messages come from its
`Architecture::locale_resources` (for the 68000, `arch/m68k/locales/`); the GUI's own resources
are the root `locales/{en,fr}/*.ftl`, embedded at compile time. It never reads files at run time.

**M1 (implemented):** `lang.rs` and `localizer.rs`. **M4 WP-D (wave 2, commit `1966bd7`):**
`ui.rs` and `system.rs`, and the `Setting` and `System` variants of `LangSource`; `resolve_lang`
and the M1 signatures are unchanged (m4 §14.2). Nothing outside the crate calls the GUI items yet:
the GUI wiring is WP-G.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | Re-exports `Lang`, `LangSource`, `LangChoice`, `resolve_lang`, `Localizer`, `LocalizerError`, `RenderedDiagnostic`, `system_ui_language`, `resolve_ui_lang`, `ui_resources` |
| `lang.rs` | module | `Lang` (`En`, `Fr`; `parse` accepts exactly `en`/`fr`), `LangSource`, `LangChoice`, `resolve_lang(flag, &var)` (the CLI): `--lang`, then `VA_LANG` (`en`, `fr` or `auto`; any other value is ignored and reported in `ignored_va_lang`), then `auto` |
| `localizer.rs` | module | `Localizer`: one concurrent `FluentBundle` for the language and one for English, `message(id, attribute, args)`, `render(&diagnostic)`, `for_architecture(lang, arch)`; `LocalizerError` for build defects |
| `ui.rs` | module | The GUI (m4 §4.6): `ui_resources(lang)`, the three root resources `ui.ftl`, `commands.ftl`, `config.ftl` embedded with `include_str!`; `Localizer::for_ui(lang, arch)`, one bundle per language holding them and the CPU's resources (a shared id is a `Duplicate` error); `resolve_ui_lang(flag, setting, &var, &system)` |
| `system.rs` | module | `system_ui_language()`: on Windows the first `sys_locale::get_locales()` entry (a BCP 47 tag such as `fr-FR`), elsewhere `None`. Called by the GUI only |

## Entry points
- `resolve_lang` then `Localizer::for_architecture`: what `va lex` and `va isa` do at start-up.
- `resolve_ui_lang` then `Localizer::for_ui`: what the GUI will do (WP-G).
- `Localizer::render` for diagnostics; `Localizer::message` for documentation and UI text.

Behaviour (m1 §6.2, ratified in §12.2; m4 §4.6):
- **The CLI's `auto`** reads `LC_ALL`, `LC_MESSAGES`, `LANG` (first non-empty): a value starting
  with `fr` gives French, anything else English. With `--lang`, no variable is read. `va` keeps
  this M1 rule and never reads the Windows UI language (m4 §4.6).
- **The GUI's precedence** is `--lang`, then `VA_LANG`, then the `ui.language` setting, then
  `auto`. `VA_LANG=auto` goes straight to `auto` and skips the setting; an invalid `VA_LANG` is
  reported and the choice goes on with the setting. The GUI's `auto` reads the same locale
  variables, and only when none is set and non-empty asks `system_ui_language` (Windows), with the
  same `fr`/`en` prefix rule; with no answer it gives English.
- **Fallback** works per pattern: the requested language first, then English (a French message
  without `.help` gets the English `.help`). A message missing everywhere gives `None` from `message`,
  and `render` then uses the code itself, never a panic. Resolver errors (a missing argument) give
  best-effort text, never a fallback.
- Bundles are `fluent_bundle::concurrent::FluentBundle` (`Localizer` is `Send + Sync`) with
  isolation marks off, so no U+2068/U+2069 reaches a terminal or egui. Arguments are inserted
  verbatim: escaping for a terminal is the caller's job (`apps/va/src/report.rs`).
- User overrides in `<config>/locales/<lang>/*.ftl` are deferred (architecture §5; not loaded in
  M4).
- `render` ignores `Related` entries: the caller renders each related place with `message`
  (`related.code.fluent_id()`, `related.args`).

## Dependencies
- Internal: `va-core` (`arch::Architecture`, `diag`); the root `locales/` (embedded by `ui.rs`).
- External: `fluent-bundle` 0.16.0, `unic-langid` 0.9.6, and `sys-locale` 0.3.2 on Windows only
  (`system.rs`). See `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va-workspace --test i18n_ui
cargo test -p va-workspace --test i18n_localizer
```
The tests are in `crates/va-workspace/tests/` (see its README).
