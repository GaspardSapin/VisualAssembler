# crates/va-workspace/src

## Purpose
Sources of `va-workspace`. The `i18n` module is implemented (M1). `paths`, `config` and `persist`
are the M4 WP0 scaffold (m4-editor-shell.md §4): frozen signatures with `todo!()` bodies,
implemented in wave 2 by WP-B (`paths`, `config`) and WP-C (`persist`). Each stub module starts
with `#![expect(…, clippy::todo, reason = "WP0 stub: …")]` listing exactly the lints its stubs
trigger; each lint goes in the commit that makes it stop firing, and the attribute with the last
`todo!()` (m4 §15.1 M3).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate doc (responsibility, allowed internal dependency `va-core`) and `pub mod config`, `i18n`, `paths`, `persist` |
| `paths.rs` | module | §4.1 (WP-B, frozen): `Paths` (config and data folders, portable flag), `PathInputs` (OS, executable folder, environment and file checks injected for tests), `PathWarning`, `resolve_paths`: `VA_CONFIG_DIR`/`VA_DATA_DIR`, then a `portable` file next to the executable, then the OS folders. Stub |
| `i18n/` | folder | Language selection and the Fluent localizer (M1). See its README. |
| `config/` | folder | `settings.toml`, `keybindings.toml`, `.visualasm/project.toml` and their problems (M4 scaffold, WP-B). See its README. |
| `persist/` | folder | The `va-io` worker, atomic writes, session, lock and hot-exit backups (M4 scaffold, WP-C). See its README. |

## Entry points
- `i18n/mod.rs` (M1). From M4: `paths::resolve_paths`, `config::parse_settings`,
  `persist::IoWorker`.

## Dependencies
- Internal: `va-core`.
- External: `fluent-bundle`, `unic-langid` (`i18n/`); `toml` (`config/`); `serde` and
  `sys-locale` (declared for WP-B, WP-C and WP-D, not used by the scaffold).

## Tests / usage
```bash
cargo test -p va-workspace
```
Tests live in `crates/va-workspace/tests/`, never in `src/` (architecture §8).
