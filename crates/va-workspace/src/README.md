# crates/va-workspace/src

## Purpose
Sources of `va-workspace`. In M1 the crate holds the `i18n` module only; the other modules of
architecture §6 arrive in M4.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate doc (responsibility, allowed internal dependency `va-core`) and `pub mod i18n` |
| `i18n/` | folder | Language selection and the Fluent localizer. See its README. |

## Entry points
- `i18n/mod.rs`. Projected layout (m1 §2.2): `project/`, `settings/`, `keybindings/`, `session/`,
  `backup/` and `watch/` in M4, with the architecture-independent UI strings in Fluent files.

## Dependencies
- Internal: `va-core`.
- External: `fluent-bundle`, `unic-langid` (used by `i18n/`).

## Tests / usage
```bash
cargo test -p va-workspace
```
Tests live in `crates/va-workspace/tests/`, never in `src/` (architecture §8).
