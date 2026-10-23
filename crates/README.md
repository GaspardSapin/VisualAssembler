# crates

## Purpose
The CPU-agnostic library crates of VisualAssembler. CPU-specific code lives in `arch/`, binaries in
`apps/`. At the end of M1, `va-core` (the frozen contracts) and the `i18n` module of `va-workspace`
are implemented; `va-editor`, `va-lang`, `va-debug` and `va-ui` are still skeletons (a manifest and a
`lib.rs` with only a doc comment).

## Contents
Copy of architecture.md §2 (the source of truth; update both together):

| Package | Folder | Responsibility | Allowed internal dependencies | First code |
|---------|--------|----------------|-------------------------------|------------|
| `va-core` | `va-core/` | CPU-agnostic traits and data types. No I/O, no threads, no UI, no user-facing text. | none | M1 (done) |
| `va-editor` | `va-editor/` | Text buffer (ropey), selections, grouped undo/redo, search/replace. No UI. | `va-core` | M4 |
| `va-lang` | `va-lang/` | Language services and the background-assembly service. No UI. | `va-core`, `va-editor` | M5 |
| `va-debug` | `va-debug/` | Debug session worker that owns a `dyn Emulator`. No UI. | `va-core` | M3 |
| `va-workspace` | `va-workspace/` | Project, settings, session, backups, file watching, Fluent localization. No UI. | `va-core` | M1 (i18n), M4 (the rest) |
| `va-ui` | `va-ui/` | egui widgets and panels. `egui` only, never `eframe`/`winit`/`wgpu`. | `va-core`, `va-editor`, `va-lang`, `va-debug`, `va-workspace` | M4 |

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
- `va-workspace/src/i18n/mod.rs`: the localizer shared by the CLI and, from M4, the GUI.

## Dependencies
- Internal: see the table.
- External: `fluent-bundle` and `unic-langid` in `va-workspace` (M1). See `DEPENDANCE.md`.

## Tests / usage
```bash
cargo test -p va-core -p va-editor -p va-lang -p va-debug -p va-workspace -p va-ui
```
In M1 this runs the tests of `va-core/tests/` and `va-workspace/tests/`; the four skeleton crates
have no test yet. A new crate needs: a folder here, an entry in the root `[workspace].members`, a
row in the layering table of `xtask/src/layering.rs` (an unknown package fails `cargo xtask ci`), a
README and a parent-README line.
