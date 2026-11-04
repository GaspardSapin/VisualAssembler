# crates/va-ui/src

## Purpose
Sources of `va-ui` (m4-editor-shell.md §5). **WP0 scaffold (commit `b4e7aae`); implemented in wave 2
by WP-E and WP-F.** Implemented by WP-E (commit `2cc6e9e`): `editor/`, `theme.rs`, `fonts.rs`.
Still the WP0 scaffold: `shell/` (WP-F), whose `mod.rs` holds its frozen signatures with `todo!()`
bodies under a module-level `#![expect(…, clippy::todo, reason = "WP0 stub: …")]` listing exactly
the lints its stubs trigger; each lint goes in the commit that makes it stop firing, and the
attribute with the last `todo!()` (m4 §15.1 M3).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate doc (rules R2 and R3, allowed internal dependencies); declares `editor`, `fonts`, `shell`, `theme`; re-exports the shell API of §5.1. Final |
| `theme.rs` | module | §5.4 (WP-E): `ThemeId`, `Theme` (editor colours, one per token role), `Theme::light`/`dark`/`from_id`, `token_color`, `visuals`, `effective_theme` (session override, then `ui.theme`, then the system theme), `contrast_ratio` (WCAG 2.x; every token colour at least 4.5 against the background and the current line) |
| `fonts.rs` | module | §5.5 (WP-E): `CODE_FONT_FAMILY` (`va-code`), `EMBEDDED_CODE_FONT` and `EMBEDDED_FONT_LICENSE` (`include_bytes!`/`include_str!` of `assets/fonts/`), `font_definitions` (custom font, then the embedded one, then egui's monospace fallbacks), `code_font`, `setup_context` (egui's keyboard zoom off), `validate_font`, `FontError`, `MAX_FONT_BYTES` (32 MiB), the zoom bounds, `clamp_zoom`, `step_zoom` |
| `editor/` | folder | The virtualized, lexer-coloured editor widget (§5.2–§5.6, WP-E). See its README. |
| `shell/` | folder | The window content and its command registry (§5.1, §5.7–§5.13, WP-F). See its README. |

## Entry points
- `lib.rs`: `Shell` and the rest of the §5.1 API.
- `editor/mod.rs`: `EditorView::show`.

## Dependencies
- Internal: `va-core` (`arch`, `syntax`), `va-editor` (`document`, `edit`), `va-workspace`
  (`config`, `i18n`, `paths`, `persist`).
- External: `egui`.

## Tests / usage
```bash
cargo test -p va-ui
```
Tests live in `crates/va-ui/tests/` (see its README), never in `src/` (architecture §8).
