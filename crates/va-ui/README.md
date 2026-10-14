# crates/va-ui

## Purpose
Package `va-ui`. egui widgets and panels of the IDE: editor view, gutter, tabs, palette, registers, memory, problems. Depends on `egui` only, never on `eframe`, `winit` or `wgpu` (rule R3).

**M0 skeleton:** no logic yet; the first content arrives in M4.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Package template of m0-foundations.md §3.6: workspace version, edition, licence and lints; no dependency in M0 |
| `src/` | folder | Crate sources. See its README. |

## Entry points
- `src/lib.rs`: the crate documentation, which states the responsibility and the allowed internal dependencies.

## Dependencies
- Internal (allowed): `va-core`, `va-editor`, `va-lang`, `va-debug`, `va-workspace`. Declared only when first used (m0-foundations.md §3.6).
- External: none in M0. Planned (m0-foundations.md §13.3): `egui`. `egui` and `egui_kittest` are the only GUI crates `va-ui` may use. Every addition goes through the Architect, an exact
  pin in the root `[workspace.dependencies]`, `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va-ui
```
In M0 this runs no test. The `tests/` folder appears with the first test (M4).
