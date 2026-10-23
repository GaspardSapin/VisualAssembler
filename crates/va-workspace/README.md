# crates/va-workspace

## Purpose
Package `va-workspace`. Project model and tree, settings and keybindings TOML, session and hot-exit
backups, file watching, Fluent loading and rendering (architecture §5 and §6). No UI.

**M1 (implemented):** the `i18n` module only: language selection (`--lang`, `VA_LANG`, locale
variables) and the Fluent localizer that renders diagnostics and instruction documentation
(m1-core-isa-lexer.md §6.2). `va` uses it. **Not implemented yet:** project, settings, keybindings,
session, backup and watch modules (M4).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Dependencies `va-core`, `fluent-bundle`, `unic-langid` (all `workspace = true`, m1 §8) |
| `src/` | folder | Crate sources. See its README. |
| `tests/` | folder | One test file per source module. See its README. |

## Entry points
- `src/i18n/mod.rs`: `resolve_lang` and `Localizer`.

## Dependencies
- Internal (allowed): `va-core`.
- External: `fluent-bundle` 0.16.0 and `unic-langid` 0.9.6 (`DEPENDANCE.md`, `IMPORT.md`).
  Planned (m0-foundations.md §13.3): `toml`, `directories`, `notify` in M4; `directories` and
  `notify` need per-crate licence exceptions (§9). Every addition goes through the Architect, an
  exact pin in the root `[workspace.dependencies]`, `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va-workspace
```
