# crates/va-ui/tests

## Purpose
The headless kittest suite of `va-ui` (m4-editor-shell.md §10.2, exit criteria E5 and E6). Tests
drive egui through `egui_kittest::Harness` and read the AccessKit tree: no window, no GPU. They use
a fake CPU instead of `arch/*` (rule R2 forbids that edge, dev-dependencies included).

**M4, wave 2 in progress.** In: `locales.rs` (WP-D, commit `1966bd7`), the shared `support/`
module of the WP0 scaffold (commit `b4e7aae`), and the editor, theme and font tests (WP-E, commit
`2cc6e9e`). Decided, not written yet: WP-F adds the `shell_*.rs` files (tabs, explorer, palette,
find, status, hot exit) (m4 §2, §10.2).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `locales.rs` | test | UI i18n completeness (m4 §4.6, exit criterion E6), no harness: every `ui-*`/`cmd-*` string literal of `crates/va-ui/src/` and `apps/visual-assembler/src/` (a text scan) and every command title of the §5.12 registry exist in `en` and `fr`; no `ui-*`/`cmd-*` id is unused; both languages define the same ids; the registry table matches `shell/commands.rs` once it exists; no CPU resource under `arch/*/locales/` collides with the root ones in `Localizer::for_ui`. On today's header-only `ui.ftl` and `commands.ftl` every check runs and holds, and tightens as WP-F fills them |
| `editor_view.rs` | test | WP-E (m4 §5.2, exit criterion E5): only visible lines are drawn on 100,000 lines, content height, scrolling to the caret, long lines, gutter width, the AccessKit label, zoom and style changes, tabs, `PlainSyntax`, the paint order, autoscroll while dragging |
| `editor_input.rs` | test | WP-E (m4 §5.6, exit criterion E5): text, Enter and Backspace, arrows with Shift and Ctrl, PageDown, Tab and the focus, Escape, clipboard with two carets, read-only, clicks, double and triple clicks, dragging, IME, and the reactive blink rule |
| `editor_cache.rs` | test | WP-E (m4 §5.3, exit criterion E5): a warm frame lexes nothing, one keystroke re-lexes exactly one line, scrolling back hits, style, theme, font and tab changes clear the cache, `LINE_CACHE_CAPACITY` holds |
| `theme.rs` | test | WP-E (m4 §5.4): WCAG contrast of every token colour, the exhaustive token mapping, the palettes of the spec table, the visuals, `effective_theme`, theme switching headless |
| `fonts.rs` | test | WP-E (m4 §5.5), no harness: the `va-code` family order (custom, embedded, egui fallbacks), font validation, the embedded licence, `setup_context`, zoom steps, the key names of `keybindings.toml` |
| `support/` | folder | Shared test code: the fake lexer and architecture, documents, harness and editor helpers. See its README. |

## Entry points
- `support/mod.rs`, included by each kittest file with `mod support;` (`locales.rs` and
  `fonts.rs` do not need it).

## Dependencies
- Internal: `va-ui`, `va-core`, `va-editor`, `va-workspace` (`i18n`, in `locales.rs`).
- External: `egui_kittest` (dev-dependency), `egui`.

## Tests / usage
```bash
cargo test -p va-ui
cargo test -p va-ui --test locales
```
On 2026-11-04 on Linux the suite ran 58 tests, all passing: `editor_cache` 7, `editor_input` 15,
`editor_view` 13, `fonts` 8, `locales` 7, `theme` 8.
