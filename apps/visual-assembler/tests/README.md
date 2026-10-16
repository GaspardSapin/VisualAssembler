# apps/visual-assembler/tests

## Purpose
Tests of the GUI package. Every file except `fallback.rs` runs headless, with no window and no GPU,
so it runs on the CI runners. These tests go through the library's public API, or spawn the debug
binary, which uses the console subsystem on Windows. `fallback.rs` opens real windows, so its tests
are `#[ignore]`d and run locally.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `binary.rs` | test | Spawns `visual-assembler`: `--version`, `--help` (renderer values), usage errors (exit 2) before any window, the exact `resolve` messages, non-Unicode arguments, exit 1 when clap's message cannot be written (closed pipes; `/dev/full` on Linux). On Linux without a display: exit 3 with the window message and no fallback (`platform_failure_never_falls_back`), selection warnings first and in order, empty, non-Unicode and control-character selection variables (escaped), and the `VA_WAYLAND` warning following the plan |
| `cli.rs` | test | `GuiArgs::try_parse_from` + `resolve` (idle duration 1 to 3600 s, repeated or malformed flags); the pure selection layer: `renderer_values_parse`, `renderer_precedence`, `auto_order_per_target`, `injection_list_parses`, `fallback_eligibility`, every `classify_failure` message (one fallback, both renderers failing, forced, after the first frame), `window_system_policy`, and warnings that echo values escaped |
| `renderer_backend.rs` | test | `native_options` of every attempt, without a window: window settings, the Windows DX12 pin on every wgpu attempt, wgpu's GL backend left out after a glow failure, the X11 hook; `wgpu_backend_env_is_effective` (false for unset, empty, blank, `,`, unrecognized and non-Unicode values; true for `vulkan`, `gl`, `DX12`, ` vk , bogus`); eframe's default `WgpuSetup` and the backend names of the bench line |
| `spike.rs` | test | `egui_kittest` over the AccessKit tree: the title and subtitle are shown, and the static UI schedules no repaint (reactive mode) |
| `bench_report.rs` | test | The `va.bench.run/2` line: one line, the 22 keys in order, `null` for `None`, the §19.5 example reproduced byte for byte, escaped driver strings |
| `bench_process.rs` | test | `parse_vm_hwm` edge cases, and `ProcessStats::sample()` on the real process. This exercises the unsafe OS code on every CI runner. |
| `bench_recorder.rs` | test | Timing constants and `watchdog_limit` (60 s for startup; 60 + N + 3 s for idle) |
| `fallback.rs` | test | 10 ignored display tests of the renderer fallback (m0 §19.3, §19.9 WP2-10), below |

## Entry points
- `binary.rs` for the command-line contract, `cli.rs` for the renderer rules, `spike.rs` for the UI.

**Display tests (`fallback.rs`).** Each test spawns the GUI with `--bench startup` (or `idle`), so a
window opens and closes by itself; the interactive test kills its window after 2 s. Every run removes
`VA_RENDERER`, `VA_TEST_FAIL_RENDERER` and `VA_WAYLAND`, and under WSL also `WAYLAND_DISPLAY`.
- On Windows aarch64, `injected_wgpu_failure_falls_back_to_glow` replaces
  `injected_glow_failure_falls_back_to_wgpu`, and the glow-first tests are compiled out (4 tests
  remain there; never run, no such machine).
- `wsl_defaults_to_x11_with_wayland_display_present` is Linux-only and passes as skipped outside
  WSLg, or when `WAYLAND_DISPLAY` is unset. It keeps `WAYLAND_DISPLAY` and forces glow, which never
  crashes the compositor; run it by name without the prefix to exercise it.
- Single-renderer builds compile the file to an empty test binary.
- **Rule (m0 §19.9 S19-5):** run them on Windows and under WSLg X11 whenever `Cargo.lock` changes
  eframe, egui, egui-wgpu, egui_glow, wgpu, glutin or winit, and before each milestone closes.
- Last run, 2026-10-16: 8 passed on Windows x64; 9 passed under WSLg X11 (the WSL test, run again
  by name with `WAYLAND_DISPLAY` set, also passed).

**Not automated** (m0-foundations.md §11). Each needs a display, a GPU or a human:
- the real window on screen (title readable; move, resize, minimize, restore, close; no flicker);
- the bench numbers;
- a genuine OpenGL failure (root README, "Renderer selection");
- the WSLg behaviour.

They are covered by the renderer protocol (§12, §17.13, §19.7) and the G2 checklist in
`docs/perf/M0-backend.md` §4, which is still pending a human.

## Dependencies
- Internal: the `visual_assembler` library and the `visual-assembler` binary of this package.
- External: `egui_kittest` (dev-dependency, without its `wgpu` and `snapshot` features), `clap`,
  `serde_json`, `eframe`.

## Tests / usage
```bash
cargo test -p visual-assembler --all-features
cargo test -p visual-assembler --test spike
cargo test -p visual-assembler --test fallback -- --ignored --test-threads=1
env -u WAYLAND_DISPLAY cargo test -p visual-assembler --test fallback -- --ignored --test-threads=1
cargo test -p visual-assembler --test fallback -- --ignored wsl_defaults_to_x11
```
The third command is the Windows form; the last two are the WSLg forms (CARGO_TARGET_DIR exported as
in the root README). All three open windows.
