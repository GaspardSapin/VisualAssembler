# CLAUDE.md

## The project in 5 lines
VisualAssembler is a native, offline Rust IDE for students learning assembly; V1 is the Motorola 68000
end to end (edit, assemble, run, debug). GUI `visual-assembler` (egui/eframe), headless CLI `va`,
task runner `cargo xtask`. CPU-agnostic crates in `crates/`, one crate per CPU in `arch/`.
Current milestone: **M1 core contracts, 68000 ISA tables, lexer, `va lex`/`va isa`**: done (audited,
QA green, measured, pushed, CI green in run 40027188396). M0 (workspace, tooling, CI, measured window)
is done. Still skeletons: `va-editor`, `va-lang`, `va-debug`, `va-ui`; the GUI is the M0 window.
Specs: `docs/specs/architecture.md`, then the milestone specs.
- `docs/specs/m1-core-isa-lexer.md`: **§12 (ratified implementation, fix-now rows R12-1…R12-4, QA
  round 1 in §12.8) wins over the body**.
- `docs/specs/m0-foundations.md`: precedence **§20 > §19 > §18 > §17 > body (§0–§16)**. §18.8
  indexes the body passages that §17/§18/§19/§20 supersede. §19 is the user's renderer decision (it
  replaces the rule of §12.3/§12.4/§17.13 and outcome W); §19.9 ratifies its implementation, rows
  19-8…19-11. §20 pins the CI runner labels (no `-latest`).

## Stack and versions
- Rust **1.99.0** pinned (`rust-toolchain.toml`, with rustfmt and clippy), edition 2024, resolver 3,
  `rust-version = "1.99"`. Windows host: MSVC; Linux: WSL Ubuntu 26.04.
- GUI: eframe/egui 0.36.2 (default features off) with two renderers, glow 0.17.0 and wgpu 30.0.1.
  Both compiled into the shipped binary (`default = ["renderer-glow", "renderer-wgpu"]`), glow first
  (m0 §19). winit 0.30.13 is a direct, Linux-only dependency (X11 hook under WSL).
- CLI and tooling: clap 4.6.7, serde 1.0.229, serde_json 1.0.151, anyhow 1.0.104 (xtask only).
- Localization (M1): fluent-bundle 0.16.0 and unic-langid 0.9.6 in `va-workspace`; fluent-bundle is
  also a dev-dependency of `va-arch-m68k` (locale tests).
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
| `make ci` | `cargo xtask ci`, 9 steps: step 0 build config, fmt, clippy ×4 (all features, glow only, wgpu only, `va --no-default-features`), layering + source scan, tests (workspace, then `va --no-default-features`; m1 §12.7 R12-4) |
| `make bench SCENARIO=… RENDERER=… RUNS=…` | `cargo xtask bench` on the shipped binary, `RENDERER=auto\|glow\|wgpu\|all` (default `auto`; needs a display) |
| `make bench-lexer` | release lexer bench on 100,000 lines, one `va.bench.lexer/1` line; budget median ≤ 50 ms (measured 7.157 ms, `docs/perf/2026-10-23-m1-lexer.md`) |
| `make clean` | `cargo clean` |

WSL: `wsl.exe -d Ubuntu -e bash -lc 'cd "/mnt/c/Users/sapin/Documents/Code Info/VisualAssembler" && export CARGO_TARGET_DIR=$HOME/.cache/visualassembler/target && <cmd>' 2>&1 | tr -d '\0'`.
Live layering check alone: `cargo test -p xtask --test layering -- --ignored live_workspace --nocapture`.

## Conventions
- **English only** in the repository: code, comments, docs, file names, commit messages. French
  user-facing text lives only in `fr/*.ftl` Fluent files (ids in English kebab-case), written in our
  own words, never copied from the Motorola manuals. Diagnostics and ISA docs go through Fluent since
  M1, UI strings from M4 (the two English literals of `apps/visual-assembler/src/spike.rs` are a
  documented M0 exception). `va`'s own CLI messages stay English.
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
  stderr prefixed `error: `; global `--json` prints one document with `"schema": "va.<cmd>/<major>"`,
  and a failure prints `va.error/1` on stdout too. M1 commands: `va info`, `va lex <FILE|->`
  (`va.lex/1`; exit 1 on any error diagnostic, blocking or not), `va isa <MNEMONIC[.SIZE]>` /
  `va isa --list` (`va.isa/1`, `va.isa.list/1`; unknown spelling or size exits 1). Global
  `--lang en|fr` (`auto` is a usage error) > `VA_LANG` (`en|fr|auto`, invalid = warning) >
  `LC_ALL`/`LC_MESSAGES`/`LANG`. Input: UTF-8 only, ≤ 16 MiB per input, ≤ 64 KiB per line (longer =
  one `lex-line-too-long` token); non-UTF-8, too large or unreadable input exits 3. The GUI's codes
  differ (m0 §17.10, §19.4): 1 runtime I/O, 2 usage, 3 window or renderer, 4 bench watchdog.
- **Commits**: `<type>(<scope>): <what changed since the previous step>`, one per completed step,
  **never co-signed by an AI tool** (no `Co-Authored-By`, no "Generated with", no robot emoji). The
  `commit-msg` hook and the CI `docs` job enforce it. Never change git config.
- **Docs**: a README in every folder (role, contents, entry points, how to test), parent cascade up
  to the root, `make docs` exit 0 before closing a task. Every command written in a README has been run.
  Dot-folders and folders without a file of their own are exempt (`check_project_docs.py` skips
  them): `.cargo/`, `arch/m68k/fixtures/` and `arch/m68k/locales/` have no README, while
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
- CI: `cargo xtask ci` is the single source of the check list; 4 native runners with pinned labels
  `windows-2025`, `windows-11-arm`, `ubuntu-26.04`, `ubuntu-24.04-arm` (never `-latest`, m0 §20);
  actions pinned by SHA (m0 §8).
- `.cargo/config.toml` must equal `CANONICAL_CARGO_CONFIG` in `xtask/src/ci.rs` (m0 §18.4).
- CPU traits use dynamic dispatch (`dyn Architecture`), diagnostics are kebab-case codes rendered by
  Fluent (architecture §3, §5). 68000 emulation will use the `m68k` crate behind `Emulator` (M3).
- `va-core` contracts are frozen since M1 (m1 §3): add items, never remove or rename a public one
  without the Architect. `va-core` has no dependency and no prose (no `Display` on its errors).
- 68000 ISA tables are hand-written static Rust data checked by tests (m1 §4.2), not a parsed data
  file or `build.rs`. Fixtures live in `arch/<cpu>/fixtures/<area>/` (depth 4), LF-only.
- **Strict A68k dialect, user decision of 2026-10-17** (m1 §5.7): the operand field ends at the first
  unquoted space, the rest is a comment (`dc.b 1, 2, 3` = one byte, as in a68k). Operand-like text
  there becomes a `SuspectComment` with the **non-blocking** error `lex-operands-in-comment` and,
  for rules R1/R2, a quick fix; real comments stay `Comment`. `lab::` is accepted (§12.1 R12-2). A
  permissive EASy68K/vasm dialect is a later per-project profile (architecture §10.3).
- Localizer: `fluent_bundle::concurrent` bundles, isolation marks off, fallback to English per
  pattern (m1 §6.2, §12.2). `auto` reads only `LC_ALL`, `LC_MESSAGES`, `LANG` until M4.
- `.cargo/deny.toml` ignores exactly RUSTSEC-2026-0370/0371/0372 (zvariant, zbus_names,
  zvariant_derive "unmaintained", Linux AT-SPI path of eframe 0.36.2; `va` does not contain them).
  Accepted risk (m1 §12.8 F-2): revisit at the next eframe/egui bump, at the latest in M8; never
  extend the list to vulnerability or unsound advisories.
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
- A CI runner label is part of a required check name (`check (<label>)`): changing one in `ci.yml`
  blocks every pull request until branch protection lists the new name (m0 §20.3).
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
- **Any new `va` text output that carries outside text** (source text, file names, arguments,
  environment values) must go through `report::terminal_safe`, or `report::terminal_safe_json` for a
  JSON string printed on a text line (Security M1-S1/S3/S4/S8/S9). `--json` documents stay raw.
- **Cycle numbers live only in the timing tables** (`arch/m68k/src/isa/timing.rs`, `table_*.rs`);
  the `.ftl` docs never repeat a count the tables give (m1 §12.1 R12-3).
- **Timing values and `VALID_OPWORDS = 45_816` are provisional** until the M3 cross-check against the
  `m68k` crate and SingleStepTests (m1 §12.3); never cite them as verified.
- `va lex` reads have no timeout and a Windows UNC path triggers SMB (Security M1-S6, open until the
  M7 path sandbox): tests and scripts pass local files only.

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
