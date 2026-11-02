# crates/va-ui/tests

## Purpose
The headless kittest suite of `va-ui` (m4-editor-shell.md §10.2, exit criteria E5 and E6). Tests
drive egui through `egui_kittest::Harness` and read the AccessKit tree: no window, no GPU. They use
a fake CPU instead of `arch/*` (rule R2 forbids that edge, dev-dependencies included).

**M4, wave 2 in progress.** `locales.rs` is in (WP-D, commit `1966bd7`) with the shared
`support/` module of the WP0 scaffold (commit `b4e7aae`). Decided, not written yet: WP-E adds
`editor_view.rs`, `editor_input.rs`, `editor_cache.rs`, `theme.rs` and `fonts.rs`, WP-F the
`shell_*.rs` files (tabs, explorer, palette, find, status, hot exit) (m4 §2, §10.2).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `locales.rs` | test | UI i18n completeness (m4 §4.6, exit criterion E6), no harness: every `ui-*`/`cmd-*` string literal of `crates/va-ui/src/` and `apps/visual-assembler/src/` (a text scan) and every command title of the §5.12 registry exist in `en` and `fr`; no `ui-*`/`cmd-*` id is unused; both languages define the same ids; the registry table matches `shell/commands.rs` once it exists; no CPU resource under `arch/*/locales/` collides with the root ones in `Localizer::for_ui`. On today's header-only `ui.ftl` and `commands.ftl` every check runs and holds, and tightens as WP-F fills them |
| `support/` | folder | Shared test code: the fake lexer and architecture, documents and harness helpers. See its README. |

## Entry points
- `support/mod.rs`, included by each kittest file with `mod support;` (`locales.rs` does not
  need it).

## Dependencies
- Internal: `va-ui`, `va-core`, `va-editor`, `va-workspace` (`i18n`, in `locales.rs`).
- External: `egui_kittest` (dev-dependency), `egui`.

## Tests / usage
```bash
cargo test -p va-ui
cargo test -p va-ui --test locales
```
On 2026-11-02 `locales.rs` ran 7 tests, all passing; it is the only test target so far.
