# crates/va-workspace/src/i18n

## Purpose
Language selection and Fluent rendering (m1-core-isa-lexer.md §6.2, architecture §5), shared by the
CLI now and by the GUI from M4. It is the single renderer of diagnostics and instruction
documentation. It holds no text of its own: the messages come from each CPU's
`Architecture::locale_resources` (for the 68000, `arch/m68k/locales/`). It never reads files.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | Re-exports `Lang`, `LangSource`, `LangChoice`, `resolve_lang`, `Localizer`, `LocalizerError`, `RenderedDiagnostic` |
| `lang.rs` | module | `Lang` (`En`, `Fr`; `parse` accepts exactly `en`/`fr`), `resolve_lang(flag, &var)`: `--lang`, then `VA_LANG` (`en`, `fr` or `auto`; any other value is ignored and reported in `ignored_va_lang`), then `auto` |
| `localizer.rs` | module | `Localizer`: one concurrent `FluentBundle` for the language and one for English, `message(id, attribute, args)`, `render(&diagnostic)`, `for_architecture(lang, arch)`; `LocalizerError` for build defects |

## Entry points
- `resolve_lang` then `Localizer::for_architecture`: what `va lex` and `va isa` do at start-up.
- `Localizer::render` for diagnostics; `Localizer::message` for documentation.

Behaviour (m1 §6.2, ratified in §12.2):
- **`auto` in M1** reads `LC_ALL`, `LC_MESSAGES`, `LANG` (first non-empty): a value starting with
  `fr` gives French, anything else English. The Windows UI language and the `ui.language` setting
  are **not implemented**; they arrive in M4. With `--lang`, no variable is read.
- **Fallback** works per pattern: the requested language first, then English (a French message
  without `.help` gets the English `.help`). A message missing everywhere gives `None` from `message`,
  and `render` then uses the code itself, never a panic. Resolver errors (a missing argument) give
  best-effort text, never a fallback.
- Bundles are `fluent_bundle::concurrent::FluentBundle` (`Localizer` is `Send + Sync`) with
  isolation marks off, so no U+2068/U+2069 reaches a terminal or egui. Arguments are inserted
  verbatim: escaping for a terminal is the caller's job (`apps/va/src/report.rs`).
- `render` ignores `Related` entries in M1 (no M1 producer emits one); rendered related entries are
  deferred to M2.

## Dependencies
- Internal: `va-core` (`arch::Architecture`, `diag`).
- External: `fluent-bundle` 0.16.0, `unic-langid` 0.9.6. See `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va-workspace
```
The tests are in `crates/va-workspace/tests/` (see its README).
