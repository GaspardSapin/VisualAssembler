# M0 — Foundations: file-level specification

| | |
|---|---|
| Status | Ready for development |
| Owner | Architect role |
| Date | 2026-10-13 |
| Depends on | `docs/specs/architecture.md` (read it first: crate map, rules R1–R6, policies §9) |
| Milestone goal | Workspace, tooling, CI on 4 runners, a measurable empty egui window, and a renderer chosen from numbers |

Normative words: **must** = required; *rejected* = considered and refused, with the reason given.
Blocks marked *normative content* are the exact intended file contents. Developers may add comments
but must not change their values.

---

## 0. Exit criteria (all required)

| # | Criterion | How it is checked |
|---|---|---|
| E1 | `make test`, `make lint`, `make docs`, `make security` exit 0 on Windows (GNU make 4.4.1 + Git for Windows) and in WSL Ubuntu 26.04 | Run by QA on both |
| E2 | CI green: jobs `fmt`, `docs`, `deny` and `check` on the 4 runners. Only the step marked informational may fail. | GitHub Actions; `gh run list --workflow ci.yml` |
| E3 | `cargo build --release --locked --target aarch64-unknown-linux-gnu -p visual-assembler -p va` succeeds in WSL | QA, recorded in `docs/perf/M0-backend.md` ("compiled only" tier) |
| E4 | Windows release `visual-assembler.exe` and `va.exe` import neither `VCRUNTIME140.dll` nor `api-ms-win-crt-*` | `dumpbin /dependents` (VS developer prompt), output pasted into the report |
| E5 | `docs/perf/M0-backend.md` follows §12. The chosen renderer is the default feature, justified by numbers. | Performance agent + Orchestrator |
| E6 | `make hooks` installs the hook, and a commit message containing `Co-Authored-By: Claude …` is rejected | QA |
| E7 | `python scripts/check_project_docs.py .` exits 0 | QA, and the CI `docs` job |

---

## 1. Work packages (disjoint file scopes)

| WP | Role / wave | Files it owns |
|---|---|---|
| WP1 workspace | Developer, wave 2a (first) | `Cargo.toml`, `Cargo.lock` (generated, never hand-edited), `rust-toolchain.toml`, `.cargo/config.toml`, `.cargo/deny.toml`, `.gitattributes`, `crates/*/{Cargo.toml,src/lib.rs}`, `arch/m68k/{Cargo.toml,src/lib.rs}`. It also writes **stub** `apps/va`, `apps/visual-assembler` and `xtask` manifests plus `fn main() {}` so the workspace builds; WP2–WP4 then own and replace them. |
| WP2 gui | Developer, wave 2b | `apps/visual-assembler/**` (code and tests; READMEs belong to WP7) |
| WP3 cli | Developer, wave 2b | `apps/va/**` (code and tests) |
| WP4 xtask | Developer, wave 2b | `xtask/**` (code and tests) |
| WP5 automation | Developer, wave 2b | `Makefile`, `.github/workflows/ci.yml`, `scripts/check_project_docs.py`, `scripts/gen_agencement.py` |
| WP6 measurement | Performance, wave 4 | `docs/perf/M0-backend.md`, plus the outcome B/C/D follow-up of §12.4 (code change goes back to WP2) |
| WP7 docs | Documentarian, wave 6 | every `README.md`, `CLAUDE.md`, `AGENCEMENT.md`, `DEPENDANCE.md`, `IMPORT.md`, `.gitignore`, `.claudeignore`, `docs/README.md`, `docs/specs/README.md`, `docs/perf/README.md`, `bench/README.md`, `scripts/README.md`, `scripts/git-hooks/README.md` |
| — | Security, wave 3 | `SECURITY.md`: offline enforcement, unsafe allow-list, commit-msg hook, static CRT |

- Members add external dependencies only from the §3.1 list. A new dependency goes back to the Architect.
- `Cargo.lock` is deterministic because every pin is exact (`=`). Any WP may regenerate it, and the last writer wins.

---

## 2. File trees

### 2.1 Target tree at the end of M0

```
VisualAssembler/
├── .cargo/
│   ├── config.toml              # xtask alias, static CRT for windows-msvc, aarch64 Linux linker
│   └── deny.toml                # cargo-deny policy (licences, network ban, advisories, sources)
├── .github/workflows/ci.yml     # fmt, docs, cargo-deny, check × 4 runners
├── .gitattributes               # LF everywhere (hooks, rustfmt, Makefile)
├── .gitignore / .claudeignore   # existing; .claudeignore gains bench/results/
├── README.md, CLAUDE.md, AGENCEMENT.md, DEPENDANCE.md, IMPORT.md, SECURITY.md   # pilot files (WP7/Security)
├── Cargo.toml / Cargo.lock      # virtual workspace manifest, committed lock
├── Makefile                     # thin wrapper over cargo / cargo xtask
├── rust-toolchain.toml          # Rust 1.99.0 + rustfmt + clippy
├── apps/
│   ├── README.md
│   ├── va/                      # CLI binary "va"
│   │   ├── Cargo.toml, README.md
│   │   ├── src/{README.md, main.rs, cli.rs}
│   │   ├── src/commands/{README.md, mod.rs, info.rs}
│   │   └── tests/{README.md, binary.rs}
│   └── visual-assembler/        # GUI binary + lib (lib makes the UI testable headless)
│       ├── Cargo.toml, README.md
│       ├── src/{README.md, main.rs, lib.rs, cli.rs, spike.rs}
│       ├── src/bench/{README.md, mod.rs, recorder.rs, report.rs, process.rs,
│       │              process_windows.rs, process_linux.rs, platform.rs}
│       └── tests/{README.md, binary.rs, cli.rs, spike.rs, bench_report.rs, bench_process.rs}
├── arch/
│   ├── README.md
│   └── m68k/{Cargo.toml, README.md, src/{README.md, lib.rs}}   # package va-arch-m68k (skeleton)
├── bench/README.md              # results/ and generated/ are gitignored
├── crates/
│   ├── README.md                # table of crates + dependency rules (copy of architecture §2)
│   └── va-core/ va-editor/ va-lang/ va-debug/ va-workspace/ va-ui/
│       └── each: Cargo.toml, README.md, src/{README.md, lib.rs}   # skeletons
├── docs/
│   ├── README.md
│   ├── specs/{README.md, architecture.md, m0-foundations.md}
│   └── perf/{README.md, M0-backend.md}
├── scripts/
│   ├── README.md, check_project_docs.py, gen_agencement.py     # both patched (§10)
│   └── git-hooks/{README.md, commit-msg}
└── xtask/
    ├── Cargo.toml, README.md
    ├── src/{README.md, main.rs, lib.rs, ci.rs, docs.rs, hooks.rs, exec.rs, layering.rs, locate.rs}
    ├── src/bench/{README.md, mod.rs, runner.rs, stats.rs, report.rs, timestamp.rs}
    └── tests/{README.md, bench_stats.rs, bench_report.rs, bench_timestamp.rs, layering.rs}
```

- Root files: 13 (budget ≤ 15 leaves room for the licence file(s), §16).
- Deepest folders: `apps/visual-assembler/src/bench/` and `apps/va/src/commands/`, each 4 levels deep.
- The skeleton crates have no `tests/` folder yet. It appears with their first test in M1.

### 2.2 Projected tree at ~6 months (after M5/M6)

| Path | Purpose |
|---|---|
| `crates/va-core/src/{arch,syntax,asm,emu,diag,image,source}/` | Traits and types grouped by contract |
| `crates/va-editor/src/{buffer,selection,history,search}/` | Rope buffer, multi-cursor, undo groups, regex search |
| `crates/va-lang/src/{analysis,symbols,navigation,completion,hover,outline}/` | Background assembly service + language features |
| `crates/va-debug/src/{session,breakpoints,stepping,history}/` | Worker loop, line↔address mapping, step over/out, undo |
| `crates/va-workspace/src/{project,settings,keybindings,session,backup,i18n,watch}/` | Files of architecture §6, Fluent loading |
| `crates/va-ui/src/{editor,gutter,tabs,explorer,palette,problems,debug,theme}/` | egui widgets and panels |
| `crates/va-gdb/` | (after V1) GDB RSP emulator adapter, architecture §10.2 |
| `arch/m68k/src/{isa,syntax,asm,disasm,emu,io}/` | 68000 implementation |
| `arch/m68k/locales/{en,fr}/` | m68k diagnostics + instruction docs (`.ftl`) |
| `arch/m68k/tests/{isa,syntax,asm,disasm,emu,io,fixtures}/` | Mirrors `src/`; golden files in `fixtures/` |
| `arch/m68k/benches/` | criterion: lexer, assembler |
| `arch/<cpu>/` | Future CPUs (z80, mos6502, riscv, …), same layout |
| `apps/visual-assembler/src/{app,bench}/` + `arch_registry.rs` | Shell wiring, bench, CPU registration |
| `apps/va/src/commands/` | `info, lex, isa, asm, run, debug, symbols, hover` (one file each) |
| `xtask/src/{bench,package,generate,sst}/` | Bench, cargo-packager driving, 5k/100k file generator, SingleStepTests fetch |
| `locales/{en,fr}/{ui,diagnostics}.ftl` | Architecture-independent strings |
| `assets/{fonts,icons,themes}/` | JetBrains Mono, app icon, themes |
| `examples/<nn>-<name>/` | ≥ 5 programs + expected outputs |
| `docs/user/{en,fr}/` | User guide (M8) |
| `.github/workflows/release.yml` | Packaging, x86_64 + aarch64 (M8) |

---

## 3. Workspace and toolchain (WP1)

### 3.1 Root `Cargo.toml` (normative content)

```toml
[workspace]
resolver = "3"
members = [
    "crates/va-core",
    "crates/va-editor",
    "crates/va-lang",
    "crates/va-debug",
    "crates/va-workspace",
    "crates/va-ui",
    "arch/m68k",
    "apps/visual-assembler",
    "apps/va",
    "xtask",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.99"
publish = false
# No `license` key until the project licence is decided (m0-foundations.md §16).

# Exact pins. Only crates used by at least one member (cargo-deny: unused = "deny").
# Internal crates are added here when first used (M1+), e.g. va-core = { path = "crates/va-core" }.
# Every entry needs a DEPENDANCE.md line and an IMPORT.md entry.
[workspace.dependencies]
anyhow = "=1.0.104"
clap = { version = "=4.6.7", features = ["derive"] }
eframe = { version = "=0.36.2", default-features = false, features = ["accesskit", "default_fonts", "wayland", "x11"] }
egui_kittest = "=0.36.2"
libc = "=0.2.189"
raw-window-handle = "=0.6.2"
serde = { version = "=1.0.229", features = ["derive"] }
serde_json = "=1.0.151"
windows-sys = { version = "=0.61.2", features = ["Win32_Foundation", "Win32_System_ProcessStatus", "Win32_System_Threading"] }

[workspace.lints.rust]
unsafe_code = "deny"
missing_docs = "warn"
rust_2018_idioms = { level = "warn", priority = -1 }

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
dbg_macro = "warn"
todo = "warn"
unimplemented = "warn"
unwrap_used = "warn"
undocumented_unsafe_blocks = "warn"

[profile.release]
lto = "thin"

[profile.dev.package."*"]
opt-level = 2
```

- `eframe` has default features off. As a result, `links` (opens URLs in a browser) is excluded, and each renderer is opted into by `visual-assembler` (§4.1). Wayland and X11 libraries still load at runtime: `wayland-sys` resolves with its `dlopen` feature, and `x11-dl`/`xkbcommon-dl` dlopen their libraries (verified on the eframe 0.36.2 graph, §14 F9).
- CI runs clippy with `-D warnings`, so every `warn` above blocks a merge.
- The dev profile builds dependencies at `opt-level = 2`, which keeps a debug GUI usable. Workspace crates stay at opt-level 0 for debuggability.
- *Rejected:* `lto = "fat"` + `codegen-units = 1` for `release`: too slow for CI and the dev loop. A `dist` profile with those settings arrives in M8.

### 3.2 `rust-toolchain.toml` (normative content)

```toml
[toolchain]
channel = "1.99.0"
profile = "minimal"
components = ["rustfmt", "clippy"]
```

- No `targets` key. Cross targets are added explicitly where needed (`rustup target add aarch64-unknown-linux-gnu` in WSL), so every machine and runner is not forced to download extra std libraries.

### 3.3 `.cargo/config.toml` (normative content)

```toml
[alias]
xtask = "run --package xtask --"

# Static MSVC runtime: Windows binaries need no VC++ redistributable.
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]

[target.aarch64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]

# aarch64 Linux: cross builds from x86_64 (apt: gcc-aarch64-linux-gnu). Native arm64 Ubuntu
# ships the same triplet-prefixed gcc name, so the ubuntu-24.04-arm runner works unchanged.
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```

- The `RUSTFLAGS` env var replaces these `rustflags` entirely. It **must not** be set in CI, scripts or the Makefile. Warnings are denied through `clippy -- -D warnings` instead.

### 3.4 `.gitattributes` (normative content)

```
* text=auto eol=lf
*.png binary
*.ico binary
*.ttf binary
*.otf binary
*.woff2 binary
```

- Windows runners and many Windows machines default to `core.autocrlf=true`. Without this file, `scripts/git-hooks/commit-msg` would get a `#!/bin/sh\r` shebang, and `cargo fmt --check` would report line-ending diffs.

### 3.5 Formatting and lint configuration

- **No `rustfmt.toml`.** rustfmt defaults apply: `max_width = 100`, and the style edition is inferred from edition 2024. `.gitattributes` guarantees LF line endings.
- **No `clippy.toml`.** Lints live in `[workspace.lints]`, and every member has `[lints] workspace = true`.
- Every crate root (`lib.rs`, `main.rs`) and every test file starts with a `//!` doc comment.
- Test files also start with `#![allow(clippy::unwrap_used)]`.
- *Rejected:* `rustfmt.toml`/`clippy.toml` holding only cosmetic options. They add root files without changing behaviour, and clippy's test detection does not cover integration-test helper functions.

### 3.6 Member manifests

Template, used by every package (normative):

```toml
[package]
name = "<package>"
description = "<one line, see table>"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[lints]
workspace = true
```

| Package (path) | `description` | M0 dependencies |
|---|---|---|
| `va-core` (`crates/va-core`) | CPU-agnostic traits and data types of VisualAssembler. | none |
| `va-editor` (`crates/va-editor`) | Text buffer, selections, undo/redo and search, without UI. | none |
| `va-lang` (`crates/va-lang`) | Language services and background assembly, without UI. | none |
| `va-debug` (`crates/va-debug`) | Debug session worker driving any Emulator, without UI. | none |
| `va-workspace` (`crates/va-workspace`) | Project, settings, session, backups and localization, without UI. | none |
| `va-ui` (`crates/va-ui`) | egui widgets and panels of the IDE. | none |
| `va-arch-m68k` (`arch/m68k`) | Motorola 68000 support: ISA, syntax, assembler, disassembler, emulator, I/O. | none |
| `visual-assembler` (`apps/visual-assembler`) | VisualAssembler desktop IDE. | see §4.1 |
| `va` (`apps/va`) | VisualAssembler command-line interface. | `clap`, `serde`, `serde_json` |
| `xtask` (`xtask`) | Developer task runner (bench, ci, docs, hooks). | `anyhow`, `clap`, `serde`, `serde_json` |

Each skeleton `src/lib.rs` contains **only** a crate-level doc comment with three paragraphs:
1. The responsibility (copy of the architecture §2 cell).
2. `Allowed internal dependencies: …` (copy of the architecture §2 cell).
3. `M0 skeleton: no logic yet; first content in <milestone>`, where the milestone is va-core → M1, va-arch-m68k → M1, va-editor → M4, va-workspace → M4, va-ui → M4, va-lang → M5, va-debug → M3.

Internal dependencies are declared nowhere in M0: the apps wire crates only when they first use them.
*Rejected:* declaring every internal dependency now. Unused dependencies are noise, and cargo-deny's
`unused` check would flag them in `[workspace.dependencies]`.

---

## 4. GUI spike — `apps/visual-assembler` (WP2)

### 4.1 Manifest additions (normative)

```toml
[features]
default = ["renderer-wgpu"]
renderer-wgpu = ["eframe/wgpu"]
renderer-glow = ["eframe/glow"]

[dependencies]
clap = { workspace = true }
eframe = { workspace = true }
raw-window-handle = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[target.'cfg(windows)'.dependencies]
windows-sys = { workspace = true }

[target.'cfg(target_os = "linux")'.dependencies]
libc = { workspace = true }

[dev-dependencies]
egui_kittest = { workspace = true }
```

Targets are auto-discovered: the lib is `visual_assembler` (`src/lib.rs`), the bin is `visual-assembler` (`src/main.rs`).

### 4.2 Behaviour

- One window:
  - title `VisualAssembler`;
  - Wayland app id `visualassembler`;
  - inner size 1280×800 logical px, centered;
  - eframe defaults otherwise: vsync on, no persistence.
- Static UI drawn by `spike::draw`:
  - `CentralPanel::default_margins().show_inside(ui, …)`;
  - `ui.heading(WINDOW_TITLE)`;
  - `ui.label(SUBTITLE)`, with `SUBTITLE = "Foundations build (M0): the editor, assembler and debugger arrive in later milestones."`.
  - These English literals are a documented M0 exception to the i18n rule. M4 replaces them with Fluent ids.
- **Reactive repaint only.** The normal path never calls `request_repaint*`. The only repaint requests in the whole binary are the bench's (§4.5).
- `#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]` in `main.rs`. In Windows release builds, stdout and stderr exist only when redirected (xtask pipes them, so bench works). `visual-assembler --help` typed in a console prints nothing there, which is documented in the app README; use the debug build or `va` for terminal work.
- `compile_error!` in `lib.rs` when neither `renderer-wgpu` nor `renderer-glow` is enabled.

### 4.3 Command line and exit codes

```
visual-assembler [--renderer <wgpu|glow>] [--bench <startup|idle>] [--duration-secs <N>]
visual-assembler --help | --version          # version line: "visual-assembler 0.1.0"
```

| Flag | Values | Rule |
|---|---|---|
| `--renderer` | `wgpu`, `glow` | Default: `wgpu` if compiled in, else `glow`. Asking for a renderer that is not compiled in is a usage error. |
| `--bench` | `startup`, `idle` | Runs a measurement scenario (§4.5), prints exactly one JSON line on stdout, exits. |
| `--duration-secs` | 1..=3600, default 10 | Only valid with `--bench idle`; anywhere else it is a usage error. |

| Exit code | Meaning |
|---|---|
| 0 | Normal exit, or a bench scenario completed |
| 2 | Usage error (clap errors; the `resolve` errors of §4.4). Message on stderr starting with `error:`. |
| 3 | Window or renderer creation failed (`eframe::run_native` returned `Err`); message on stderr |
| 4 | Bench watchdog fired: no measurement within the hard limit (§4.5); message on stderr |

### 4.4 Modules (public API used by tests)

| File | Content |
|---|---|
| `src/main.rs` | `fn main() -> ExitCode`. Its **first statement** is `let started = Instant::now();`, then `visual_assembler::run(started)`. |
| `src/lib.rs` | `pub mod bench; pub mod cli; pub mod spike;` · `pub const EXIT_USAGE: u8 = 2; EXIT_WINDOW: u8 = 3; EXIT_BENCH_TIMEOUT: u8 = 4;` · `pub fn run(started: Instant) -> ExitCode`: parse (`GuiArgs::try_parse`; clap errors call `e.print()` and exit with `e.exit_code()`), call `cli::resolve`, build `NativeOptions`, call `eframe::run_native("VisualAssembler", …)`. |
| `src/cli.rs` | `#[derive(Parser)] pub struct GuiArgs { renderer: Option<RendererChoice>, bench: Option<BenchScenario>, duration_secs: Option<u64> }` · `#[derive(ValueEnum)] pub enum RendererChoice { Wgpu, Glow }` and `pub enum BenchScenario { Startup, Idle }`, both always present whatever the features · `pub fn compiled_renderers() -> &'static [RendererChoice]` (cfg-built) · `pub fn resolve(args: &GuiArgs, compiled: &[RendererChoice]) -> Result<LaunchPlan, UsageError>` · `pub struct LaunchPlan { pub renderer: RendererChoice, pub bench: Option<BenchPlan> }` · `pub struct BenchPlan { pub scenario: BenchScenario, pub idle_duration: Duration }` · `UsageError: Display`, messages `renderer '<x>' is not compiled into this binary (build with --features renderer-<x>)` and `--duration-secs requires --bench idle`. |
| `src/spike.rs` | `pub const WINDOW_TITLE`, `pub const SUBTITLE`, `pub fn draw(ui: &mut egui::Ui)`, `pub struct SpikeApp` implementing `eframe::App`. `logic` forwards to `BenchRecorder::on_logic`; `ui` calls `draw` and then `BenchRecorder::on_ui`. |
| `src/bench/mod.rs` | Constants `IDLE_SETTLE_MS = 1000`, `DEFAULT_IDLE_SECS = 10`, `STARTUP_WATCHDOG_SECS = 60`, `IDLE_WATCHDOG_EXTRA_SECS = 60`; module declarations (`process_windows`/`process_linux` are private and cfg-gated). |
| `src/bench/recorder.rs` | `pub struct BenchRecorder` running the algorithm of §4.5. |
| `src/bench/report.rs` | `pub const RUN_SCHEMA: &str = "va.bench.run/1"`, `#[derive(Serialize)] pub struct BenchRun` (fields of §4.6, in that order), `pub fn to_json_line(&self) -> String` (single line, no trailing newline). |
| `src/bench/process.rs` | `pub struct ProcessStats { pub cpu_ms: Option<f64>, pub peak_rss_bytes: Option<u64>, pub peak_private_bytes: Option<u64> }` · `ProcessStats::sample()` dispatches to the OS module and returns all `None` on other OSes · `pub fn parse_vm_hwm(proc_status: &str) -> Option<u64>` (portable; returns bytes). |
| `src/bench/process_windows.rs` | `#[allow(unsafe_code)]`: `GetProcessTimes(GetCurrentProcess())` → kernel + user `FILETIME` (100 ns units) ÷ 10,000 = ms; `GetProcessMemoryInfo` → `PeakWorkingSetSize` (peak RSS) and `PeakPagefileUsage` (peak private bytes). |
| `src/bench/process_linux.rs` | `#[allow(unsafe_code)]`: `libc::getrusage(RUSAGE_SELF)` → `ru_utime + ru_stime` in ms; `VmHWM` read from `/proc/self/status` (std::fs) via `parse_vm_hwm`; `peak_private_bytes = None`. |
| `src/bench/platform.rs` | `pub struct RendererInfo { renderer, backend, adapter, driver, window_system }` · `RendererInfo::from_frame(&eframe::Frame)` · `pub fn is_wsl() -> bool` (Linux: `/proc/sys/kernel/osrelease` contains `microsoft` or `WSL`, case-insensitive; else `false`). |

`RendererInfo::from_frame` sources:

| Field | wgpu (`frame.wgpu_render_state()`, `renderer-wgpu`) | glow (`frame.gl()`, `renderer-glow`) |
|---|---|---|
| `renderer` | `"wgpu"` | `"glow"` |
| `backend` | `adapter.get_info().backend.to_str()` → `dx12`/`vulkan`/`gl`/`metal`/`noop` | `"gl"` |
| `adapter` | `AdapterInfo::name` | `GL_RENDERER` (`unsafe { gl.get_parameter_string(glow::RENDERER) }`, via `eframe::glow::HasContext`; this is the file's only unsafe) |
| `driver` | `format!("{} {}", info.driver, info.driver_info)`, trimmed | `GL_VERSION` |
| `window_system` | `frame.window_handle()` (`raw_window_handle::HasWindowHandle`) → `Win32`→`win32`, `Wayland`→`wayland`, `Xlib`→`xlib`, `Xcb`→`xcb`, anything else or an error → `unknown` | same |

### 4.5 Bench algorithm (normative)

Definitions (architecture §7):

- **First frame presented** is the start of the first `App::logic` call where `ctx.cumulative_frame_nr() >= 1`.
- eframe paints frame 1, presents it, then makes the window visible (`post_rendering`) before running frame 2. Display latency after `present` is not measured.

Steps:

1. `BenchRecorder::new(plan, started)` spawns a watchdog thread. After `STARTUP_WATCHDOG_SECS` (startup) or `IDLE_WATCHDOG_EXTRA_SECS + duration + 1 s` (idle), it prints `error: bench timed out` to stderr and calls `std::process::exit(4)`.
2. `on_ui(ctx, frame)`, every frame:
   - store `ctx.cumulative_frame_nr() + 1` in `frames: Arc<AtomicU64>`;
   - on the first call, capture `RendererInfo::from_frame(frame)` and call `ctx.request_repaint()` once, so that frame 2 exists for the measurement.
3. `on_logic(ctx, frame)`: on the first call with `ctx.cumulative_frame_nr() >= 1` and no recorded first frame:
   - set `first_frame = Instant::now()` and `startup_ms = (first_frame - started)` in ms (f64);
   - **startup** scenario: `stats = ProcessStats::sample()`; build `BenchRun` with `frames_painted = frames.load()` (expected 1) and `process_cpu_ms = stats.cpu_ms` (CPU since process creation); print the line, flush stdout, `ctx.send_viewport_cmd(ViewportCommand::Close)`;
   - **idle** scenario: spawn the thread `va-bench-idle` with a clone of `ctx`, `frames`, `startup_ms` and `RendererInfo`. That thread:
     1. sleeps `IDLE_SETTLE_MS`;
     2. takes sample A = (`Instant`, `ProcessStats`, `frames`);
     3. sleeps `duration`;
     4. takes sample B;
     5. builds `BenchRun` with `frames_painted = B.frames − A.frames` (expected 0), `process_cpu_ms = B.cpu − A.cpu`, `idle_window_ms = B.t − A.t`, `settle_ms = 1000`;
     6. prints the line, flushes, sends `ViewportCommand::Close`.

     The UI thread is never woken during the window, so measuring does not pollute what is measured.
4. In bench mode nothing else may print to stdout. There is no logger in M0.
5. Process exit code 0 after the window closes. If the user closes the window before the JSON line is printed, xtask treats the run as a failure.

### 4.6 JSON line schema `va.bench.run/1` (normative; keys in this order)

| Key | Type | Startup | Idle |
|---|---|---|---|
| `schema` | string | `"va.bench.run/1"` | same |
| `scenario` | string | `"startup"` | `"idle"` |
| `renderer` | string | `"wgpu"` \| `"glow"` | same |
| `backend` | string | e.g. `"dx12"`, `"vulkan"`, `"gl"` | same |
| `adapter` | string \| null | GPU/adapter name | same |
| `driver` | string \| null | driver / GL version string | same |
| `window_system` | string | `win32` \| `wayland` \| `xlib` \| `xcb` \| `unknown` | same |
| `wsl` | bool | `platform::is_wsl()` | same |
| `os` / `arch` | string | `std::env::consts::OS` / `ARCH` | same |
| `version` | string | `CARGO_PKG_VERSION` | same |
| `startup_ms` | number | `main()` → first frame presented | same |
| `frames_painted` | integer | frames started until measurement (expected 1) | frames during the idle window (expected 0) |
| `process_cpu_ms` | number \| null | process CPU since creation | process CPU during the idle window |
| `peak_rss_bytes` | integer \| null | lifetime peak (Windows PeakWorkingSetSize, Linux VmHWM) | same |
| `peak_private_bytes` | integer \| null | Windows PeakPagefileUsage; Linux `null` | same |
| `settle_ms` | integer \| null | `null` | `1000` |
| `idle_window_ms` | number \| null | `null` | measured window length |

Example (values illustrative):
`{"schema":"va.bench.run/1","scenario":"idle","renderer":"wgpu","backend":"dx12","adapter":"AMD Radeon(TM) 860M Graphics","driver":"…","window_system":"win32","wsl":false,"os":"windows","arch":"x86_64","version":"0.1.0","startup_ms":182.4,"frames_painted":0,"process_cpu_ms":15.6,"peak_rss_bytes":91226112,"peak_private_bytes":40960000,"settle_ms":1000,"idle_window_ms":10000.7}`

*Rejected for OS statistics:* the `sysinfo` crate, which is a large multi-OS abstraction pulling many
dependencies for two numbers. `windows-sys` 0.61.2 is already in eframe's graph (no duplicate), and
`libc` 0.2.189 is already pulled by winit/wgpu.

---

## 5. CLI skeleton — `apps/va` (WP3)

- `src/cli.rs`:
  - `#[derive(Parser)] #[command(name = "va", version, about = "VisualAssembler command-line interface", arg_required_else_help = true)] struct Cli { #[arg(long, global = true)] json: bool, #[command(subcommand)] command: Command }`;
  - `enum Command { /// Print build information. Info }`.
- `src/commands/info.rs`:
  - builds `InfoReport { version, os, arch, architectures: Vec<String> }` (`architectures` is empty in M0: no CPU is registered before M1);
  - human output on stdout: `va 0.1.0\nos: <os>\narch: <arch>\narchitectures: (none)\n`;
  - with `--json`, exactly one line: `{"schema":"va.info/1","version":"0.1.0","os":"windows","arch":"x86_64","architectures":[]}`.
- `src/main.rs`: `Cli::try_parse()`. On error: `e.print()` and exit with `e.exit_code()`. That gives 0 for `--help`/`--version` on stdout, and 2 for a missing subcommand or a bad flag, with the message on stderr (clap 4.6.7 behaviour, verified in clap_builder source).
- **Conventions fixed now** (architecture G5), to be implemented by the first command that needs them:

  | Topic | Convention |
  |---|---|
  | Exit codes | 0 success · 1 the operation ran and found problems (e.g. assembly errors) · 2 usage · 3 I/O or environment error · 101 panic (always a bug) |
  | Human errors | stderr, prefixed `error: ` |
  | `--json` | global flag. stdout carries exactly one JSON document (or JSON Lines for streaming commands such as `run --trace`). Every document has `"schema": "va.<command>/<major>"`. A fatal error in JSON mode prints `{"schema":"va.error/1","code":"<kebab>","message":"<english>"}` on stdout plus a human line on stderr. clap parse errors stay human-only. |
  | `--lang en\|fr` | global from M2 |

  M0 does not implement the error printer or the codes 1/3: dead code would fail `-D warnings`.

---

## 6. xtask (WP4)

`cargo xtask <command>` runs with the workspace root as working directory. The root comes from
`env!("CARGO_MANIFEST_DIR")/..`; the target directory comes from `cargo metadata --format-version 1 --no-deps` (`target_directory`).

| Command | Behaviour | Exit |
|---|---|---|
| `bench [startup\|idle\|all] [--renderer wgpu\|glow\|all] [--runs N] [--warmup N] [--duration-secs N] [--json]` | §6.1 | 0 ok · 1 build/run failure or timeout · 2 usage |
| `ci [--no-fmt]` | Runs, in order and stopping at the first failure: (1) `cargo fmt --all -- --check` (skipped with `--no-fmt`); (2) `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; (3) `cargo clippy -p visual-assembler --all-targets --no-default-features --features renderer-glow --locked -- -D warnings`; (4) the in-process layering + source checks (§6.3); (5) `cargo test --workspace --all-features --locked`. Prints `==> <step>` to stderr before each step. | 0 / 1 |
| `docs [--check]` | Runs `<python> scripts/gen_agencement.py .` (skipped with `--check`), then `<python> scripts/check_project_docs.py .`. `<python>` = `$VA_PYTHON`, else `python` on Windows, else `python3`. *Rejected:* trying `python3` first on Windows, where it can resolve to the Microsoft Store stub. | the check script's code; 1 if Python is missing |
| `hooks` | `git rev-parse --git-path hooks` → create the folder if missing → copy `scripts/git-hooks/commit-msg` there → on Unix, set mode 0o755 → print `installed <path>`. | 0 / 1 |

Modules:

| File | Role |
|---|---|
| `main.rs` | clap dispatch; prints `error: {e:#}` on stderr for any error |
| `lib.rs` | Exposes `bench`, `ci`, `docs`, `hooks`, `layering`, `locate`, `exec` |
| `exec.rs` | Runs an external command with inherited stdio; turns a non-zero status into an error |
| `locate.rs` | Workspace root and target directory |
| `ci.rs`, `docs.rs`, `hooks.rs`, `layering.rs` | One command or check each |
| `bench/mod.rs` | `BenchArgs` and orchestration |
| `bench/runner.rs` | Build and spawn |
| `bench/stats.rs` | Pure statistics |
| `bench/report.rs` | Report JSON, markdown table, file name |
| `bench/timestamp.rs` | Pure UTC formatting |

### 6.1 Bench orchestration

Defaults:

| Option | Default |
|---|---|
| scenario | `all` (= startup then idle) |
| `--renderer` | `wgpu` |
| `--runs` | 20 for startup, 3 for idle (one value overrides both) |
| `--warmup` | 1 |
| `--duration-secs` | 10 |

For each renderer, in the order `wgpu`, `glow`:

1. **Build:** `cargo build --release -p visual-assembler --no-default-features --features renderer-<r> --message-format=json-render-diagnostics`. Take the `executable` of the `compiler-artifact` message whose target is `visual-assembler`. Copy it to `<target_directory>/bench/<r>/visual-assembler[.exe]` and record `binary_bytes`.
   - Each measured binary contains exactly one renderer, as shipped. *Rejected:* one binary with both renderers and a runtime switch, which would measure a stack that never ships and hide the binary-size difference.
2. **Runs:** for each scenario, run `warmup` runs (discarded, but they must succeed), then `runs` measured runs, sequentially. Each run:
   1. `Instant` t0;
   2. spawn `<copy> --bench <scenario> [--duration-secs N]` with stdin null, stdout and stderr piped;
   3. a reader thread sends `(Instant, line)` over an mpsc channel; the main thread waits with `recv_timeout`. The timeout is 60 s for startup and `N + 60` s for idle; on timeout, kill the child;
   4. the first stdout line that parses as JSON with `schema == "va.bench.run/1"` gives `spawn_to_first_frame_ms` = its receipt time − t0;
   5. wait for exit (`try_wait` polling, same deadline).

   A non-zero exit, a missing line, or a timeout aborts the whole command with exit code 1 and prints the child's stderr. No report is written.
3. The parent environment passes through unchanged, so `WGPU_BACKEND=gl cargo xtask bench`, or `env -u WAYLAND_DISPLAY …` (forces X11 in WSLg), is how backends and window systems are selected. The report records those variables.
4. **Report:** write `bench/results/<YYYYMMDDTHHMMSSZ>-<sha8>.json` (create the folder; on a name collision add `-2`, `-3`, …). Then print the markdown table and `report: <path>` on stdout, or the report JSON instead of both with `--json`.
   - `sha8` comes from `git rev-parse --short=8 HEAD` (`nogit` when that fails).
   - `dirty` comes from `git status --porcelain --untracked-files=no` being non-empty.

### 6.2 Report schema `va.bench.report/1` (normative)

```json
{
  "schema": "va.bench.report/1",
  "created_utc": "2026-10-20T14:03:11Z",
  "git": { "sha": "1a2b3c4d", "dirty": false },
  "host": { "os": "windows", "arch": "x86_64", "wsl": false, "logical_cpus": 16 },
  "profile": "release",
  "results": [
    {
      "scenario": "startup",
      "renderer": "wgpu",
      "binary_bytes": 9876543,
      "warmup": 1,
      "runs": 20,
      "duration_secs": null,
      "env": { "WGPU_BACKEND": null, "WGPU_ADAPTER_NAME": null, "WGPU_POWER_PREF": null,
               "WAYLAND_DISPLAY": null, "DISPLAY": null },
      "backend": "dx12", "adapter": "…", "driver": "…", "window_system": "win32",
      "summary": {
        "spawn_to_first_frame_ms": { "median": 0.0, "p95": 0.0, "min": 0.0, "max": 0.0 },
        "startup_ms":          { "median": 0.0, "p95": 0.0, "min": 0.0, "max": 0.0 },
        "process_cpu_ms":      { "…": "same shape, or null" },
        "peak_rss_bytes":      { "…": "same shape, or null" },
        "peak_private_bytes":  null,
        "frames_painted":      { "…": "same shape" },
        "idle_cpu_percent":    null
      },
      "samples": [ { "spawn_to_first_frame_ms": 205.1, "run": { "schema": "va.bench.run/1", "…": "…" } } ]
    }
  ]
}
```

- `host.logical_cpus` comes from `std::thread::available_parallelism`.
- `host.wsl` uses the same rule as the GUI's `is_wsl`.
- `backend`/`adapter`/`driver`/`window_system` are taken from the measured samples, or `"mixed"` when they differ.
- A summary metric is `null` when any sample has `null` for it.
- `idle_cpu_percent` exists for idle results only: per sample, `process_cpu_ms / idle_window_ms × 100`.

**Stats** (`bench/stats.rs`): `pub fn summarize(values: &[f64]) -> Option<Summary>`, where `Summary { median, p95, min, max }`.

- Returns `None` for an empty input or any non-finite value.
- Median: the middle value, or the mean of the two middle values when n is even.
- p95: **nearest rank**, i.e. the value at 1-based rank `ceil(0.95 × n)` of the sorted values, clamped to [1, n].
- The input order does not matter.

**Markdown table** (one row per result; ms with 1 decimal, % with 2, MiB with 1, `n/a` for null):

```
| scenario | renderer | backend | window | spawn→frame p50/p95 ms | startup p50/p95 ms | idle CPU p50/p95 % | frames p50/max | peak RSS p50 MiB | binary MiB |
```

**Timestamp** (`bench/timestamp.rs`): computed from `SystemTime` with the days→civil-date algorithm, no dependency.

- `pub fn utc_compact(t: SystemTime) -> String` gives `YYYYMMDDTHHMMSSZ`.
- `pub fn utc_iso(t: SystemTime) -> String` gives `YYYY-MM-DDTHH:MM:SSZ`.
- `report.rs` provides `pub fn report_file_name(t: SystemTime, sha8: &str) -> String`.

### 6.3 Layering and source checks (`layering.rs`)

- `pub fn check_layering(metadata: &serde_json::Value) -> Vec<Violation>`. Its input is the output of `cargo metadata --format-version 1 --no-deps`. A dependency is internal when its name is a workspace package. Rules (all dependency kinds):

  | Package | Allowed internal dependencies |
  |---|---|
  | `va-core`, `xtask` | none |
  | `va-editor`, `va-debug`, `va-workspace`, `va-arch-*` | `va-core` |
  | `va-lang` | `va-core`, `va-editor` |
  | `va-ui` | `va-core`, `va-editor`, `va-lang`, `va-debug`, `va-workspace` |
  | `va` | everything except `va-ui` |
  | `visual-assembler` | everything |
  | any other package | violation `no layering rule for <name>` (forces the table to be updated) |

  The GUI crate set (`eframe`, `egui`, `egui-winit`, `egui-wgpu`, `egui_glow`, `egui_kittest`, `winit`, `wgpu`, `glow`, `glutin`, `raw-window-handle`) may appear only in `visual-assembler`, plus `egui`/`egui_kittest` in `va-ui`.
- `pub fn scan_sources(root: &Path) -> std::io::Result<Vec<Violation>>`. It walks `crates/`, `arch/` and `apps/` under `root` (all `.rs` files, skipping `target/`):
  - **network:** the substrings `std::net`, `TcpStream`, `TcpListener`, `UdpSocket` are forbidden. `NET_ALLOWLIST` is empty in M0;
  - **unsafe:** `allow(unsafe_code)` is forbidden outside `UNSAFE_ALLOWLIST = ["apps/visual-assembler/src/bench/process_windows.rs", "apps/visual-assembler/src/bench/process_linux.rs", "apps/visual-assembler/src/bench/platform.rs"]`.
- `Violation { rule: &'static str, location: String, detail: String }` implements `Display`.
- xtask itself is not scanned (it contains these strings as data). cargo-deny still covers its dependencies.

---

## 7. Makefile (WP5, normative content)

```make
# VisualAssembler: every project command. Thin wrapper; all logic lives in cargo / cargo xtask.
# Each recipe line is a single command with no shell syntax, so it behaves the same whether
# GNU make uses sh (Git for Windows, Ubuntu) or cmd.exe (Windows without sh on PATH).

CARGO    ?= cargo
SCENARIO ?= all
RENDERER ?= wgpu
RUNS     ?=

.DEFAULT_GOAL := help
.PHONY: help install hooks dev run test lint format security docs build clean bench ci

help:
	$(info VisualAssembler - make targets)
	$(info   install   install the pinned toolchain and cargo-deny 0.20.2 then fetch crates)
	$(info   hooks     install the commit-msg git hook)
	$(info   dev       run the GUI from a debug build)
	$(info   run       run the GUI from a release build)
	$(info   test      run every test of the workspace)
	$(info   lint      check formatting and run clippy with warnings as errors)
	$(info   format    format all Rust code)
	$(info   security  run cargo-deny on advisories bans licenses and sources)
	$(info   docs      regenerate AGENCEMENT.md and verify the documentation ritual)
	$(info   build     release build of the GUI and the CLI)
	$(info   clean     remove build artifacts)
	$(info   bench     measure the GUI - SCENARIO=startup|idle|all RENDERER=wgpu|glow|all RUNS=n)
	$(info   ci        run the CI checks locally - fmt clippy layering tests)

install:
	rustup toolchain install
	$(CARGO) install cargo-deny --version 0.20.2 --locked
	$(CARGO) fetch

hooks:
	$(CARGO) xtask hooks

dev:
	$(CARGO) run -p visual-assembler

run:
	$(CARGO) run --release -p visual-assembler

test:
	$(CARGO) test --workspace --all-features

lint:
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

format:
	$(CARGO) fmt --all

security:
	$(CARGO) deny check

docs:
	$(CARGO) xtask docs

build:
	$(CARGO) build --release -p visual-assembler -p va

clean:
	$(CARGO) clean

bench:
	$(CARGO) xtask bench $(SCENARIO) --renderer $(RENDERER) $(if $(RUNS),--runs $(RUNS))

ci:
	$(CARGO) xtask ci
```

- `help` uses make's built-in `$(info)`, so no shell runs at all. Avoid commas inside `$(info …)`.
- `rustup toolchain install` with no argument installs the active toolchain from `rust-toolchain.toml` (rustup ≥ 1.28; the machine has 1.29.1).
- Local `test`/`build` targets omit `--locked` for ergonomics. CI and `cargo xtask ci` always pass it.
- *Rejected:* recipes with `&&`, `if`, `rm -rf` or `cp`, which break under cmd.exe. Logic that needs a shell becomes an xtask command (e.g. `hooks`).

---

## 8. CI — `.github/workflows/ci.yml` (WP5, normative content)

Actions are pinned to commit SHAs, resolved on 2026-10-13. Bumping one requires re-resolving its SHA.

```yaml
name: ci

on:
  push:
    branches: [main]
  pull_request:
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: "1"
  # Never set RUSTFLAGS here: it would override .cargo/config.toml (+crt-static).

jobs:
  fmt:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - uses: dtolnay/rust-toolchain@e2a55d2ffb04f378e9626c28d38b36d230d1e12f # master, 2026-10-13
        with:
          toolchain: "1.99.0"
          components: rustfmt
      - run: cargo fmt --all -- --check

  docs:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
          fetch-depth: 50          # check_project_docs.py inspects the last 50 commits
      - run: python3 scripts/check_project_docs.py .

  deny:
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - uses: EmbarkStudios/cargo-deny-action@3c6349835b2b7b196a839186cb8b78e02f7b5f25 # v2.1.1 (cargo-deny 0.20.2)
        with:
          command: check
          rust-version: "1.99.0"   # the action's image ships Rust 1.85

  check:
    name: check (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    timeout-minutes: 60
    strategy:
      fail-fast: false
      matrix:
        os: [windows-latest, windows-11-arm, ubuntu-latest, ubuntu-24.04-arm]
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - uses: dtolnay/rust-toolchain@e2a55d2ffb04f378e9626c28d38b36d230d1e12f # master, 2026-10-13
        with:
          toolchain: "1.99.0"
          components: clippy
      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2
        with:
          key: ${{ matrix.os }}
          save-if: ${{ github.ref == 'refs/heads/main' }}
      - name: clippy, layering, tests
        run: cargo xtask ci --no-fmt
      - name: release build
        run: cargo build --release --locked -p visual-assembler -p va
      - name: GUI startup smoke (informational, never a support claim)
        if: runner.os == 'Windows'
        continue-on-error: true
        timeout-minutes: 3
        shell: bash
        run: ./target/release/visual-assembler.exe --bench startup
```

- **Linux runners need no apt packages in M0.** The build scripts were checked:
  - `wayland-sys` resolves with `dlopen`, so it does not link;
  - `x11-dl` only queries pkg-config for library dirs and tolerates failures;
  - `khronos-egl` uses pkg-config only with its `static` feature, which is off;
  - `wayland-backend` compiles C only with its `log` feature, which is off.

  Tests run no window. If a future dependency needs system libraries, add one `sudo apt-get install --no-install-recommends …` step and document it here.
- Runtime libraries that a Linux *user* needs (libwayland-client0, libxkbcommon0/-x11-0, libx11-6, libxcursor1, libxrandr2, libxi6, libegl1, libgl1, libvulkan1, Mesa drivers) become `.deb` dependencies in M8.
- The GUI smoke step uses `bash` because PowerShell does not wait for GUI-subsystem executables. It reports whether wgpu initializes on the hosted Windows images, likely through WARP (no GPU). It never blocks and is never evidence for a support tier.
- **Reading CI status** from the dev machine: `gh` 2.102, authenticated, at `"C:\Program Files\GitHub CLI\gh.exe"` (not on PATH in agent shells, so call it by full path):
  - `gh run list --workflow ci.yml --limit 5`;
  - `gh run watch <id>`;
  - `gh run view <id> --log-failed`.

  The workflow itself never calls `gh`.

---

## 9. cargo-deny — `.cargo/deny.toml` (WP1, normative content)

cargo-deny discovers `.cargo/deny.toml` automatically (verified in the 0.20.2 source), so the root stays lean.
*Rejected:* a root `deny.toml`, which would take a root-file slot needed for licence files.

```toml
[graph]
targets = [
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
]
all-features = true

[advisories]
unmaintained = "workspace"
unsound = "all"
yanked = "deny"
ignore = []

[licenses]
confidence-threshold = 0.93
allow = [
    "Apache-2.0",
    "MIT",
    "BSL-1.0",      # clipboard-win, error-code (Windows clipboard)
    "ISC",          # libloading
    "Unicode-3.0",  # unicode-ident
    "Zlib",         # foldhash, zlib-rs
]
exceptions = [
    # egui's embedded default fonts (Ubuntu-Light, Hack, Noto Emoji): font licences, data only.
    { crate = "epaint_default_fonts", allow = ["OFL-1.1", "Ubuntu-font-1.0"] },
]
unused-allowed-license = "warn"

[licenses.private]
ignore = true   # workspace crates are publish = false and carry no licence yet (§16)

[bans]
multiple-versions = "warn"   # winit/wgpu ecosystem: ~20 unavoidable duplicates (bitflags, hashbrown, windows-sys…)
wildcards = "deny"
allow-wildcard-paths = true
deny = [
    { crate = "reqwest",     reason = "offline product: no HTTP client" },
    { crate = "ureq",        reason = "offline product: no HTTP client" },
    { crate = "hyper",       reason = "offline product: no HTTP stack" },
    { crate = "h2",          reason = "offline product: no HTTP stack" },
    { crate = "h3",          reason = "offline product: no HTTP stack" },
    { crate = "quinn",       reason = "offline product: no QUIC stack" },
    { crate = "curl",        reason = "offline product: no HTTP client" },
    { crate = "curl-sys",    reason = "offline product: no HTTP client" },
    { crate = "isahc",       reason = "offline product: no HTTP client" },
    { crate = "attohttpc",   reason = "offline product: no HTTP client" },
    { crate = "surf",        reason = "offline product: no HTTP client" },
    { crate = "minreq",      reason = "offline product: no HTTP client" },
    { crate = "ehttp",       reason = "offline product: no HTTP client (egui ecosystem)" },
    { crate = "tiny_http",   reason = "offline product: no HTTP server" },
    { crate = "tungstenite", reason = "offline product: no WebSocket" },
    { crate = "websocket",   reason = "offline product: no WebSocket" },
    { crate = "native-tls",  reason = "no network, hence no TLS" },
    { crate = "openssl",     reason = "no network, hence no TLS" },
    { crate = "openssl-sys", reason = "no network, hence no TLS" },
    { crate = "rustls",      reason = "no network, hence no TLS" },
    { crate = "sentry",      reason = "no telemetry" },
    { crate = "opentelemetry", reason = "no telemetry" },
]

[bans.workspace-dependencies]
duplicates = "deny"
include-path-dependencies = true
unused = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

**Licence decisions.** The allow-list was derived from the real M0 graph on all 4 targets, with all
features enabled (2026-10-13).

| Licence | Decision | Why |
|---|---|---|
| MIT, Apache-2.0 | allow | Base of the Rust ecosystem; permissive |
| BSL-1.0, ISC, Zlib | allow | Permissive, attribution-only; each is required by a crate in the graph (comments above) |
| Unicode-3.0 | allow | Permissive data licence (Unicode tables) |
| OFL-1.1, Ubuntu-font-1.0 | **exception for `epaint_default_fonts` only** | Font licences for embedded font data. *Rejected:* allowing them globally, which could admit code under a font licence. |
| **MPL-2.0** | **not allowed globally; per-crate exception only** | It first appears in M4 through `directories` → `dirs-sys` → `option-ext` 0.2.0. MPL-2.0 is file-level copyleft: using the unmodified crate is fine (sources stay available on crates.io), but modifying or vendoring it brings obligations. M4 adds `{ crate = "option-ext", allow = ["MPL-2.0"] }`. *Rejected:* a global allow, which would admit future MPL crates without review. |
| CC0-1.0 | per-crate exception, M4 | `notify` 8.2.0 is CC0-1.0 (public-domain dedication). M4 adds `{ crate = "notify", allow = ["CC0-1.0"] }`. |
| Unlicense, BSD-2/3-Clause, 0BSD, LLVM-exception | not needed | Every occurrence in the graph is an `OR` with MIT/Apache. Adding them only when a crate requires them keeps the list minimal. |
| GPL, LGPL, AGPL, SSPL, BUSL, CC-BY-NC, any "non-commercial" licence | never | Incompatible with redistribution under a permissive project licence |

**Network crates.** The M0 graph contains none of the banned names on any target (verified, 184–255
packages per target). Note that `scoped-tls` is thread-local storage, not TLS.

**Advisories.** A failing advisory is fixed by bumping the affected pin. If no fixed version exists, the
advisory goes into `ignore = [{ id = "RUSTSEC-…", reason = "…" }]`. Every ignore needs Security's
approval and a `SECURITY.md` entry (Limitations field). Advisories are never silenced to make CI green.

---

## 10. Script patches (WP5) and ignore files

`scripts/check_project_docs.py`:
1. `collect_dependencies` also reads root `[workspace.dependencies]`, excluding tables that have a `path` key, so every external crate pinned there must appear in DEPENDANCE.md and IMPORT.md. Today the script reads only `[dependencies]`, which a virtual workspace does not have.
2. New check `[deps]`: for each `[workspace].members` manifest, every entry of `dependencies`, `dev-dependencies`, `build-dependencies` and their `target.*.` variants must be a table with `workspace = true`. Otherwise report `[deps] <member>/Cargo.toml: '<name>' must use workspace = true` (rule R6). The check needs `tomllib` (Python ≥ 3.11). Without it, the script prints a warning and skips the check, consistent with the existing `parse_toml` fallback.

Both scripts (`walk` in `gen_agencement.py`, and `iter_dirs` plus the volume count in `check_project_docs.py`):

3. Skip paths ignored by git. Get them once from `git ls-files --others --ignored --exclude-standard --directory -z`, run in the root. When git is unavailable or the folder is not a repository, keep today's behaviour.
   - Without this, the gitignored `bench/results/*.json` would demand a README and make `AGENCEMENT.md` machine-dependent, so CI and local runs would disagree.
   - *Rejected:* moving bench results outside the repository; the plan fixes `bench/results/`.

Ignore files:

- `.gitignore` already contains `target/`, `bench/results/`, `bench/generated/` and `xtask/cache/`, so it needs no change.
- WP7 adds `bench/results/` to `.claudeignore`.

---

## 11. Tests for M0 (live in each crate's `tests/`)

| File | Test (name → assertion) |
|---|---|
| `apps/va/tests/binary.rs` | `version_prints_and_exits_zero` → `va --version`: exit 0, stdout `va 0.1.0\n` (from `CARGO_PKG_VERSION`), stderr empty · `help_lists_info` → `--help`: exit 0, stdout contains `Usage:` and `info` · `no_subcommand_is_usage_error` → `va`: exit 2, stdout empty, stderr contains `Usage:` · `unknown_flag_is_usage_error` → `va --bogus`: exit 2, stderr starts with `error:` · `info_human` → exit 0, stdout starts with `va 0.1.0` · `info_json` → `va info --json` and `va --json info`: exit 0, stdout is one JSON line, `schema == "va.info/1"`, `version`/`os`/`arch` strings, `architectures == []` |
| `apps/visual-assembler/tests/binary.rs` | `version` → exit 0, stdout `visual-assembler 0.1.0\n` · `help_lists_bench_flags` → stdout contains `--bench`, `--renderer`, `--duration-secs` · `bad_bench_value` → `--bench bogus`: exit 2 · `duration_without_idle` → `--duration-secs 5`: exit 2, stderr contains `--duration-secs`. None of these opens a window, so they are safe on headless runners. |
| `apps/visual-assembler/tests/cli.rs` | Via `GuiArgs::try_parse_from` + `resolve`: default renderer is `Wgpu` with `[Wgpu, Glow]` and `Glow` with `[Glow]` · `--renderer glow` with `[Wgpu]` gives an error containing `not compiled` · `--bench idle` gives a 10 s duration · `--bench idle --duration-secs 3` gives 3 s · `--bench startup --duration-secs 3` is an error · `--duration-secs 0` is a parse error |
| `apps/visual-assembler/tests/spike.rs` | `shows_title_and_subtitle` → `Harness::new_ui(spike::draw)`, `run()`, then `get_by_label(WINDOW_TITLE)` and `get_by_label(SUBTITLE)` (trait `egui_kittest::kittest::Queryable`) · `static_ui_schedules_no_repaint` → after `run()`, `harness.output().viewport_output[&ViewportId::ROOT].repaint_delay == Duration::MAX`, egui's "no repaint scheduled" sentinel (headless proof of reactive mode). No GPU and no window: `egui_kittest` is used without its `wgpu`/`snapshot` features. |
| `apps/visual-assembler/tests/bench_report.rs` | `run_line_has_exact_schema` → a `BenchRun` with fixed values serializes to one line with no `\n`; its key set equals the 18 keys of §4.6; `schema == "va.bench.run/1"`; `None` fields serialize as `null` |
| `apps/visual-assembler/tests/bench_process.rs` | `parse_vm_hwm` → `"VmPeak:\t 2048 kB\nVmHWM:\t    1024 kB\n"` gives `Some(1_048_576)`; missing line gives `None`; non-numeric gives `None` · `sample_reports_real_values` → on Windows and Linux, `ProcessStats::sample()` has `cpu_ms >= 0` and `peak_rss_bytes > 0`; on Windows `peak_private_bytes > 0`; on Linux it is `None`. This exercises the unsafe code on all 4 CI runners. |
| `xtask/tests/bench_stats.rs` | `[]` gives None · `[5]` gives median = p95 = min = max = 5 · `[3,1,2]` gives median 2, p95 3 · `[1,2,3,4]` gives median 2.5, p95 4 · `1..=20` gives median 10.5, p95 19 · `1..=100` gives median 50.5, p95 95 · `[1, NaN]` gives None |
| `xtask/tests/bench_timestamp.rs` | `utc_compact`: epoch 0 → `19700101T000000Z`; 951782400 → `20000229T000000Z` (leap day); 1700000000 → `20231114T221320Z`; 4102444800 → `21000101T000000Z` (2100 is not a leap year) · `utc_iso(1700000000)` → `2023-11-14T22:13:20Z` |
| `xtask/tests/bench_report.rs` | `report_file_name(1700000000 s, "1a2b3c4d")` → `20231114T221320Z-1a2b3c4d.json` · the markdown header equals §6.2's line exactly · a synthetic report serializes with `schema == "va.bench.report/1"` and `idle_cpu_percent == null` for startup results |
| `xtask/tests/layering.rs` | A synthetic metadata JSON where `va-core` depends on `va-ui` gives one violation · `va` depending on `va-ui` gives a violation · `va-arch-z80` depending on `va-core` is ok · an unknown package gives `no layering rule` · `va-lang` depending on `eframe` gives a GUI-crate violation · `scan_sources` on a temp tree (`std::env::temp_dir()/va-xtask-<pid>-<n>`) flags `use std::net::TcpStream;` in `crates/x/src/lib.rs` and `#![allow(unsafe_code)]` outside the allow-list, and accepts the allow-listed path |

**Not automated in M0**, listed in the app README with the reason:

- the real window on screen;
- the bench numbers, which need a display and a GPU;
- the WSLg behaviour.

These are covered by the §12 protocol and its manual checklist.

---

## 12. Renderer decision protocol (WP6)

### 12.1 Configurations to measure (reference machine)

| # | Environment | Renderer | How |
|---|---|---|---|
| C1 | Windows 11 x64 native | wgpu (default backend, expected `dx12`) | `cargo xtask bench all --renderer wgpu` |
| C2 | Windows 11 x64 native | glow (WGL) | `cargo xtask bench all --renderer glow` |
| C3 | WSLg Wayland | wgpu (default backend) | `cargo xtask bench all --renderer wgpu` in WSL |
| C4 | WSLg Wayland | wgpu forced to GL | `WGPU_BACKEND=gl cargo xtask bench all --renderer wgpu` |
| C5 | WSLg Wayland | glow (EGL) | `cargo xtask bench all --renderer glow` |
| C6 | WSLg X11 | wgpu (default backend) | `env -u WAYLAND_DISPLAY cargo xtask bench all --renderer wgpu` |
| C7 | WSLg X11 | glow | `env -u WAYLAND_DISPLAY cargo xtask bench all --renderer glow` |
| C8 | WSLg X11 | wgpu forced to GL (only if C6 fails G1–G3) | `env -u WAYLAND_DISPLAY WGPU_BACKEND=gl cargo xtask bench all --renderer wgpu` |

Conditions:

- Mains power, Windows "Best performance" power mode.
- No other heavy process, and nobody touches the mouse or keyboard during runs.
- Default run counts.
- In WSL, `export CARGO_TARGET_DIR=$HOME/.cache/visualassembler/target`. Binaries must sit on ext4: launching from `/mnt/c` (9P) would distort startup, and sharing `target/` with Windows thrashes fingerprints.
- Check that the JSON `window_system` is `wayland` for C3–C5 and `xlib`/`xcb` for C6–C7.

### 12.2 Gates

| Gate | Pass condition |
|---|---|
| G1 reliability | 100 % of warm-up and measured runs exit 0 with a JSON line (startup and idle) |
| G2 visual (manual, one launch per config via `cargo run --release -p visual-assembler --no-default-features --features renderer-<r>`) | The window appears with title and text readable; it can be moved, resized, minimized, restored and closed; there is no flicker or black frame after resizing |
| G3 idle | Median `frames_painted == 0` and median `idle_cpu_percent <= 1.0` |
| G4 startup | p95 `spawn_to_first_frame_ms <= 500` on Windows native. Reported only for WSLg. |

### 12.3 Decision rule

- **wgpu is chosen** when both of the following hold:
  - C1 passes G1–G4;
  - for each of Wayland and X11, at least one wgpu configuration (C3/C4 for Wayland, C6/C8 for X11) passes G1–G3.
- Rationale for preferring wgpu: Windows ARM64 has no native OpenGL driver guaranteed (OpenGL needs an optional compatibility pack), while DX12 is native. That ARM64 GUI cannot be checked by a human, so this a-priori criterion takes the place of a measurement.
- Peak RSS, CPU time and binary size are reported and discussed, but they do not decide alone. A wgpu peak RSS more than 100 MB above glow's on C1 must be explained in the report.

### 12.4 Outcomes and follow-up

| Outcome | Situation | Action (WP2 code change, then QA re-run) |
|---|---|---|
| A | wgpu passes everywhere with its default backend | Keep `default = ["renderer-wgpu"]`. `renderer-glow` stays a fallback that CI compiles (`xtask ci` step 3). |
| B | wgpu fails in WSLg with its default backend but passes with `WGPU_BACKEND=gl` | Keep wgpu. In `lib.rs`, when `platform::is_wsl()` is true and `WGPU_BACKEND` is unset, set `NativeOptions::wgpu_options` instance backends to `wgpu::Backends::GL`. Document it in the app README. Re-measure C3/C6. |
| C | wgpu fails in WSLg on every backend, and glow passes | On Linux targets, the default features become `renderer-wgpu` + `renderer-glow`, with a runtime fallback: if wgpu initialization fails (`run_native` error before the first frame), retry once with glow. Windows keeps wgpu only. |
| D | wgpu fails C1 | Stop and escalate to the user (Windows ARM64 would have no viable renderer). |

Independent conditional: if G2 on WSLg Wayland shows an undecorated or immovable window, add
`winit = { version = "=0.30.13", default-features = false, features = ["wayland-csd-adwaita"] }`
as a Linux-only dependency of `visual-assembler`. It exists only for feature unification, because eframe's
default features (which carry it) are off. The Architect re-checks its licences (sctk-adwaita, ab_glyph) before merging.

### 12.5 Report `docs/perf/M0-backend.md` (required sections)

1. **Context:** date, git sha, CPU/GPU/RAM, Windows build, GPU driver version, WSL/WSLg/Mesa/kernel versions, rustc 1.99.0.
2. **Commands:** the exact commands run, with their environment variables.
3. **Results:** one row per C1–C7 with renderer, backend, adapter, window system, spawn→frame p50/p95, startup_ms p50/p95, idle CPU % p50/p95, idle frames p50/max, peak RSS p50, peak private p50 (Windows), binary size.
4. **G2 checklist:** per configuration, with observations.
5. **Gate table:** G1–G4 × C1–C7, each pass or fail.
6. **Decision:** the outcome letter, the rule references, and follow-up commits.
7. **Cold start:** right after a reboot, `cargo xtask bench startup --runs 1 --warmup 0 --renderer all` on Windows, reported separately.
8. **Toolchain evidence:** E3 cross-build output and E4 `dumpbin /dependents` output.
9. **Raw data:** names of the `bench/results/*.json` files (not committed) and the CI GUI smoke JSON lines from the Windows x64/ARM64 runners, both clearly marked informational.
10. **Limitations:**
    - no physical display latency;
    - Windows CPU-time granularity is about 15.6 ms (about 0.16 % over 10 s);
    - WSLg results depend on Mesa/WSLg versions.

---

## 13. External dependencies

Versions were verified on crates.io on 2026-10-13. Versions published fewer than ~14 days earlier were
skipped. Every MSRV is ≤ 1.95, so all support rustc 1.99.

### 13.1 Pinned in M0 (direct)

| Crate | Version (published) | Licence | Used by | Why not std / why this one |
|---|---|---|---|---|
| eframe | 0.36.2 (2026-09-08) | MIT OR Apache-2.0 | visual-assembler | Final plan decision (egui framework, reactive mode, AccessKit) |
| egui_kittest | 0.36.2 (2026-09-08) | MIT OR Apache-2.0 | visual-assembler (dev) | Headless UI tests, required by the command-line testability rule |
| clap | 4.6.7 (2026-09-14) | MIT OR Apache-2.0 | va, visual-assembler, xtask | Help, errors, exit codes and value validation; hand-rolled parsing would be untested surface |
| serde | 1.0.229 (2026-07-18) | MIT OR Apache-2.0 | va, visual-assembler, xtask | Derive for JSON schemas (and TOML from M4) |
| serde_json | 1.0.151 (2026-07-20) | MIT OR Apache-2.0 | va, visual-assembler, xtask | `--json` output, bench lines, cargo metadata parsing. 1.0.152–154 (Oct 10–11) are too recent. |
| anyhow | 1.0.104 (2026-07-18) | MIT OR Apache-2.0 | xtask | Error context in the task runner (plan: anyhow for binaries) |
| windows-sys | 0.61.2 (2025-10-06) | MIT OR Apache-2.0 | visual-assembler (Windows) | `GetProcessTimes`, `GetProcessMemoryInfo`; std has no API for them; same version eframe already uses |
| libc | 0.2.189 (2026-07-21) | MIT OR Apache-2.0 | visual-assembler (Linux) | `getrusage`; already in the graph. 0.2.190 (Oct 2) is too recent. |
| raw-window-handle | 0.6.2 (2024-05-17) | MIT OR Apache-2.0 OR Zlib | visual-assembler | Trait to read the real window system (Wayland vs X11) for the bench; already in the graph |

### 13.2 Notable transitive dependencies (for IMPORT.md)

| Crate | Version | Licence | Note |
|---|---|---|---|
| egui / egui-winit / egui-wgpu / egui_glow | 0.36.2 | MIT OR Apache-2.0 | via eframe |
| wgpu | 30.0.1 | MIT OR Apache-2.0 | Backends DX12, Vulkan, GL (and Metal, unused) |
| glow | 0.17.0 | MIT OR Apache-2.0 OR Zlib | via egui_glow |
| winit | 0.30.13 | Apache-2.0 | Windowing; Wayland/X11 loaded at runtime |
| glutin / glutin-winit | 0.32.3 / 0.5.0 | Apache-2.0 | GL context (glow only) |
| accesskit (+ platform adapters) | 0.24.x / 0.25.x | MIT OR Apache-2.0 | Accessibility; zbus/AT-SPI on Linux (local D-Bus, not network) |
| epaint_default_fonts | 0.36.2 | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 | Embedded UI fonts (deny exception) |
| kittest | 0.4.0 | MIT OR Apache-2.0 | via egui_kittest (dev) |

### 13.3 Planned (re-verify the version when the milestone starts)

| Crate | Candidate version | Licence | Milestone | Purpose |
|---|---|---|---|---|
| thiserror | 2.0.21 | MIT OR Apache-2.0 | M1 | Library error types |
| fluent-bundle | 0.16.0 | Apache-2.0 OR MIT | M1 | i18n rendering (architecture §5) |
| unic-langid | 0.9.6 | MIT OR Apache-2.0 | M1 | Language identifiers for Fluent |
| criterion | 0.8.2 | Apache-2.0 OR MIT | M1 (dev) | Micro-benchmarks |
| m68k | 0.14.3 (0.14.4 of 2026-10-12 is too recent) | MIT | M3 | 68000 core behind `Emulator`, without the `jit` feature |
| ropey | 1.6.1 | MIT | M4 | Rope buffer (plan decision; 2.0 is still beta) |
| regex | 1.13.1 | MIT OR Apache-2.0 | M4 | Search/replace |
| toml | 1.1.6 | MIT OR Apache-2.0 | M4 | Settings/project/session files |
| directories | 6.0.0 | MIT OR Apache-2.0 (pulls option-ext 0.2.0, MPL-2.0) | M4 | Config/data dirs; needs the MPL-2.0 exception |
| notify | 8.2.0 (9.0 still RC) | CC0-1.0 | M4 | File watching; needs the CC0-1.0 exception |
| rfd | 0.17.2 | MIT | M4 | File dialogs (XDG portal, no GTK; internal explorer as fallback under WSLg) |
| tracing / tracing-subscriber | 0.1.44 / 0.3.23 | MIT | M4 | Local logs only |
| egui (direct, for va-ui) | 0.36.2 | MIT OR Apache-2.0 | M4 | va-ui depends on egui, not eframe |

### 13.4 Tools and assets (not linked)

| Item | Version | Licence | When |
|---|---|---|---|
| cargo-deny | 0.20.2 | MIT OR Apache-2.0 | M0 (`make install`, CI action v2.1.1 bundles it) |
| cargo-packager | 0.11.8 | Apache-2.0 OR MIT | M8 |
| JetBrains Mono | 2.304 | OFL-1.1 | M4 (asset) |
| SingleStepTests 680x0 vectors | upstream HEAD | **no licence published** | M3: fetched into gitignored `xtask/cache/`, never committed or shipped |

---

## 14. Known fragile points and mitigations

| # | Fragile point | Mitigation / test |
|---|---|---|
| F1 | WSLg + wgpu: Vulkan goes through Mesa (dzn over D3D12) or falls back to llvmpipe/lavapipe (CPU), and is possibly unusable | The JSON records backend and adapter; C4 forces GL; outcomes B/C (§12.4) |
| F2 | WSLg Wayland decorations: eframe default features are off, so there is no adwaita client-side frame | G2 check; conditional winit feature (§12.4) |
| F3 | ezwinports make picks `sh` if it is on PATH, else `cmd.exe` | Shell-free recipes, `$(info)` help (§7); QA runs `make` under both |
| F4 | kittest without a GPU | Only AccessKit queries; no `wgpu`/`snapshot` features; no image tests in M0 |
| F5 | The `windows_subsystem = "windows"` release exe prints nothing unless redirected | Documented; xtask pipes; tests use debug builds |
| F6 | The `RUSTFLAGS` env var silently drops `+crt-static` | Forbidden in CI/scripts; E4 checks imports |
| F7 | No MSVC ARM64 tools on the dev machine | `aarch64-pc-windows-msvc` is built natively on `windows-11-arm` only (assumes the runner image ships VS ARM64 build tools; the first CI run confirms) |
| F8 | `aarch64-linux-gnu-gcc` linker config also applies on the native arm64 runner | Ubuntu's native gcc package provides that name; if the first CI run fails, the fix is a `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER` override on that runner only |
| F9 | Wayland `dlopen` comes from feature unification, not from a feature we set | Verified for the eframe 0.36.2 graph (wgpu-only, glow-only, both); E3 cross-build is the guard; re-check on every eframe bump |
| F10 | CRLF checkouts break the hook and rustfmt | `.gitattributes` (§3.4) |
| F11 | The cargo-deny action image ships Rust 1.85 | `rust-version: "1.99.0"` input |
| F12 | Bench noise: Defender scanning new executables, power plans, the Parsec virtual display adapter, multiple GPUs | Warm-up run, fixed conditions (§12.1), adapter recorded; `WGPU_POWER_PREF`/`WGPU_ADAPTER_NAME` honoured and recorded |
| F13 | "First frame presented" ≠ photons on screen | The definition is fixed (§4.5); limitation stated in the report |
| F14 | Gitignored folders confuse the doc scripts | Git-aware walk (§10) |
| F15 | `python` vs `python3` | `VA_PYTHON`, OS default (§6) |
| F16 | GitHub-hosted Windows runners have no GPU, so the GUI smoke may fail | `continue-on-error`, informational only |
| F17 | `windows-latest`/`ubuntu-latest` move to new images | Accepted (brief); failures are triaged as image changes first |

---

## 15. Decision log (decisions taken here that the plan did not make)

| Decision | Rejected alternative (why) |
|---|---|
| Rust 1.99.0 pinned, MSRV 1.99 | Floating `stable` (unreproducible builds and lint drift) |
| Exact `=` pins in `[workspace.dependencies]` | Caret ranges + lock only (`cargo update` would drift away from DEPENDANCE.md) |
| `[workspace.dependencies]` holds only crates already used (cargo-deny `unused = "deny"`) | Pre-declaring future crates (stale versions, noise) |
| `.cargo/deny.toml` instead of a root `deny.toml` | Root file (root-file budget, §2.1) |
| No `rustfmt.toml` / `clippy.toml`; lints in `[workspace.lints]` | Cosmetic config files (§3.5) |
| `.gitattributes` forcing LF | `newline_style = "Native"` (the hook would still break) |
| GUI app = lib + bin; spike UI in the app, `va-ui` stays a skeleton | Spike in `va-ui` (the M0 brief keeps skeletons logic-free) |
| Bench builds one single-renderer binary per renderer | One dual-renderer binary (§6.1) |
| Startup gate uses the outer `spawn_to_first_frame_ms`; inner `startup_ms` is informational | Inner only (misses loader and DLL time) |
| Idle window: 1 s settle + N s, sampled from a side thread | `request_repaint_after` timer (would paint frames and pollute the metric) |
| `raw-window-handle` direct dependency to report the real window system | Inferring from `WAYLAND_DISPLAY` (wrong when winit falls back to X11) |
| Linux: `VmHWM` + `getrusage`; Windows: `windows-sys`; `peak_private_bytes` added | `sysinfo` crate (heavy) |
| xtask layering + source scans (network, unsafe allow-list) | Review-only enforcement |
| `va info [--json]` as M0's only subcommand; error JSON on stdout in `--json` mode | Errors on stderr only in JSON mode (consumers would parse two streams) |
| CI: `cargo xtask ci` as the single source of the check list; fmt once; informational Windows GUI smoke | Duplicating cargo commands in YAML (drift) |
| No apt step on Linux runners | Installing -dev packages "just in case" (hides real requirements) |
| Arch-specific `.ftl` in `arch/<cpu>/locales/` (architecture §5) | One global `locales/` (each CPU would touch shared files) |
| JetBrains Mono for code (architecture §9.6) | Cascadia Code |
| `settings.toml` never written by the app (architecture §6) | `toml_edit` round-trip |
| MPL-2.0 and CC0-1.0 only as per-crate exceptions | Global allow |

---

## 16. Open questions

| # | Question | Blocks |
|---|---|---|
| Q1 | **Project licence.** Recommendation: `MIT OR Apache-2.0` (Rust norm, compatible with every dependency). Until decided: no `license` key, `publish = false`, cargo-deny ignores private crates. | Making the repository public with a licence. Not development. |
| Q2 | User consent for the installs the plan lists: rustup + 1.99.0 + target `aarch64-unknown-linux-gnu` + `gcc-aarch64-linux-gnu` in WSL; `cargo-deny` 0.20.2 on Windows (`make install`). | WP6 (C3–C7), E1 in WSL, E3 |
| Q3 | Git identity (user.name/email) must be configured by the user before the first commit (plan). | Commits |
| Q4 | SingleStepTests 680x0 publishes no licence: confirm that fetching it at test time (never committing it) is acceptable. | M3 only |

---

## 17. Amendments after audit round 1 (2026-10-13)

Inputs: the round-1 Security, QA and conformance reports, the fix-agent reports, and the developer
deviation reports.

Precedence rules:

- Where §17 conflicts with §0–§16, **§17 wins**. Everything else in §0–§16 stays normative.
- Normative blocks below replace the named parts of earlier blocks; they do not replace whole files unless stated.
- Durable rules R7, R8, the temp-folder rule, §9.3, §9.4 and §9.7 were added to `architecture.md`.

### 17.1 User decisions (final)

- **Q1 closed.** Licence `MIT OR Apache-2.0` (WP1):
  - Root `Cargo.toml` `[workspace.package]` gains `license = "MIT OR Apache-2.0"`, and the comment "No `license` key until…" is removed.
  - Every member manifest adds `license.workspace = true` after `publish.workspace = true`:
    - WP1 edits `crates/*` and `arch/m68k`;
    - WP2 edits `apps/visual-assembler`;
    - WP3 edits `apps/va`;
    - WP4 edits `xtask`.
  - `publish = false` stays. *Rejected:* `publish = true` — publishing crates is not a goal, and short names such as `va` invite crates.io name conflicts.
  - `.cargo/deny.toml` sets `[licenses.private] ignore = false`, so cargo-deny now also verifies our own crates. This was verified on 2026-10-13: a member without a licence fails as `error[unlicensed]`.
  - `LICENSE-APACHE` is a byte-for-byte copy of `~/.cargo/registry/src/index.crates.io-*/serde-1.0.229/LICENSE-APACHE` (the Apache-2.0 terms, 176 lines, LF), sha256 `62c7a1e35f56406896d7aa7ca52d0cc0d272ac022b5d2796e7d6905db8a3636a`.
  - `LICENSE-MIT` (normative content, LF). The holder line is "The VisualAssembler authors". *Rejected:* a personal name the user has not confirmed. The formula stays true as contributors join.

```
MIT License

Copyright (c) 2026 The VisualAssembler authors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- The root now holds 15 files (§2.1's 13 plus the two licence files). That is exactly the check script's
  warning threshold, so **the root is full**: any future root file must displace one.
- **Q4 closed.** SingleStepTests 680x0 vectors are fetched on demand at test time into gitignored `xtask/cache/` (M3), never committed or shipped.
- The repository `GaspardSapin/VisualAssembler` (public) is created and pushed by the orchestrator after M0 is audited and committed. E2 becomes observable only then.

### 17.2 Layering (QA-1-F3) — replaces the §6.3 rule table

| Package | Allowed internal dependencies |
|---|---|
| `va-core`, `xtask` | none |
| `va-editor`, `va-debug`, `va-workspace`, `va-arch-*` | `va-core` |
| `va-lang` | `va-core`, `va-editor` |
| `va-ui` | `va-core`, `va-editor`, `va-lang`, `va-debug`, `va-workspace` |
| `va` | `va-core`, `va-editor`, `va-lang`, `va-debug`, `va-workspace`, `va-arch-*` |
| `visual-assembler` | `va-core`, `va-editor`, `va-lang`, `va-debug`, `va-workspace`, `va-ui`, `va-arch-*` |
| any other package | violation `no layering rule for <name>` |

- Every package is now listed with what it *may* use (no more "everything except").
- `visual-assembler`, `va` and `xtask` can therefore never be dependencies (architecture R7).
- *Rejected:* a transitive GUI-crate check through the full `cargo metadata` resolve. R3 + R7 make the CLI's closure GUI-free by construction, while a transitive check would depend on platform filtering and run slower.

### 17.3 Manifest and build-flag integrity (Security F2, QA-1-F4.1)

1. **`scripts/check_project_docs.py`, new check `[lints]` (WP5).**
   - For every `[workspace].members` manifest, the parsed `lints` table must equal `{"workspace": True}` exactly. Otherwise report `[lints] <member>/Cargo.toml: lints must be exactly 'workspace = true' (no [lints.*] override)`.
   - The root manifest must have `workspace.lints.rust.unsafe_code == "deny"`. Otherwise report `[lints] Cargo.toml: workspace.lints.rust.unsafe_code must stay "deny"`.
   - It uses the same tomllib fallback as `[deps]`.
   - *Rejected:* a text-based TOML check in xtask, or adding the `toml` crate to xtask before M4. The script already parses member manifests correctly.
2. **`cargo xtask ci`, new step 0 "build configuration" (WP4).** It runs before step 1, also with `--no-fmt`. It fails with exit 1 when:
   - (a) any of `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `CARGO_BUILD_RUSTFLAGS`, or any variable matching `CARGO_TARGET_*_RUSTFLAGS`, is set and non-empty. Message: `<VAR> is set: it replaces .cargo/config.toml (+crt-static) and can silence lints; unset it`.
   - (b) `.cargo/config.toml` breaks the whitelist. Read it as text, ignoring blank lines and `#` comments:
     - allowed table headers are exactly `[alias]`, `[target.x86_64-pc-windows-msvc]`, `[target.aarch64-pc-windows-msvc]` and `[target.aarch64-unknown-linux-gnu]`;
     - no key line may start with `build.` or `target.`;
     - every line containing `rustflags` must, with all whitespace removed, equal `rustflags=["-C","target-feature=+crt-static"]`, and must sit under one of the two windows-msvc headers.

   Tests go in `xtask/tests/ci.rs`: the step order now starts with step 0, and pure functions `check_build_env(vars)` and `check_cargo_config(text)` are tested with violating and clean inputs.
3. **Accepted limitation (recorded in `SECURITY.md`):** a user-level `~/.cargo/config.toml` `build.rustflags` cannot be detected on stable cargo (`cargo config get` is unstable). CI runners have none.

### 17.4 Source scan (Security F1, QA-1-F4.2) — replaces the `scan_sources` part of §6.3

The scan is a tripwire against accidental use and an aid to review, not a defence against a malicious
contributor (SECURITY.md Limitations). All rules below are normative for `xtask/src/layering.rs` (WP4).

| # | Rule |
|---|---|
| S1 Roots | Scan the package folder (parent of `manifest_path`) of **every** workspace member from `cargo metadata --no-deps`, `xtask` included. `scan_sources(root, members)` takes the member folders as input so tests can pass synthetic trees. |
| S2 Exemption | `SCAN_DATA_EXEMPT = ["xtask/src/layering.rs", "xtask/tests/layering.rs"]`. These files hold the patterns as data, so S5–S7 skip them; S3, S4 and S8 still apply. *Rejected:* building the patterns with `concat!` so the checker contains no literal; that makes the checker and its tests unreadable. |
| S3 Skipped folders | Skip only the cargo target folder (`target_directory` from metadata) and `<root>/target`. Every other folder named `target` is scanned. |
| S4 Symlinks | Any symlink (file or folder) under a scanned root is a violation `[symlink]`, and it is never followed. Use `fs::symlink_metadata`. |
| S5 Code view | Patterns are matched on a *code view* of each `.rs` file: comments (nested block comments included), and the contents of string, raw-string, byte-string and char literals, are replaced by spaces. Line numbers are preserved. Reuse the existing bracket/literal scanner. |
| S6 Unsafe | Outside `UNSAFE_ALLOWLIST`: (a) any whole-word `unsafe` token in the code view is a violation; (b) any whole-word `unsafe_code` token is a violation unless the innermost enclosing parenthesis group is introduced by the identifier `deny` or `forbid`. This supersedes the `allow(`/`expect(` line and attribute rules: `cfg_attr(x, deny(unsafe_code))` passes; `allow(dead_code, unsafe_code)`, `expect(unsafe_code, …)`, `permit!(unsafe_code)` and multi-line forms fail. |
| S7 Network | Outside `NET_ALLOWLIST`, on the code view: (a) with all whitespace removed, the substrings `std::net` and `core::net`, or a brace-matched `std::{…}`/`core::{…}` group containing `net` as a path segment (`{net::`, `,net::`, `{net,`, `,net,`, `{net}`, `,net}`); (b) the whole-word identifiers `TcpStream`, `TcpListener`, `UdpSocket`, `ToSocketAddrs`, `to_socket_addrs`, `lookup_host`. |
| S8 Inclusion | `include!(…)` and `#[path = …]` must take a plain string literal that ends in `.rs` and has no `..` component. Anything else (non-`.rs` targets, `concat!`, `env!`) is a violation `[include]`. The included `.rs` file is scanned anyway, because it lies under a root. `include_str!` and `include_bytes!` are allowed (data, e.g. `.ftl`). A future build script that generates code must get an allow-list entry. |
| S9 File types | Non-`.rs` files are not pattern-scanned. S8 is the only way they can become code. |

**Regression tests (in `xtask/tests/layering.rs`), one per bypass**, all in fresh temp trees (§17.9):

- a wrapped multi-line `#![allow(…, unsafe_code)]`;
- `macro_rules! permit` + `permit!(unsafe_code)`;
- an `unsafe {}` block with no attribute;
- `include!("hidden.inc")` with network code;
- `#[path = "x.inc"] mod x;`;
- a module folder `crates/x/src/target/` containing network code;
- `use std::{io, net::ToSocketAddrs as _};`;
- `use std :: net;`;
- a symlinked folder (on Windows, skipped with a printed note when symlink creation is not permitted);
- a member outside `crates/`, `arch/` and `apps/`;
- `unsafe` inside a comment or a string, which is **not** flagged;
- `#![deny(unsafe_code)]` and `cfg_attr(x, forbid(unsafe_code))`, which are **not** flagged.

The live workspace must still report 0 violations.

### 17.5 cargo-deny policy and command (Security F3, F8, F9; brief item 7)

**`.cargo/deny.toml` changes (WP1, normative values):**

- `[advisories] unmaintained = "all"` (was `"workspace"`). Each hit is triaged under the existing advisory rule (§9): bump the pin, or add an `ignore` entry with a reason, Security's approval and a SECURITY.md Limitations entry.
- `[licenses.private] ignore = false` (§17.1).
- `[bans] deny` keeps the 22 entries and appends these names: `socket2`, `tokio`, `async-std`, `async-net`, `smol`, `hickory-resolver`, `hickory-proto`, `trust-dns-resolver`, `trust-dns-proto`, `cadence`, `statsd`, `git2`, `libgit2-sys`, `ssh2`, `libssh2-sys`, `lettre`, `ws`, `tonic`, `axum`, `actix-web`, `warp`, `sentry-core`, `opentelemetry_sdk`, `opentelemetry-otlp`. Reasons: socket/async-runtime crates get "offline product: no sockets or async network runtime (architecture T5, §9.3)"; DNS, RPC, servers, mail, git and ssh crates get "offline product: no network client or server"; the sentry/opentelemetry names get "no telemetry".
- New key under `[bans]`: `features = [{ crate = "mio", deny = ["net"], reason = "mio is allowed for file watching (notify, M4) but never with sockets" }]`.
- Not banned, with the reason: `async-io` and `polling`. zbus needs them (eframe → accesskit_winit → accesskit_unix → zbus).
- *Rejected:* a reviewed crate allow-list (`[bans] allow`) generated from the graph. Every eframe/wgpu bump would rewrite ~270 entries, and review fatigue defeats the purpose.

All values above were verified on the repo graph on 2026-10-13: `advisories ok, bans ok`. Licences pass
once every member carries `license.workspace = true`.

**Command form.** The spec never used the invalid `… check --config …` order. From now on, the explicit form is used everywhere, so a stray root `deny.toml` (searched first) can never shadow the policy:

```
cargo deny --manifest-path Cargo.toml --config .cargo/deny.toml check
```

- Makefile `security` target: `$(CARGO) deny --manifest-path Cargo.toml --config .cargo/deny.toml check` (WP5).

**CI `deny` job (Security F8; WP5, normative replacement of the §8 job).** The Docker action downloads an unverified release tarball, so it is replaced by a crates.io install, which is checksummed and locked:

```yaml
  deny:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - uses: dtolnay/rust-toolchain@e2a55d2ffb04f378e9626c28d38b36d230d1e12f # master, 2026-10-13
        with:
          toolchain: "1.99.0"
      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2
        with:
          key: deny
          save-if: ${{ github.ref == 'refs/heads/main' }}
      - name: install cargo-deny 0.20.2 (crates.io checksums, locked)
        run: cargo install cargo-deny --version 0.20.2 --locked
      - run: cargo deny --manifest-path Cargo.toml --config .cargo/deny.toml check
```

- *Rejected:* keeping `EmbarkStudios/cargo-deny-action` with an added checksum step. The download happens inside the action's Docker build, where no step can verify it.
- This also removes fragile point F11 (Rust 1.85 image).

### 17.6 Commit-message hook (QA-1-F2, Security F4, D17) — owner: WP5

`scripts/git-hooks/commit-msg` now belongs to **WP5** (normative content, LF, POSIX sh):

```sh
#!/bin/sh
# Reject commit messages that credit an AI tool (agentic-project framework, section 10).
# Install: make hooks (runs cargo xtask hooks).
# Keep the names and trailer keys in sync with COAUTHOR_RE in scripts/check_project_docs.py.
# Limits: client-side only (--no-verify skips it); the CI docs job re-checks the last 50 commits.

msg_file=$1
if [ -z "$msg_file" ] || [ ! -r "$msg_file" ]; then
    echo "commit-msg hook: cannot read the commit message file '$msg_file'." >&2
    exit 1
fi

names='claude|anthropic|copilot|cursor|gpt|openai|codex|gemini|aider|devin|windsurf|codeium'
pattern="(co-authored-by|assisted-by|generated-by):.*($names)|generated with .*($names)"
robot=$(printf '\360\237\244\226')

# Ignore the verbose diff below the scissors line and every comment line.
if ! body=$(sed -e '/^# -\{24\} >8 -\{24\}$/,$d' -e '/^#/d' "$msg_file"); then
    echo "commit-msg hook: cannot read the commit message." >&2
    exit 1
fi

reject() {
    echo "commit rejected: the message credits an AI tool ($1)." >&2
    echo "Remove the Co-Authored-By / Assisted-by / Generated-with line or the robot emoji, then retry." >&2
    exit 1
}

printf '%s\n' "$body" | grep -qiE -e "$pattern"
case $? in
    0) reject "co-author or generated-with mention" ;;
    1) ;;
    *) echo "commit-msg hook: grep failed; refusing the commit." >&2; exit 1 ;;
esac

printf '%s\n' "$body" | grep -qF -e "$robot"
case $? in
    0) reject "robot emoji" ;;
    1) ;;
    *) echo "commit-msg hook: grep failed; refusing the commit." >&2; exit 1 ;;
esac

exit 0
```

`COAUTHOR_RE` in `scripts/check_project_docs.py` (WP5) becomes:

```python
_AI_NAMES = r"(claude|anthropic|copilot|cursor|gpt|openai|codex|gemini|aider|devin|windsurf|codeium)"
COAUTHOR_RE = re.compile(
    rf"(co-authored-by|assisted-by|generated-by):.*{_AI_NAMES}|generated with .*{_AI_NAMES}|\U0001F916",
    re.IGNORECASE,
)
```

Accepted limitations (SECURITY.md):

- the hook can be bypassed with `--no-verify`, rebase or cherry-pick;
- a trailer folded onto a continuation line is not caught;
- `git commit --cleanup=verbatim` comment lines are not caught, though the CI log check still sees them.

### 17.7 `cargo xtask hooks` safety (Security F6) — WP4

- New syntax: `cargo xtask hooks [--force]`. The Makefile `hooks` target stays without `--force`.
- Compute the hooks folder (as today) and the common git folder (`git rev-parse --git-common-dir`, joined to the root). Canonicalize both.
  - If the hooks folder is **not** inside the common folder (a global `core.hooksPath`), fail with exit 1: `core.hooksPath points outside this repository (<dir>); refusing to install a project hook there. Install it yourself or unset core.hooksPath.`
  - `--force` does **not** override this. *Rejected:* overriding with `--force`, which would silently impose the project policy on every repository of the developer.
- If the target exists with identical bytes: print `already installed <path>`, re-apply the Unix mode, exit 0.
- If the target exists with different bytes:
  - without `--force`: fail with exit 1, `<path> already exists and differs; rerun with --force to replace it (a backup is kept)`, and leave the file untouched;
  - with `--force`: rename the existing file to `commit-msg.bak.<YYYYMMDDTHHMMSSZ>` (`timestamp::utc_compact`), then install.
- Signature: `hooks::run(root: &Path, force: bool) -> Result<PathBuf>`.
- Tests go in the new file `xtask/tests/hooks.rs`, using temp git repositories (§17.9; `git` is required, as in CI):
  - fresh install;
  - idempotent re-run;
  - a differing file without `--force` is refused and left unchanged;
  - `--force` creates the backup and installs the new content;
  - `git -c core.hooksPath=<outside>` is refused even with `--force`.
- The same file tests **hook behaviour** by running `sh <repo>/scripts/git-hooks/commit-msg <file>`:
  - exit 1 for `Co-Authored-By: Claude <noreply@anthropic.com>`, `Co-authored-by: Codex <codex@openai.com>`, `Assisted-by: Gemini`, a message containing the UTF-8 robot emoji, and `Generated with [Claude Code]`;
  - exit 0 for a clean message, the literal text `xf0x9fxa4x96`, a comment line `# Co-authored-by: Claude`, and a Claude trailer placed only below the scissors line;
  - exit 1 for a missing message file.
  - If spawning `sh` fails with `NotFound` on Windows, these behaviour tests print `skipped: sh not found` and pass. On Unix that is a failure.

### 17.8 Bench runner (Security F5, D07, D08) — WP4

- **Deadlines (D07).**
  - xtask `STARTUP_TIMEOUT_SECS = 75` and `IDLE_TIMEOUT_EXTRA_SECS = 75` (idle deadline = N + 75 s).
  - These are strictly larger than the GUI watchdog: 60 s for startup, and `IDLE_WATCHDOG_EXTRA_SECS (60) + N + settle (3 s, §17.13)` = N + 63 s for idle. The GUI's own `bench timed out` / exit 4 therefore surfaces under xtask.
  - Update the timeout tests in `xtask/tests/bench_runner.rs`.
- **Bounded reader shutdown (F5).**
  - Each reader thread signals completion on a channel.
  - After the child has exited or been killed, xtask waits at most `READER_JOIN_TIMEOUT = 5 s` for both readers. Past that, it detaches them and fails the run with `output pipes still open after the GUI exited (a descendant process may hold them)`.
  - The kept stderr is capped to its last 64 KiB. Stdout lines longer than 1 MiB are truncated and can never count as the run line.
  - *Rejected:* process-group or Job-Object kill. It needs unsafe code or a new dependency, and the bench GUI spawns no child processes. This is recorded as an accepted risk.
- **Idle results (D08).**
  - For idle results, `samples[].spawn_to_first_frame_ms` and `summary.spawn_to_first_frame_ms` are `null`, and the markdown column shows `n/a`. The field's type becomes `number | null`.
  - The inner `startup_ms` stays valid for idle runs.
  - *Rejected:* keeping the value under a scenario-dependent meaning, a trap for every reader of the report.
  - Tests go in `xtask/tests/bench_report.rs`.

### 17.9 Test temp folders (Security F7) — WP4 (and any test that creates folders)

- Replace `temp_root()` / `temp_dir()` in `xtask/tests/layering.rs` and `xtask/tests/bench_report.rs` (and use the same approach in the new `xtask/tests/hooks.rs`) with this pattern:
  - `PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{tag}-{}-{n}", std::process::id()))`;
  - created with `std::fs::create_dir`, which fails if the path exists;
  - never `exists()` + `remove_dir_all` + `create_dir_all`.
- This is now a durable rule (architecture §8).

### 17.10 Exit codes (D03, D04) — replaces the §4.3 table and amends §5

**GUI (`visual-assembler`):**

| Code | Meaning |
|---|---|
| 0 | Normal exit, or a bench scenario completed |
| 1 | Runtime I/O or infrastructure failure: clap's help, version or error text cannot be written; a bench thread cannot be created; the bench JSON line cannot be written or flushed. `error: …` on stderr when possible. |
| 2 | Usage error |
| 3 | Window or renderer creation failed |
| 4 | Bench watchdog: no measurement within the hard limit, unless the measurement was already reported or the app shut down (D06a) |

- WP2 change: in `run`, when `err.print()` fails, return 1, not `err.exit_code()`.
- *Rejected:* code 3 for the GUI print failure. It would collide with "window creation failed".
- New Linux-only test in `apps/visual-assembler/tests/binary.rs`: `--help` and `--version` with stdout on `/dev/full` exit 1 and print `error:` on stderr.

**CLI (`va`), D03 ratified:**

- Exit 3 when stdout cannot be written or flushed, including clap's help, version and error output. The message `error: cannot write to stdout: …` (or `…print the command-line message…`) goes to stderr.
- No `va.error/1` document is printed when stdout is the stream that failed.
- The §5 sentence "M0 does not implement … the codes 1/3" is amended: code 3 exists in M0 for stdout failures only. Code 1 still waits for the first command that needs it.

### 17.11 Other conformance deviations — ratified unless stated

| ID | Decision |
|---|---|
| D05 | Ratified: `CentralPanel::default_margins().show(ui, …)`. `show_inside` is deprecated in egui 0.36.2, so §4.2 now reads `.show`. |
| D06 | Ratified (a)–(j). §4.5 step 1 now reads "… unless the measurement was already reported or the app shut down". The watchdog and recorder are created before `run_native`. §4.6: `renderer`/`backend` may be `"unknown"` only when the frame exposes no renderer (headless); empty adapter/driver strings become `null`. |
| D09 | Ratified: on Windows, `ci` parks its running executable (`xtask.exe` → `xtask.exe.old`) before the step that rebuilds xtask. Recorded in the §6 `ci` row; WP7 documents it; QA confirms it on both Windows runners on the first CI run. |
| D10 | Ratified: `xtask/tests/bench_runner.rs`, `xtask/tests/ci.rs` and the ignored live test (`cargo test -p xtask --test layering -- --ignored --nocapture`), plus the new `xtask/tests/hooks.rs` (§17.7). |
| D11 | Superseded by S6/S7 (§17.4). The `-`/`_` name equivalence for GUI crates and the extra `[deps]` coverage are ratified. |
| D12 | Ratified, including the stderr `note:` line when git is unavailable. Amended: both scripts call `git ls-files --others --ignored --exclude-per-directory=.gitignore --directory -z` instead of `--exclude-standard` (WP5), so results depend only on committed `.gitignore` files, not on `core.excludesFile` or `.git/info/exclude`. *Rejected:* `--exclude-standard`, which makes AGENCEMENT.md machine-dependent. |
| D13 | Decided: `va` adds `bin_name = "va"` (WP3), so the usage line is `Usage: va [OPTIONS] <COMMAND>` on every OS. New assertion in `apps/va/tests/binary.rs` (`help_usage_line_is_os_independent`). *Rejected:* an OS-dependent usage line, which would make docs and tests differ per OS. The GUI's `bin_name`, `about` and `long_about` are ratified in §4.4. |
| D14 | Ratified: qualified references `(architecture §N)` in skeleton crate docs. |
| D16 | Accepted as is: `make help` loses the two-space indent and ends with `Nothing to be done for 'help'` (exit 0). `make -s help` is the clean form. |
| xtask interpretations | Ratified: a failing `gen_agencement.py` makes `docs` fail; an empty `VA_PYTHON` counts as unset; `--runs` ≥ 1; `--duration-secs` 1..=3600, accepted with `bench startup` but passed only to idle runs; `dirty = false` plus a stderr warning when git fails; `logical_cpus` `integer \| null`; a run line whose scenario or renderer differs from the request is a run failure; the report file is pretty-printed JSON. |
| va internals | Ratified: the internal `CommandError`, public `Cli` fields, the `InfoDocument` with flattened report, comma-separated architectures, `NO_COLOR=1` in tests. |
| D15 | Process item: `rustup toolchain install` (1.99.0 on Windows) was run by WP1. It falls under Q2 and goes to the user for after-the-fact confirmation (§17.15). |

### 17.12 Tree additions (D01, D10) — amends §2.1 and §1

- `.github/workflows/README.md` is added to the tree and to **WP7's file list**. The README rule in `check_project_docs.py` is **not** weakened. Content: the jobs, the SHA-pin rule, the informational GUI smoke step, and how to read runs with `gh`.
- `xtask/tests/` now lists `README.md, bench_stats.rs, bench_report.rs, bench_runner.rs, bench_timestamp.rs, ci.rs, hooks.rs, layering.rs` (8 files).
- Root: `LICENSE-MIT` and `LICENSE-APACHE` (§17.1).
- `scripts/git-hooks/commit-msg` now belongs to WP5 (§17.6).

### 17.13 Renderer protocol amendments (§12; WSLg crash; WP2 observations)

**Settle (WP2 observation).** On the reference machine, wgpu/Vulkan painted one late frame about 2 s
after the first frame.

- `IDLE_SETTLE_MS` becomes **3000** (WP2), so the idle window measures steady state, not post-map transients.
- The idle watchdog becomes `IDLE_WATCHDOG_EXTRA_SECS + N + 3 s`.
- *Rejected:* keeping 1 s, which would make G3 measure compositor timing after the window is mapped.

**Windows backend (WP2/QA observation).** On the reference machine, wgpu picked Vulkan, not DX12
(spawn→frame about 1.66 s and peak RSS about 380 MiB in an unconditioned 2-run trial).

- Add configuration **C1d**: `WGPU_BACKEND=dx12 cargo xtask bench all --renderer wgpu`.
- Rule: among C1 and C1d, the configuration that passes G1–G3 with the lower p95 spawn→frame wins.
- A difference under 10 % goes to **dx12**, because DX12 is native on Windows ARM64, where Vulkan drivers are not guaranteed.
- If dx12 wins and wgpu's default pick is not dx12, WP2 makes the app request `Backends::DX12` on Windows (through `NativeOptions::wgpu_options`) when `WGPU_BACKEND` is unset.

**G4 is a target, not a blocking gate.**

- If no Windows configuration reaches p95 ≤ 500 ms, choose by the rule above. The report records the gap with suspected causes, and the gap becomes an M7 input. M0 still closes.
- **Outcome D** now triggers only when wgpu fails G1, G2 or G3 on Windows native with **both** C1 and C1d.

**WSLg compositor crash procedure (WP6).** WP2 observed a SIGSEGV of weston 9.0.0 (WSLg 1.0.73.2) after `rdp_rail_wake_handler`.

1. **Never run `wsl --shutdown`** or a reboot without the user's consent, relayed by the orchestrator. It stops every WSL distro.
2. **No `sudo apt install`.** A non-VisualAssembler Wayland client is used for cross-checking only if one is already installed.
3. **Pre-flight**, once per session:
   - record `wsl.exe --version`;
   - in the distro, check that `/mnt/wslg/runtime-dir/wayland-0` and `/tmp/.X11-unix/X0` exist;
   - copy the last 100 lines of `/mnt/wslg/weston.log` and `/mnt/wslg/stderr.log` to the scratchpad as a baseline (not to the repo).
4. **Order:** X11 first (C7 glow, C6 wgpu, C8), then Wayland (C5 glow, C4 wgpu-gl, C3 wgpu default). Testing renderer-independent paths first localizes the trigger.
5. **Probe launches.** Before any `bench all` on a configuration, run up to **3** single launches of the bench copy with `--bench startup`.
   - The compositor counts as crashed when **any** of these holds: new crash lines appear in `weston.log`/`stderr.log` (`SIGSEGV`, `Segmentation fault`, a backtrace); the `wayland-0` socket disappears; or the GUI exits 3 with a connection error.
   - Only configurations with 3 clean probes go on to the full bench.
6. **Recovery.** After a crash, poll every 5 s for up to **180 s**, waiting until the socket is back **and** an X11 probe (`env -u WAYLAND_DISPLAY … --renderer glow --bench startup`) exits 0.
   - If it recovers by itself, continue.
   - If not, stop all WSLg work and escalate for consent.
   - Stop WSLg work after **3 compositor crashes** in a session, even if each one recovered.
7. **Classification.**
   - A compositor crash during a probe or a run is a **G1 failure** of that configuration ("compositor crash") whatever the attribution. The report gives k/3 per configuration and the log excerpts.
   - Attribution: a weston SIGSEGV is a WSLg defect, since a client must not be able to crash the compositor. VisualAssembler is called a *reproducible trigger* when k ≥ 2.
     - If it is renderer-independent, it is classified as a windowing trigger (winit Wayland/RAIL).
     - If it is renderer-specific, it is classified as a GPU-path trigger.
   - Filing an issue on `microsoft/wslg` needs the user's consent (public post).
8. **Outcome W** applies when Wayland is unusable and X11 works: C3, C4 and C5 all fail G1, and at least one X11 configuration passes G1–G3.
   - The WSLg renderer decision uses the X11 results only.
   - WP2 adds a Linux-only dependency `winit = { version = "=0.30.13", default-features = false, features = ["x11"] }` (Apache-2.0, already in the graph; GUI crate allowed in `visual-assembler`; DEPENDANCE/IMPORT entries required).
   - When `platform::is_wsl()` is true and env `VA_WAYLAND` is not `1`, WP2 sets `NativeOptions::event_loop_builder` to call `winit::platform::x11::EventLoopBuilderExtX11::with_x11`.
   - Native Linux keeps winit's default (Wayland first).
   - The README documents the WSLg limitation and the `VA_WAYLAND=1` override.
   - If X11 under WSLg also fails, the WSLg GUI is listed as **not supported** in the support tiers (the CLI stays tested), the user is informed, and M0 closes on the Windows decision.

### 17.14 E1 and cargo-deny in WSL (brief item 9)

- E1 now reads: `make test`, `make lint` and `make docs` on Windows **and** in WSL; `make security` on **Windows only**.
- Linux coverage of the policy comes from the CI `deny` job (ubuntu). cargo-deny's result is host-independent, because `[graph] targets` fixes all four targets.
- No cargo-deny install is required in WSL. *Rejected:* requiring it, which would duplicate an identical result and need another consent.

### 17.15 Windows DLL search path (Security F10) — deferred to M8, normative now

- **Before M8 ships any Windows artifact:**
  - Both binaries call `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)` as the first statement of `main`. This needs the windows-sys feature `Win32_System_LibraryLoader` and a new allow-listed file `apps/<app>/src/dll_hardening.rs`.
  - The windows-msvc rustflags gain `"-C", "link-arg=/DEPENDENTLOADFLAG:0x800"`. §17.3's whitelist is updated at the same time.
  - The result is verified with Process Monitor: no DLL is probed in the executable's folder.
- *Rejected for M0:* doing it now. M0 binaries are never distributed, and it would widen the unsafe allow-list for no present benefit.
- Recorded as an accepted risk in SECURITY.md until M8.

### 17.16 Fix-wave table

Write scopes are disjoint:

- WP1: root config and licence files, `crates/*`, `arch/m68k`;
- WP2: `apps/visual-assembler/**`;
- WP3: `apps/va/**`;
- WP4: `xtask/**`;
- WP5: `Makefile`, `.github/workflows/ci.yml`, `scripts/**` (git-hooks included).

| # | Owner | Change | Acceptance check |
|---|---|---|---|
| 1.1 | WP1 | Add `LICENSE-APACHE` (serde copy) and `LICENSE-MIT` (§17.1 text) | `sha256sum LICENSE-APACHE` gives `62c7a1e3…3636a`; `LICENSE-MIT` equals the §17.1 block; both LF |
| 1.2 | WP1 | `license = "MIT OR Apache-2.0"` in `[workspace.package]`; `license.workspace = true` in `crates/*` and `arch/m68k` manifests | `cargo metadata --no-deps` shows the licence for those 7 packages |
| 1.3 | WP1 | `.cargo/deny.toml`: §17.5 values (`unmaintained = "all"`, `private.ignore = false`, +24 bans, `features` mio/net) and the header comment's command form | After rows 2.3, 3.2 and 4.7: `cargo deny --manifest-path Cargo.toml --config .cargo/deny.toml check` gives `advisories ok, bans ok, licenses ok, sources ok` |
| 2.1 | WP2 | Exit 1 when clap output cannot be printed (§17.10); constant `EXIT_RUNTIME: u8 = 1` replaces ad-hoc `ExitCode::FAILURE` | New Linux test (`/dev/full`) passes; existing binary tests pass |
| 2.2 | WP2 | `IDLE_SETTLE_MS = 3000`; idle watchdog = 60 + N + 3 s | `cargo test -p visual-assembler --all-features` passes; `settle_ms` = 3000 in an idle line |
| 2.3 | WP2 | `license.workspace = true` in `apps/visual-assembler/Cargo.toml` | Covered by 1.3 |
| 2.4 | WP2 (after WP6) | Conditional: DX12 pin (§17.13), WSL GL pin (outcome B), X11 under WSL (outcome W), CSD winit feature (§12.4) | Re-measured config passes; QA re-run |
| 3.1 | WP3 | `bin_name = "va"` | `help_usage_line_is_os_independent` passes on Windows and Linux |
| 3.2 | WP3 | `license.workspace = true` in `apps/va/Cargo.toml` | Covered by 1.3 |
| 4.1 | WP4 | Layering table §17.2 (`va` and `visual-assembler` allow-lists; leaves) | New tests: `va → visual-assembler`, `va → xtask`, `visual-assembler → va`, `visual-assembler → xtask` are violations; live workspace 0 violations |
| 4.2 | WP4 | Scan rules S1–S9 + regression tests (§17.4) | All listed tests pass; live workspace 0 violations; `cargo xtask ci` green |
| 4.3 | WP4 | `ci` step 0 build configuration (§17.3.2) | `xtask/tests/ci.rs` covers env and config cases; `RUSTFLAGS=-Aunsafe_code cargo xtask ci` exits 1 before step 1 |
| 4.4 | WP4 | Runner: deadlines 75/N+75, bounded reader join (5 s), 64 KiB stderr / 1 MiB line caps, idle `spawn_to_first_frame_ms = null` | Updated `bench_runner.rs`/`bench_report.rs` tests pass; idle markdown row shows `n/a` |
| 4.5 | WP4 | `hooks [--force]` safety + `xtask/tests/hooks.rs` (install and hook-behaviour cases, §17.7) | All `hooks.rs` tests pass once 5.1 is merged |
| 4.6 | WP4 | Temp folders via `CARGO_TARGET_TMPDIR` + `create_dir` in every xtask test | `grep -rn "std::env::temp_dir\|create_dir_all" xtask/tests` returns nothing |
| 4.7 | WP4 | `license.workspace = true` in `xtask/Cargo.toml` | Covered by 1.3 |
| 5.1 | WP5 | `scripts/git-hooks/commit-msg` = §17.6 block | `sh scripts/git-hooks/commit-msg` on the §17.7 fixtures gives the expected codes (Git Bash and WSL); `make hooks` then reinstall prints `already installed` |
| 5.2 | WP5 | `COAUTHOR_RE` = §17.6 block | A throwaway repo whose last commit has `Co-authored-by: Codex <codex@openai.com>` makes the check report `[git]` |
| 5.3 | WP5 | `[lints]` check (§17.3.1) | A scratch member with `[lints.rust] unsafe_code = "allow"` and one without `[lints]` are both reported; the real repo is clean |
| 5.4 | WP5 | `--exclude-per-directory=.gitignore` in both scripts (D12) | Under `GIT_CONFIG_GLOBAL=<tmp file setting core.excludesFile to a file listing apps/>`, the output matches the baseline |
| 5.5 | WP5 | Makefile `security` explicit form (§17.5) | `make -n security` prints the explicit command; `make security` exits 0 on Windows |
| 5.6 | WP5 | `ci.yml` `deny` job = §17.5 block | YAML parses; every `uses:` is pinned to a 40-hex SHA; the job runs on the first push |
| — | WP7 (wave 6) | `.github/workflows/README.md`; README notes (exit-code tables §17.10, Windows exe parking, hooks `--force`, settle 3 s, WSLg procedure); root README licence section | `python scripts/check_project_docs.py .` exits 0 (E7) |
| — | Security (wave 3) | SECURITY.md: updated ban/feature list, scan rules + tripwire limitation, R8 checks, hook limits, accepted risks (F5 no group kill, user-level cargo config, F10 until M8, zbus/X11 environment transports) | Security re-audit |

Dependencies inside the wave:

- Row 1.3's acceptance needs 2.3, 3.2 and 4.7.
- Row 4.5's hook-behaviour tests need 5.1.
- QA runs once after all rows land, on Windows and WSL: `make test`, `make lint`, `make docs`, `cargo xtask ci`, `make security` (Windows).

### 17.17 Items for the user (via the orchestrator)

1. Confirm after the fact that WP1's `rustup toolchain install` (1.99.0 on Windows) is covered by your Q2 consent (D15).
2. Consent for `wsl --shutdown` is requested only if the WSLg compositor does not recover by itself (§17.13). Filing a `microsoft/wslg` issue is optional and needs consent.
3. Once `GaspardSapin/VisualAssembler` exists: consent to enable branch protection on `main` with the required checks `fmt`, `docs`, `deny` and the four `check (…)` jobs (via `gh api`). That makes the AI co-author log check and the policy jobs blocking (Security F4).

---

## 18. Amendments after audit round 2 (2026-10-13)

Inputs: the round-2 QA (QA2-F1…F6), Security (R2-1…R2-13) and conformance (CR1-01…11) reports, and the
orchestrator's binding arbitration on the tripwire threat model.

- Precedence: **§18 > §17 > §0–§16**.
- WP1, WP2 and WP3 have no changes in this wave.

### 18.1 Threat model of the tripwires (binding; also in architecture §9.3/§9.4)

The xtask source scan (S1–S9) and the `ci` step-0 build-configuration check are **tripwires** against
*accidental or careless* introduction of network code, unsafe code or build-flag overrides. They are
**not** a sandbox against a deliberately malicious contributor. Deliberate evasion is covered by code
review, cargo-deny on dependencies, and branch protection with required CI (§17.17 item 3).

From now on, QA and every auditor must classify each new scan or configuration bypass under one of two classes:

| Class | Definition | Examples | Handling |
|---|---|---|---|
| **Blocking** | A *realistic accidental* pattern: something a contributor writes in good faith or by habit, or that a tool, editor or OS produces. It compiles with the workspace configuration and is caught by nothing: no tripwire, no compiler lint, no cargo-deny. | rustfmt-wrapped attributes; a file saved as `.RS` on Windows; an empty `RUSTFLAGS` exported by a shell profile; a `[build]`/`runner` line added to try something; a forgotten `[patch]`; a new crate folder missing from `members`; a direct dependency on an egui add-on crate; the locale git sets for hooks | Fix in the next wave, with a regression test |
| **Not blocking** | Needs deliberate obfuscation | Renamed or aliased built-in macros (`use std::include as inc`); raw-identifier attributes (`#[r#path]`, `std::r#net`); out-of-tree target paths (`[lib] path`/`build =` outside the package); `[workspace] exclude` combined with path tricks; crate renaming through `package =`; macros that ignore their input; cargo config `include` or escaped quoted keys (now caught anyway by §18.4) | A SECURITY.md Limitations line only. Never an audit failure, never reopened. |

### 18.2 Commit-message hook (QA2-F1) — replaces the §17.6 hook block (WP5)

Git for Windows' grep 3.0 does not match a 4-byte UTF-8 character under a UTF-8 locale, and git sets
`LC_CTYPE=C.UTF-8` for hooks. The hook therefore forces the C locale for every tool it runs: all its
patterns are ASCII or explicit bytes, so byte-level matching is exact on every grep.

*Rejected:* `LC_ALL=C` on the emoji grep only. The `sed` and the case-insensitive `grep` would still depend on the caller's locale.

```sh
#!/bin/sh
# Reject commit messages that credit an AI tool (agentic-project framework, section 10).
# Install: make hooks (runs cargo xtask hooks).
# Keep the names and trailer keys in sync with COAUTHOR_RE in scripts/check_project_docs.py.
# Limits: client-side only (--no-verify skips it); the CI docs job re-checks the last 50 commits.

# Byte-level matching on every platform (Git for Windows' grep misses 4-byte UTF-8 characters
# under the UTF-8 locale git sets for hooks).
LC_ALL=C
export LC_ALL

msg_file=$1
if [ -z "$msg_file" ] || [ ! -r "$msg_file" ]; then
    echo "commit-msg hook: cannot read the commit message file '$msg_file'." >&2
    exit 1
fi

names='claude|anthropic|copilot|cursor|gpt|openai|codex|gemini|aider|devin|windsurf|codeium'
pattern="(co-authored-by|assisted-by|generated-by):.*($names)|generated with .*($names)"
robot=$(printf '\360\237\244\226')

# Ignore the verbose diff below the scissors line and every comment line.
if ! body=$(sed -e '/^# -\{24\} >8 -\{24\}$/,$d' -e '/^#/d' "$msg_file"); then
    echo "commit-msg hook: cannot read the commit message." >&2
    exit 1
fi

reject() {
    echo "commit rejected: the message credits an AI tool ($1)." >&2
    echo "Remove the Co-Authored-By / Assisted-by / Generated-with line or the robot emoji, then retry." >&2
    exit 1
}

printf '%s\n' "$body" | grep -qiE -e "$pattern"
case $? in
    0) reject "co-author or generated-with mention" ;;
    1) ;;
    *) echo "commit-msg hook: grep failed; refusing the commit." >&2; exit 1 ;;
esac

printf '%s\n' "$body" | grep -qF -e "$robot"
case $? in
    0) reject "robot emoji" ;;
    1) ;;
    *) echo "commit-msg hook: grep failed; refusing the commit." >&2; exit 1 ;;
esac

exit 0
```

- `COAUTHOR_RE` is unchanged: Python matches the emoji as a code point, whatever the locale.
- **Required tests** (`xtask/tests/hooks.rs`, WP4 keeps QA's tests). They must pass on Windows and Linux:
  - `hook_rejects_ai_credits_under_utf8_locales` runs every rejecting fixture of §17.7 under `LC_ALL=C.UTF-8`, `LC_ALL=en_US.UTF-8`, `LANG=en_US.UTF-8` only, and with no locale variable;
  - `hook_installed_by_xtask_and_run_by_git_rejects_every_ai_credit` runs the hook through `git hook run`, which applies git's own hook environment.
- **R2-11 (accepted, Not blocking).** SECURITY.md wording: "Comment lines and a scissors line are ignored by the hook but kept in the commit when the message comes from `-m`/`-F`, `--cleanup=verbatim`/`whitespace`, or a non-default `core.commentChar`. Such a message passes locally and is caught by the CI `docs` job's log check after the push, which becomes a required check with branch protection."

### 18.3 Build environment (QA2-F6, R2-6, CR1-04) — amends §17.3.2(a) (WP4)

- Step 0 fails when any of `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `CARGO_BUILD_RUSTFLAGS` or `CARGO_TARGET_*_RUSTFLAGS` is **set, even to an empty value**: cargo 1.99 treats an empty `RUSTFLAGS` as an override and drops `+crt-static`.
- The message stays `<VAR> is set: it replaces .cargo/config.toml (+crt-static) and can silence lints; unset it`.
- CR1-03(1) is ratified: names match case-insensitively on every OS (fail-closed; a false positive on Unix is improbable).
- The test `clean_or_empty_build_environment_passes` becomes `empty_build_variable_is_rejected` (an empty `RUSTFLAGS` gives one problem) plus `clean_build_environment_passes`.

### 18.4 Canonical `.cargo/config.toml` (R2-5, CR1-03) — replaces §17.3.2(b) (WP4)

The whitelist of headers and keys becomes an exact comparison.

- Drop blank lines and full-line `#` comments, and remove all whitespace from each remaining line. The result must equal, line for line and in order, the constant `CANONICAL_CARGO_CONFIG` in `xtask/src/ci.rs`:

```
[alias]
xtask="run--packagextask--"
[target.x86_64-pc-windows-msvc]
rustflags=["-C","target-feature=+crt-static"]
[target.aarch64-pc-windows-msvc]
rustflags=["-C","target-feature=+crt-static"]
[target.aarch64-unknown-linux-gnu]
linker="aarch64-linux-gnu-gcc"
```

- Every other line is a violation that names the line number. This includes top-level keys, `include`, quoted keys, `runner`, an extra or changed alias, and trailing comments.
- Also violations: a missing or unreadable `.cargo/config.toml`, and the existence of an extension-less `.cargo/config` (CR1-03(2), ratified).
- *Rejected:* extending the per-key whitelist of §17.3.2(b). Every escape found in round 2 (`include`, quoted keys, `runner`, alias values) came from rules that matched shapes rather than content.
- Any intended change to the file (e.g. M8's `/DEPENDENTLOADFLAG:0x800`) updates the file and the constant in the same commit.
- Tests in `xtask/tests/ci.rs`: one each for `include = ["x.toml"]`, `"build"."rustflags"`, a `runner` line, a modified `xtask` alias, a trailing comment, and `.cargo/config` present. The real file passes.

### 18.5 Source-scan rule changes (amends §17.4) (WP4)

| Rule | Change |
|---|---|
| S1 | Ratified signature (CR1-01): `scan_sources(root: &Path, members: &[PathBuf], target_directory: &Path)`. |
| S2 | **Removed** (R2-12). `SCAN_DATA_EXEMPT` is deleted: the code view already blanks the pattern literals. The live-workspace test then covers `xtask/src/layering.rs` and `xtask/tests/layering.rs` under every rule, and must still report 0 violations. |
| S5 | **Shebang (QA2-F5, R2-1).** Same as `rustc_lexer::strip_shebang`: after an optional UTF-8 BOM, if the text starts with `#!` and the first token after `#!` (skipping whitespace and comments) is not `[`, the whole first line is blanked as a comment. Test: `shebang_line_does_not_hide_the_code_after_it` must pass, and a crate starting with `#![deny(unsafe_code)]` is still lexed as code. |
| S6(b) | Ratified (CR1-02): the *innermost enclosing group of any kind* must be introduced by `deny`/`forbid`. |
| S8 | Ratified extras (CR1-02): absolute targets, any backslash, prefixed or raw literals, and `cfg_attr(…, path = …)` are checked; the empty allow-list constant is named `INCLUDE_ALLOWLIST`. The `.rs` suffix test becomes ASCII-case-insensitive. |
| S9 / walk | **Case-insensitive extension (QA2-F2, R2-4).** A file is pattern-scanned when its extension equals `rs` ignoring ASCII case, on every OS, so the result never depends on the host. Test: `module_file_with_an_upper_case_rs_extension_is_scanned` must pass. |
| GUI crates (R2-13) | Direct dependencies are matched **by family**: the normalized name (`_`≡`-`) starts with `egui`, `eframe`, `epaint`, `emath`, `ecolor`, `wgpu`, `naga`, `winit`, `glutin`, `glow` or `accesskit`, or equals `raw-window-handle`. `va-ui` may use exactly `egui` and `egui_kittest`; `visual-assembler` may use anything. Tests: `va → egui_extras` and `va-core → epaint` are violations. *Rejected:* a resolved-graph check for `va` (`cargo tree`), which is platform-dependent and slower; families cover the realistic accident. |

**Accepted, Not blocking (§18.1); SECURITY.md Limitations only:**

- QA2-F3 `#[r#path = …]`;
- QA2-F4/R2-2 renamed or aliased `include!`;
- R2-3 out-of-tree `[lib]`/`[[bin]]`/`build` paths;
- crate aliasing through `package =`;
- input-ignoring macros;
- `std::r#net`;
- R2-7 Unix-domain sockets (`std::os::unix::net`). These are local IPC, not network, so the offline rule does not cover them.

QA's tests `raw_identifier_path_attribute_is_checked` and `renamed_include_macro_is_flagged` are kept as
documentation with `#[ignore = "accepted tripwire limit: m0-foundations §18.1"]`. *Rejected:* deleting them,
which would lose the record of the known bypass.

### 18.6 Manifest checks in `scripts/check_project_docs.py` (R2-8, R2-9) (WP5)

Both are realistic accidents and cheap to catch, so they are fixed now:

1. **Implicit members.** Any `Cargo.toml` with a `[package]` table under the root that is not matched by `[workspace].members` is reported as `[deps] <path>: package inside the workspace is not listed in [workspace].members`. The walk skips the paths already skipped (target, `IGNORED_DIRS`, gitignored). This closes the `[lints]`/`[deps]` gap where cargo silently adds path dependencies as members.
2. **Patching.** A root `[patch]` (any registry) or `[replace]` table is reported as `[deps] Cargo.toml: [patch]/[replace] tables are not allowed (they bypass cargo-deny's source checks)`. A future need goes through an Architect decision.

### 18.7 cargo-deny job and command (R2-10) — replaces the §17.5 job and command (WP5)

- The command everywhere (Makefile `security` and CI) is `cargo deny --locked --manifest-path Cargo.toml --config .cargo/deny.toml check`.
- `--locked` makes a stale `Cargo.lock` an error instead of re-resolving it. It is a top-level option, verified in cargo-deny 0.20.2 `--help`.

```yaml
  deny:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - uses: dtolnay/rust-toolchain@e2a55d2ffb04f378e9626c28d38b36d230d1e12f # master, 2026-10-13
        with:
          toolchain: "1.99.0"
      - name: install cargo-deny 0.20.2 outside the checkout (crates.io checksums, locked, never cached)
        working-directory: ${{ runner.temp }}
        run: |
          cargo install cargo-deny --version 0.20.2 --locked --root "$RUNNER_TEMP/cargo-deny"
          echo "$RUNNER_TEMP/cargo-deny/bin" >> "$GITHUB_PATH"
      - run: cargo deny --locked --manifest-path Cargo.toml --config .cargo/deny.toml check
```

- No rust-cache in this job: the policy tool is rebuilt from crates.io on every run, outside the repository's `.cargo/config.toml`.
- *Rejected:* keeping rust-cache with `cache-bin: false`. The gain is small, and it is one more moving part in the job whose integrity matters most.

### 18.8 Conformance items

| ID | Decision |
|---|---|
| CR1-01 | Refuted as a defect. The 3-parameter signature is ratified (§18.5 S1). |
| CR1-02 | Ratified (§18.5 S6(b), S8). |
| CR1-03 | (1) and (2) ratified (§18.3, §18.4); (3) superseded by the canonical rule (§18.4): trailing comments are not allowed. |
| CR1-04 | Fixed by §18.3. |
| CR1-05 | Ratified: `apps/visual-assembler/tests/bench_recorder.rs` joins the §2.1 test list, and `pub fn watchdog_limit(&BenchPlan) -> Duration` joins the §4.4 recorder row (60 s for startup; 60 + N + 3 s for idle). |
| CR1-06 | Ratified: the repo-local `core.hooksPath` form exercises the same refusal path. The `GIT_CONFIG_GLOBAL` variant is optional. |
| CR1-07 | Ratified: StatsD clients (`cadence`, `statsd`) count as telemetry; `ws` counts as a network client/server. |
| CR1-08 | Fixed in architecture §7 (settle 3 s). |
| CR1-09 | Decided: no body rewrite during M0 (no work package should edit a moving text). The Architect folds §17/§18 into the body once, when M0 closes. Until then, the index below is authoritative. The §4.6 example line stays illustrative (`"settle_ms":1000`) because two serializer tests embed it; the constant is 3000. |
| CR1-10, CR1-11 | No spec change: Security (wave 3) and WP6/WP7 deliverables. |

**Superseded passages (CR1-09 index):**

| Body passage | Read instead |
|---|---|
| §2.1 tree (root files, `xtask/tests`, `apps/visual-assembler/tests`) | §17.1, §17.12, §18.8 CR1-05 |
| §4.2 `show_inside` | §17.11 D05 |
| §4.3 exit codes; §4.4 lib.rs clap exit; §5 "codes 1/3" | §17.10 |
| §4.4/§4.5/§4.6 settle 1000 ms, watchdog "+ 1 s" | §17.13, §17.11 D06 |
| §6 `ci` and `hooks` rows | §17.3, §17.7, §18.3, §18.4 |
| §6.1 step 2.3 timeouts; §6.2 idle `spawn_to_first_frame_ms` | §17.8 |
| §6.3 rule table and `scan_sources` | §17.2, §17.4, §18.5 |
| §7 `security` target; §8 `deny` job; §9 deny values | §17.5, §18.7 |
| §10 item 3 `--exclude-standard` | §17.11 D12 |
| §11 temp folders | §17.9 |
| §12 protocol | §17.13 |
| §14 F11 | Removed by §17.5 |
| §0 E1 | §17.14 |
| §17.6 hook block | §18.2 |
| §17.3.2 | §18.3, §18.4 |
| §3.1 comment "Only crates used by at least one member (cargo-deny: unused = "deny")"; §3.6 "cargo-deny's `unused` check would flag them"; §15 row "`[workspace.dependencies]` holds only crates already used" | §19.9 S19-3, row 19-11 |
| §4.1 `default = ["renderer-wgpu"]` | §19.1 |
| §4.3 `--renderer` row and its default | §19.1, §19.4 |
| §4.4 rows `lib.rs`, `cli.rs`, `spike.rs`, `bench/recorder.rs`, `bench/report.rs`; §4.5 step 1 `BenchRecorder::new(plan, started)` | §19.9 WP2-2, WP2-3 |
| §4.6 `va.bench.run/1` and its example | §19.5 |
| §6.1 step 1 and the renderer loop; §6.2 `va.bench.report/1` | §19.5, §19.9 WP4-1…WP4-6 |
| §8 smoke bullet "reports whether wgpu initializes" | §19.1 (the step reports what `auto` starts); §19.9 CR19-06 for the second step |
| §10 item 2 (`[deps]`, from members to the workspace only) | §19.9 row 19-9(b) (reverse direction added) |
| §12.3/§12.4 decision rule; outcome W as the WSLg decision | §19 preamble, §19.1 |

### 18.9 Fix-wave table (only what must change)

| # | Owner | Change | Acceptance check |
|---|---|---|---|
| 18-1 | WP5 | `scripts/git-hooks/commit-msg` = §18.2 block | On Windows, `cargo test -p xtask --test hooks` passes, including the UTF-8-locale and `git hook run` tests; a real commit with the robot emoji is rejected on Windows and in WSL |
| 18-2 | WP5 | `check_project_docs.py`: implicit-member and `[patch]`/`[replace]` checks (§18.6) | Scratch fixtures (an unlisted in-tree package; a root `[patch.crates-io]`) are each reported; the real repository has no `[deps]` problem |
| 18-3 | WP5 | `ci.yml` `deny` job = §18.7 block; Makefile `security` gets `--locked` | YAML parses; every `uses:` is pinned to a 40-hex SHA; `make -n security` prints the §18.7 command; `make security` exits 0 on Windows |
| 18-4 | WP4 | Step 0: variables set even when empty (§18.3); canonical config (§18.4) | `RUSTFLAGS= cargo xtask ci` exits 1 at step 0; the §18.4 tests pass; the real config passes |
| 18-5 | WP4 | Scan: case-insensitive `.rs` (walk + S8), shebang strip, S2 removed, GUI families; 2 QA tests marked `#[ignore]` with the §18.1 reason | `cargo test -p xtask --test layering` passes (upper-case and shebang tests active); the family tests pass; the live workspace (layering files included) reports 0 violations; `cargo xtask ci` is green on Windows and WSL |
| — | Security (wave 3) | SECURITY.md: §18.1 threat model and class table; the Not-blocking list of §18.5; R2-11 wording; S2 removal; canonical config; `--locked` deny command; stale line ranges (CR1-10) | Security re-audit uses the §18.1 classes |
| — | WP7 (wave 6) | READMEs: `bench_recorder.rs`; `Usage: va …` on every OS; tripwire threat model in the xtask README | E7 |

Rows 18-1…18-3 (WP5) and 18-4…18-5 (WP4) touch disjoint files and run in parallel. QA then re-runs on
Windows and WSL: `make test`, `make lint`, `cargo xtask ci`, and `make security` (Windows).

---

## 19. Renderer decision by the user (2026-10-14)

Precedence: **§19 > §18 > §17 > §0–§16**.

The user's decision is **final**: glow by default everywhere, automatic fallback to wgpu when OpenGL
initialization fails at startup, a setting and a CLI option to force either renderer, wgpu directly on
Windows ARM64, the fallback tested by simulating an OpenGL failure, and the behaviour documented in the
README. It **replaces** the decision rule of §12.3/§12.4/§17.13 and outcome W.

- The §12 gates stay as *measurements* to report; they no longer decide.
- The durable policy is in architecture §6 (precedence, settings key) and §9.8.
- The DX12 pin (`pin_windows_wgpu_backend`) stays.

### 19.1 Build and selection

| Topic | Decision | Rejected alternative |
|---|---|---|
| Features | `apps/visual-assembler` `default = ["renderer-glow", "renderer-wgpu"]`. Single-renderer builds stay compilable (CI clippy runs glow-only and wgpu-only, §19.6) but are no longer shipped or benchmarked. | Shipping glow-only with no wgpu: the user asked for a wgpu fallback. |
| Choice values | `auto`, `glow`, `wgpu` everywhere: CLI `--renderer`, env `VA_RENDERER`, M4 setting `[render] renderer`. `auto` is an explicit CLI value; an absent flag defers to the next precedence level. | "Absent flag = auto" with no `auto` value: a user with `VA_RENDERER=wgpu` (or, later, a setting) could not get back to auto for one run. |
| Precedence | `--renderer` > `VA_RENDERER` > `settings.toml` `render.renderer` (M4) > built-in `auto`. An invalid `VA_RENDERER` value prints `warning: ignoring VA_RENDERER='<v>' (expected auto, glow or wgpu)` on stderr and is ignored. An invalid CLI value is a usage error (exit 2). | Settings file above env (architecture §6). |
| `auto` order | Windows aarch64: **wgpu, then glow**. Every other target (Windows x64, Linux x64 and aarch64): **glow, then wgpu**. In a single-renderer build, `auto` is reduced to the compiled renderer. | wgpu only on Windows ARM64 (the user's "wgpu directly"): keeping glow as a second chance there costs no code, and wgpu still goes first as asked. |
| Forced (`glow`/`wgpu`) | Exactly that renderer. **No fallback.** A failure exits with 3 (§19.4). | Falling back even when forced: it would hide the failure and make benches of a forced renderer measure the other one. |
| WSL window system | When `platform::is_wsl()` is true and `VA_WAYLAND` is not `1`, **every** renderer uses X11. This is the §17.13 item 8 winit dependency (`winit = { version = "=0.30.13", default-features = false, features = ["x11"] }`, Linux-only, added to root `[workspace.dependencies]` by WP1) and `NativeOptions::event_loop_builder` calling `EventLoopBuilderExtX11::with_x11`. The hook must be set on the **first** attempt, because eframe creates its thread-local event loop once and ignores the hook on later `run_native` calls (§19.2). With `VA_WAYLAND=1` and a plan that includes wgpu, print `warning: VA_WAYLAND=1 with the wgpu renderer can crash the WSLg compositor (weston 9)`. Native Linux keeps winit's default. | X11 only when wgpu is used: in-process fallback cannot switch window system, and glow on X11 under WSLg measured fine (C7, 0 crashes in 28 launches). |
| Windows wgpu | The DX12 pin applies to every wgpu use, forced or fallback, unless `WGPU_BACKEND` is set. | — (already decided in §17.13) |

### 19.2 Fallback mechanism (verified against eframe 0.36.2 / winit 0.30.13 sources)

**What the source shows:**

- `eframe::run_native` with the default `run_and_return = true` runs through `with_event_loop`, a **thread-local** `EventLoop` created on the first call and **reused** by every later call ("so we can support closing and opening an eframe window multiple times").
- A glow initialization error happens in `GlowWinitApp::resumed` → `init_run_state`:
  - `Error::Glutin` (display or context creation);
  - `Error::NoGlutinConfigs` (from `DisplayBuilder::build`);
  - `Error::OpenGL(PainterError)` (GL or shader version unsupported by egui_glow);
  - `Error::AppCreation` (our creator).

  `WinitAppWrapper::handle_event_result` stores it and calls `event_loop.exit()`. `run_app_on_demand` then returns normally and `run_native` returns `Err`, leaving the event loop reusable. winit's run-on-demand supports Windows, X11 and Wayland.
- **Decision: in-process retry.** Call `run_native` again with the next renderer, in the same thread.
- *Rejected:* re-executing the program with `--renderer wgpu`. On Windows the parent must stay alive to forward the exit code, while still holding the OpenGL driver it loaded. The bench timing would have to cross processes. It also needs a loop guard. In-process retry has none of these costs.

**Exactly what triggers a fallback.** All three conditions must hold:

1. the plan is `auto` and has a next renderer left;
2. `run_native` returned `Err` **before the first frame**, meaning `SpikeApp::ui` has never been called (an `Arc<AtomicBool>` set at the top of the first `ui`);
3. the error is a **renderer initialization error**:
   - glow attempt: `Glutin`, `NoGlutinConfigs`, `OpenGL`, or `AppCreation` carrying our `RendererInitError` (the injection of §19.3, or a GL version check the creator may add);
   - wgpu attempt (Windows aarch64 order): `Wgpu`, or `AppCreation` carrying `RendererInitError`.

**What never triggers a fallback:**

- `WinitEventLoop` and `Winit` errors (no display, window system failure): the other renderer would hit the same failure;
- any error after the first frame (the app exits with 3, as today);
- a **panic**, for example eframe's `expect` on an empty glutin config list. Panics are not caught: winit's event loop state after an unwind is unspecified, so the panic propagates (exit 101), and the README tells the user to force the other renderer;
- a forced renderer.

**State across attempts:**

- Each attempt builds fresh `NativeOptions` and a fresh app creator.
- The bench `BenchRecorder` and its watchdog are created once, before the first attempt. They live outside the creator (a shared `Option` slot that the creator takes from), so a failed attempt neither drops them nor resets the watchdog limit.
- `startup_ms` keeps counting from `main()`, so the fallback cost is included.
- The fallback happens **at most once**, because the order has two entries.

**Reporting:**

- stderr on fallback: `warning: the <glow|wgpu> renderer failed to initialize: <error>; falling back to <wgpu|glow>`.
- Bench JSON: see §19.5.
- After the run, the app does not show the fallback in the UI in M0. M4 adds a status-bar note.

### 19.3 Failure injection and tests

**Hook.** `VA_TEST_FAIL_RENDERER=<list>`, where `<list>` is comma-separated values among `glow` and `wgpu`.

- It is **compiled into every build**, release included.
- When a listed renderer's app creator runs, it returns `Err(AppCreation(RendererInitError::Injected))` instead of the app. The failure happens after the real OpenGL or wgpu initialization, so the event-loop reuse path is exercised exactly as in a genuine failure.
- On startup, the app prints `warning: VA_TEST_FAIL_RENDERER=<list> is set: simulating a renderer initialization failure (test hook)`.
- Bench JSON records it in `injected_failure`.
- Unknown values produce a warning and are ignored.
- *Rejected:* `cfg(debug_assertions)` only. The shipped release binary's fallback path, which is what users run, could then never be exercised, and the fallback cost (§19.6) could not be measured on it.
- Footgun analysis for SECURITY.md: the variable is env-only, with no file or setting. It can only make startup take the fallback path or exit with 3. Anyone who sets a process's environment already controls it. The loud warning keeps it from going unnoticed.

**CI tests (headless; no window can open):**

| File | Test | Assertion |
|---|---|---|
| `apps/visual-assembler/tests/cli.rs` | `renderer_values_parse` | `--renderer auto\|glow\|wgpu` parse; `vulkan` is a usage error |
| same | `renderer_precedence` | Pure `resolve_renderer(cli, env, compiled, os, arch)`: no CLI and no env gives `auto`/`default`; env `wgpu` gives forced wgpu/`env`; CLI `auto` + env `wgpu` gives `auto`/`cli`; env `vulkan` gives `auto` plus one warning |
| same | `auto_order_per_target` | `(windows, x86_64)` → [glow, wgpu]; `(windows, aarch64)` → [wgpu, glow]; `(linux, x86_64)` and `(linux, aarch64)` → [glow, wgpu]; forced → one entry; compiled `[glow]` only → [glow] |
| same | `injection_list_parses` | `glow` → {glow}; `glow,wgpu` → both; empty → none; `vulkan` → none plus a warning |
| same | `fallback_eligibility` | Pure `is_fallback_eligible(&eframe::Error, ui_started)`: `AppCreation(RendererInitError)` before the first frame → true; after it → false; `AppCreation(other)` → false; every other variant a test can construct is classified as in §19.2 |
| `apps/visual-assembler/tests/binary.rs` | `platform_failure_never_falls_back` (Linux only) | With `DISPLAY`, `WAYLAND_DISPLAY` and `WAYLAND_SOCKET` removed and `VA_TEST_FAIL_RENDERER=glow`, `--bench startup` exits 3; stderr contains `cannot create the window` and **not** `falling back` |
| `apps/visual-assembler/tests/bench_report.rs` | v2 schema | The §19.5 key set and order; the §19.5 example line reproduced byte for byte (it replaces the §4.6 example in both serializer tests) |

**Local tests (need a display).** File `apps/visual-assembler/tests/fallback.rs`, every test marked `#[ignore = "needs a display; run with cargo test -p visual-assembler --test fallback -- --ignored"]`:

- `injected_glow_failure_falls_back_to_wgpu`: `VA_TEST_FAIL_RENDERER=glow --bench startup` exits 0. JSON: `renderer == "wgpu"`, `fallback_from == "glow"`, `renderer_requested == "auto"`. stderr has the fallback warning.
- `both_failures_exit_3`: `VA_TEST_FAIL_RENDERER=glow,wgpu` exits 3 with the §19.4 messages.
- `forced_renderer_never_falls_back`: `--renderer glow` with `VA_TEST_FAIL_RENDERER=glow` exits 3, and stderr has no `falling back`.
- `auto_uses_glow_first` (not on Windows aarch64): plain `--bench startup` gives `renderer == "glow"` and `fallback_from == null`.

QA runs them on Windows and WSLg X11.

**Genuine OpenGL failure** (QA, local, WSLg, no sudo; manual command recorded in M0-backend.md):

```
env -u WAYLAND_DISPLAY __GLX_VENDOR_LIBRARY_NAME=va-none \
    __EGL_VENDOR_LIBRARY_FILENAMES=/nonexistent/egl_vendor.json \
    "$CARGO_TARGET_DIR/bench/shipped/visual-assembler" --bench startup
```

- Expected: glutin's GLX and EGL loading fail through libglvnd, the JSON shows `renderer == "wgpu"` and `fallback_from == "glow"`, and wgpu runs on Vulkan (lavapipe).
- If glow still initializes on that machine, QA records it, and the injection tests remain the acceptance basis.
- No genuine failure is attempted on Windows (it would need admin changes).

**CI GUI smoke (informational, Windows runners, WP5).** Add a second step, `VA_TEST_FAIL_RENDERER=glow ./target/release/visual-assembler.exe --bench startup` (bash, `continue-on-error`). Its JSON line documents the fallback on the hosted x64 and ARM64 images.

### 19.4 Exit codes and messages

| Situation | stderr | Exit |
|---|---|---|
| Fallback succeeded | `warning: the glow renderer failed to initialize: <error>; falling back to wgpu` | 0 at normal exit |
| `auto`, both renderers failed | the fallback warning above, then `error: no renderer could be initialized (tried glow, then wgpu): <last error>` plus `note: see the README section "Renderer selection"` | **3** |
| Forced renderer failed | `error: the <r> renderer failed to initialize: <error> (forced by <--renderer \| VA_RENDERER>; try --renderer auto)` | **3** |
| Platform failure (no display) | `error: cannot create the window or the renderer: <error>` (unchanged) | **3** |
| Error after the first frame | `error: cannot create the window or the renderer: <error>` (unchanged) | **3** |

The §17.10 table is otherwise unchanged (1 runtime I/O, 2 usage, 4 watchdog).

### 19.5 Bench contract (WP2 JSON, WP4 runner and report)

**GUI line `va.bench.run/2`.** The 18 keys of §4.6 plus 4 new ones, in this order:

`schema, scenario, renderer_requested, renderer, fallback_from, fallback_reason, injected_failure, backend, adapter, driver, window_system, wsl, os, arch, version, startup_ms, frames_painted, process_cpu_ms, peak_rss_bytes, peak_private_bytes, settle_ms, idle_window_ms`

New keys:

- `renderer_requested`: `"auto" | "glow" | "wgpu"`, the resolved choice.
- `renderer`: the renderer actually running.
- `fallback_from`: `null | "glow" | "wgpu"`.
- `fallback_reason`: `null | string`, the first line of the error.
- `injected_failure`: `null | string`, the raw `VA_TEST_FAIL_RENDERER` value when it is set.

Normative example (it replaces the §4.6 example):

`{"schema":"va.bench.run/2","scenario":"startup","renderer_requested":"auto","renderer":"wgpu","fallback_from":"glow","fallback_reason":"injected renderer failure (VA_TEST_FAIL_RENDERER)","injected_failure":"glow","backend":"dx12","adapter":"AMD Radeon(TM) 860M Graphics","driver":"…","window_system":"win32","wsl":false,"os":"windows","arch":"x86_64","version":"0.1.0","startup_ms":512.3,"frames_painted":1,"process_cpu_ms":437.5,"peak_rss_bytes":358612992,"peak_private_bytes":120586240,"settle_ms":null,"idle_window_ms":null}`

**xtask runner (replaces §6.1 step 1 and the renderer loop):**

- Build the **shipped** binary once: `cargo build --release -p visual-assembler --message-format=json-render-diagnostics`, with default features, so both renderers are in.
- Copy it to `<target_directory>/bench/shipped/visual-assembler[.exe]` and record `binary_bytes` of that copy.
- `--renderer auto|glow|wgpu|all`:
  - the default becomes **`auto`**, which is what ships;
  - `all` runs glow, then wgpu, then auto;
  - each run passes `--renderer <r>` to the GUI.
- Run-line validation:
  - `renderer_requested` must equal `<r>`;
  - for `glow`/`wgpu`, `fallback_from` must be `null` and `renderer` must equal `<r>`;
  - for `auto`, any `renderer` is valid, and a fallback is counted, not rejected.
- *Rejected:* keeping single-renderer builds. They no longer ship, and the dual binary's size and load time are what users get.
- Makefile (WP5): `RENDERER ?= auto`, and the help line reads `RENDERER=auto|glow|wgpu|all`.

**Report `va.bench.report/2`.** Each result object:

- replaces `renderer` with `renderer_requested` plus `renderer`, the used renderer taken from the samples, or `"mixed"`;
- adds `fallbacks`, the integer count of samples with a non-null `fallback_from`;
- adds `"binary": "shipped"`.

The `env` map also records `VA_RENDERER`, `VA_TEST_FAIL_RENDERER` and `VA_WAYLAND`.

Markdown header:

```
| scenario | requested | renderer | fallbacks | backend | window | spawn→frame p50/p95 ms | startup p50/p95 ms | idle CPU p50/p95 % | frames p50/max | peak RSS p50 MiB | binary MiB |
```

### 19.6 CI changes

- **`cargo xtask ci` (WP4):** step 3 (glow-only clippy) stays. A new **step 3b** runs `cargo clippy -p visual-assembler --all-targets --no-default-features --features renderer-wgpu --locked -- -D warnings`. `xtask/tests/ci.rs` is updated.
- **`ci.yml` (WP5):** the second informational smoke step of §19.3. No other job changes.

### 19.7 Renderer report `docs/perf/M0-backend.md` (WP6)

Add **"§11 Decision (user, §19)"**. It quotes the decision, states that it supersedes §6 and outcome W, and gives the numbers that motivated it:

- Windows: glow p95 302 ms / 93 MiB against wgpu-DX12 465 ms / 342 MiB;
- WSLg Wayland: glow passes, while wgpu crashes weston;
- WSLg X11: both pass.

All new measurements use the **shipped** binary:

| # | Where | Command | Purpose |
|---|---|---|---|
| N1 | Windows, mains power, Best performance | `cargo xtask bench all --renderer all` | Shipped default (auto → glow) vs forced glow and wgpu; closes the open G4 AC item; shipped binary size |
| N2 | WSLg X11 (default under WSL) | `cargo xtask bench all --renderer all` | Same on WSLg |
| N3 | WSLg Wayland, glow only | `VA_WAYLAND=1 cargo xtask bench all --renderer glow` | Reference for the opt-in Wayland path. wgpu on Wayland is **not** run (compositor crash). |
| N4 | Windows and WSLg X11 | `VA_TEST_FAIL_RENDERER=glow cargo xtask bench startup --renderer auto` vs `… --renderer wgpu` | **Fallback cost** = p50/p95 difference. The injection fails after full GL initialization, so this is an upper bound. |
| N5 | WSLg X11 | The §19.3 genuine-failure command (one launch) | Does a real GL failure fall back? |
| N6 | Windows | `VA_TEST_FAIL_RENDERER=glow,wgpu` (one launch) | Exit 3 and the messages |

E5 now reads: the report contains §11 with N1–N6, and the default features match the decision.

### 19.8 Fix-wave table

| # | Owner | Change | Acceptance check |
|---|---|---|---|
| 19-1 | WP1 | Root `[workspace.dependencies]` gains `winit = { version = "=0.30.13", default-features = false, features = ["x11"] }` | `cargo deny --locked --manifest-path Cargo.toml --config .cargo/deny.toml check` is all ok (winit is now used, so `unused` is satisfied) |
| 19-2 | WP2 | Default features both; `RendererChoice::Auto`; `resolve_renderer` (precedence, `VA_RENDERER`, target order); in-process fallback loop (§19.2); recorder outside the creator; WSL → X11 hook on the first attempt; injection hook; §19.4 messages; JSON v2; Linux `winit` target dependency | §19.3 CI tests pass on 4 runners; §19.3 local ignored tests pass on Windows and WSLg X11; `cargo xtask ci` green |
| 19-3 | WP4 | Shipped-binary runner, `--renderer auto\|glow\|wgpu\|all` (default `auto`), run-line validation, report v2 + markdown, env keys; ci step 3b | Updated `bench_runner.rs`, `bench_report.rs` and `ci.rs` tests pass; `cargo xtask bench startup --runs 2` on Windows prints a v2 row with `requested auto`, `renderer glow` |
| 19-4 | WP5 | Makefile `RENDERER ?= auto` + help; ci.yml second smoke step | `make -n bench` prints `--renderer auto`; YAML parses; SHA pins intact |
| 19-5 | WP6 | M0-backend.md §11 with N1–N6 | Orchestrator review (E5) |
| 19-6 | WP7 | README "Renderer selection": order per target, forced vs auto, `--renderer`/`VA_RENDERER` (settings key from M4), fallback messages, exit 3, WSL X11 + `VA_WAYLAND=1` warning, Windows DX12 pin, `VA_TEST_FAIL_RENDERER` as a test hook | E7 + Orchestrator review |
| 19-7 | Security | SECURITY.md entry for the injection hook (§19.3 footgun analysis), the new winit dependency (no new network surface: X11 TCP was already accepted), the panic-not-caught decision | Security re-audit |

Order: 19-1 before 19-2 (workspace dependency). 19-2 and 19-3 can run in parallel, because the runner
needs only the §19.5 contract. 19-5 runs after 19-2/19-3 are QA-green.

### 19.9 Ratified implementation decisions (§19 audit round 1, 2026-10-16)

Part of §19, with the same precedence. It settles every point where the landed §19 implementation
goes beyond, interprets or diverges from §19.1–§19.8. Where §19.9 and an earlier §19 passage
disagree, §19.9 wins: §19.9.7 lists the §19 passages it replaces, and the §18.8 index now also lists
the body passages superseded by §19 and §19.9. §19 is folded into the body with §17/§18 when M0
closes (CR1-09).

Every item is **ratified** (the spec now says what the code does), **fixed** by a row of the §19.9.6
fix wave, a **limitation** (accepted, recorded in SECURITY.md), or **deferred** to a named milestone.
Ids come from the round-1 reports: developer deviations (WP2-n, WP4-n), conformance (CR19-nn),
Security (S19-n) and QA (QA-n).

#### 19.9.1 Renderer selection and fallback (WP2)

| ID | Decision | The spec now says |
|---|---|---|
| WP2-1 (CR19-01) | Ratified | New §19.1 row **"wgpu after a failed glow attempt"**: that wgpu instance leaves out `wgpu::Backends::GL` (`exclude_wgpu_gl_backend`) unless `WGPU_BACKEND` is effective (S19-4). glutin drops a failed glow context without unbinding it, so the context stays current on the thread, and wgpu-hal 30.0.1 panics on `EGL_BAD_ACCESS` when its GL backend then probes EGL (reproduced under WSLg X11: exit 101 without the rule, fallback to Vulkan with it). The rule only touches a wgpu attempt that follows a glow failure in the same process: forced wgpu and the Windows aarch64 order are unaffected, and on Windows the DX12 pin already leaves GL out. Accepted consequence: a Linux host whose only wgpu backend is GL (no Vulkan driver, not even Mesa's lavapipe) exits 3 after a glow failure instead of retrying the OpenGL driver that has just failed. `WGPU_BACKEND=gl` brings wgpu-GL back at the user's risk (exit 101). *Rejected:* keeping GL, which turns the fallback into a crash on the hosts where glow fails. |
| WP2-2 (CR19-04) | Ratified | §4.4 API. `cli.rs`: `RendererChoice { Auto, Glow, Wgpu }` (`as_str`, `from_name`, `forced`) next to the concrete `Renderer { Glow, Wgpu }`; `compiled_renderers() -> &'static [Renderer]` (membership only, the order is `auto_order`); `resolve(&GuiArgs, &[Renderer])`; `LaunchPlan { renderer: Option<RendererChoice>, bench }`, the command-line level only; `UsageError::RendererNotCompiled(Renderer)`; and the pure selection layer, kept in `cli.rs` because §19.3 puts its tests in `tests/cli.rs`: `ENV_RENDERER`, `ENV_FAIL_RENDERER`, `ENV_WAYLAND`, `auto_order`, `resolve_renderer` → `RendererResolution { plan: RendererPlan { requested, source: ChoiceSource, order }, warnings }`, `parse_injected_failures` → `InjectedFailures { renderers, raw, warnings }`, `window_system_policy` → `WindowSystemPolicy { force_x11, warning }`, `RendererInitError::Injected`, `is_renderer_init_error`, `is_fallback_eligible`, `describe_failure`, `classify_failure` → `AttemptFailure { FallBack { next, warning, reason }, Fatal { error, note } }`. `lib.rs`: `pub fn native_options(Renderer, force_x11: bool, after_glow_failure: bool) -> Result<eframe::NativeOptions, UsageError>`, public so tests check every attempt's options without a window (row 19-8 adds `wgpu_backend_env_is_effective`). `spike.rs`: `SpikeApp<'a>::new(Option<&'a mut BenchRecorder>, Arc<AtomicBool>)`, the flag set at the top of the first `ui`. `bench/recorder.rs`: `BenchRecorder::new(BenchPlan, Instant, SelectionFacts)`, `SelectionFacts { requested, fallback: Option<(Renderer, String)>, injected_failure: Option<String> }`, `record_fallback(Renderer, String)`. `bench/report.rs`: `RUN_SCHEMA = "va.bench.run/2"`, `BenchRun` with the 22 keys of §19.5. |
| WP2-3 (CR19-04) | Ratified | §19.2 "State across attempts": the recorder is **owned by `launch`**, created before the first attempt and lent to each attempt's app (`Option<&'a mut BenchRecorder>`), instead of a shared `Option` slot that the creator takes from. The §19.2 guarantees are unchanged, and they also cover a failure after the creator returned but before the first frame: one recorder and one watchdog, no reset of its limit, the §17.11 D06 disconnect on shutdown. |
| WP2-4 | Ratified | §19.1 WSL row: the X11 hook is installed on **every** attempt. eframe calls it only when it creates its thread-local event loop (the first attempt), so the first attempt still fixes the window system of the process, and the code stays correct if eframe ever builds a new loop. |
| WP2-5 (CR19-02 b, c) | Ratified | `VA_RENDERER` takes exactly `auto`, `glow` or `wgpu`. Anything else, including an empty value, case variants, surrounding spaces and a non-Unicode value (shown lossily, escaped by row 19-8), prints the §19.1 warning and is ignored. The warning is printed **even when `--renderer` overrides the variable**: every level that is set is validated, and only the winning level is checked against the build (an overridden variable never causes exit 2). M4's `render.renderer` follows the same rule. *Rejected:* the §17.11 `VA_PYTHON` reading (empty = unset): an empty renderer variable is more likely a mistake worth one warning line, and the outcome (`auto`) is the same. |
| WP2-6 (CR19-02 a) | Ratified | `VA_TEST_FAIL_RENDERER` is a comma-separated list. Each item is trimmed; blank items are skipped silently; `glow` and `wgpu` are exact and duplicates merge; every other item prints `warning: ignoring unknown VA_TEST_FAIL_RENDERER value '<v>' (expected glow or wgpu)`, once per item, in input order; a non-Unicode value is ignored as a whole with `warning: ignoring VA_TEST_FAIL_RENDERER='<v>' (not valid Unicode; expected glow, wgpu or glow,wgpu)`. The §19.3 startup warning is printed **only when at least one renderer is armed** (it says "simulating"). It quotes the raw value and is printed even when the plan never reaches the armed renderer. `injected_failure` holds the raw (lossy) value **whenever the variable is set**, even empty or with nothing armed, so a bench run with a stray variable stays visible. |
| WP2-7 | Ratified | `VA_WAYLAND` keeps Wayland only when it is exactly `1` (` 1`, `0` and non-Unicode values mean X11). The compositor warning is printed only under WSL, with a plan that includes wgpu. |
| WP2-8 (CR19-03) | Ratified | §19.4 additions. (a) A forced renderer that meets a platform failure, or any error after the first frame, gets the unchanged `cannot create the window or the renderer: <error>`. (b) In `auto`, a non-renderer error on the second attempt also gets that message; a renderer initialization error before the first frame on the last attempt gets `no renderer could be initialized …`. (c) The `tried …` list is the plan order joined by `, then ` (`tried wgpu, then glow` on Windows aarch64, `tried glow` for a single-renderer `auto`). (d) The renderer-initialization set is the union of the two §19.2 sets, whatever renderer was attempted; eframe only returns the attempted renderer's variants, so this cannot be observed. (e) `fallback_reason` is the first line of the description; the warning carries the whole text. (f) An attempt index outside the plan is fatal with the window message, never a panic. |
| WP2-9 | Ratified | `--renderer` help reads `[default: $VA_RENDERER, else auto]`, with one user-facing line per value (clap help text, like the GUI `about` ratified in §17.11 D13). |
| WP2-10 (CR19-05, CR19-07) | Ratified | §19.3 local tests (`tests/fallback.rs`, all ignored with the §19.3 reason): `injected_glow_failure_falls_back_to_wgpu` (not on Windows aarch64) and `injected_wgpu_failure_falls_back_to_glow` (Windows aarch64 only, where `auto` tries wgpu first); `both_failures_exit_3` (its `tried` order follows the target); `forced_renderer_never_falls_back` (through `--renderer` and through `VA_RENDERER`); `auto_uses_glow_first` (not on Windows aarch64); QA's `injected_glow_failure_falls_back_in_the_idle_scenario`, `forced_renderer_ignores_an_injection_of_the_other_renderer`, `unknown_injection_values_change_nothing` and `interactive_fallback_keeps_the_window_open` (the glow-first ones compiled out on Windows aarch64); and `wsl_defaults_to_x11_with_wayland_display_present` (Linux: keeps `WAYLAND_DISPLAY`, forces glow, expects `xlib` by default and with `VA_WAYLAND=0`, `wayland` with `VA_WAYLAND=1`; it closes CR19-07). stderr is matched line by line in order, not for equality, because Mesa prints its own `warning:` lines. QA runs them with `--test-threads=1` on Windows and under WSLg X11 (`env -u WAYLAND_DISPLAY`). |
| WP2-11 | Ratified | Tests changed because §19 changed what they assert: `tests/cli.rs` (`default_renderer_is_wgpu_when_compiled` became `default_renderer_defers_to_auto`; `default_renderer_is_glow_without_wgpu` goes through `resolve_renderer`; the startup plan carries no command-line renderer; `Renderer` in compiled lists and `UsageError`), `tests/bench_report.rs` (22 keys, `/2`, the §19.5 example), `tests/binary.rs` (`missing_display_is_a_window_error` removes the `VA_*` selection variables). Single-renderer builds are compiled and linted (ci steps 3 and 3b), never tested: a test that needs both renderers is gated on `all(feature = "renderer-glow", feature = "renderer-wgpu")`, as QA did for `forced_renderer_platform_failure_keeps_the_window_message`. |
| WP2-12 | Ratified | §19.3 genuine OpenGL failure: on the reference WSLg the libglvnd command does not fail GL (libglvnd falls back to Mesa), which is recorded as §19.3 allows. A second genuine command joins §19.3 and N5: `env -u WAYLAND_DISPLAY MESA_GL_VERSION_OVERRIDE=1.1 MESA_GLES_VERSION_OVERRIDE=1.0 "$CARGO_TARGET_DIR/bench/shipped/visual-assembler" --bench startup`. Observed by WP2 and QA (debug and release): `fallback_reason` `glutin error: [a7] GLXBadFBConfig`, `renderer` `wgpu` on Vulkan, `xlib`, exit 0. |
| WP2-13 | Limitation | A crash inside the graphics driver kills the process before any error is returned, so it cannot fall back. Observed: `LIBGL_ALWAYS_INDIRECT=1` makes glow's GLX path end in SIGSEGV (exit 139) under WSLg; forced wgpu works there. As for a panic (§19.2), the README tells the user to force the other renderer. |
| QA-1 | Ratified | Under WSL with `DISPLAY` unset, the forced X11 path cannot connect, even when `WAYLAND_DISPLAY` is set: a platform failure (exit 3, window message, no fallback). The README says to set `DISPLAY`, or to use `VA_WAYLAND=1` with glow. |

#### 19.9.2 Bench runner and report (WP4)

| ID | Decision | The spec now says |
|---|---|---|
| WP4-1 | Ratified | Key order of a `va.bench.report/2` result: `scenario, renderer_requested, renderer, fallbacks, binary, binary_bytes, warmup, runs, duration_secs, env, backend, adapter, driver, window_system, summary, samples`. The `env` map lists the five §6.2 variables, then `VA_RENDERER`, `VA_TEST_FAIL_RENDERER`, `VA_WAYLAND`. |
| WP4-2 | Ratified | A result with no samples has `renderer: "unknown"`, like `backend` and `window_system` (unreachable, since `--runs` ≥ 1). |
| WP4-3 | Ratified | Each run passes `--renderer <r> --bench <scenario> [--duration-secs N]`, in that order. |
| WP4-4 | Ratified | Under `auto`, validation is the scenario rule plus `renderer_requested == "auto"`. No check relates `renderer` and `fallback_from`: that invariant belongs to the GUI and is tested there. |
| WP4-5 | Ratified | xtask does not warn when `VA_RENDERER` is set: the `--renderer` it always passes overrides the variable, and the report's `env` map records it. |
| WP4-6 | Ratified | Copies left under `<target>/bench/wgpu/` and `<target>/bench/glow/` by the pre-§19 runner are not cleaned up. xtask never reads them, and `make clean` removes them. |

#### 19.9.3 Automation and CI (WP5)

| ID | Decision | The spec now says |
|---|---|---|
| CR19-13 | Ratified | The scope extension of row 19-4 is accepted. The AGENCEMENT check (check 4 of `check_project_docs.py`) inspects tree entries only (`iter_entries` and `unannotated_entries` in `gen_agencement.py`). It warns `[layout] some entries are still unannotated (<n>): <paths>`, with at most 10 paths and then `and <k> more`, for each entry whose annotation is empty or still holds `<!-- annotate -->`. The header line never counts, and `--strict` exits 1 only when such entries exist. The generator footer may count a `<!-- annotate --> text` entry differently; the check is authoritative. Regression tests: row 19-10. |
| CR19-06 | Fixed (row 19-9 a) | The second Windows smoke step injects a failure into the **first renderer of the runner's `auto` order**: `glow` on `windows-latest` (x64), `wgpu` on `windows-11-arm`. Its log then documents a fallback on both images (glow to wgpu, wgpu to glow). On ARM64 the glow fallback may itself fail when the hosted image has no usable OpenGL; the §19.4 lines are then the record. Both outcomes are informational. |

#### 19.9.4 Security round 1

| ID | Decision | The spec now says |
|---|---|---|
| S19-1 | Limitation (M0), deferred to M4 | On the Windows release build (GUI subsystem, §4.2), stderr exists only when redirected, so the §19.3 startup warning, the selection warnings and every §19.4 message are invisible when the app starts from Explorer, the Start menu or a PowerShell prompt. Accepted for M0: nothing is distributed before M8, whoever sets the environment already controls the process, and bench JSON records `renderer`, `fallback_from`, `fallback_reason` and `injected_failure`, so no measurement is misattributed. The last sentence of the §19.3 footgun analysis now reads: "The loud warning keeps it from going unnoticed on a console; on the Windows release build, bench JSON and, from M4, the status bar do." The README says these messages appear only on a console (Git Bash, a pipe, the debug build). **M4:** the §19.2 status-bar note shows a fallback **and** an armed `VA_TEST_FAIL_RENDERER` hook, and M4 decides how a startup failure that exits 3 before any window reaches a user who has no console. *Rejected:* a ` (test hook active)` window-title suffix in M0, which would widen the spike.rs i18n exception for a test-only path. |
| S19-2 | Fixed (row 19-8 a) | Every environment value echoed in a warning goes through `str::escape_debug` of its lossy text: the invalid `VA_RENDERER` warning, both `VA_TEST_FAIL_RENDERER` warnings and the startup warning. A stray CR, newline or escape sequence becomes visible (`'glow\r'`) instead of breaking the line or driving the terminal. `injected_failure` keeps the unescaped lossy value, which serde_json escapes. |
| S19-3 | Fixed (rows 19-9 b, 19-10, 19-11) | cargo-deny 0.20.2 `unused = "deny"` only proves that a pin's name, source and version exist in the resolved graph. A pin that no member declares passes when a dependency already pulls that version, and an internal path pin (M1+) would match its own member package. "Only crates declared by at least one member" is now enforced by a reverse check in `scripts/check_project_docs.py` check 3b. The row 19-1 acceptance check proves only that the graph is acceptable; that winit is used was proven by row 19-2's Linux target dependency and is now kept by check 3b. The root `Cargo.toml` comment, DEPENDANCE.md and CLAUDE.md stop attributing the rule to cargo-deny alone. |
| S19-4 | Fixed (row 19-8 b) | `WGPU_BACKEND` is **effective** only when wgpu itself takes a non-empty backend set from it: the value is valid Unicode (wgpu-types 30.0.1 reads it with `std::env::var(..).ok()`) and `Backends::from_comma_list` of it is not empty. The Windows DX12 pin and the WP2-1 GL exclusion step aside only for an effective value, so an empty, blank, unrecognized or non-Unicode value no longer drops the pin or brings GL back after a glow failure. "Unless `WGPU_BACKEND` is set" in §17.13 and §19.1 means "unless it is effective", as architecture §9.8 now says. wgpu still applies its own reading: on Linux an empty or unrecognized value leaves eframe's instance with no backend, so forced wgpu exits 3 (accepted: the environment asked wgpu for no backend). |
| S19-5 | Limitation | The real in-process retry needs a window, and the hosted runners only run informational GUI steps (architecture §9.1: never evidence). Compensating controls: (1) the CR19-06 step exercises a fallback on both Windows images, still `continue-on-error`; (2) **binding QA rule:** run `cargo test -p visual-assembler --test fallback -- --ignored --test-threads=1` on Windows and under WSLg X11 whenever `Cargo.lock` changes the version of eframe, egui, egui-wgpu, egui_glow, wgpu, glutin or winit, and before each milestone closes; (3) the headless tests cover the classification, the messages and every attempt's options. *Rejected:* an assertion inside the smoke step, because a `continue-on-error` step cannot gate. Revisit when branch protection makes CI required (§17.17 item 3). |

#### 19.9.5 Items without a spec change

- CR19-08, CR19-09, CR19-11 and CR19-12 are documentation and report deliverables of rows 19-5,
  19-6 and the docs owner. Their content now includes the §19.9 items named in row "WP7" below.
- CR19-10 and CR19-14 were refuted by verification round 1. The hosted-runner checks (the four
  `check (…)` jobs, Windows ARM64, both smoke steps) happen after the orchestrator pushes.
- WP1: the row 19-1 manifest value is exact; its cargo-deny observation is S19-3.

#### 19.9.6 Fix-wave table (continues §19.8)

| # | Owner | Change | Acceptance check |
|---|---|---|---|
| 19-8 | WP2 | (a) S19-2: in `cli.rs`, `resolve_renderer` (invalid `VA_RENDERER`), `parse_injected_failures` (non-Unicode and unknown-item warnings) and `InjectedFailures::startup_warning` print the echoed value as `escape_debug()` of the lossy `&str` (of the trimmed item, for unknown items). No other text changes; `InjectedFailures::raw` stays unescaped. (b) S19-4: `lib.rs` gains `#[cfg(feature = "renderer-wgpu")] pub fn wgpu_backend_env_is_effective(raw: Option<&std::ffi::OsStr>) -> bool`, true exactly when `raw` is valid Unicode and `eframe::wgpu::Backends::from_comma_list` of it is not empty. `pin_windows_wgpu_backend` and `exclude_wgpu_gl_backend` call it with `std::env::var_os("WGPU_BACKEND").as_deref()` instead of testing `var_os(..).is_none()`; their doc comments say "effective". | `tests/cli.rs`: `VA_RENDERER` = `"glow\r\n\u{1b}[2J"` gives exactly one warning, with no CR, LF or ESC character in it and the value spelled with backslash escapes; `VA_TEST_FAIL_RENDERER` = `"glow,bad\nerror: forged"` gives one unknown-item warning with the item escaped, a startup warning with no control character, and `raw` equal to the input; the existing exact-text tests pass unchanged. `tests/renderer_backend.rs`: the helper is false for `None`, `""`, `"  "`, `","`, `"bogus"` and a non-Unicode value (Unix `OsStr::from_bytes(b"\xff")`, Windows `OsString::from_wide(&[0xD800])`), and true for `"vulkan"`, `"gl"`, `"DX12"` and `" vk , bogus"`. The env-dependent branches of `dx12_pin_applies_to_every_wgpu_attempt` and `wgpu_gl_backend_is_left_out_after_a_glow_failure` use the helper: unset keeps today's assertions; effective gives eframe's choice; set but not effective gives DX12 on Windows and no GL after a glow failure. Clippy `-D warnings` is clean in the dual, glow-only and wgpu-only builds. |
| 19-9 | WP5 | (a) CR19-06: the second Windows smoke step of `ci.yml` becomes exactly the block below; nothing else in `ci.yml` changes. (b) S19-3: check 3b of `scripts/check_project_docs.py` also reports, once per key and in sorted order, every key of the root `[workspace.dependencies]` (path entries included) that no member declares with `workspace = true` in any dependency table (`dependencies`, `dev-dependencies`, `build-dependencies` and their `target.<cfg>` variants, plus the root package's own tables when the root manifest has `[package]`): `[deps] Cargo.toml: workspace dependency '<name>' is not declared by any member (rule R6: remove the pin or declare it with workspace = true)`. Keys compare exactly. The reverse check is skipped when a member manifest or one of its tables is unusable, which is already reported. The module docstring (item 3b) says so. | (a) YAML parses; every `uses:` line is unchanged and pinned to a 40-hex SHA; no `${{` appears inside a `run:` value. After the push, the `windows-latest` log shows `"fallback_from":"glow"`, and the `windows-11-arm` log shows `"fallback_from":"wgpu"` or the `no renderer could be initialized (tried wgpu, then glow)` lines. (b) The real repository reports no `[deps]` problem; row 19-10 passes. |
| 19-10 | WP4 | `xtask/tests/docs.rs`, with the existing manifest and golden-project helpers: `unused_workspace_dependency_is_reported` (a `serde` pin that no member declares gives exactly the 19-9(b) line for `serde`); `workspace_dependency_used_only_in_a_target_table_counts` (the member declares it under `[target.'cfg(target_os = "linux")'.dependencies]`: no `[deps]` line); `unused_path_workspace_dependency_is_reported` (`va-core = { path = "crates/va-core" }` pinned and declared by nobody); `unannotated_tree_entry_is_reported_with_its_path` (a fully annotated generated tree gives no `still unannotated` line and `--strict` exits 0; one entry set back to `# <!-- annotate -->` gives the warning with its path, and `--strict` exits 1). | `cargo test -p xtask --test docs` passes on Windows and in WSL |
| 19-11 | WP1 | Root `Cargo.toml` line 28 becomes exactly `# Exact pins, each declared by at least one member (rule R6, enforced by check_project_docs.py check 3b).` It stays one line, so the SECURITY.md line references stay valid. | Only that line differs; `Cargo.lock` is unchanged; `cargo metadata --format-version 1 --no-deps` exits 0 |
| — | WP7 | READMEs, CLAUDE.md, DEPENDANCE.md and IMPORT.md document WP2-1, WP2-5, WP2-6, WP2-13, QA-1, S19-1, S19-3, S19-4, S19-5 (QA rule), CR19-06 and CR19-13, together with row 19-6 and CR19-08/09/12 | E7 + Orchestrator review |
| — | WP6 | M0-backend.md N5 records both genuine-failure commands with their outcomes (WP2-12) and the driver-crash case (WP2-13) | Orchestrator review (E5) |
| — | Security | SECURITY.md: WP2-1 ratified; S19-1 and S19-5 accepted with their controls; S19-2, S19-3 and S19-4 closed once rows 19-8 and 19-9 land; the CR19-06 step; re-pointed line ranges | Security re-audit |

The `ci.yml` block of row 19-9(a):

```yaml
      # Section 19.9 (CR19-06): fail the first renderer of this runner's auto order on purpose, so
      # the log documents a fallback on both images (x64: glow then wgpu; ARM64: wgpu then glow).
      - name: GUI renderer fallback smoke (informational, never a support claim)
        if: runner.os == 'Windows'
        continue-on-error: true
        timeout-minutes: 3
        shell: bash
        env:
          VA_TEST_FAIL_RENDERER: ${{ runner.arch == 'ARM64' && 'wgpu' || 'glow' }}
        run: ./target/release/visual-assembler.exe --bench startup
```

The expression sits in the step's `env:`, never inside `run:`, and `runner.arch` is set by the
runner, not by a pull request.

Order: rows 19-8, 19-9 and 19-11 touch disjoint files and run in parallel; 19-10 runs after 19-9(b).
QA then re-runs on Windows and WSL: `make test`, `make lint`, `make docs`, `cargo xtask ci`
(`--no-fmt` in WSL), `make security` (Windows), and the ignored fallback tests on Windows and WSLg
X11, because row 19-8 touches `native_options`. No row changes the cost of the startup path: 19-8
only reformats warnings and parses one variable that eframe already reads.

#### 19.9.7 Superseded §19 passages

| §19 passage | Read instead |
|---|---|
| §19.1 WSL row: "The hook must be set on the **first** attempt" | WP2-4 |
| §19.1 Windows wgpu row: "unless `WGPU_BACKEND` is set" | S19-4 ("effective"), and the new WP2-1 row |
| §19.2 State across attempts: "a shared `Option` slot that the creator takes from" | WP2-3 |
| §19.2 Reporting: "M4 adds a status-bar note" | S19-1 |
| §19.3 Hook: startup warning, unknown values, last sentence of the footgun analysis | WP2-6, S19-1 |
| §19.3 local tests list | WP2-10 |
| §19.3 genuine OpenGL failure | WP2-12 |
| §19.3 CI GUI smoke (command and purpose); §19.6 `ci.yml` bullet | CR19-06, row 19-9(a) |
| §19.4 table (the cases it leaves open) | WP2-8 |
| §19.5 report result key order, runner arguments and validation | WP4-1…WP4-6 |
| §19.8 row 19-1 acceptance: "(winit is now used, so `unused` is satisfied)" | S19-3 |
