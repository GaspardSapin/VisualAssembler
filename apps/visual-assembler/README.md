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
| `Cargo.toml` | manifest | Features `renderer-wgpu` (today's only default) and `renderer-glow`; m0 §19 makes both default (not implemented yet). Dependencies `eframe`, `clap`, `serde`, `serde_json`, `raw-window-handle`; `windows-sys` on Windows, `libc` on Linux; dev-dependency `egui_kittest` |
| `src/` | folder | Library, binary, command line, window and bench. See its README. |
| `tests/` | folder | Headless tests, plus the list of what is not automated. See its README. |

## Entry points
`src/lib.rs` (`run`). Command line:

```text
visual-assembler [--renderer <wgpu|glow>] [--bench <startup|idle>] [--duration-secs <N>]
visual-assembler --help | --version        # version line: "visual-assembler 0.1.0"
```

| Exit code | Meaning (m0-foundations.md §17.10) |
|-----------|------------------------------------|
| 0 | Normal exit, or a bench scenario completed |
| 1 | Runtime I/O or infrastructure failure: clap's help, version or error text cannot be written; a bench thread cannot be created; the bench JSON line cannot be written or flushed |
| 2 | Usage error (clap errors; a renderer not compiled in; `--duration-secs` without `--bench idle`) |
| 3 | The window or the renderer cannot be created |
| 4 | Bench watchdog: no measurement within the hard limit |

m0 §19.4 keeps these codes. It adds the fallback warning and the "no renderer could be initialized"
and forced-renderer messages, both with exit 3 (root README, "Renderer selection").

**Renderer, decided policy (not implemented yet).** The user decided on 2026-10-14 (m0-foundations.md
§19, architecture §9.8): both renderers in the shipped binary, `auto` = glow then wgpu (wgpu then glow
on Windows aarch64), one fallback on a renderer initialization error before the first frame, no
fallback for a forced renderer, `--renderer` > `VA_RENDERER` > setting (M4) > `auto`, X11 under WSL
unless `VA_WAYLAND=1`, and the `VA_TEST_FAIL_RENDERER` test hook. This replaces the earlier measured
choice of wgpu and outcome W of `docs/perf/M0-backend.md` §6. The user-facing description is the
root README, section "Renderer selection". The code lands with fix-wave row 19-2 (m0 §19.8).

**Renderer, current build.** wgpu is the only default feature. `--renderer` accepts `wgpu` or `glow`;
the default is wgpu when it is compiled in, else glow, and a renderer that is not compiled in is a
usage error. There is no `auto` value and no fallback. On Windows, when `WGPU_BACKEND` is unset, the
app restricts wgpu to DX12 (this pin stays under §19); set `WGPU_BACKEND` (for example `vulkan`) to
let wgpu choose. `cargo xtask ci` step 3 lints the glow-only build. A build needs at least one
renderer feature.

**WSLg limitation.** With WSLg 1.0.73.2 (weston 9.0.0), wgpu on Wayland crashed the compositor in 3 of
5 launches, closing every WSLg window. X11 worked in 59 of 59 launches, and glow on Wayland was fine.
m0 §19.1 decides that under WSL every renderer uses X11 unless `VA_WAYLAND=1`; that is not
implemented yet. Until it is, launch the GUI under WSLg with `env -u WAYLAND_DISPLAY` so winit uses
X11. WSLg renders in software (llvmpipe/lavapipe). Native Linux Wayland was not measured.

**Windows release console output.** Release builds use the Windows GUI subsystem. Typed in a console,
`visual-assembler.exe --help` prints nothing, and PowerShell does not wait for it. Its output
appears only when redirected: Git Bash, a pipe, or xtask. For terminal work, use the debug build
(`cargo run -p visual-assembler -- …`) or the `va` CLI.

**Not automated**: the window on screen, the bench numbers and the WSLg behaviour need a display and a
human (see `tests/README.md`). The G2 visual checklist is in `docs/perf/M0-backend.md` §4.

## Dependencies
- Internal: none in M0 (allowed: every `crates/*` and `arch/*` package; never `va` or `xtask`).
- External: see `DEPENDANCE.md`; platform support in `IMPORT.md`.

## Tests / usage
```bash
cargo test -p visual-assembler --all-features
cargo run -p visual-assembler -- --version
make run                                  # release build, opens the window
env -u WAYLAND_DISPLAY make run           # WSLg: force X11
cargo run --release -p visual-assembler --no-default-features --features renderer-glow
```
The last command is the G2 launch of the glow build (§12.2). Measurements go through
`cargo xtask bench` (see `xtask/README.md` and `bench/README.md`).
