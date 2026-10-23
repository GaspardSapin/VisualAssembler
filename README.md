# VisualAssembler

A native, offline IDE for students learning assembly language, written in Rust. Version 1 targets the
Motorola 68000 end to end: edit, assemble, run and debug, in a light window that starts fast and stays
idle at about 0 % CPU. Everything is also reachable from a terminal (the `va` CLI, `make`,
`cargo xtask`). CPU support sits behind traits, so later CPUs are new `arch/<cpu>` crates.

## Status: milestone M1 (core contracts, 68000 ISA tables, lexer)

M1 is done: implemented, audited, measured, pushed, and green in CI (run 40027188396). It adds the
frozen CPU-agnostic contracts, the complete 68000 instruction tables, the A68k-dialect lexer, English
and French instruction documentation, and two CLI commands, `va lex` and `va isa`. **There is no
editor, assembler or emulator yet**: the window is still the M0 one.

| Piece | State at the end of M1 |
|-------|------------------------|
| `va` (CLI) | `info`, `lex`, `isa`; global `--json` and `--lang en\|fr`. See [The `va` command line](#the-va-command-line). |
| `arch/m68k` | 68000 instruction tables (126 spellings, 74 entries, forms, flags, opword encodings, cycle counts), the A68k-dialect line lexer, the register file, EN/FR docs in Fluent. Cycle counts are provisional until the M3 cross-check (`arch/m68k/src/isa/README.md`). |
| `crates/va-core` | Frozen contracts: sources and lines, diagnostics, the lexer contract, the instruction view, registers, program image and source map, `Architecture` |
| `crates/va-workspace` | Language selection and the Fluent localizer (`i18n`); the rest arrives in M4 |
| Other `crates/*` | Skeletons (`va-editor`, `va-lang`, `va-debug`, `va-ui`) |
| `visual-assembler` (GUI) | Unchanged since M0: one window with a static text, in egui's reactive mode. Measures its own startup and idle cost (`--bench startup\|idle`). |
| `cargo xtask` | `bench`, `ci` (9 steps, now also clippy and tests of `va` built without any CPU), `docs`, `hooks` |
| CI (`.github/workflows/ci.yml`) | 4 native runners. Run 40027188396 (2026-10-23, commit `5f45f9b`, the M1 tree) passed all 7 jobs. The run before it, 40023018227 on `8994d44`, failed only `deny`: `clipboard-win` 5.4.1 (Windows clipboard, pulled through eframe) had been yanked upstream for an out-of-bounds read, and `5f45f9b` bumps it to 5.4.3 (`SECURITY.md`, M1-S10). |
| Renderer | Both renderers ship. `auto` starts glow and falls back to wgpu if OpenGL fails to initialize (wgpu first on Windows ARM64); `--renderer` and `VA_RENDERER` force one. See [Renderer selection](#renderer-selection). |

Roadmap: M0 foundations (done) · **M1 core contracts, ISA tables, lexer (done)**
· M2 assembler and disassembler · M3 headless emulation and debugging · M4 editing and GUI shell ·
M5 language services · M6 debugging UI · M7 performance and hardening · M8 distribution. A milestone
gets its spec in `docs/specs/` when it starts.

Specifications: `docs/specs/architecture.md` (durable), `docs/specs/m0-foundations.md` (M0) and
`docs/specs/m1-core-isa-lexer.md` (M1). Measurements: `docs/perf/M0-backend.md` and
`docs/perf/2026-10-23-m1-lexer.md`.

## Platform support

Support tiers, verbatim from architecture §9.1:

| Tier | Meaning | Platforms |
|---|---|---|
| **Tested manually** | A human ran the GUI and the CLI | Windows 11 x64 (reference machine: Ryzen AI 7 350 / Radeon 860M); Ubuntu 26.04 x64 under WSLg, through X11 (Xwayland, the default under WSL, §9.8). Wayland under WSLg is tested with glow only (`VA_WAYLAND=1`). |
| **Tested automatically** | The full test suite, including headless kittest, runs in CI; nobody looks at the UI | GitHub runners `windows-latest` (x64), `windows-11-arm`, `ubuntu-latest` (x64), `ubuntu-24.04-arm` |
| **Compiled only** | Built, never executed | `aarch64-unknown-linux-gnu` cross-built from WSL |

"§9.8" is the renderer policy of the architecture (see [Renderer selection](#renderer-selection)).

A platform moves up a tier only with evidence: a report in `docs/perf/` or a completed manual
checklist. What the evidence shows at the end of M0 (`docs/perf/M0-backend.md` §11, round 3 on the
shipped binary, 2026-10-16):

| Platform | Evidence today |
|----------|----------------|
| Windows 11 x64, reference machine | `make test`, `make lint`, `make docs` and `make security` pass. The shipped binary passed every automated bench run on mains power: `auto` (which runs glow), forced glow and forced wgpu on DX12, with 0 idle frames in 18 of 18 idle runs. With an injected glow failure, 40 of 40 runs fell back to wgpu on DX12. The ignored display tests of the fallback pass (8 tests). The **human visual check (G2) and the cold-start run are still pending.** |
| Ubuntu 26.04 x64 under WSLg, **X11** (the WSL default) | `make test`, `make lint` and `make docs` pass. The shipped binary passed every bench run with `auto` (glow), glow and wgpu, on X11 even though `WAYLAND_DISPLAY` was set, with 0 compositor crashes in 223 X11 launches. A genuine OpenGL failure (Mesa version override) fell back to wgpu on Vulkan. The ignored display tests pass (9 tests). G2 is pending. Rendering is software only (llvmpipe/lavapipe). |
| Ubuntu 26.04 x64 under WSLg, **Wayland** (`VA_WAYLAND=1`) | glow passes the reliability and idle gates (28 launches, G2 pending). **wgpu crashed the WSLg compositor** (weston 9.0.0, WSLg 1.0.73.2) in 3 of 5 launches in round 1, which closes every WSLg window, so wgpu is never run on Wayland. |
| Windows 11 ARM64, Ubuntu ARM64 (CI runners) | **Never checked by a human.** CI run 38969165969 (2026-10-16, `6ed43c8`, with the renderer work) passed the headless tests and the release build there. The informational GUI smokes on `windows-11-arm` run on WARP; they are not evidence. The `auto` order tries wgpu first on Windows ARM64, because DX12 is native there and OpenGL needs an optional compatibility pack. |
| `aarch64-unknown-linux-gnu` | Cross-built from WSL (exit criterion E3), again on 2026-10-16 with the renderer work; never executed |
| Native Linux desktops, macOS | Not measured. macOS is a non-goal: it may compile, but is never tested or claimed. |

M1 adds no platform-specific code. Its `va` commands ran on Windows 11 x64 and in WSL Ubuntu 26.04
x64: Security built a release `va` on both and ran crafted inputs through it (`SECURITY.md`), and
Performance measured the lexer on both (`docs/perf/2026-10-23-m1-lexer.md`). On the four CI runners,
ARM64 included, run 40027188396 (2026-10-23, `5f45f9b`) passed clippy, the layering checks, every
headless test and the release build of the M1 tree.

## Performance status

Targets (architecture §7, gate G4): startup p95 **≤ 500 ms** from process spawn to the first presented frame
on the reference machine, warm cache; idle **0 frames and ≤ 1 % CPU**. In M0 the startup target is
tracked, not blocking (m0-foundations.md §17.13). All numbers come from `docs/perf/M0-backend.md` §11
(round 3, shipped binary, Windows on mains power in Best performance mode).

**Provisional**: one unattended session per platform, with no human check (G2) and no cold start yet.
Sessions on the same build differed by about 25 %, and p50 drifted up by about 15 % within the session.

| Configuration (Windows 11 x64 unless stated) | spawn→frame p50 / p95 | Idle | Peak RSS at startup |
|-----------------------------------------------|-----------------------|------|---------------------|
| **`auto`, the shipped default (runs glow)** | 236.1 / **267.3 ms**: target met | 0 frames, 0.00 % CPU | 91.5 MiB |
| `--renderer glow` | 216.3 / 234.2 ms | 0 frames, 0.00 % CPU | 91.5 MiB |
| `--renderer wgpu` (DX12) | 344.3 / 368.6 ms | 0 frames, 0.00 % CPU | 331.1 MiB |
| `auto` after an injected glow failure (fallback path, upper bound) | 538.0 / 594.2 ms: **misses the target** | not measured | 354.6 MiB |
| WSLg X11, `auto` (glow, software rendering) | 93.9 / 104.0 ms | median 0 frames, 0.01 % CPU | 120.3 MiB |
| WSLg X11, `--renderer wgpu` (lavapipe) | 118.6 / 125.9 ms | 0 frames, 0.01 % CPU | 136.1 MiB |
| WSLg Wayland, `VA_WAYLAND=1 --renderer glow` | 76.3 / 88.3 ms | 0 frames, 0.01 % CPU | 125.6 MiB |

- `auto` and forced glow run the same path; the gap between them is drift within the session, not a cost.
- **Fallback cost**, measured with the test hook, which fails glow after its full initialization (so an
  upper bound): +142.4 / +167.9 ms (p50 / p95) on Windows and +18.8 / +29.8 ms under WSLg X11. A genuine
  OpenGL failure under WSLg cost nothing measurable, because it fails before the expensive OpenGL work.
- One WSLg X11 idle run of `auto` painted 6 frames at 0.58 % CPU; the 3-run replication gave 0 frames.
  The cause is unknown (an input or focus event is suspected).
- The measured shipped binary (both renderers) was 15,528,448 B on Windows, 2.8 % more than the
  wgpu-only build, with no measurable memory cost on either path.
- wgpu uses about 240 MiB more memory than glow on this shared-memory GPU (GPU allocator block sizes,
  driver images; an M7 input). That gap, and glow's lower startup time, are why the user chose glow
  by default.
- The RAM target (< 500 MB with a 5,000-line project) cannot be measured before M4.
- The true cold start (right after a reboot) is pending a human.

**Lexer throughput (M1, exit criterion E7, passed)**, from `docs/perf/2026-10-23-m1-lexer.md`:
`make bench-lexer` lexes 100,000 generated lines (3.3 MB) in a median of **7.157 ms** (463.2 MB/s)
on the reference machine, on mains in "Best performance", against a budget of ≤ 50 ms; 7.104 ms in
WSL. The bench covers line splitting and lexing only, not `va lex`'s file I/O and JSON output.

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
make run          # under WSL the window uses X11 (see Renderer selection)
```

**ARM64 Linux cross-build** from WSL. It needs the apt package `gcc-aarch64-linux-gnu` and the Rust
target:

```bash
rustup target add aarch64-unknown-linux-gnu
cargo build --release --locked --target aarch64-unknown-linux-gnu -p visual-assembler -p va
```

Quick CLI check: `cargo run -p va -- info --json` (see [The `va` command line](#the-va-command-line)).

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
| `bench` | `cargo xtask bench $(SCENARIO) --renderer $(RENDERER)` (defaults `all` and `auto`), e.g. `make bench SCENARIO=startup RUNS=3`. It measures the shipped binary. |
| `bench-lexer` | `cargo test -p va-arch-m68k --release --test syntax_bench -- --ignored --nocapture`: lexer throughput on 100,000 generated lines, one `va.bench.lexer/1` JSON line. Measure on mains power. |
| `ci` | `cargo xtask ci`, 9 steps: build-configuration check, fmt, clippy (all features, glow only, wgpu only, then `va` without any CPU), layering and source checks, tests (workspace, then `va` without any CPU) |

Never set `RUSTFLAGS` (not even empty): it silently replaces the static-CRT flags of
`.cargo/config.toml`, and `make ci` refuses to run with it.

## The `va` command line

```text
va [--json] [--lang en|fr] info | lex <FILE|-> | isa <MNEMONIC[.SIZE]> | isa --list
```

```bash
cargo run -p va -- lex arch/m68k/fixtures/lex/basic.asm   # tokens on stdout, diagnostics on stderr
cargo run -p va -- --json lex arch/m68k/fixtures/lex/errors.asm   # one va.lex/1 document, exit 1
cargo run -p va -- isa move.l                              # forms, modes, flags, cycles of MOVE.L
cargo run -p va -- --lang fr isa move.l                    # the same in French
cargo run -p va -- isa --list                              # the 126 spellings with a summary
```

- **`va lex`** lexes a 68000 source in the A68k dialect (a68k 2.71.F3w): one token per line on
  stdout, GNU-style diagnostics (`<file>:<line>:<column>: error[<code>]: …`) on stderr. Exit 0 when
  clean, 1 with any error diagnostic.
- **`va isa`** shows an instruction's summary, details, forms, addressing modes, flag effects and cycle
  counts. An unknown spelling exits 1 with suggestions (`va isa MOV` suggests `MOVE, MOVEA, MOVEM`).
- **Language**: `--lang en|fr`, else `VA_LANG` (`en`, `fr` or `auto`), else `LC_ALL`, `LC_MESSAGES`,
  `LANG`. An invalid `VA_LANG` is ignored with a warning. Only diagnostics and instruction docs are
  translated; CLI messages stay English. The Windows display language is read from M4 on, not yet.
- **`--json`** prints one document (`va.lex/1`, `va.isa/1`, `va.isa.list/1`, `va.info/1`), or
  `va.error/1` on failure.
- **Input**: UTF-8 only, at most 16 MiB per file and 64 KiB per line. Invalid UTF-8, a larger file or
  an unreadable path exits 3. Pass local files only: reads have no timeout (a FIFO or `CON` blocks),
  and on Windows a UNC path connects to the remote host (`apps/va/README.md`).
- **Terminal safety**: text output escapes control, C1 and bidi characters coming from the source or
  the arguments; `--json` output keeps them as they are.

**Strict A68k operand spacing (user decision, 2026-10-17).** As in a68k, the operand field ends at
the first space, and the rest of the line is a comment. `dc.b 1, 2, 3` assembles one byte, and
`move.l d0, d1` is an a68k error. VisualAssembler keeps exactly that meaning, but when the ignored
text looks like more operands, it reports the error `lex-operands-in-comment` with a fix. The error
is non-blocking: it never changes the bytes produced. A real comment, after `;` or after free text,
stays a plain comment.

```text
$ printf ' dc.b 1, 2, 3\n' | va lex -
1:2 directive "dc"
1:4 size-suffix ".b"
1:7 number "1"
1:8 punctuation ","
1:10 suspect-comment "2, 3" lex-operands-in-comment
-:1:10: error[lex-operands-in-comment]: a68k will ignore “2, 3”: the operands end at the first space.
  help: Remove the spaces after the commas.
  fix:  dc.b 1,2,3
```

A permissive dialect (spaces after commas, EASy68K/vasm style) is planned as a per-project profile
after M1; it is not implemented. Details: `arch/m68k/src/syntax/README.md` and `apps/va/README.md`.

## Renderer selection

The GUI draws through one of two renderers, both compiled into the shipped binary: **glow** (OpenGL)
and **wgpu** (DX12 on Windows, usually Vulkan on Linux). This is the user's decision of 2026-10-14
(m0-foundations.md §19 and §19.9, architecture §9.8).

### Choosing

- The choice is `auto`, `glow` or `wgpu`. The first level that is set wins:
  1. the `--renderer` option;
  2. the `VA_RENDERER` environment variable;
  3. the `[render] renderer` key of `settings.toml`, which arrives in M4 (not read yet);
  4. the built-in `auto`.
- `--renderer auto` is a real value: it brings back `auto` for one run, whatever `VA_RENDERER` says.
- An invalid `--renderer` value is a usage error (exit 2).
- `VA_RENDERER` accepts exactly `auto`, `glow` or `wgpu`. Any other value, an empty one included, prints
  `warning: ignoring VA_RENDERER='<v>' (expected auto, glow or wgpu)`, with control characters
  escaped, even when `--renderer` overrides it, and is ignored.
- `auto` tries the renderers in this order:

  | Target | Order |
  |--------|-------|
  | Windows x64, Linux x64 and aarch64, other targets | glow, then wgpu |
  | Windows aarch64 | wgpu, then glow (DX12 is native there; OpenGL needs an optional compatibility pack) |

- A **forced** renderer (`glow` or `wgpu`) runs exactly that renderer, never falls back, and exits with
  3 if it fails.

```bash
cargo run --release -p visual-assembler -- --renderer wgpu    # force wgpu for one run
VA_RENDERER=glow make run                                     # force glow through the environment
./target/release/visual-assembler.exe --bench startup | cat   # Windows: one JSON line; "renderer" names the one that started
```

### Fallback (`auto` only)

- With `auto`, the app moves to the second renderer **once**, in the same process, and only when the
  first one fails to initialize before the first frame. It prints
  `warning: the glow renderer failed to initialize: <error>; falling back to wgpu` (glow and wgpu
  swapped on Windows ARM64).
- **No fallback** for:
  - a missing display or a window-system failure, because the other renderer would fail the same way;
  - an error after the first frame;
  - a forced renderer;
  - a panic (exit 101) or a crash inside the graphics driver. For example, `LIBGL_ALWAYS_INDIRECT=1`
    under WSLg crashes glow's GLX path. Start with `--renderer wgpu` (or `glow`) instead.
- **wgpu after a glow failure.** The wgpu fallback does not use wgpu's OpenGL backend, because the
  failed OpenGL context can stay current and wgpu's GL backend would crash. On Linux, a machine without
  any Vulkan driver therefore stops with exit 3 when glow fails. A `WGPU_BACKEND` that names at least one
  backend wgpu recognizes (`vulkan`, `dx12`, `gl`, `metal`, …) lifts this exclusion and the Windows DX12
  pin; `WGPU_BACKEND=gl` can then crash with exit 101 after a glow failure. An empty or unrecognized
  `WGPU_BACKEND` does not lift them.
- Bench JSON (`va.bench.run/2`) records `renderer_requested`, `renderer`, `fallback_from` and
  `fallback_reason`.

**Exit code 3** when no renderer starts:

| Situation | Message on stderr |
|-----------|-------------------|
| `auto`, every renderer failed | the fallback warning, then `error: no renderer could be initialized (tried glow, then wgpu): <last error>` and `note: see the README section "Renderer selection"`. The `tried …` list follows the target order (`tried wgpu, then glow` on Windows ARM64). |
| Forced renderer failed | `error: the <r> renderer failed to initialize: <error> (forced by <--renderer \| VA_RENDERER>; try --renderer auto)` |
| No display, a window-system failure, or an error after the first frame | `error: cannot create the window or the renderer: <error>` |

### Under WSL

- Every renderer uses **X11** (Xwayland), even when `WAYLAND_DISPLAY` is set, unless `VA_WAYLAND=1`
  (exactly `1`). wgpu Wayland clients crash the WSLg compositor (weston 9.0.0, WSLg 1.0.73.2), which
  closes every WSLg window, and an in-process fallback cannot change the window system.
- `VA_WAYLAND=1` keeps Wayland. With wgpu in the plan (`auto` or forced wgpu) the app prints
  `warning: VA_WAYLAND=1 with the wgpu renderer can crash the WSLg compositor (weston 9)`. **Never run
  wgpu with `VA_WAYLAND=1`**; use it with glow only:

  ```bash
  VA_WAYLAND=1 VA_RENDERER=glow make run
  ```

- The default X11 path needs `DISPLAY`. Without it the window cannot be created (exit 3, no fallback),
  even when `WAYLAND_DISPLAY` is set. Set `DISPLAY`, or use `VA_WAYLAND=1` with `--renderer glow`.
- Native Linux (not WSL) keeps winit's default, which tries Wayland first; `VA_WAYLAND` has no effect
  there. Native desktops were not measured.

### On Windows

- wgpu is pinned to **DX12**, whether it was forced or reached by fallback, unless `WGPU_BACKEND` names
  a backend wgpu recognizes (for example `WGPU_BACKEND=vulkan`). An empty or unrecognized value keeps
  the pin.
- The release `visual-assembler.exe` has no console. Every warning and error above is visible only when
  stderr is redirected (Git Bash, a pipe such as `2>&1 | cat`, or the debug build). Bench JSON still
  records `renderer`, `fallback_from`, `fallback_reason` and `injected_failure`. From M4 the status bar
  shows a fallback and an active test hook.

### Test hook `VA_TEST_FAIL_RENDERER`

**`VA_TEST_FAIL_RENDERER` is a test hook; leave it unset.** It is compiled into every build, release
included, so that the shipped fallback path can be exercised.

- It takes comma-separated `glow`/`wgpu`. Spaces around items are ignored, and each unknown item is
  warned (`warning: ignoring unknown VA_TEST_FAIL_RENDERER value '<v>' (expected glow or wgpu)`) and
  ignored.
- A listed renderer fails on purpose right after its real initialization, so `auto` falls back exactly
  as after a genuine failure, and a forced listed renderer exits with 3.
- The warning `warning: VA_TEST_FAIL_RENDERER=<list> is set: simulating a renderer initialization
  failure (test hook)` appears when at least one renderer is armed.
- Bench JSON records the raw value in `injected_failure` whenever the variable is set.

```bash
VA_TEST_FAIL_RENDERER=glow ./target/release/visual-assembler.exe --bench startup 2>&1 | cat
```

On Windows x64 this prints the two warnings, then a bench line with `"renderer":"wgpu"`,
`"fallback_from":"glow"` and `"backend":"dx12"`.

### How the fallback is tested

- **Headless, in CI**: the selection rules, the messages, every attempt's options and the bench line
  (`apps/visual-assembler/tests/`, see its README).
- **With a display, locally** (they open and close windows):

  ```bash
  cargo test -p visual-assembler --test fallback -- --ignored --test-threads=1
  ```

  Under WSLg, prefix it with `env -u WAYLAND_DISPLAY`. On 2026-10-16 it passed 8 tests on Windows x64
  and 9 under WSLg X11. Run it again whenever `Cargo.lock` changes eframe, egui, egui-wgpu, egui_glow,
  wgpu, glutin or winit.
- **A genuine OpenGL failure** under WSLg X11: Mesa told to report an old OpenGL version makes glutin
  fail (`GLXBadFBConfig`), and the app falls back to wgpu on Vulkan:

  ```bash
  env -u WAYLAND_DISPLAY MESA_GL_VERSION_OVERRIDE=1.1 MESA_GLES_VERSION_OVERRIDE=1.0 "$CARGO_TARGET_DIR/release/visual-assembler" --bench startup
  ```

- **CI**: an informational smoke step injects a failure into the first renderer of each Windows runner
  (`.github/workflows/README.md`).

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
above, and the folders that hold only sub-folders (`arch/m68k/fixtures/`, `arch/m68k/locales/`),
which their parent README describes. `scripts/check_project_docs.py` skips folders whose name starts
with a dot and folders without a file of their own (see `scripts/README.md`). Pilot files at the root: `CLAUDE.md` (working rules for contributors and
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
