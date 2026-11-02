# crates/va-workspace/src/config

## Purpose
The configuration files of the IDE and their problems (m4-editor-shell.md §4.2–§4.5):
`settings.toml` and `keybindings.toml` in the config folder, and `.visualasm/project.toml` in a
project. Every file is capped at 1 MiB (`MAX_CONFIG_BYTES`) and parsed with spans, then checked key
by key: a bad key never discards its neighbours, and every problem becomes a non-blocking
`ConfigProblem` with its position and a `cfg-*` code, whose message is the Fluent `diag-<code>`
of `locales/{en,fr}/config.ftl`.

**Implemented in wave 2 by WP-B (commit `1cf9729`).** The WP0 scaffold (commits `b4e7aae` and
`570afb1`) held everything in `mod.rs`; WP-B split it into four private files and removed the stub
expectation with the last `todo!()`. `mod.rs` now holds only the module doc and the `pub use` of
each frozen item by name (no glob), so the paths `va_workspace::config::*` never moved
(m4 §15.1 M2). The settings types of §4.3 (frozen 2026-10-24) and their defaults are unchanged.
Nothing outside the crate calls the parsers yet: the GUI reads these files from WP-G.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | Module doc; declares the four private modules and re-exports their public items |
| `problems.rs` | module | §4.2: `MAX_CONFIG_BYTES`, `ConfigProblem` (file, code, byte range after a stripped BOM, 1-based line and column, arguments; `to_diagnostic` gives a non-blocking warning), the 12 `CONFIG_CODES`, `read_config_text` (missing file is `Ok(None)`; unreadable, too large or non-UTF-8 is one problem); the private span-aware helpers every parser reports through |
| `settings.rs` | module | §4.3: `Settings` and its `UiSettings`, `EditorSettings`, `FileSettings`, `ExplorerSettings`, `RenderSettings`; `ThemeKind`, `EolKind`, `RendererKind`, `AutosaveMode` with `id`/`from_id`; `parse_settings` (the defaults overridden by every valid key; a syntax error or a `schema_version` other than 1 gives the defaults and that one problem); `DEFAULT_SETTINGS_TOML` |
| `keybindings.rs` | module | §4.4: `KeyChord`, `When`, `Binding`, `KEY_NAMES` (83), `parse_chord` (modifiers and keys in any case, aliases such as `Esc`), `is_reserved_chord` (the motion, editing and clipboard keys the editor keeps), `parse_keybindings` (defaults, then `-<id>` removals, then user bindings in file order) |
| `project.rs` | module | §4.5: `PROJECT_FILE`, `Project` (the later milestones' tables kept raw), `parse_project` (every path relative to the project root, without `..`, a drive or a leading separator, else `cfg-bad-path`), `project_arch_problem` (an `arch` that names no registered CPU, checked by the app) |
| `default_settings.toml` | data (TOML) | The commented default `settings.toml`: `schema_version = 1`, then every key of §4.3 commented out with its default value and a one-line explanation. Embedded as `DEFAULT_SETTINGS_TOML` (`include_str!`); the app writes it once when the file is missing and never rewrites it |

## Entry points
- `parse_settings(path, text)`, `parse_keybindings(…)`, `parse_project(root, text)`: each returns
  the parsed value and its problems, never an error. `text` is `None` when the file is missing;
  `read_config_text` gives it.
- `DEFAULT_SETTINGS_TOML`: what a new `settings.toml` contains. Any change to a key or a default
  in §4.3 updates this file in the same change; `tests/config_settings.rs` checks it (m4 §4.3).

## Dependencies
- Internal: `va-core` (`diag`, `source`), `crate::i18n` (`Lang`).
- External: `toml` (the spanned `DeTable` parser; `toml::Table` keeps the reserved project tables
  raw) and `serde` (`Deserialize` of those tables). See `DEPENDANCE.md`.

## Tests / usage
```bash
cargo test -p va-workspace --test config_settings
cargo test -p va-workspace --test config_keybindings
cargo test -p va-workspace --test config_project
```
On 2026-11-02 they ran 31, 22 and 13 tests, all passing. The `diag-cfg-*` messages are also
checked by `tests/i18n_ui.rs` (every `CONFIG_CODES` entry has its message in both languages).
