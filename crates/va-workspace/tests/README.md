# crates/va-workspace/tests

## Purpose
Tests of `va-workspace`, one file per source module, named after its path
(`src/i18n/lang.rs` → `i18n_lang.rs`; architecture §8). In M1 they cover the i18n module only. They
use synthetic Fluent resources and a fake environment, so they need no CPU crate and read no real
environment variable.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `i18n_lang.rs` | test | `Lang` codes and `parse`; the `resolve_lang` precedence (`--lang`, `VA_LANG`, then `LC_ALL`/`LC_MESSAGES`/`LANG`) through a fake `var` closure, including empty, differently-cased and invalid `VA_LANG` values |
| `i18n_localizer.rs` | test | Fallback to English, missing ids (`None`, or the code in `render`), arguments and plurals, no isolation marks, parse and duplicate errors on synthetic resources, `for_architecture`, `Send + Sync` |

## Entry points
- `i18n_localizer.rs` for the rendering rules every caller depends on.

## Dependencies
- Internal: `va-workspace`, `va-core`.
- External: none beyond the crate's own (`fluent-bundle`, `unic-langid`).

## Tests / usage
```bash
cargo test -p va-workspace
cargo test -p va-workspace --test i18n_localizer
```
