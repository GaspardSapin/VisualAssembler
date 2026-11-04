# crates/va-ui

## Purpose
Package `va-ui`. egui widgets and panels of the IDE (m4-editor-shell.md §5): the editor view, the
shell (tabs, explorer, palette, find bar, status bar), themes and fonts; registers, memory and
problems panels in later milestones. Depends on `egui` only, never on `eframe`, `winit` or `wgpu`
(rule R3); CPU specifics arrive only as `&dyn Architecture` (rule R2).

**M4 WP0 scaffold (wave 1, commit `b4e7aae`); implemented in wave 2 by WP-E (editor, theme, fonts)
and WP-F (shell).** WP-E is in (commit `2cc6e9e`): the virtualized editor view, its colouring cache,
input and caret, the themes and the fonts. The shell is still the WP0 scaffold: its frozen types
are final, its functions are `todo!()` stubs and panic when called.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Dependencies `egui`, `va-core`, `va-editor`, `va-workspace`; dev-dependency `egui_kittest` (all `workspace = true`, m4 §8.2) |
| `src/` | folder | Crate sources: `editor/`, `shell/`, `theme.rs`, `fonts.rs`. See its README. |
| `tests/` | folder | The headless kittest suite: the shared `support/` module, `locales.rs` (UI i18n completeness, WP-D) and the editor, theme and font tests (WP-E) so far. See its README. |

## Entry points
- `src/lib.rs`: the shell API of m4 §5.1, re-exported at the crate root (`Shell`, `ShellConfig`,
  `ShellHost`, …), which `visual-assembler` calls (WP-G).

## Dependencies
- Internal (allowed): `va-core`, `va-editor`, `va-lang`, `va-debug`, `va-workspace`. Declared now:
  `va-core`, `va-editor`, `va-workspace`.
- External: `egui` 0.36.2, a direct dependency since M4 (the same version eframe pulls), and the
  dev-dependency `egui_kittest` 0.36.2. These are the only GUI crates `va-ui` may use. Every
  addition goes through the Architect, an exact pin in the root `[workspace.dependencies]`,
  `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va-ui
```
On 2026-11-04 on Linux this ran 58 tests in six targets, all passing; the shell tests arrive
with WP-F.
