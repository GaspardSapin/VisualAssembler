# crates

## Purpose
The CPU-agnostic library crates of VisualAssembler. CPU-specific code lives in `arch/`, binaries in
`apps/`. At the end of M1, `va-core` (the frozen contracts) and the `i18n` module of `va-workspace`
are implemented. During M2 ∥ M4: in wave 1, `va-core` gained the frozen assembler and
disassembler contracts (M2 WP-A), and `va-editor`, `va-ui` and the rest of `va-workspace` got the
M4 WP0 scaffold, frozen signatures with `todo!()` bodies. Wave 2 so far implemented `va-editor`
(WP-A), all of `va-workspace` (`paths`, `config` by WP-B, the GUI localization by WP-D,
`persist` by WP-C) and the editor view, themes and fonts of `va-ui` (WP-E); still stubs: the
`va-ui` shell (WP-F). `va-lang` and
`va-debug` are still skeletons (a manifest and a `lib.rs` with only a doc comment).

## Contents
Copy of architecture.md §2 (the source of truth; update both together):

| Package | Folder | Responsibility | Allowed internal dependencies | First code |
|---------|--------|----------------|-------------------------------|------------|
| `va-core` | `va-core/` | CPU-agnostic traits and data types. No I/O, no threads, no UI, no user-facing text. | none | M1 (done), M2 contracts (done) |
| `va-editor` | `va-editor/` | Text buffer (ropey), selections, grouped undo/redo, search/replace. No UI. | `va-core` | M4 (done in wave 2) |
| `va-lang` | `va-lang/` | Language services and the background-assembly service. No UI. | `va-core`, `va-editor` | M5 |
| `va-debug` | `va-debug/` | Debug session worker that owns a `dyn Emulator`. No UI. | `va-core` | M3 |
| `va-workspace` | `va-workspace/` | Paths and portable mode, project, settings and keybindings TOML, session and its lock, hot-exit backups, the `va-io` worker, Fluent localization. File watching is deferred. No UI. | `va-core` | M1 (i18n), M4 (GUI i18n, paths, config, persist done) |
| `va-ui` | `va-ui/` | egui widgets and panels. `egui` only, never `eframe`/`winit`/`wgpu`. | `va-core`, `va-editor`, `va-lang`, `va-debug`, `va-workspace` | M4 (editor, themes, fonts done; shell scaffold) |

The other packages: `va-arch-m68k` (`arch/m68k`, allowed `va-core`); `visual-assembler` (every crate
and `arch/*`); `va` (every crate except `va-ui`, plus `arch/*`); `xtask` (none).

```text
visual-assembler ──► va-ui ──► va-lang ──► va-editor ──► va-core
       │               ├─────► va-debug ─────────────────► va-core
       │               └─────► va-workspace ─────────────► va-core
       └──► arch/* (va-arch-m68k, …) ────────────────────► va-core
va (CLI) ──► va-lang, va-debug, va-workspace, arch/* ────► va-core
xtask: no workspace dependency (drives cargo and the built binaries as processes)
```

Rules (architecture §2):
- **R1** Only the edges above. Dev-dependencies follow the same rules.
- **R2** `crates/*` never depends on `arch/*`; CPU specifics reach the core as trait objects.
- **R3** GUI crates (the `egui`, `eframe`, `epaint`, `emath`, `ecolor`, `wgpu`, `naga`, `winit`,
  `glutin`, `glow`, `accesskit` families and `raw-window-handle`) appear only in `visual-assembler`,
  plus exactly `egui` and `egui_kittest` in `va-ui`.
- **R4** `va` never depends on `va-ui` or a GUI crate. **R5** `xtask` depends on no workspace crate.
- **R6** External dependencies are pinned once, exactly (`=`), in the root `[workspace.dependencies]`;
  members write `name = { workspace = true }`.
- **R7** Apps and xtask are leaves: nothing depends on them.
- **R8** Every member has exactly `[lints] workspace = true` and `license.workspace = true`; build
  flags come only from `.cargo/config.toml`.

Enforcement: `cargo xtask ci` (layering step: R1–R5, R7; step 0: R8 build flags),
`scripts/check_project_docs.py` (R6, R8 lints, unlisted packages), cargo-deny (R8 licence).

## Entry points
- `va-core/src/lib.rs`: every other crate builds on its contracts (architecture §3, m1 §3).
- `va-workspace/src/i18n/mod.rs`: the localizer shared by the CLI and the GUI (`Localizer::for_ui`, M4).

## Dependencies
- Internal: see the table.
- External: `fluent-bundle` and `unic-langid` in `va-workspace` (M1); from M4, `ropey` and `regex`
  in `va-editor`, `serde`, `toml` and (Windows) `sys-locale` in `va-workspace`, `egui` and the
  dev-dependency `egui_kittest` in `va-ui`. See `DEPENDANCE.md`.

## Tests / usage
```bash
cargo test -p va-core -p va-editor -p va-lang -p va-debug -p va-workspace -p va-ui
```
This runs the tests of `va-core/tests/`, `va-editor/tests/`, `va-workspace/tests/` and
`va-ui/tests/` (only `locales.rs` there so far); `va-lang` and `va-debug` have no test yet. On
2026-11-02 the last three ran 102, 138 and 7 tests, all passing. A new crate needs: a folder here, an entry in the root `[workspace].members`, a
row in the layering table of `xtask/src/layering.rs` (an unknown package fails `cargo xtask ci`), a
README and a parent-README line.
