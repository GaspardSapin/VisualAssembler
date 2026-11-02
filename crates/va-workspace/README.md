# crates/va-workspace

## Purpose
Package `va-workspace`. Project model, settings and keybindings TOML, session and hot-exit backups,
Fluent loading and rendering (architecture §5 and §6; m4-editor-shell.md §4). No UI; it knows
nothing of ropey or egui: text crosses its API as `&str` or as a writer closure.

**M1 (implemented):** the `i18n` module: language selection (`--lang`, `VA_LANG`, locale variables)
and the Fluent localizer that renders diagnostics and instruction documentation
(m1-core-isa-lexer.md §6.2). `va` uses it. **M4, implemented in wave 2:** the GUI additions to
`i18n` (WP-D, commit `1966bd7`: `ui_resources`, `Localizer::for_ui`, `resolve_ui_lang` with the
`ui.language` setting and, on Windows, `system_ui_language`), `paths` and `config` (WP-B, commit
`1cf9729`: the config and data folders with portable mode, and the readers of `settings.toml`,
`keybindings.toml` and `.visualasm/project.toml`). **M4 WP0 scaffold (wave 1, commits `b4e7aae`
and `570afb1`); implemented in wave 2 by WP-C:** `persist` holds its frozen types and signatures
with `todo!()` bodies; the session types and their defaults are final. Nothing outside the crate
calls the M4 parts yet: the GUI wiring is WP-G. File watching is deferred (architecture §4;
`notify` was rejected for M4, m4 §8.3).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Dependencies `va-core`, `fluent-bundle`, `unic-langid` (m1 §8), `serde`, `toml`, and `sys-locale` on Windows only (m4 §8.2); all `workspace = true` |
| `src/` | folder | Crate sources. See its README. |
| `tests/` | folder | One test file per source module. See its README. |

## Entry points
- `src/i18n/mod.rs`: `resolve_lang` and `Localizer` (CLI), `resolve_ui_lang` and
  `Localizer::for_ui` (GUI).
- `src/paths.rs` (`resolve_paths`) and `src/config/mod.rs` (`parse_settings`,
  `parse_keybindings`, `parse_project`, `DEFAULT_SETTINGS_TOML`).
- `src/persist/mod.rs` (`IoWorker`, `write_atomic`): stubs until WP-C.

## Dependencies
- Internal (allowed): `va-core`.
- External: `fluent-bundle` 0.16.0 and `unic-langid` 0.9.6 (M1); `toml` 1.1.8 and `serde` 1.0.229
  for the config, session and backup files, and `sys-locale` 0.3.2 for the Windows UI language
  (M4, approved by the user on 2026-10-24, m4 §8.3). `toml` makes `va` link the TOML crates too,
  although `va` reads no TOML. See `DEPENDANCE.md` and `IMPORT.md`. Every addition goes through
  the Architect, an exact pin in the root `[workspace.dependencies]`, `DEPENDANCE.md` and
  `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va-workspace
```
On 2026-11-02 this ran 138 tests in seven targets on Windows, all passing (`paths.rs` holds one
Windows-only and one Unix-only test). The `persist` tests arrive with WP-C.
