# VisualAssembler

A native, offline IDE for students learning assembly language, written in Rust. Version 1 targets the
Motorola 68000 end to end: edit, assemble, run and debug, in a light window that starts fast and stays
idle at about 0 % CPU. Everything is also reachable from a terminal (the `va` CLI, `make`,
`cargo xtask`). CPU support sits behind traits, so later CPUs are new `arch/<cpu>` crates.

## Status: milestone M0 (foundations)

M0 builds the workspace, the tooling, the CI and a measurable empty window. **There is no editor,
assembler or emulator yet.**

| Piece | State at the end of M0 |
|-------|------------------------|
| `visual-assembler` (GUI) | Opens one window with a static text, in egui's reactive mode. Measures its own startup and idle cost (`--bench startup\|idle`). |
| `va` (CLI) | One subcommand, `info` (`--json` supported) |
| `cargo xtask` | `bench`, `ci`, `docs`, `hooks` |
| `crates/*`, `arch/m68k` | Skeletons: a manifest and a documented, empty `lib.rs`. First code in M1. |
| CI (`.github/workflows/ci.yml`) | Written for 4 native runners. **Not run yet**: the GitHub repository is created and pushed after M0 is committed. |
| Renderer | **Decided by the user on 2026-10-14, not implemented yet**: glow by default, with an automatic fallback to wgpu (wgpu first on Windows ARM64). The current build still ships wgpu only, pinned to DX12 on Windows. See [Renderer selection](#renderer-selection). |

Specifications: `docs/specs/architecture.md` (durable) and `docs/specs/m0-foundations.md` (M0).
Measurements: `docs/perf/M0-backend.md`.

## Platform support

Support tiers, verbatim from architecture §9.1:

| Tier | Meaning | Platforms |
|---|---|---|
| **Tested manually** | A human ran the GUI and the CLI | Windows 11 x64 (reference machine: Ryzen AI 7 350 / Radeon 860M); Ubuntu 26.04 x64 under WSLg, through X11 (Xwayland, the default under WSL, §9.8). Wayland under WSLg is tested with glow only (`VA_WAYLAND=1`). |
| **Tested automatically** | The full test suite, including headless kittest, runs in CI; nobody looks at the UI | GitHub runners `windows-latest` (x64), `windows-11-arm`, `ubuntu-latest` (x64), `ubuntu-24.04-arm` |
| **Compiled only** | Built, never executed | `aarch64-unknown-linux-gnu` cross-built from WSL |

"§9.8" is the renderer policy of the architecture. Its X11 default under WSL and `VA_WAYLAND=1` are
decided but not implemented yet: today Wayland is winit's default under WSLg, and X11 needs
`env -u WAYLAND_DISPLAY` (see [Renderer selection](#renderer-selection)).

A platform moves up a tier only with evidence: a report in `docs/perf/` or a completed manual
checklist. What the evidence shows at the end of M0:

| Platform | Evidence today |
|----------|----------------|
| Windows 11 x64, reference machine | `make test`, `make lint`, `make docs` and `make security` pass. The automated bench opened and closed the window more than 100 times: glow and wgpu on DX12 both pass the reliability (G1) and idle (G3) gates. The **human visual check (G2) and the cold-start run are still pending.** |
| Ubuntu 26.04 x64 under WSLg, **X11** | `make test`, `make lint` and `make docs` pass. The bench passes G1 and G3 with wgpu and with glow, with 0 compositor crashes in 59 X11 launches. G2 is pending. Rendering is software only (llvmpipe/lavapipe). |
| Ubuntu 26.04 x64 under WSLg, **Wayland** | glow passes G1 and G3 (G2 pending). **wgpu crashed the WSLg compositor** (weston 9.0.0, WSLg 1.0.73.2) in 3 of 5 launches, which closes every WSLg window. The decided policy uses X11 under WSL unless `VA_WAYLAND=1`. It is not implemented yet, so launch the GUI under WSLg with `env -u WAYLAND_DISPLAY` (X11). |
| Windows 11 ARM64, Ubuntu ARM64 (CI runners) | **Never checked by a human.** CI runs the headless tests there, but has not run yet. The ARM64 GUI is never looked at. The decided `auto` order tries wgpu first on Windows ARM64, because DX12 is native there and OpenGL needs an optional compatibility pack. |
| `aarch64-unknown-linux-gnu` | Cross-built from WSL (exit criterion E3); never executed |
| Native Linux desktops, macOS | Not measured. macOS is a non-goal: it may compile, but is never tested or claimed. |

## Performance status

Targets (architecture §7, gate G4): startup p95 **≤ 500 ms** from process spawn to the first presented frame
on the reference machine, warm cache; idle **0 frames and ≤ 1 % CPU**. In M0 the startup target is
tracked, not blocking (m0-foundations.md §17.13). All numbers come from `docs/perf/M0-backend.md`.

| Configuration (Windows 11 x64 unless stated) | Startup p50 / p95 | Idle | Peak RSS |
|-----------------------------------------------|-------------------|------|----------|
| wgpu DX12, mains power (C1d) | 435.5 / **464.8 ms**: target met, 7 % margin | 0 frames, 0.00 % CPU | 342.4 MiB |
| wgpu default build after the DX12 pin (C1), **battery only** | 586.8 / 617.2 ms | 0 frames, 0.00 % CPU | 340.5 MiB |
| glow (C2), mains power | 278.6 / 301.8 ms | 0 frames, 0.00 % CPU | 92.7 MiB |
| WSLg X11, wgpu / glow (software rendering) | 136.6 / 151.8 ms · 97.3 / 107.8 ms | 0 frames, 0.02 % CPU | 135.7 / 118.7 MiB |

- Every row is a single-renderer build. The decided shipped binary (both renderers, `auto` → glow on
  Windows x64) has not been measured yet. m0-foundations.md §19.7 asks for that run (N1, on mains
  power) and for the fallback cost (N4).
- The current wgpu build (C1, with the DX12 pin) has **no mains-power number in the report yet**. On
  battery it matches the DX12 control within 0.2 %, and battery adds about a third to startup. Run N1
  replaces that owed run.
- wgpu uses about 250 MiB more memory than glow on this shared-memory GPU. The report explains the gap
  (GPU allocator block sizes, driver images); it is an M7 input. That gap, and glow's lower startup
  time, are why the user chose glow by default.
- The true cold start (right after a reboot) is pending a human.

## Build, run and test

Prerequisites:
- Rust through rustup. `rust-toolchain.toml` pins **1.99.0** with rustfmt and clippy; rustup installs
  it on first use.
- GNU make (4.4.1 tested). On Windows, run it from Git Bash; the recipes contain no shell syntax.
- Windows: the MSVC x64 build tools (Visual Studio 2022 Build Tools). Binaries link the C runtime
  statically, so users need no VC++ redistributable.
- Python 3.11+ for `make docs` (`python` on Windows, `python3` on Linux, or set `VA_PYTHON`), and `git`.
- cargo-deny 0.20.2 for `make security` (installed by `make install`). It is needed on Windows only;
  CI covers Linux.
- Linux: building needs no system `-dev` package. Running the GUI needs the usual desktop libraries
  (Wayland or X11, xkbcommon, EGL/GL or Vulkan, Mesa).

**Windows** (Git Bash, at the repository root):

```bash
make install      # once: toolchain, cargo-deny 0.20.2, crates
make hooks        # once: commit-msg hook that rejects AI co-author mentions
make build        # target/release/visual-assembler.exe and va.exe
make test
make lint
make docs
make security
make run          # opens the window
```

**Linux / WSL** (Ubuntu 26.04). Keep the target folder on ext4 and never share Windows' `target/`:

```bash
export CARGO_TARGET_DIR=$HOME/.cache/visualassembler/target
make test
make lint
make docs
env -u WAYLAND_DISPLAY make run    # under WSLg: force X11 (see Renderer selection)
```

**ARM64 Linux cross-build** from WSL. It needs the apt package `gcc-aarch64-linux-gnu` and the Rust
target:

```bash
rustup target add aarch64-unknown-linux-gnu
cargo build --release --locked --target aarch64-unknown-linux-gnu -p visual-assembler -p va
```

Quick CLI check: `cargo run -p va -- info --json`.

### Make targets

`make -s help` lists them. Every recipe calls `cargo` or `cargo xtask`.

| Target | Runs |
|--------|------|
| `install` | `rustup toolchain install`, `cargo install cargo-deny --version 0.20.2 --locked`, `cargo fetch` |
| `hooks` | `cargo xtask hooks`: installs `scripts/git-hooks/commit-msg` |
| `dev` | `cargo run -p visual-assembler` (debug build) |
| `run` | `cargo run --release -p visual-assembler` |
| `test` | `cargo test --workspace --all-features` |
| `lint` | `cargo fmt --all -- --check`, then `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| `format` | `cargo fmt --all` |
| `security` | `cargo deny --locked --manifest-path Cargo.toml --config .cargo/deny.toml check` |
| `docs` | `cargo xtask docs`: regenerates `AGENCEMENT.md`, then runs `scripts/check_project_docs.py` |
| `build` | `cargo build --release -p visual-assembler -p va` |
| `clean` | `cargo clean` |
| `bench` | `cargo xtask bench $(SCENARIO) --renderer $(RENDERER)` (defaults `all`, `wgpu`), e.g. `make bench SCENARIO=startup RUNS=3`. The decided renderer default is `auto` (m0 §19.5, not implemented yet). |
| `ci` | `cargo xtask ci`: build-configuration check, fmt, clippy (all features, then glow only), layering and source checks, tests. m0 §19.6 adds a wgpu-only clippy step (not implemented yet). |

Never set `RUSTFLAGS` (not even empty): it silently replaces the static-CRT flags of
`.cargo/config.toml`, and `make ci` refuses to run with it.

## Renderer selection

The GUI draws through one of two renderers: **glow** (OpenGL) or **wgpu** (DX12 on Windows, Vulkan or
GL elsewhere).

### Decided policy (not implemented yet)

The user decided this on 2026-10-14 (m0-foundations.md §19, architecture §9.8). It replaces the
earlier rule, which picked wgpu from the measurements. The code changes are listed in the fix-wave
table of m0 §19.8 and have **not landed yet**. Until they do, the build behaves as described in
[Current build](#current-build).

- The shipped binary contains both renderers.
- The choice is `auto`, `glow` or `wgpu`. The first source that is set wins:
  1. the `--renderer` option;
  2. the `VA_RENDERER` environment variable;
  3. the `[render] renderer` key of `settings.toml` (from M4);
  4. the built-in `auto`.
- An invalid `VA_RENDERER` value is ignored with a warning. An invalid `--renderer` value is a usage
  error (exit 2).
- `auto` tries the renderers in this order:

  | Target | Order |
  |--------|-------|
  | Windows x64, Linux x64 and aarch64, other targets | glow, then wgpu |
  | Windows aarch64 | wgpu, then glow (DX12 is native there; OpenGL needs an optional compatibility pack) |

- **Fallback.** With `auto`, the app moves to the next renderer once, and only when the first one fails
  to initialize before the first frame. It then prints
  `warning: the glow renderer failed to initialize: <error>; falling back to wgpu`. A missing display
  or a window-system failure never triggers a fallback, because the other renderer would fail the
  same way. Neither does an error after the first frame.
- **Forced renderer.** `glow` or `wgpu` runs exactly that renderer, with no fallback.
- **Panics are not caught.** If a renderer panics during startup (exit code 101), force the other one
  with `--renderer` or `VA_RENDERER`.
- **Exit code 3** when no renderer starts:

  | Situation | Message on stderr |
  |-----------|-------------------|
  | `auto`, both renderers failed | the fallback warning, then `error: no renderer could be initialized (tried glow, then wgpu): <last error>` and `note: see the README section "Renderer selection"` |
  | Forced renderer failed | `error: the <r> renderer failed to initialize: <error> (forced by <--renderer \| VA_RENDERER>; try --renderer auto)` |
  | No display, or an error after the first frame | `error: cannot create the window or the renderer: <error>` |

- **Under WSL**, every renderer uses X11 (Xwayland) unless `VA_WAYLAND=1`, because wgpu Wayland
  clients crash the WSLg compositor (weston 9.0.0, WSLg 1.0.73.2). With `VA_WAYLAND=1` and wgpu in the
  plan, the app prints
  `warning: VA_WAYLAND=1 with the wgpu renderer can crash the WSLg compositor (weston 9)`. Native
  Linux keeps winit's default, which tries Wayland first.
- **wgpu on Windows** uses DX12 unless `WGPU_BACKEND` is set, whether wgpu was forced or reached by
  fallback.
- **Test hook.** `VA_TEST_FAIL_RENDERER=glow`, `wgpu` or `glow,wgpu` simulates an initialization
  failure of the listed renderers, to exercise the fallback. It is compiled into every build, prints a
  warning at startup, and is meant for tests only.

### Current build

- The default features enable `renderer-wgpu` only, so the default binary runs wgpu.
- `--renderer <wgpu|glow>` picks a renderer compiled into the binary. A renderer that is not compiled
  in is a usage error (exit 2). There is no `auto` value and no fallback. `VA_RENDERER`, `VA_WAYLAND`
  and `VA_TEST_FAIL_RENDERER` are not read.
- glow build: `cargo run --release -p visual-assembler --no-default-features --features renderer-glow`.
- On Windows, wgpu is restricted to DX12 unless `WGPU_BACKEND` is set (for example `vulkan`).
- Under WSLg, nothing forces X11 yet: start the GUI with `env -u WAYLAND_DISPLAY`.
- A window or renderer failure prints `error: cannot create the window or the renderer: <error>` and
  exits with 3.

## Repository layout

| Folder | Purpose |
|--------|---------|
| `apps/` | Shipped binaries: `visual-assembler` (GUI) and `va` (CLI) |
| `crates/` | CPU-agnostic library crates (`va-core`, `va-editor`, `va-lang`, `va-debug`, `va-workspace`, `va-ui`) |
| `arch/` | One crate per CPU: `m68k` (`va-arch-m68k`) |
| `xtask/` | Rust task runner behind `make` |
| `bench/` | Bench outputs (gitignored `results/`) |
| `docs/` | `specs/` (architecture, milestone specs) and `perf/` (measurement reports) |
| `scripts/` | Documentation checks and `git-hooks/` |
| `.github/workflows/` | CI workflow |
| `.cargo/` | `config.toml` (xtask alias, static CRT, aarch64 linker) and `deny.toml` (cargo-deny policy) |

Every folder has a README, except the dot-folder `.cargo/`, whose two files are described in the table
above. `scripts/check_project_docs.py` skips folders whose name starts with a dot (see
`scripts/README.md`). Pilot files at the root: `CLAUDE.md` (working rules for contributors and
agents), `AGENCEMENT.md` (annotated tree), `DEPENDANCE.md` and `IMPORT.md` (dependencies, licences,
platform support), `SECURITY.md` (protections actually in place and their limits).

## Security and privacy

The product never initiates network I/O and has no telemetry. cargo-deny bans network, TLS and
telemetry crates, and `cargo xtask ci` scans the sources for network APIs and unsafe code. These
checks are tripwires against accidents, not a sandbox; see `SECURITY.md` for what they cover and what
they do not.

## Licence

VisualAssembler is licensed under either of the [MIT licence](LICENSE-MIT) or the
[Apache License 2.0](LICENSE-APACHE), at your option (`MIT OR Apache-2.0`).
