# apps/visual-assembler

## Purpose
The VisualAssembler desktop IDE (package `visual-assembler`): an eframe window plus a library
(`visual_assembler`) that makes the UI, the command line and the bench testable headless. In M0 it
opens one window with a static text, in egui's reactive mode (no continuous repaint), and can measure
its own startup and idle cost. It is the only package allowed to use GUI crates (rule R3), and no
package may depend on it (rule R7).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Features `renderer-glow` and `renderer-wgpu`, **both default**: that is the shipped binary. Single-renderer builds still compile (`cargo xtask ci` steps 3 and 3b lint them) but are neither shipped nor benchmarked. Dependencies `eframe`, `clap`, `serde`, `serde_json`, `raw-window-handle`; `windows-sys` on Windows; `libc` and `winit` on Linux; dev-dependency `egui_kittest` |
| `src/` | folder | Library, binary, command line, renderer selection, window and bench. See its README. |
| `tests/` | folder | Headless tests, the ignored display tests of the renderer fallback, and the list of what is not automated. See its README. |

## Entry points
`src/lib.rs` (`run`). Command line:

```text
visual-assembler [--renderer <auto|glow|wgpu>] [--bench <startup|idle>] [--duration-secs <N>]
visual-assembler --help | --version        # version line: "visual-assembler 0.1.0"
```

Environment: `VA_RENDERER` (`auto`, `glow` or `wgpu`, below `--renderer`), `VA_WAYLAND` (`1` keeps
Wayland under WSL), `VA_TEST_FAIL_RENDERER` (test hook, leave it unset) and wgpu's own `WGPU_BACKEND`.
The user-facing description is the root README, section "Renderer selection".

| Exit code | Meaning (m0-foundations.md §17.10, §19.4) |
|-----------|-------------------------------------------|
| 0 | Normal exit, or a bench scenario completed (a fallback included) |
| 1 | Runtime I/O or infrastructure failure: clap's help, version or error text cannot be written; a bench thread cannot be created; the bench JSON line cannot be written or flushed |
| 2 | Usage error (clap errors, an invalid `--renderer` value included; a forced renderer that is not compiled in, possible only in single-renderer builds; `--duration-secs` without `--bench idle`) |
| 3 | The window or the renderer cannot be created: a platform failure, an error after the first frame, a forced renderer that fails, or `auto` with every renderer failing |
| 4 | Bench watchdog: no measurement within the hard limit |
| 101 | A panic. Panics are not caught, so they never fall back. |

**Renderer policy** (m0 §19 and §19.9, architecture §9.8):
- `auto` tries glow, then wgpu (wgpu, then glow on Windows aarch64). It falls back once, in the same
  process, only for a renderer initialization error before the first frame. A forced renderer
  (`--renderer` > `VA_RENDERER` > `settings.toml` from M4) never falls back.
- After a failed glow attempt, the wgpu attempt leaves out wgpu's GL backend
  (`exclude_wgpu_gl_backend`): glutin leaves the failed OpenGL context current, and wgpu-hal's GL
  backend would then panic. On Linux with no Vulkan driver, `auto` therefore exits 3 when glow fails.
- On Windows every wgpu attempt is pinned to DX12 (`pin_windows_wgpu_backend`).
- Both overrides step aside only when `WGPU_BACKEND` is **effective**: valid Unicode, and wgpu's
  `Backends::from_comma_list` of it is not empty (`wgpu_backend_env_is_effective`). An empty, blank,
  unrecognized or non-Unicode value keeps them. `WGPU_BACKEND=gl` after a glow failure can end in
  exit 101.
- A crash inside the graphics driver kills the process before any error is returned, so it cannot
  fall back. Observed: `LIBGL_ALWAYS_INDIRECT=1` under WSLg ends glow's GLX path in SIGSEGV (exit 139).
  The user starts with the other renderer forced.
- Under WSL every renderer uses X11 unless `VA_WAYLAND=1`. The hook is installed on every attempt,
  and eframe applies it when it creates its event loop on the first attempt. The X11 path needs
  `DISPLAY`: without it the launch is a platform failure (exit 3), even when `WAYLAND_DISPLAY` is set.

**WSLg.** With WSLg 1.0.73.2 (weston 9.0.0), wgpu on Wayland crashed the compositor in 3 of 5
launches, closing every WSLg window. That is why the X11 default exists. On X11, 223 launches of the
shipped binary had 0 compositor crashes, and glow on Wayland was fine. WSLg renders in software
(llvmpipe/lavapipe). Native Linux Wayland was not measured.

**Windows release console output.** Release builds use the Windows GUI subsystem. Typed in a console,
`visual-assembler.exe --help` prints nothing, and PowerShell does not wait for it. Its output,
including every selection warning, the fallback warning and the exit-3 messages, appears only when
redirected: Git Bash, a pipe (`2>&1 | cat`), or xtask. Bench JSON still records the renderer, the
fallback and the test hook. From M4 the status bar shows a fallback and an active test hook. For
terminal work, use the debug build (`cargo run -p visual-assembler -- …`) or the `va` CLI.

**Not automated**: the window on screen, the bench numbers, the real fallback and the WSLg behaviour
need a display and a human (see `tests/README.md`). The G2 visual checklist is in
`docs/perf/M0-backend.md` §4, and the G2 launches of the shipped binary are listed in its §11.8.

## Dependencies
- Internal: none in M0 (allowed: every `crates/*` and `arch/*` package; never `va` or `xtask`).
- External: see `DEPENDANCE.md`; platform support in `IMPORT.md`.

## Tests / usage
```bash
cargo test -p visual-assembler --all-features
cargo run -p visual-assembler -- --version
make run                                                       # release build, opens the window (auto)
cargo run --release -p visual-assembler -- --renderer wgpu     # force wgpu for one run
cargo test -p visual-assembler --test fallback -- --ignored --test-threads=1
```
The last command needs a display and opens and closes windows; under WSLg, prefix it with
`env -u WAYLAND_DISPLAY`. Measurements go through `cargo xtask bench` (see `xtask/README.md` and
`bench/README.md`).
