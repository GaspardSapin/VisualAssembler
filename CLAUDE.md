# CLAUDE.md

## The project in 5 lines
VisualAssembler is a native, offline Rust IDE for students learning assembly; V1 is the Motorola 68000
end to end (edit, assemble, run, debug). GUI `visual-assembler` (egui/eframe), headless CLI `va`,
task runner `cargo xtask`. CPU-agnostic crates in `crates/`, one crate per CPU in `arch/`.
Current milestone: **M0 foundations**, in its closing stretch: workspace, tooling, CI, measured empty
window; every library crate is still a skeleton. Specs: `docs/specs/architecture.md`, then
`docs/specs/m0-foundations.md`, with precedence **§19 > §18 > §17 > body (§0–§16)**. §18.8 indexes
the body passages that §17/§18/§19 supersede. §19 is the user's renderer decision: it replaces the
decision rule of §12.3/§12.4/§17.13 and outcome W, and §19.8 lists its fix wave. §19.9 ratifies the
landed implementation (same precedence as §19) and adds fix-wave rows 19-8…19-11.

## Stack and versions
- Rust **1.99.0** pinned (`rust-toolchain.toml`, with rustfmt and clippy), edition 2024, resolver 3,
  `rust-version = "1.99"`. Windows host: MSVC; Linux: WSL Ubuntu 26.04.
- GUI: eframe/egui 0.36.2 (default features off) with two renderers, glow 0.17.0 and wgpu 30.0.1.
  Both compiled into the shipped binary (`default = ["renderer-glow", "renderer-wgpu"]`), glow first
  (m0 §19). winit 0.30.13 is a direct, Linux-only dependency (X11 hook under WSL).
- CLI and tooling: clap 4.6.7, serde 1.0.229, serde_json 1.0.151, anyhow 1.0.104 (xtask only).
- Tools: GNU make 4.4.1 (Git Bash on Windows), Python 3.11+ (`tomllib`), cargo-deny 0.20.2, git,
  GitHub Actions. `gh` lives at `C:/Program Files/GitHub CLI/gh.exe` (not on `PATH` in agent shells).
- Licence: `MIT OR Apache-2.0`; every crate is `publish = false`.

## Commands
Everything goes through the Makefile (`make -s help`); logic lives in `cargo xtask`.

| Target | Effect |
|--------|--------|
| `make install` | rustup toolchain, cargo-deny 0.20.2, `cargo fetch` |
| `make hooks` | install the `commit-msg` hook (`cargo xtask hooks`) |
| `make build` / `make run` / `make dev` | release build of both binaries / run the GUI in release / in debug |
| `make test` | `cargo test --workspace --all-features` |
| `make lint` | fmt check + clippy `-D warnings` |
| `make format` | `cargo fmt --all` |
| `make docs` | regenerate `AGENCEMENT.md` + `scripts/check_project_docs.py` (must exit 0) |
| `make security` | cargo-deny, explicit `--locked --manifest-path … --config .cargo/deny.toml` form (Windows only locally) |
| `make ci` | `cargo xtask ci`: step 0 build config, fmt, clippy ×3 (all features, glow only, wgpu only), layering + source scan, tests |
| `make bench SCENARIO=… RENDERER=… RUNS=…` | `cargo xtask bench` on the shipped binary, `RENDERER=auto\|glow\|wgpu\|all` (default `auto`; needs a display) |
| `make clean` | `cargo clean` |

WSL: `wsl.exe -d Ubuntu -e bash -lc 'cd "/mnt/c/Users/sapin/Documents/Code Info/VisualAssembler" && export CARGO_TARGET_DIR=$HOME/.cache/visualassembler/target && <cmd>' 2>&1 | tr -d '\0'`.
Live layering check alone: `cargo test -p xtask --test layering -- --ignored live_workspace --nocapture`.

## Conventions
- **English only** in the repository: code, comments, docs, file names, commit messages. UI strings go
  through Fluent from M4 (the two English literals of `apps/visual-assembler/src/spike.rs` are a
  documented M0 exception).
- **Tests live in each crate's `tests/`**, mirroring `src/` (`src/bench/report.rs` → `tests/bench_report.rs`).
  Never an inline `#[cfg(test)]` module in a production file. Every test file starts with a `//!` line
  and `#![allow(clippy::unwrap_used)]`; every crate root starts with `//!`. Test temp folders:
  `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` created with `fs::create_dir`, never `std::env::temp_dir`.
- **Dependencies (rule R6)**: one exact `=` pin per crate in the root `[workspace.dependencies]`;
  members write `name = { workspace = true }`. Only crates declared by at least one member:
  `scripts/check_project_docs.py` check 3b enforces it (m0 §19.9 S19-3). cargo-deny `unused = "deny"`
  only proves the pinned version is in the resolved graph.
  A new dependency goes back to the Architect, then gets a `DEPENDANCE.md` line and an `IMPORT.md` entry.
- **Lints (rule R8)**: every member has exactly `[lints] workspace = true` and `license.workspace = true`.
  No `rustfmt.toml`/`clippy.toml`. CI runs clippy with `-D warnings`, so every `warn` blocks.
- **Layering (architecture §2, R1–R8)**: every library builds on `va-core`; `crates/*` never depend on `arch/*`;
  GUI crates only in `visual-assembler` (plus exactly `egui`, `egui_kittest` in `va-ui`); `va` never
  uses `va-ui`; `xtask` uses no workspace crate; apps and xtask are leaves. A new package needs a row
  in the layering table of `xtask/src/layering.rs`, or `cargo xtask ci` fails.
- **Unsafe**: `unsafe_code = "deny"` workspace-wide. Allowed only in `UNSAFE_ALLOWLIST`
  (`apps/visual-assembler/src/bench/{process_windows,process_linux,platform}.rs`), every block with a
  `// SAFETY:` comment. Network APIs are forbidden in every member (`NET_ALLOWLIST` is empty).
- **Tripwire threat model (m0-foundations.md §18.1, binding)**: the source scan and `ci` step 0 catch
  *accidents*. A bypass is blocking only if it is a realistic accidental pattern that compiles and
  nothing catches. Deliberate obfuscation (renamed macros, raw identifiers, out-of-tree target paths,
  crate aliasing, config `include`, …) is a `SECURITY.md` limitation, never reopened.
- **CLI contract** (`va`): exit codes 0 / 1 problems found / 2 usage / 3 I/O / 101 panic; errors on
  stderr prefixed `error: `; global `--json` prints one document with `"schema": "va.<cmd>/<major>"`.
  The GUI's codes differ (m0 §17.10, §19.4): 1 runtime I/O, 2 usage, 3 window or renderer, 4 bench
  watchdog.
- **Commits**: `<type>(<scope>): <what changed since the previous step>`, one per completed step,
  **never co-signed by an AI tool** (no `Co-Authored-By`, no "Generated with", no robot emoji). The
  `commit-msg` hook and the CI `docs` job enforce it. Never change git config.
- **Docs**: a README in every folder (role, contents, entry points, how to test), parent cascade up
  to the root, `make docs` exit 0 before closing a task. Every command written in a README has been run.
  Dot-folders are exempt (`check_project_docs.py` skips them): `.cargo/` has no README, while
  `.github/workflows/` has one. Behaviour that is decided but not implemented is marked as such.

## Architecture decisions
- egui/eframe for the GUI, reactive mode (no continuous repaint) — light and testable headless with
  `egui_kittest` (architecture §1, §8).
- Renderer: **user decision of 2026-10-14, final** (m0 §19, architecture §9.8). Both renderers ship.
  `auto` = glow then wgpu, except Windows aarch64 = wgpu then glow. Fallback at most once, only for a
  renderer initialization error before the first frame (never for platform errors, later errors,
  panics or a forced renderer). A forced renderer that fails exits 3. Precedence `--renderer` >
  `VA_RENDERER` > `settings.toml` `[render] renderer` (M4) > `auto`. Under WSL, X11 for every
  renderer unless `VA_WAYLAND=1`. wgpu on Windows stays pinned to DX12 (override below).
  Test hook `VA_TEST_FAIL_RENDERER` is compiled into every build. *Rejected:* wgpu by default, the
  earlier measured rule (`docs/perf/M0-backend.md` §6). Implemented (rows 19-1…19-4, 19-8…19-11).
  After a glow failure, the wgpu fallback leaves out wgpu's GL backend. `WGPU_BACKEND` lifts the DX12
  pin and that exclusion only when it names a backend wgpu recognizes (m0 §19.9 WP2-1, S19-4). The
  test hook warns only when a renderer is armed (§19.9 WP2-6).
- GUI app = library + thin binary, so CLI parsing, UI and bench are testable without a window (m0 §4).
- Bench: the gating startup number is the outer `spawn_to_first_frame_ms` (m0 §6.1, §15). The runner
  measures the shipped dual-renderer binary with `--renderer auto|glow|wgpu|all` (default `auto`;
  m0 §19.5), schemas `va.bench.run/2` and `va.bench.report/2`.
- Static MSVC CRT (`+crt-static` in `.cargo/config.toml`), so no VC++ redistributable (architecture §9.2).
- Offline product, no telemetry: cargo-deny bans network/TLS/telemetry crates and `mio/net`; source
  scan for network APIs (architecture §9.3).
- Makefile is a thin shell-free wrapper; logic that needs a shell becomes an xtask command (m0 §7).
- CI: `cargo xtask ci` is the single source of the check list; 4 native runners; actions pinned by SHA (m0 §8).
- `.cargo/config.toml` must equal `CANONICAL_CARGO_CONFIG` in `xtask/src/ci.rs` (m0 §18.4).
- CPU traits use dynamic dispatch (`dyn Architecture`), diagnostics are kebab-case codes rendered by
  Fluent (architecture §3, §5). 68000 emulation will use the `m68k` crate behind `Emulator` (M3).
- Licence `MIT OR Apache-2.0`, `publish = false` (architecture §9.7).

## Known pitfalls
- `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `CARGO_BUILD_RUSTFLAGS`, `CARGO_TARGET_*_RUSTFLAGS` set **even
  to an empty value** replace `.cargo/config.toml` and drop `+crt-static`; `cargo xtask ci` step 0 fails.
- Any change to `.cargo/config.toml` must update `CANONICAL_CARGO_CONFIG` in the same commit.
- WSL: always `export CARGO_TARGET_DIR=$HOME/.cache/visualassembler/target` (ext4). Never share the
  Windows `target/` with WSL.
- **WSLg: wgpu on Wayland crashes the compositor** (weston 9.0.0, WSLg 1.0.73.2). The app defaults
  to X11 under WSL; never run wgpu with `VA_WAYLAND=1`. `wsl --shutdown` only as a last resort, after
  the compositor has not recovered within 180 s (m0 §17.13); never reboot; no `sudo`/`apt` (give the
  user the command instead).
- Windows release `visual-assembler.exe` uses the GUI subsystem: it prints nothing in a console and
  PowerShell does not wait for it. Use Git Bash, a pipe, the debug build, or `va`. `VA_*` warnings
  and §19.4 messages are visible only with a redirected stderr (§19.9 S19-1).
- A failed glow attempt leaves its GL context current; `WGPU_BACKEND=gl` plus a glow failure panics
  (exit 101) under WSLg (§19.9 WP2-1).
- Whenever `Cargo.lock` changes eframe, egui, egui-wgpu, egui_glow, wgpu, glutin or winit, QA runs
  `cargo test -p visual-assembler --test fallback -- --ignored --test-threads=1` on Windows and under
  WSLg X11 (`env -u WAYLAND_DISPLAY`) (§19.9 S19-5).
- On Windows, `cargo xtask ci` renames the running `xtask.exe` to `xtask.exe.old` before the test
  step; two concurrent runs on one target folder fail.
- The live layering test must be run **by name** (`live_workspace`): two other ignored tests record
  accepted bypasses and fail by design.
- The `xtask` alias has no `--locked`: `cargo xtask …` can silently rewrite a stale `Cargo.lock`.
  After a manifest change, commit the updated lock; `make security` (`--locked`) catches a mismatch.
- `make help` ends with "Nothing to be done for 'help'"; `make -s help` is the clean form.
- `python` vs `python3`: xtask uses `$VA_PYTHON`, else `python` on Windows, else `python3`.
- Bench numbers depend on power: measure on mains in "Best performance" (battery adds about 33 %).
- dumpbin from Git Bash needs `MSYS_NO_PATHCONV=1` for `/nologo`-style flags.
- The root holds 15 files, the check script's limit: a new root file must displace one. Folders: about
  12 files (15 tolerated), depth ≤ 4.
- A new crate folder must be added to `[workspace].members` (else `check_project_docs.py` reports it)
  and to the layering table. No `[patch]`/`[replace]` table at the root.
- Before the first commit, `check_project_docs.py` prints a `note:` (commits not checkable) and bench
  reports are named `…-nogit.json`.
- `make docs` regenerates `AGENCEMENT.md`; a new file or folder appears there as `<!-- annotate -->`.
  Replace it with a one-line note (no `#` inside it) before closing the task.

## Sensitive areas
- `C:\68000` — **read-only**, never modify; never hardcode its path; never redistribute `a68k.exe` or
  `d68k.exe` (no licence). Their paths are a user setting.
- `docs/specs/` — Architect only. `SECURITY.md` — Security agent only. `docs/perf/` — Performance agent.
- Normative files (change only through a spec revision): `Cargo.toml`, `rust-toolchain.toml`,
  `.cargo/config.toml` (+ `CANONICAL_CARGO_CONFIG`), `.cargo/deny.toml`, `.gitattributes`, `Makefile`,
  `.github/workflows/ci.yml`, `scripts/git-hooks/commit-msg`, `LICENSE-MIT`, `LICENSE-APACHE`.
- `xtask/src/layering.rs` allow-lists (`UNSAFE_ALLOWLIST`, `NET_ALLOWLIST`, `INCLUDE_ALLOWLIST`) —
  Security review and a `SECURITY.md` record for every entry.
- Git: the orchestrator alone commits and pushes, to the public repository
  `GaspardSapin/VisualAssembler`. `main` is protected (the 7 CI checks are required; admins are not
  included).
