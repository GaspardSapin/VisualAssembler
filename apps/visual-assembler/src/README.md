# apps/visual-assembler/src

## Purpose
Sources of the GUI package: a thin binary (`main.rs`) over a library (`visual_assembler`) that holds
the command line, the window and the bench, so all of them are testable without a window. In M0 the
window shows a static text; the editor, assembler and debugger arrive in later milestones.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `main.rs` | binary root | First statement `let started = Instant::now();`, then `visual_assembler::run(started)`. Uses the Windows GUI subsystem in release builds. |
| `lib.rs` | library root | `run` (parse, resolve, launch), the exit-code constants, the eframe options (title `VisualAssembler`, app id `visualassembler`, 1280×800 logical px, centered), `pin_windows_wgpu_backend`, and a `compile_error!` when no renderer feature is enabled |
| `cli.rs` | module | `GuiArgs` (clap), `RendererChoice`, `BenchScenario`, `compiled_renderers()`, `resolve()` → `LaunchPlan` / `BenchPlan`, and `UsageError` |
| `spike.rs` | module | `WINDOW_TITLE`, `SUBTITLE`, `draw(ui)` and `SpikeApp` (the `eframe::App`). Reactive mode: the normal path never requests a repaint. |
| `bench/` | folder | The `--bench startup\|idle` instrumentation. See its README. |

## Entry points
- `lib.rs` → `run`, then `launch`.
- `cli::resolve`: every rule that depends on the build or on several flags together.
- `spike::draw`: what the window shows; `tests/spike.rs` drives it headless.

Exit-code constants (m0-foundations.md §17.10): `EXIT_RUNTIME = 1`, `EXIT_USAGE = 2`,
`EXIT_WINDOW = 3`, `EXIT_BENCH_TIMEOUT = 4`. The meaning of each is in `../README.md`.

`pin_windows_wgpu_backend` (Windows and `renderer-wgpu` only): when `WGPU_BACKEND` is unset, it
restricts wgpu to DX12. wgpu's own pick on the reference machine was Vulkan, which failed the idle
gate (docs/perf/M0-backend.md §6.1). Setting `WGPU_BACKEND` restores wgpu's own choice.

m0-foundations.md §19 adds `RendererChoice::Auto`, `resolve_renderer` (precedence and per-target
order), the in-process fallback loop, the X11-under-WSL hook and the `VA_TEST_FAIL_RENDERER` hook.
None of it is implemented yet (fix-wave row 19-2); the files above describe the current code.

The English texts of `spike.rs` are a documented M0 exception to the i18n rule; M4 replaces them
with Fluent message ids.

## Dependencies
- Internal: none in M0. The app may use every `crates/*` and `arch/*` package, never `va` or
  `xtask` (§17.2).
- External: `eframe`, `clap`, `serde`, `serde_json`, `raw-window-handle`, `windows-sys` (Windows),
  `libc` (Linux). See `DEPENDANCE.md`.

## Tests / usage
```bash
cargo test -p visual-assembler --all-features
cargo run -p visual-assembler -- --help
```
