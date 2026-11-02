# crates/va-workspace/src

## Purpose
Sources of `va-workspace`. The `i18n` module is implemented (M1, extended for the GUI by M4 WP-D),
and so are `paths` and `config` (M4 WP-B, m4-editor-shell.md §4.1–§4.5). `persist` is still the
M4 WP0 scaffold (m4 §4.7–§4.8): frozen signatures with `todo!()` bodies, implemented in wave 2 by
WP-C. Its `mod.rs` starts with `#![expect(…, clippy::todo, reason = "WP0 stub: …")]` listing
exactly the lints its stubs trigger; each lint goes in the commit that makes it stop firing, and
the attribute with the last `todo!()` (m4 §15.1 M3).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate doc (responsibility, allowed internal dependency `va-core`) and `pub mod config`, `i18n`, `paths`, `persist` |
| `paths.rs` | module | §4.1 (WP-B, frozen): `Paths` (config and data folders, portable flag, the file paths and the `create_*_dir` helpers used on first write), `PathInputs` (OS, executable folder, environment and file checks injected for tests), `PathWarning`, `resolve_paths`: each folder takes `VA_CONFIG_DIR`/`VA_DATA_DIR`, then a `portable` file next to the executable, then the OS folders (on Windows `%APPDATA%` and `%LOCALAPPDATA%`, else under `%USERPROFILE%`; elsewhere `$XDG_CONFIG_HOME` and `$XDG_DATA_HOME`, else under `$HOME`); empty or relative values are skipped with a warning |
| `i18n/` | folder | Language selection, the Fluent localizer and the GUI's resources and precedence. See its README. |
| `config/` | folder | `settings.toml`, `keybindings.toml`, `.visualasm/project.toml` and their problems (WP-B). See its README. |
| `persist/` | folder | The `va-io` worker, atomic writes, session, lock and hot-exit backups (M4 scaffold, WP-C). See its README. |

## Entry points
- `i18n/mod.rs`: `resolve_lang`, `resolve_ui_lang`, `Localizer`.
- `paths::resolve_paths`, `config::parse_settings`; `persist::IoWorker` once WP-C lands.

## Dependencies
- Internal: `va-core`.
- External: `fluent-bundle`, `unic-langid` and, on Windows, `sys-locale` (`i18n/`); `toml` and
  `serde` (`config/`, and `persist/` from WP-C).

## Tests / usage
```bash
cargo test -p va-workspace
```
Tests live in `crates/va-workspace/tests/`, never in `src/` (architecture §8).
