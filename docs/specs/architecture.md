# VisualAssembler — Architecture

| | |
|---|---|
| Status | Approved baseline (written for M0, durable afterwards) |
| Owner | Architect role |
| Date | 2026-10-13 |
| Scope | Global, long-lived decisions. Milestone specs (`docs/specs/mN-*.md`) refine this document and must not contradict it. |
| Change rule | Only an Architect spec revision changes this file. Every decision names one rejected alternative. |
| Amendments | 2026-10-24: §9.1 (m0 §20, CI runner labels); §1, §2, §3, §7, §8, §9.5 (`m2-assembler.md`); §2, §4, §5, §6, §7, §9.6, §9.8, §10.1, §11 (`m4-editor-shell.md`) |

The approved Phase 0 plan (kept by the project owner, outside the repository) is the origin of the
final technology choices: egui/eframe, ropey, an in-house lexer and assembler, the `m68k` crate behind
an `Emulator` trait, Fluent, TOML, Makefile → `cargo xtask`, GitHub Actions on native runners,
cargo-packager, and a cargo-deny network ban. This document restates everything developers need, so
they never have to open the plan.

---

## 1. Goals and non-goals

**Goals (V1)**

| # | Goal |
|---|------|
| G1 | A native IDE for students learning Motorola 68000 assembly: edit → assemble → run → debug, end to end. |
| G2 | Light and fast: startup < 500 ms, RAM < 500 MB with a 5,000-line project, ≈ 0 % CPU/GPU at idle, keystroke → frame < 16 ms, smooth on 100,000-line files (§7). |
| G3 | CPU-agnostic core: a new CPU means a new `arch/<cpu>` folder plus one registration line per app (§10.1). |
| G4 | Offline, no telemetry, enforced by tooling rather than by promise (§9.3). |
| G5 | Every feature reachable from a terminal (`va` CLI, `make`, `cargo xtask`), with reliable exit codes and `--json` output. |
| G6 | English and French UI and diagnostics. |
| G7 | Windows 11 and Linux, on x86_64 and aarch64. |

**Non-goals (V1).** Nothing here is designed against; each item can be reopened by a future spec.

- macOS (it may compile; it is never tested and never claimed).
- A web/wasm build, an LSP/DAP server, plugins or scripting.
- Separate compilation and linking (XREF across object files, Amiga hunk output). Multi-file programs use `INCLUDE`.
  - Like `a68k -s`, the assembler refuses `SECTION`, `CODE`, `DATA`, `BSS`, `XDEF` and `XREF` (and the aliases `CSEG`, `DSEG`,
    `PUBLIC`) with a blocking error that points to `ORG` (user decision, 2026-10-24; m2 §4.5, §13.3). *Rejected:* accepting
    sections as consecutive segments, which a68k's S-record mode refuses.
- Emulating the peripherals of real machines (Amiga, Atari ST). Only the I/O profiles of §3 are emulated.
- Auto-update or any other network feature.
- Shipping `a68k.exe` / `d68k.exe` (no redistribution licence). They remain optional external tools whose paths the user configures.
- Measuring latency all the way to the physical display. Latency is measured inside the application only.

---

## 2. Crate map and dependency rules

| Package | Path | Responsibility | Allowed internal dependencies |
|---|---|---|---|
| `va-core` | `crates/va-core` | CPU-agnostic traits and data types (§3). No I/O, no threads, no UI, no user-facing text. | none |
| `va-editor` | `crates/va-editor` | Text buffer (ropey), selections, grouped undo/redo, search/replace. No UI. | `va-core` |
| `va-lang` | `crates/va-lang` | Language services over assembler output: symbols, definition/references, outline, completion, hover, plus the background-assembly service (§4). No UI. | `va-core`, `va-editor` |
| `va-debug` | `crates/va-debug` | Debug session: a worker thread that owns a `dyn Emulator`, command/event channels, line↔address breakpoints, step over/out, run budget, undo, change tracking. No UI. | `va-core` |
| `va-workspace` | `crates/va-workspace` | Paths and portable mode, project model, settings/keybindings TOML, session, session lock and hot-exit backups, the `va-io` worker, Fluent loading and rendering (§5, §6). File watching is deferred (§4). No UI. | `va-core` |
| `va-ui` | `crates/va-ui` | egui widgets and panels: editor view, gutter, tabs, palette, registers, memory, problems… Depends on `egui` only, never on `eframe`/`winit`/`wgpu`. | `va-core`, `va-editor`, `va-lang`, `va-debug`, `va-workspace` |
| `va-arch-m68k` | `arch/m68k` | 68000: ISA tables, syntax/lexer, A68k-dialect assembler, disassembler, emulator adapter over the `m68k` crate, I/O profiles, its own `.ftl` resources. Internal layout: `src/{isa,syntax,asm,disasm,emu,io}/`. | `va-core` |
| `visual-assembler` | `apps/visual-assembler` | GUI binary: eframe window, wiring of every crate, architecture registry, bench instrumentation, native file dialogs and the startup error box (`rfd`, a Windows-only dependency of this package alone; m4 §6.4, §6.6). | all `crates/*`, all `arch/*` (never `va` or `xtask`) |
| `va` | `apps/va` | Headless CLI: `info` (M0), `lex`, `isa` (M1), `asm`, `disasm` (M2), then `run`, `debug --script`, `symbols`, `hover`. | `va-core`, `va-editor`, `va-lang`, `va-debug`, `va-workspace`, all `arch/*` (never `va-ui`, `visual-assembler` or `xtask`) |
| `xtask` | `xtask` | Developer task runner: `bench`, `ci`, `docs`, `hooks` (M0); later packaging, file generators, SingleStepTests. | none |

```
visual-assembler ──► va-ui ──► va-lang ──► va-editor ──► va-core
       │               ├─────► va-debug ─────────────────► va-core
       │               └─────► va-workspace ─────────────► va-core
       └──► arch/* (va-arch-m68k, …) ────────────────────► va-core
va (CLI) ──► va-lang, va-debug, va-workspace, arch/* ────► va-core
xtask: no workspace dependency (drives cargo and the built binaries as processes)
```

**Rules**

| # | Rule | Rejected alternative |
|---|------|----------------------|
| R1 | Only the edges in the table are allowed. Dev-dependencies follow the same rules (integration tests that need a real CPU live in the app or arch crate). | Relaxed rules for dev-dependencies: hidden coupling would show up later in normal dependencies. |
| R2 | `crates/*` never depends on `arch/*`. CPU specifics reach the core only as trait objects. | Letting `va-ui` depend on `va-arch-m68k` for custom register views: every new CPU would become a change in `crates/`. |
| R3 | GUI crates (`eframe`, `egui`, `egui-winit`, `egui-wgpu`, `egui_glow`, `winit`, `wgpu`, `glow`, `glutin`, `raw-window-handle`, `egui_kittest`) may appear only in `visual-assembler`, plus `egui`/`egui_kittest` in `va-ui`. | Using egui types (e.g. `Color32`) in core crates "for convenience": that drags the UI stack into the CLI. |
| R4 | `va` never depends on `va-ui` or any GUI crate. The CLI builds and runs on headless machines. | A single binary with a `--cli` mode: it would carry the GPU stack into scripts and CI. |
| R5 | `xtask` depends on no workspace crate. | xtask importing app crates: every app change would rebuild the task runner and couple tooling to internals. |
| R6 | Every external dependency is declared once, with an exact `=` pin, in the root `[workspace.dependencies]`. Members write `name = { workspace = true }`. | Per-crate versions: drift and duplicate versions; DEPENDANCE.md would no longer match reality. |
| R7 | Apps and xtask are leaves: no package depends on `visual-assembler`, `va` or `xtask`. Together with R3, this keeps GUI crates out of the CLI's whole dependency closure by construction. (Added after M0 audit round 1.) | A transitive GUI-crate check over the fully resolved graph: heavier, platform-dependent, and redundant once R3 and R7 hold. |
| R8 | Every member manifest contains exactly `[lints] workspace = true`, with no `[lints.*]` override. Every member inherits `license.workspace = true`. Build flags come only from `.cargo/config.toml`, never from `RUSTFLAGS`-type environment variables. (Added after M0 audit round 1.) | Per-crate lint tables: one member could silently re-allow `unsafe_code`. |

**Enforcement:**

- `cargo xtask ci` has a *layering* step that checks R1–R5 and R7 against `cargo metadata` and fails when a workspace package has no rule. It also has a build-configuration step that checks the build-flag part of R8.
- `scripts/check_project_docs.py` checks R6 and the lint part of R8.
- cargo-deny (`[licenses.private] ignore = false`) checks the licence part of R8.
- Details are in the M0 spec, §17.

---

## 3. Core contracts (`va-core`, signature-level sketch)

The names and shapes below are binding in spirit. M1 finalizes lifetimes, error types and
module paths. Trait objects are `Send` (and `Sync` when shared) so they can move to worker threads.

> **Finalized by `m1-core-isa-lexer.md` §3** (which wins over this sketch): spans use a `Copy`
> `TextRange { start, end }` instead of `Range<u32>`; `TokenKind` adds `Whitespace`, `MacroName` and
> `SuspectComment(DiagCode)`, and `Invalid` carries its `DiagCode`; `Diagnostic` gains `blocking`
> (an error may be shown without stopping output); `Architecture` gains `instruction_set()` (a CPU-agnostic
> instruction view for `va isa` and hover) and `diagnostic_codes()`; `assembler`/`disassembler`
> land in M2 and `emulators`/`io_profiles` in M3, with the signatures of m1 §3.7.
>
> **Amended by `m2-assembler.md` §3 (2026-10-24; wins over m1 §3.7 on these points):**
> `assembler()`/`disassembler()` return `Option<Box<dyn …>>` and, like M3's `emulators()`/`io_profiles()`,
> are provided methods with a neutral default (rule below); `SourceProvider` gains the provided
> methods `resolve_binary`/`binary` (for `INCBIN`); `AssembleOutput` gains `cancelled`; `DecodedInsn`
> is `{ address, len, text, tokens, target, flow }`. In code these are additions only: no public
> item of `va-core` is renamed, removed or re-typed (m2 §3.1 states why the m1 return type could not
> be added as is).

```rust
// ── Registry ──────────────────────────────────────────────────────────────
pub trait Architecture: Send + Sync {
    fn id(&self) -> &'static str;                               // "m68k"
    fn syntax(&self) -> &dyn SyntaxProvider;
    fn instruction_set(&self) -> &dyn InstructionSet;           // M1
    fn registers(&self) -> &'static [RegisterSpec];             // names, widths, groups, flag bits
    fn diagnostic_codes(&self) -> &'static [DiagCode];          // M1
    fn locale_resources(&self, lang: &str) -> &'static [&'static str]; // embedded .ftl sources (§5)
    fn assembler(&self) -> Option<Box<dyn Assembler>> { None }          // M2
    fn disassembler(&self) -> Option<Box<dyn Disassembler>> { None }    // M2
    fn emulators(&self) -> Vec<Box<dyn EmulatorFactory>> { Vec::new() } // M3; several emulators per CPU
    fn io_profiles(&self) -> Vec<Box<dyn IoProfile>> { Vec::new() }     // M3; "easy68k", "d68k-tp"
}

// ── Syntax: one lexer shared by the highlighter and the assembler ─────────
pub trait SyntaxProvider: Send + Sync {
    /// Lex one line. `state` carries cross-line context (opaque; 0 for line-oriented dialects).
    fn lex_line(&self, line: &str, state: LineState, out: &mut Vec<Token>) -> LineState;
    fn keywords(&self) -> &[KeywordInfo];                       // mnemonics, directives, registers, size suffixes
}
pub struct Token { pub kind: TokenKind, pub range: core::ops::Range<u32> }
pub enum TokenKind { Mnemonic, Directive, Register, SizeSuffix, Number, String, Label, LocalLabel,
                     Symbol, Operator, Punctuation, MacroParam, Comment, Invalid }

// ── Assembler ─────────────────────────────────────────────────────────────
pub trait Assembler: Send {
    fn assemble(&mut self, input: &AssembleInput<'_>, cancel: &CancelToken) -> AssembleOutput;
}
pub struct AssembleInput<'a> { pub root: FileId, pub sources: &'a dyn SourceProvider, pub options: &'a AsmOptions }
pub trait SourceProvider: Sync {                                // INCLUDE goes through this (path policy §9.5)
    fn text(&self, file: FileId) -> Option<&str>;
    fn resolve_include(&self, from: FileId, path: &str) -> Result<FileId, IncludeError>;
    fn resolve_binary(&self, from: FileId, path: &str) -> Result<FileId, IncludeError> { Err(IncludeError::Unsupported) } // M2, INCBIN
    fn binary(&self, file: FileId) -> Option<&[u8]> { None }                                                              // M2
}
pub struct AssembleOutput {
    pub image: Option<ProgramImage>, pub diagnostics: Vec<Diagnostic>,
    pub symbols: SymbolTable, pub source_map: SourceMap, pub listing: Option<Listing>,
    pub cancelled: bool,                                        // M2: the token tripped; no image
}
pub struct CancelToken(std::sync::Arc<std::sync::atomic::AtomicBool>);

// ── Disassembler ──────────────────────────────────────────────────────────
pub trait Disassembler: Send {
    fn decode(&self, bytes: &[u8], address: u64) -> Result<DecodedInsn, DecodeError>;
    fn flow_kind(&self, insn: &DecodedInsn) -> FlowKind;                              // drives step over/out
}
pub struct DecodedInsn { pub address: u64, pub len: u32, pub text: String, pub tokens: Vec<Token>,
                         pub target: Option<u64>, pub flow: FlowKind }               // M2 (m2 §3.3)
pub enum FlowKind { Sequential, Jump, Branch, Call { return_address: u64 }, Return, Trap }

// ── Emulation ─────────────────────────────────────────────────────────────
pub trait EmulatorFactory: Send + Sync {
    fn id(&self) -> &'static str;                               // "m68k-crate"; later e.g. "gdb-remote"
    fn capabilities(&self) -> Capabilities;
    fn create(&self, config: &EmulatorConfig) -> Result<Box<dyn Emulator>, EmuError>;
}
pub trait Emulator: Send {
    fn reset(&mut self);
    fn load(&mut self, image: &ProgramImage) -> Result<(), EmuError>;
    fn step(&mut self) -> StepOutcome;                          // one instruction: cycles, optional exception
    fn run(&mut self, budget: RunBudget) -> StopReason;         // until breakpoint, halt, or budget exhausted
    fn registers(&self) -> RegisterSnapshot;
    fn set_register(&mut self, id: RegId, value: u64) -> Result<(), EmuError>;
    fn read_memory(&self, address: u64, buf: &mut [u8]);
    fn write_memory(&mut self, address: u64, data: &[u8]) -> Result<(), EmuError>;
    fn capabilities(&self) -> Capabilities;                     // undo, cycles, watchpoints, write log
}
pub struct RunBudget { pub max_instructions: u64, pub max_duration: std::time::Duration }
pub enum StopReason { Breakpoint(u64), Halted, Exception(ExceptionInfo), InputRequired, BudgetExhausted, Stopped }

// ── I/O profiles (implemented in arch/<cpu>/src/io, chosen in project.toml) ──
pub trait IoProfile: Send + Sync {
    fn id(&self) -> &'static str;                               // "easy68k" (default), "d68k-tp"
    fn on_trap(&self, vector: u8, cpu: &mut dyn CpuState, console: &mut dyn Console) -> TrapOutcome;
    fn mmio_regions(&self) -> &[MmioRegion];                    // e.g. a memory-mapped video window
}

// ── Diagnostics and mapping ───────────────────────────────────────────────
pub struct Diagnostic {
    pub code: DiagCode, pub severity: Severity, pub span: SourceSpan,
    pub args: Vec<(&'static str, DiagArg)>,                     // typed Fluent arguments
    pub related: Vec<Related>, pub suggestion: Option<Suggestion>,
}
pub struct DiagCode(pub &'static str);                          // stable kebab-case id, e.g. "asm-undefined-symbol"
pub struct SourceSpan { pub file: FileId, pub bytes: core::ops::Range<u32> }
pub struct SourceMap { /* sorted address → span, and (file, line) → addresses; both binary-searchable */ }
pub struct ProgramImage { pub segments: Vec<Segment>, pub entry: Option<u64> }
pub struct Segment { pub address: u64, pub bytes: Vec<u8>, pub kind: SegmentKind } // Code | Data | Bss
```

- Addresses are `u64` for every CPU. Each architecture masks them to its bus width (24 bits on the 68000).
- `va-core` and the arch crates contain no user-facing prose. A `Diagnostic` carries a code and typed
  arguments, and the text is produced by Fluent (§5). Arch crates embed `.ftl` files as resources only.
- **Decision: dynamic dispatch (`dyn Trait`).** *Rejected:* a generic `Session<A: Architecture>`. It would
  monomorphize the UI per CPU and prevent picking the CPU at runtime from `project.toml`.
- **Decision: diagnostic codes are kebab-case strings** (they double as Fluent ids, see §5). *Rejected:*
  numeric codes such as `E0123`, which are opaque to students and need a separate lookup table.
- **Decision: a capability added to a frozen trait after M1 is a provided method with a neutral
  default** (`None`, an empty `Vec`, `Err(Unsupported)`), so the fake `Architecture`/`SourceProvider`
  implementations in other crates' tests, and a CPU that lacks the capability, keep compiling; callers
  hide what a CPU does not offer. A frozen public item is never renamed or removed, and a coded
  signature never changes, unless a spec states why no addition works (m2 §3.1, 2026-10-24).
  *Rejected:* required methods, which break every fake owned by a parallel milestone; null objects
  that fabricate output (an assembler that reports a fake error, a decoder that calls every word illegal).

---

## 4. Threading model

| Thread | Owned by | Work | Talks via |
|---|---|---|---|
| main / UI | eframe (`visual-assembler`) | Input, layout and paint; reads immutable snapshots only | Polls receivers with `try_recv` each frame |
| `va-assembler` | `va-lang` analysis service | Debounce, assemble the latest snapshot, cancel stale jobs | `mpsc` requests in, results out, plus a waker |
| `va-debug` | `va-debug` | Owns the `dyn Emulator` and executes commands, running in slices | `mpsc` commands/events, plus a waker |
| `va-io` | `va-workspace` | File loads and decoding, atomic writes (saves, session, hot-exit backups), directory listings, the session lock, custom-font validation (M4) | `mpsc` jobs, plus a waker |
| `va-search` | `va-ui` shell (M4) | One persistent thread: compiles the find query and runs find / replace-all over a snapshot, with a per-request cancel flag (m4 §5.10) | `mpsc` requests in, results tagged with the document revision out, plus a waker |
| `va-dialog` | `visual-assembler` host (M4, Windows only) | One short-lived thread per native file dialog (`rfd`), at most one at a time (m4 §6.6) | a reply channel, plus a waker |
| `va-watch` (not built) | — | **Deferred** (M4 decision, 2026-10-24): no `notify` watcher. Open files' stamps and expanded explorer folders are re-checked on window focus gain, on `va-io` (m4 §5.7, §5.8). M7 reconsiders a watcher if focus checks prove insufficient. *Rejected for M4:* `notify` (an extra watcher thread and dependency for a need focus checks cover). | — |

| # | Rule | Rejected alternative |
|---|------|----------------------|
| T1 | **The UI thread never blocks.** It does no file I/O, no assembly and no emulation, and holds no lock across frames. Any operation that can exceed 1 ms runs on a worker. Data crosses threads as immutable snapshots (`Arc`, cheap rope clones), never as a `Mutex` shared with workers. Two bounded exceptions, both outside the frame loop (m4 §6.2, §4.8): before the first window, the main thread reads `settings.toml`, `keybindings.toml` and `session.toml` (each ≤ 1 MiB), inside the measured startup; at exit it waits at most 2 s for the final backups (`IoWorker::flush`). | Shared `Mutex<State>`: lock contention would show up directly as frame jank. |
| T2 | Workers wake the UI through an injected `Waker = Arc<dyn Fn() + Send + Sync>`, which the app wires to `egui::Context::request_repaint`. Non-UI crates never name egui. | Passing `egui::Context` into `va-debug`/`va-lang`: violates R3. |
| T3 | **Background assembly.** Each edit bumps a `generation: u64` and sends a snapshot. The worker waits until 300 ms pass without a newer request (setting `editor.diagnostics_delay_ms`), then assembles. A newer request trips the running job's `CancelToken`. The assembler polls the token at least every 256 source lines. The UI drops results whose generation is stale. | Assembling on every keystroke on the UI thread: breaks the 16 ms budget on large files. |
| T4 | **Debug session.** Commands are `Load`, `Reset`, `Step`, `StepOver`, `StepOut`, `Run`, `Pause`, `Stop`, `SetBreakpoints`, `WriteMemory`, `SetRegister`, `Undo(n)`, `Shutdown`. `Run` loops on `Emulator::run(RunBudget { max_instructions: 100_000, max_duration: 8 ms })` and checks the command channel between slices. While running, events (`Stopped{reason, snapshot}`, `Output`, `InputRequired`, `Faulted`) are throttled to ≤ 30 Hz. | One blocking `run()` call: `Pause` could not interrupt it. |
| T5 | Channels are `std::sync::mpsc` (crossbeam-based since Rust 1.67), and workers loop on `recv_timeout`. There is no async runtime. | `crossbeam-channel` (`select!` is not needed); `tokio` (built for network fan-out, which we have none of). |
| T6 | A worker's command loop runs inside `std::panic::catch_unwind`. A panic becomes a `Faulted` event and the IDE stays alive. All profiles therefore keep `panic = "unwind"`. | `panic = "abort"`: one emulator bug would kill the student's unsaved session. |

---

## 5. Localization (i18n)

- **Engine:** Fluent (`fluent-bundle`). *Rejected:* gettext, a C library under LGPL that complicates static builds and cross-compilation.
- **Message ids:** English, kebab-case, with a domain prefix:
  - `ui-…` for interface strings;
  - `cmd-<group>-<action>` for command titles (M4, from the command id `<group>.<action>`);
  - `diag-<code>` for diagnostics (e.g. `diag-asm-undefined-symbol`), with `.help` / `.note` attributes;
  - `isa-<arch>-<mnemonic>-…` for instruction documentation (`isa-<arch>-dir-<name>` for directives, M2).
- **Files.** Architecture-independent resources live in `locales/<lang>/{ui,commands,config}.ftl` (from M4:
  `ui-*`, `cmd-*`, and `diag-cfg-*` for settings and project-file problems; m4 §4.6).
  CPU-specific resources live in `arch/<cpu>/locales/<lang>/{diagnostics,isa}.ftl` (M2 adds `directives.ftl`) and are exposed through
  `Architecture::locale_resources`, so adding a CPU touches only `arch/<cpu>`. *Rejected:* one global
  `locales/` folder for every CPU (the plan's first sketch), because each new CPU would also edit shared files.
- **Id ownership (2026-10-24).** The root resources own `ui-*`, `cmd-*` and `diag-cfg-*`; a CPU owns
  `diag-<its codes>` and `isa-<arch>-*`, and its codes never start with `cfg-`. Neither side defines
  Fluent terms without its prefix. The GUI loads both sets into one bundle per language, where a
  duplicate id is an error (`LocalizerError::Duplicate`), so the GUI tests build that bundle for every
  registered CPU (m4 §10.3). *Rejected:* letting a later resource silently override an earlier one,
  which would hide a collision.
- **Loading.** Resources are embedded with `include_str!`. Optional user overrides in `<config>/locales/<lang>/*.ftl` load afterwards; when ids collide, the override wins. Not loaded in M4 (deferred, m4 §4.6).
- **Fallback chain:** requested language → `en`. English must be complete: from M1 on, a test lists every `DiagCode` and every `ui-` id in use and fails when one is missing from `en`.
- **Single renderer.** `va-workspace` provides `Localizer::render(&Diagnostic) -> RenderedDiagnostic`, used by both the GUI and the CLI. Assemblers and emulators never produce prose.
- **Language selection (highest priority first):** CLI `--lang en|fr`, then env `VA_LANG`, then the `ui.language` setting (`auto|en|fr`), then `auto`. `auto` resolves to the OS locale, falling back to `en`.
  - From M1 (m1 §6.2, §7.1): `--lang` is a global `va` flag, and `auto` reads only `LC_ALL`, `LC_MESSAGES`, `LANG`.
  - **GUI, from M4 (m4 §4.6):** `--lang en|fr` (`auto` is not a value) > `VA_LANG` > `ui.language` > `auto`; `auto` reads
    `LC_ALL`, `LC_MESSAGES`, `LANG`, then, on Windows only, the user's UI language through `sys-locale` (`fr…` → French,
    else English).
  - **`va` keeps the M1 rule** (no `ui.language`, no Windows UI language) until a later spec decides otherwise.
    *Rejected:* changing the CLI in M4, which would make `va`'s output depend on a GUI settings file.

---

## 6. Settings, session and project files

Locations are computed with `std` only (M4 decision, 2026-10-24; m4 §4.1), each directory independently:

- Windows: config in `%APPDATA%\VisualAssembler\config\`, data in `%LOCALAPPDATA%\VisualAssembler\data\`
  (else the same folders under `%USERPROFILE%\AppData\Roaming` / `Local`).
- Linux: config in `$XDG_CONFIG_HOME/visualassembler/`, data in `$XDG_DATA_HOME/visualassembler/`
  (else `$HOME/.config/…` / `$HOME/.local/share/…`).
- An environment value is used only when non-empty and absolute; otherwise it is skipped with a warning.
  Folders are created on first write; on Unix the data dir and `backups/` get mode `0o700`.
- *Rejected:* the `directories` crate (three crates, one under MPL-2.0 needing a cargo-deny exception) for
  two environment lookups.

| File | Location | Written by | Content |
|---|---|---|---|
| `settings.toml` | config dir | the user only (the app creates a fully commented default once) | Preferences |
| `keybindings.toml` | config dir | the user only | Shortcut overrides |
| `session.toml` | data dir | the app | Open project, tabs, cursors, zoom, panel layout, theme toggle, window geometry |
| `session.lock` | data dir | the app (M4) | Held with `File::try_lock` for the life of the process; a second instance, or an unwritable data dir, runs a **read-only session**: no session, backup or default-settings write, and the status bar says so (m4 §4.7) |
| `backups/index.toml`, `backups/<id>.txt` | data dir | the app | Hot-exit copies of unsaved buffers |
| `.visualasm/project.toml` | project root | the user, or the app on an explicit "Project settings" action | `arch`, `io_profile`, entry file, include dirs, outputs |

- **Portable mode:** a regular file named `portable` next to the executable moves everything under `<exe dir>/data/{config,data}/`. A portable folder that cannot be written makes the session read-only.
- **Test isolation:** the env overrides `VA_CONFIG_DIR` / `VA_DATA_DIR` keep tests and CI away from real profiles.
- **Schema versioning:** every file starts with `schema_version = 1`.
  - Unknown keys produce a warning diagnostic and are ignored.
  - Invalid values fall back to the default with a warning. A bad file never crashes the app.
- **Precedence (lowest to highest):** compiled defaults, then `settings.toml`, then `project.toml` (project keys only), then `VA_*` environment variables, then CLI flags.
  - This is the same order as `--lang` > `VA_LANG` > `ui.language` (§5).
  - An environment variable is per-session, so it beats the persistent files and can rescue a broken setting without editing the file.
  - *Rejected:* settings files above environment variables, which would make `VA_RENDERER`/`VA_LANG` silently ineffective once a setting exists.
- **Renderer key (from M4):** `settings.toml` `[render] renderer = "auto" | "glow" | "wgpu"`, default `"auto"`.
  - Effective precedence: `--renderer` > `VA_RENDERER` > `render.renderer` > `auto` (§9.8).
  - An invalid value produces a warning and is ignored (the next level applies), like every invalid setting.
- **Atomic writes (amended 2026-10-24, m4 §4.7):** every write of either app (GUI saves, session and backups;
  `va asm` outputs, m2 §8.1) resolves an existing target with `fs::canonicalize` (a symlink is never replaced),
  creates the temp file `<name>.<pid>.<n>.tmp` in the target's folder with `create_new(true)` (`n` a per-process
  counter), writes, flushes and `sync_all`s it, then renames it over the target; on error the temp file is removed.
  *Rejected:* a fixed `<name>.tmp`, which can clobber a user's file of that name and collides between two instances.
- **The app never rewrites `settings.toml` or `keybindings.toml`.** UI toggles (zoom, theme) persist in
  `session.toml`. *Rejected:* round-trip editing with `toml_edit`, which adds complexity and risks
  destroying user comments.
- A student's source file is written only on an explicit save. Hot-exit backups never touch it.

---

## 7. Performance design and measurement definitions

**Levers:**

- egui reactive mode: no continuous repaint loop. The cursor blink is a spaced wake-up and can be disabled: on by default (user decision, 2026-10-24), 530 ms half-period, solid after 10 s without input, so an idle IDE requests no frame (m4 §5.6). Refresh is capped at ≤ 30 Hz while a program runs.
- Fonts and locales are embedded; the project loads after the first frame.
- The editor is virtualized and draws only visible lines. Lexer results are cached per line, and only the edited line is re-lexed.
- Rope buffer, line index, incremental symbol index, background assembly with debounce and cancellation (T3).

| Metric | Exact definition | Produced by | Target |
|---|---|---|---|
| `spawn_to_first_frame_ms` | From `Command::spawn` in xtask to receipt of the GUI's bench JSON line | `cargo xtask bench startup` | p95 < 500 ms on the reference machine, warm cache. This is the gating startup number. |
| `startup_ms` | From `main()` entry to the start of the first `App::logic` call that follows the presentation of frame 1. eframe keeps the window hidden until frame 1 is painted, then shows it. | GUI self-report | Informational (isolates in-process time) |
| `frames_painted` (idle) | egui frames started during the idle window | `bench idle` (from M4 with the generated 5k project open, m4 §6.8) | 0 (bench mode turns the blink off) |
| idle CPU % | Process CPU time (user + kernel, all threads) during the idle window ÷ window wall time × 100, i.e. % of one core. The window is 10 s, starting `IDLE_SETTLE_MS` = 3 s after the first frame (M0 spec §17.13). | `bench idle` (5k project from M4) | ≤ 1 % (median) |
| `peak_rss_bytes` | Windows: `PeakWorkingSetSize`. Linux: `VmHWM`. | GUI self-report | < 500 MB (median) in `bench idle` with `bench/generated/5k` open (M4+) |
| `open_ms` | `Shell::open_path` of the generated 100,000-line file → start of the frame after the first frame that painted it | `bench open` (M4) | p95 < 1,000 ms over 5 runs |
| scroll latency | One injected `PageDown` → start of the next frame after the one that handled it (present proxy, m4 §6.8), vsync off; 300 per run in the 100,000-line file | `bench scroll` (M4) | p95 < 16 ms over the pooled samples |
| keystroke latency | One injected keystroke of the deterministic type script → start of the next frame after the one that handled it (present proxy), vsync off, in the 100,000-line file | `bench type` (M4) | p95 < 16 ms over the pooled 1,500 keystrokes (3 runs × 500); max reported |
| assembler throughput | `make bench-asm`: release, in-memory sources, 25 representative lines cycled, 1 warm-up then 10 runs (m2 §10.5) | ignored release test, `va.bench.asm/1` lines | median ≤ 20 ms for 5,000 lines and ≤ 300 ms for 100,000 lines (m2 §11) |
| micro-benchmarks | Lexer, rope edits, symbol index | criterion in each crate's `benches/` from M7; until then ignored release-mode tests (lexer: `make bench-lexer`, m1 §5.6; assembler: row above) | Tracked, never pass/fail in CI |

- **Vsync off for latency scenarios (M4).** `open`, `scroll` and `type` run with `NativeOptions::vsync = false`
  and report it, so a sample measures the app's frame, not the display's refresh wait (latency to the
  physical display is a non-goal, §1). *Rejected:* measuring with vsync on, which quantizes every sample
  to the refresh period.
- **Schemas.** From M4 the GUI's run line is `va.bench.run/3` and the xtask report `va.bench.report/3`
  (m4 §7.2, §7.3); `cargo xtask bench` runs `startup|idle|open|scroll|type|all`. Each GUI run gets
  isolated `VA_CONFIG_DIR`/`VA_DATA_DIR` folders, so user settings never affect a report.
- A budget miss blocks the milestone's acceptance and goes back to the Architect; it never fails CI.

- Bench results are JSON files in `bench/results/` (gitignored).
- Every optimization ships a report at `docs/perf/YYYY-MM-DD-<topic>.md` with before/after numbers and the exact command.
- A performance claim in any document must cite such a report.
- A true cold start (after a reboot) is measured by hand and documented, never automated.

---

## 8. Testing strategy

- **Tests never live in production files** (no inline `#[cfg(test)] mod tests`). Each crate's `tests/` mirrors
  `src/`: `src/bench/report.rs` maps to `tests/bench_report.rs`, and a group that grows becomes
  `tests/<module>/main.rs`. As a consequence, tests go through the crate's public API, and a private
  helper worth testing becomes a public item of its module. *Rejected:* inline test modules, which violate
  the framework's file-organization rule (§7 of the agentic-project skill).
- Every test file starts with a `//!` line saying what it covers, and with `#![allow(clippy::unwrap_used)]`.
- Temporary folders in tests are created under `env!("CARGO_TARGET_TMPDIR")` with a unique name and
  `std::fs::create_dir`, which fails if the path exists. Tests never use predictable names in the shared
  system temp folder. (Added after M0 audit round 1.)

| Layer | What | Runs in CI |
|---|---|---|
| Crate tests | Pure logic through the public API | yes |
| CLI tests | Spawn the binary via `env!("CARGO_BIN_EXE_<name>")`; assert exit code, stdout/stderr, JSON schema | yes |
| UI tests | `egui_kittest` over the AccessKit tree; no window, no GPU. Image/snapshot tests are not used until a GPU-free renderer is decided. | yes |
| Golden files (M1 lexer, M2) | Lexer tokens, assembler output, listing, S-record, diagnostics, under `arch/m68k/fixtures/<area>/` (moved out of `tests/` for the depth rule, m1 §2.1). Areas: `lex/` (M1); `asm/`, `asm-opt/`, `asm-inc/`, `asm-diag/`, `disasm/` (M2, m2 §10.2), each flat and ≤ 15 files | yes |
| Round trip (M2) | asm → binary → disassembly → asm **with optimizations off** (the disassembly text names the exact form, which only `--no-opt` keeps): exhaustive over every valid opword with fixed extension words, plus every code-only fixture assembled both ways (m2 §10.3) | yes |
| CPU conformance (M3) | SingleStepTests 680x0 subset. The upstream repository publishes no licence, so vectors are fetched by xtask into gitignored `xtask/cache/` and never committed. | yes (fetched at test time) |
| a68k comparison (M2) | Byte-for-byte comparison with `a68k.exe` over the fixtures and a generated forms corpus, with and without optimizations: `VA_A68K_PATH=<path> make compare-a68k` (an `#[ignore]` test, outputs only under `CARGO_TARGET_TMPDIR`, m2 §10.4) | **never** (tool cannot be redistributed) |
| Benchmarks | criterion and `cargo xtask bench` | never pass/fail |

- `make test` runs `cargo test --workspace --all-features`.
- Whatever cannot be automated (visual rendering, real GPU behaviour, IME, drag-and-drop) is listed in the README of the folder concerned, with the reason, and goes on the milestone's manual checklist.

---

## 9. Platforms, toolchain and policies

### 9.1 Support tiers ("tested" vs "compiled only")

| Tier | Meaning | Platforms |
|---|---|---|
| **Tested manually** | A human ran the GUI and the CLI | Windows 11 x64 (reference machine: Ryzen AI 7 350 / Radeon 860M); Ubuntu 26.04 x64 under WSLg, through X11 (Xwayland, the default under WSL, §9.8). Wayland under WSLg is tested with glow only (`VA_WAYLAND=1`). |
| **Tested automatically** | The full test suite, including headless kittest, runs in CI; nobody looks at the UI | GitHub runners `windows-2025` (x64), `windows-11-arm`, `ubuntu-26.04` (x64), `ubuntu-24.04-arm` |
| **Compiled only** | Built, never executed | `aarch64-unknown-linux-gnu` cross-built from WSL |

- The root README carries this matrix verbatim.
- A platform moves up a tier only with evidence: a report in `docs/perf/` or a completed manual checklist.
- The informational CI GUI smoke run (M0 spec §8) never counts as evidence.
- Each CI runner label names one OS release, never a `-latest` label (M0 spec §20), so an OS move
  reaches CI only through a spec revision. x64 Linux CI runs the release of the manual Linux tier.
  *Rejected:* moving `-latest` labels, which change CI and its cache with no commit.

### 9.2 Toolchain

- Rust **1.99.0**, pinned in `rust-toolchain.toml`. Workspace `rust-version = "1.99"`.
- Edition 2024, Cargo resolver 3.
- Windows binaries link the C runtime statically (`+crt-static`), so no VC++ redistributable is needed.

### 9.3 Offline and no telemetry

- **The product never initiates network I/O.**
  - cargo-deny bans HTTP, socket, DNS, TLS, RPC and telemetry crates, plus the `net` feature of `mio` (`.cargo/deny.toml`).
  - The Linux graph does contain transports that *the environment* can select. zbus (accessibility,
    via AccessKit) connects over TCP when the D-Bus/AT-SPI bus address is `tcp:`, and X11 connects over
    TCP when `DISPLAY=host:N`. Both are accepted and recorded in `SECURITY.md`.
  - `async-io`/`polling` cannot be banned because zbus needs them.
- `cargo xtask ci` scans the sources of every workspace member for network APIs (§17.4 of the M0 spec).
  - **Threat model of the tripwires** (binding, M0 spec §18.1). The source scan and the build-configuration
    check of `cargo xtask ci` are tripwires against *accidental or careless* introduction of network
    code, unsafe code or build-flag overrides. They are **not** a sandbox against a deliberately
    malicious contributor. Deliberate evasion is covered by code review, by cargo-deny on dependencies,
    and by branch protection with required CI.
  - A bypass is **blocking** only if it is a realistic accidental pattern that compiles and is not caught.
  - A bypass that needs deliberate obfuscation is an accepted limitation, recorded in `SECURITY.md`.
  - The only planned exception is loopback in `crates/va-gdb` (§10.2). It must be an explicit allow-list entry, reviewed by Security and recorded in `SECURITY.md`.
- Developer tooling (cargo, the cargo-deny advisory DB, CI, `gh`) may use the network. xtask never links
  an HTTP crate: when a download is needed (SingleStepTests), it shells out to `git` or `curl`.

### 9.4 Unsafe code

- The workspace lint is `unsafe_code = "deny"`.
- A local `#[allow(unsafe_code)]` is permitted only in files on the allow-list kept in `xtask/src/layering.rs`.
  In M0 that means the OS process-statistics files and the GL string query in `apps/visual-assembler/src/bench/`.
- Every `unsafe` block carries a `// SAFETY:` comment, enforced by `clippy::undocumented_unsafe_blocks`.
- Outside the allow-list, the source scan rejects the `unsafe` keyword itself and any mention of the
  `unsafe_code` lint other than inside `deny(…)`/`forbid(…)`. Rule R8 keeps the lint from being re-allowed
  through manifests or build flags. (Added after M0 audit round 1.)
- These checks follow the tripwire threat model of §9.3. The compiler lint `unsafe_code = "deny"` stays
  the primary guard, and deliberate evasion of the scan is a review matter, not a tooling defect.

### 9.5 External tools and processes

- `a68k`/`d68k` run only when configured. They are spawned with `std::process::Command` and an argument vector, never through a shell.
- `INCLUDE` resolution goes through `SourceProvider` (full path sandbox, M7).
  - **First step, M2 (CLI, m2 §6.2):** a source file is untrusted and `INCLUDE`/`INCBIN` let its content choose
    paths, so `va asm` checks each path lexically before any file-system access (`/` and `\` both separators on
    every OS): it refuses an empty path or a NUL, an absolute path, a drive, UNC, `\\?\` or `\\.\` prefix, any
    `:`, a Windows device name as a component (on every OS), and a path that climbs above its base folder; the
    canonicalized candidate must stay under the canonical base and be a regular file. Bases are the root
    file's folder, then each `-I` folder. Reads are capped at 16 MiB per file and 256 files per assembly.
  - The command-line root and `-I` folders are used as given until M7; the GUI provider (M5) applies the same
    policy over `project.toml` `include_dirs`, which are relative without `..` (m4 §4.5).
  - *Rejected:* waiting for M7, which would let a course file open a UNC path (SMB, NTLM leak) from M2 on.
- Untrusted inputs are S-records, project files and settings. Each is parsed with explicit size limits as its
  parser lands: S-records ≤ 16 MiB of data (M2), settings, keybindings, session and project files ≤ 1 MiB each,
  custom font ≤ 32 MiB (M4). M7 audits them all.

### 9.6 Fonts

- Code font: **JetBrains Mono** v2.304 (OFL-1.1), **Regular only**, embedded from M4 with its `OFL.txt`
  (download approved by the user on 2026-10-24, m4 §8.4). Bold is added when a style needs it (m4 §13).
- A custom code font is a file-path setting, `editor.font_file` (a `.ttf`/`.otf`, ≤ 32 MiB, validated off the
  UI thread; any failure keeps the embedded font). *Rejected:* system font enumeration (`fontdb`/`font-kit`), a
  dependency for what a path setting covers.
- *Rejected:* Cascadia Code. Its licence is equally fine, but its distinguishing features (ligatures, cursive italics) are unused by egui's text renderer.
- The UI font is egui's default proportional set (decided in M4, m4 §5.5).

### 9.7 Licence

- VisualAssembler is licensed **`MIT OR Apache-2.0`**. The texts are in `LICENSE-MIT` and `LICENSE-APACHE` at the root.
- Every member inherits `license.workspace = true`.
- Crates stay `publish = false`: the project ships binaries and does not publish crates.
- *Rejected:* a single licence. Dual MIT/Apache is the Rust ecosystem norm and is compatible with every dependency allowed by cargo-deny.

### 9.8 Renderer policy (user decision, 2026-10-14; M0 spec §19)

- **The shipped GUI contains both renderers** (default features `renderer-glow` + `renderer-wgpu`).
- **Renderer orders by target:**

  | Target | Order with `auto` |
  |---|---|
  | Windows x64, Linux (x64, aarch64) and other targets | glow, then wgpu |
  | Windows aarch64 | wgpu, then glow (DX12 is native there; OpenGL needs an optional compatibility pack) |

  - Fallback happens at most once and only for a renderer *initialization* failure before the first frame.
  - Platform failures (no display, event-loop creation) never trigger a fallback.
- **Forced renderer** (`glow` or `wgpu`, chosen through the precedence in §6): exactly that renderer and no fallback. A failure is reported and exits with 3.
- **wgpu backends.** On Windows, wgpu is pinned to DX12. A wgpu attempt that follows a failed glow
  attempt leaves out wgpu's GL backend: the failed glow context can stay current on the thread,
  wgpu's GL backend then panics, and it could only retry the OpenGL driver that has just failed.
  Both overrides step aside only when `WGPU_BACKEND` names at least one backend that wgpu
  recognizes (M0 spec §19.9). Consequence: on Linux, a host without any Vulkan driver cannot fall
  back to wgpu after a glow failure and exits with 3.
- **What cannot fall back:** a panic (exit 101) and a crash inside the graphics driver. The user
  forces the other renderer.
- **Under WSL**, the app uses X11 (Xwayland) for every renderer unless `VA_WAYLAND=1`. wgpu Wayland clients crash the WSLg 1.0.73 compositor, and a single window system per process is what makes the in-process fallback safe. Native Linux keeps winit's default (Wayland first).
- **Reporting.** Selection warnings, the fallback and startup failures go to stderr (M0 spec §19.4),
  and bench JSON records the requested and running renderer, the fallback and the test hook. The
  Windows release build has no console, so its stderr is visible only when redirected. From M4
  (m4 §5.11, §6.4):
  - the status bar shows a fallback ("<from> → <running>", with the reason) and an armed
    `VA_TEST_FAIL_RENDERER` test hook;
  - a startup failure that exits before any window (exit 3, or 1 for a runtime failure) still prints
    on stderr and, **on Windows only**, when stderr has no handle (started from Explorer or the Start
    menu) and no `--bench` is given, also shows a native error box (`rfd::MessageDialog`) in the
    resolved language, with the hint to start it with `--renderer glow` or `--renderer wgpu` from a
    terminal. Linux keeps stderr only. *Rejected:* a log file, which nobody without a console would
    know to open.
- *Rejected:* wgpu by default, which was the earlier measured rule. The user chose glow for its lower startup time and memory: on Windows, p95 302 ms and 93 MiB, against 465 ms and 342 MiB for wgpu-DX12.

---

## 10. Extension paths

### 10.1 A new CPU

1. Create `arch/<cpu>/`: package `va-arch-<cpu>`, lowercase folder name. Implement `Architecture` and its traits, put resources in `arch/<cpu>/locales/`, and tests in `arch/<cpu>/tests/`.
2. Add an `arch-<cpu>` Cargo feature to `apps/visual-assembler` and `apps/va`, plus one line in each app's `src/arch_registry.rs` (`va`'s created in M1, the GUI's in M4; the GUI line also names the CPU's line-comment prefix, m4 §6.7).
3. Nothing changes in `crates/`. The layering rule already covers every `va-arch-*` package.
4. If the traits lack a capability, change `va-core` through an Architect spec. Never special-case a CPU id in core or UI code.

Candidate future folders: `arch/z80`, `arch/mos6502`, `arch/riscv`, `arch/arm`, `arch/x86`.

### 10.2 Out-of-process emulators (GDB Remote Serial Protocol)

- A future crate, `crates/va-gdb`, provides an `EmulatorFactory` whose `Emulator` speaks GDB RSP to an external emulator (MAME, Hatari, QEMU) that the IDE launches.
- **Transport:** a child-process stdio pipe when the emulator supports one; otherwise TCP restricted to `127.0.0.1` / `::1`. That loopback socket is the single `std::net` allow-list entry (§9.3).
- `Capabilities` reports reality (no undo, no write log), and `va-debug`/`va-ui` hide features based on capabilities, never on factory ids.
- `va-gdb` depends on `va-core` only.
- *Rejected:* running the emulator out of process from V1. In-process gives minimal latency and simpler tests; the trait keeps this door open.

### 10.3 Assembler dialect profiles (deferred, user decision 2026-10-17)

- V1 ships one 68000 dialect, strict A68k (m1 §5.7). A permissive EASy68K/vasm-style profile (spaces allowed after commas in operands) comes later as a dialect chosen per project, like the I/O profile, through a second `SyntaxProvider` in `arch/m68k`. No change in `crates/`.

---

## 11. Repository layout (top level)

Rules:

- At most 4 directory levels below the root; about 12 files per folder (15 tolerated).
- One responsibility per folder, and a `README.md` in every folder. No `utils/`, `common/` or `misc/`.
- A folder is created together with its first file.

| Folder | Purpose |
|---|---|
| `crates/` | CPU-agnostic library crates (§2) |
| `arch/` | One crate per CPU |
| `apps/` | Shipped binaries: `visual-assembler` (GUI), `va` (CLI) |
| `xtask/` | Rust task runner behind `make` |
| `locales/` | Architecture-independent Fluent resources: `<lang>/{ui,commands,config}.ftl` (from M4, §5) |
| `assets/` | Embedded fonts, icons, themes (from M4: `fonts/` with JetBrains Mono Regular and `OFL.txt`) |
| `examples/` | ≥ 5 68000 programs with expected outputs (from M3) |
| `bench/` | Bench inputs and outputs: `results/` and `generated/` are gitignored |
| `docs/` | `specs/` (architecture and milestone specs), `perf/` (measurement reports), `user/` (FR/EN guide, M8) |
| `scripts/` | Project-doc tooling (`gen_agencement.py`, `check_project_docs.py`) and `git-hooks/` |
| `.github/workflows/` | `ci.yml` (M0), `release.yml` (M8) |
| `.cargo/` | Cargo config (aliases, target flags) and `deny.toml` |

The M0 file tree and its ~6-month projection are in `docs/specs/m0-foundations.md` §2.
