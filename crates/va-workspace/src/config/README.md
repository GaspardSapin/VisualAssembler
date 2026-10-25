# crates/va-workspace/src/config

## Purpose
The configuration files of the IDE and their problems (m4-editor-shell.md §4.2–§4.5):
`settings.toml` and `keybindings.toml` in the config folder, and `.visualasm/project.toml` in a
project. Every file is capped at 1 MiB (`MAX_CONFIG_BYTES`) and parsed with spans, then checked key
by key: a bad key never discards its neighbours, and every problem becomes a non-blocking
`ConfigProblem` with its position and a `cfg-*` code.

**WP0 scaffold (wave 1, commits `b4e7aae` and `570afb1`); implemented in wave 2 by WP-B.** The
settings types of §4.3 (frozen 2026-10-24), their `Default` impls, the enum `id`/`from_id`,
`CONFIG_CODES`, `KEY_NAMES` and the default settings file are final. The readers and parsers
(`read_config_text`, `parse_settings`, `parse_chord`, `parse_keybindings`, `parse_project`, …) are
`todo!()` stubs and panic when called.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | Problems (§4.2): `MAX_CONFIG_BYTES`, `ConfigProblem`, the 12 `CONFIG_CODES`, `read_config_text`. Settings (§4.3): `Settings` and its `UiSettings`, `EditorSettings`, `FileSettings`, `ExplorerSettings`, `RenderSettings`; `ThemeKind`, `EolKind`, `RendererKind`, `AutosaveMode`; `parse_settings`; `DEFAULT_SETTINGS_TOML`. Keybindings (§4.4): `KeyChord`, `When`, `Binding`, `KEY_NAMES` (83), `parse_chord`, `parse_keybindings`. Project (§4.5): `Project`, `parse_project`, `PROJECT_FILE` |
| `default_settings.toml` | data (TOML) | The commented default `settings.toml`: `schema_version = 1`, then every key of §4.3 commented out with its default value and a one-line explanation. Embedded as `DEFAULT_SETTINGS_TOML` (`include_str!`); the app writes it once when the file is missing and never rewrites it |

WP-B splits the items into the private modules `problems.rs`, `settings.rs`, `keybindings.rs` and
`project.rs` (m4 §2) by the end of its package, and re-exports each frozen item by name from
`mod.rs` (`pub use`, no glob), so the paths `va_workspace::config::*` never move (m4 §15.1 M2).

## Entry points
- `parse_settings(path, text)`, `parse_keybindings(…)`, `parse_project(root, text)`: each returns
  the parsed value and its problems, never an error.
- `DEFAULT_SETTINGS_TOML`: what a new `settings.toml` contains. Any change to a key or a default
  in §4.3 updates this file in the same change (m4 §4.3 checks it in
  `tests/config_settings.rs`, WP-B).

`mod.rs` carries `#![expect(unused_variables, clippy::todo, reason = "WP0 stub: …")]`. A listed
lint that stops firing raises `unfulfilled_lint_expectations`, which CI denies: WP-B removes each
lint in the commit that makes it stop firing and the attribute with the last `todo!()`, never
replacing it by `allow`; a new stub file carries its own expectation (m4 §15.1 M3). The
`diag-cfg-*` messages live in `locales/{en,fr}/config.ftl` (WP-B).

## Dependencies
- Internal: `va-core` (`diag`, `source`), `crate::i18n` (`Lang`).
- External: `toml` (`toml::Table` keeps the reserved project tables raw; WP-B parses with spans)
  and `serde` (declared for WP-B, not used by the scaffold). See `DEPENDANCE.md`.

## Tests / usage
```bash
cargo test -p va-workspace
```
In the scaffold this runs only the M1 `i18n` tests. WP-B adds `tests/paths.rs`,
`tests/config_settings.rs`, `tests/config_keybindings.rs` and `tests/config_project.rs` (m4 §10.1).
