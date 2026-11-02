# crates/va-workspace/tests

## Purpose
Tests of `va-workspace`, one file per source module, named after its path
(`src/i18n/lang.rs` → `i18n_lang.rs`; architecture §8). They cover the i18n module (M1, and the
GUI additions of M4 WP-D), `paths` and `config` (M4 WP-B). They use synthetic Fluent resources, a
fake CPU, a fake environment and a fake system language, so they need no CPU crate and read no
real environment variable or user folder. Files they write go in a test temp folder
(`<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>`).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `i18n_lang.rs` | test | `Lang` codes and `parse`; the `resolve_lang` precedence (`--lang`, `VA_LANG`, then `LC_ALL`/`LC_MESSAGES`/`LANG`) through a fake `var` closure, including empty, differently-cased and invalid `VA_LANG` values |
| `i18n_localizer.rs` | test | Fallback to English, missing ids (`None`, or the code in `render`), arguments and plurals, no isolation marks, parse and duplicate errors on synthetic resources, `for_architecture`, `Send + Sync` |
| `i18n_ui.rs` | test | M4 WP-D (m4 §4.6, exit criterion E6): the `resolve_ui_lang` precedence (flag, `VA_LANG`, setting, locale variables, system language, default) with fakes; `system_ui_language`; the embedded root resources parse, keep their namespaces, define no term, and `en`/`fr` have the same ids and attributes; every `CONFIG_CODES` entry has its `diag-cfg-*` message; `Localizer::for_ui` with a fake CPU, including root/CPU id collisions |
| `paths.rs` | test | M4 WP-B (m4 §4.1): every level of the folder table with a fake environment and a fake `is_file` (variables, portable mode, OS folders and their fallbacks), the warnings for empty and relative values, no location gives `None`, folder creation in a test temp folder. One test is Windows-only and one Unix-only |
| `config_settings.rs` | test | M4 WP-B (m4 §4.2, §4.3): defaults, the three `DEFAULT_SETTINGS_TOML` checks, every key valid, of the wrong type, out of range or invalid (with its line and column), unknown keys, schema versions, syntax errors, the `read_config_text` limits, and the `diag-cfg-*` messages of `locales/*/config.ftl` |
| `config_keybindings.rs` | test | M4 WP-B (m4 §4.4): chord parsing (modifiers in any case, aliases, every `KEY_NAMES` entry), the keys kept by the editor, removals, replacement, file order, unknown commands, bad keys, `when`, and the messages of their problems |
| `config_project.rs` | test | M4 WP-B (m4 §4.5): the keys and their defaults, the reserved tables kept raw, paths that leave the project (`..`, absolute, rooted, drive prefixes) refused with `cfg-bad-path`, and `project_arch_problem` |

The `persist_*.rs` tests (atomic writes, session, lock, backups, and the kill test of exit
criterion E4) arrive with WP-C (m4 §10.1).

## Entry points
- `i18n_localizer.rs` for the rendering rules every caller depends on.
- `config_settings.rs` for the problem rules shared by every configuration file.

## Dependencies
- Internal: `va-workspace`, `va-core`.
- External: none beyond the crate's own (`fluent-bundle`, `unic-langid`, `toml`).

## Tests / usage
```bash
cargo test -p va-workspace
cargo test -p va-workspace --test i18n_localizer
cargo test -p va-workspace --test i18n_ui
cargo test -p va-workspace --test paths
```
On 2026-11-02 on Windows: `i18n_lang` 10 tests, `i18n_localizer` 22, `i18n_ui` 24, `paths` 16,
`config_settings` 31, `config_keybindings` 22, `config_project` 13; 138 in all, passing.
