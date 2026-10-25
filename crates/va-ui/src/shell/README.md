# crates/va-ui/src/shell

## Purpose
The window content of the IDE (m4-editor-shell.md §5.1, §5.7–§5.13): tab bar, explorer, editor,
status bar, command palette, quick open and find bar, driven by a command registry. The shell
owns no window: `visual-assembler` builds a `Shell` from a `ShellConfig` and a `ShellHost` (native
dialogs), then calls `Shell::ui` every frame. Every file job goes to the `va-io` worker, so
`Shell::new` does no I/O.

**WP0 scaffold (wave 1, commit `b4e7aae`); implemented in wave 2 by WP-F.** `mod.rs` holds the
frozen API of §5.1 with `todo!()` bodies (it panics when used); the data types are final.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | The shell API, re-exported at the crate root: `ArchSupport`, `RendererNote`, `ReadOnlyReason`, `Persistence`, `ShellConfig`, `DialogRequest`, `DialogReply`, the `ShellHost` trait, `Shell` (`new`, `ui`, `on_close_requested`, `shutdown`, `execute`, `go_to_line`, `open_path`, `bench_probe`) and `BenchProbe` |

WP-F adds `state.rs`, `tabs.rs`, `explorer.rs`, `palette.rs`, `fuzzy.rs`, `find.rs`, `status.rs`,
`commands.rs`, `keymap.rs` and `prompts.rs` here (m4 §2; 11 files and this README at the end of
M4).

## Entry points
- `Shell::new(config, host)`, then `Shell::ui(ui)` once per frame.
- `Shell::execute(command)`: runs a command id of the registry (m4 §5.12).

`mod.rs` carries `#![expect(unused_variables, dead_code, clippy::todo, reason = "WP0 stub: …")]`;
a listed lint that stops firing raises `unfulfilled_lint_expectations`, which CI denies. WP-F
removes each lint in the commit that makes it stop firing and the attribute with the last
`todo!()`, never replacing it by `allow`; a new stub file carries its own expectation (m4 §15.1 M3).
The §5.1 items stay public at both `va_ui::*` and `va_ui::shell::*` (m4 §15.1 M5).

## Dependencies
- Internal: `va-core` (`arch`), `va-workspace` (`config`, `i18n`, `paths`, `persist`),
  `crate::editor`, `crate::theme`, `crate::fonts`.
- External: `egui`.

## Tests / usage
```bash
cargo test -p va-ui
```
In the scaffold this runs no test. WP-F adds `crates/va-ui/tests/shell_*.rs` (tabs, explorer,
palette, find, status, hot exit; m4 §10.2), each with its own helpers.
