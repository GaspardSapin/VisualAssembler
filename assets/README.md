# assets

## Purpose
Data files compiled into the binaries: fonts now; icons and themes may follow (architecture §11).
Everything here is embedded with `include_bytes!`/`include_str!` by the crate that uses it, so
nothing is read from disk at run time. Third-party files carry their licence next to them and are
recorded in `DEPENDANCE.md` and `IMPORT.md`, because cargo-deny does not see them.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `fonts/` | folder | JetBrains Mono 2.304 Regular and its OFL 1.1 licence, the embedded code font of the editor (M4). See its README for the source, sizes and SHA-256 hashes. |

## Entry points
- `crates/va-ui/src/fonts.rs`, which embeds `fonts/`.

A new asset needs the Architect (and the user, for a download), a line in `DEPENDANCE.md` and an
entry in `IMPORT.md` when it comes from a third party, and a README line here.

## Dependencies
- Internal: used by `va-ui` only.
- External: none.

## Tests / usage
```bash
cargo test -p va-ui
```
This builds `va-ui`, which embeds the files. In the M4 scaffold it runs no test yet; the font
tests (`crates/va-ui/tests/fonts.rs`) arrive with WP-E.
