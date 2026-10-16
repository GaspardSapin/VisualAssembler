# apps/visual-assembler/src

## Purpose
Sources of the GUI package: a thin binary (`main.rs`) over a library (`visual_assembler`) that holds
the command line, the renderer selection, the window and the bench, so all of them are testable
without a window. In M0 the window shows a static text; the editor, assembler and debugger arrive in
later milestones.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `main.rs` | binary root | First statement `let started = Instant::now();`, then `visual_assembler::run(started)`. Uses the Windows GUI subsystem in release builds. |
| `lib.rs` | library root | `run` (parse, resolve, select, launch), the exit-code constants, the in-process fallback loop (`launch`), the injection-aware app creator, `native_options` (title `VisualAssembler`, app id `visualassembler`, 1280×800 logical px, centered, plus the per-attempt renderer settings), `wgpu_backend_env_is_effective`, the Windows DX12 pin, the wgpu GL exclusion after a glow failure, the X11 hook under WSL, and a `compile_error!` when no renderer feature is enabled |
| `cli.rs` | module | `GuiArgs` (clap), `RendererChoice` (`auto`/`glow`/`wgpu`), `Renderer`, `BenchScenario`, `compiled_renderers()`, `resolve()` → `LaunchPlan` / `BenchPlan`, `UsageError`; and the pure selection rules: `resolve_renderer`, `auto_order`, `parse_injected_failures`, `window_system_policy`, `is_fallback_eligible`, `classify_failure` with the exact §19.4 messages |
| `spike.rs` | module | `WINDOW_TITLE`, `SUBTITLE`, `draw(ui)` and `SpikeApp` (the `eframe::App`), which sets the "first frame started" flag at the top of its first `ui`. Reactive mode: the normal path never requests a repaint. |
| `bench/` | folder | The `--bench startup\|idle` instrumentation. See its README. |

## Entry points
- `lib.rs` → `run`, then `select` (reads `VA_RENDERER`, `VA_TEST_FAIL_RENDERER`, `VA_WAYLAND` and
  prints their warnings), then `launch` (one `eframe::run_native` per attempt).
- `cli::resolve_renderer` and `cli::classify_failure`: every renderer rule, testable without a window.
- `lib::native_options(renderer, force_x11, after_glow_failure)`: the eframe options of one attempt,
  public so tests check them without a window.
- `spike::draw`: what the window shows; `tests/spike.rs` drives it headless.

Exit-code constants (m0-foundations.md §17.10, §19.4): `EXIT_RUNTIME = 1`, `EXIT_USAGE = 2`,
`EXIT_WINDOW = 3`, `EXIT_BENCH_TIMEOUT = 4`. The meaning of each is in `../README.md`.

wgpu backend rules (m0 §19.9 WP2-1, S19-4):
- `pin_windows_wgpu_backend` (Windows, `renderer-wgpu`): restricts every wgpu attempt to DX12. wgpu's
  own pick on the reference machine was Vulkan, which failed the idle gate
  (docs/perf/M0-backend.md §6.1).
- `exclude_wgpu_gl_backend` (both renderers compiled): removes `Backends::GL` from a wgpu attempt that
  follows a failed glow attempt, because the failed GL context stays current on the thread.
- Both step aside only when `wgpu_backend_env_is_effective(WGPU_BACKEND)` is true: the value is
  valid Unicode and `Backends::from_comma_list` of it is not empty.

The English texts of `spike.rs` are a documented M0 exception to the i18n rule; M4 replaces them
with Fluent message ids.

## Dependencies
- Internal: none in M0. The app may use every `crates/*` and `arch/*` package, never `va` or
  `xtask` (§17.2).
- External: `eframe`, `clap`, `serde`, `serde_json`, `raw-window-handle`, `windows-sys` (Windows),
  `libc` and `winit` (Linux; winit only for `EventLoopBuilderExtX11::with_x11`). See `DEPENDANCE.md`.

## Tests / usage
```bash
cargo test -p visual-assembler --all-features
cargo run -p visual-assembler -- --help
```
