# apps/visual-assembler/src/bench

## Purpose
Measurement instrumentation that runs inside the real GUI window. `visual-assembler --bench
startup|idle` runs one scenario, prints exactly one `va.bench.run/2` JSON line on stdout and exits.
This folder measures one run only. Building binaries, repeating runs, statistics and the report file
belong to `xtask/src/bench/` (`cargo xtask bench`). The normative algorithm is m0-foundations.md
§4.5, with the settle and watchdog values of §17.13 and the renderer keys of §19.5.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | Constants `IDLE_SETTLE_MS = 3000`, `DEFAULT_IDLE_SECS = 10`, `STARTUP_WATCHDOG_SECS = 60`, `IDLE_WATCHDOG_EXTRA_SECS = 60`; module declarations (the two `process_*` modules are private and cfg-gated) |
| `recorder.rs` | module | `BenchRecorder`: hooked into `App::ui` / `App::logic`; `SelectionFacts` (requested choice, fallback, raw `VA_TEST_FAIL_RENDERER`) and `record_fallback`; the watchdog thread (`watchdog_limit`: 60 s for startup, 60 + N + 3 s for idle, then `error: bench timed out` and exit 4); the `va-bench-idle` thread that waits 3 s, samples, sleeps N s and samples again |
| `report.rs` | module | `RUN_SCHEMA = "va.bench.run/2"` and `BenchRun`, whose 22 fields serialize in the normative key order (§19.5: the 18 keys of §4.6 plus `renderer_requested`, `fallback_from`, `fallback_reason`, `injected_failure`); `to_json_line` |
| `process.rs` | module | `ProcessStats { cpu_ms, peak_rss_bytes, peak_private_bytes }`, `ProcessStats::sample()` (all `None` on other OSes) and the portable `parse_vm_hwm` |
| `process_windows.rs` | module | `GetProcessTimes` and `GetProcessMemoryInfo` through `windows-sys` (peak working set, peak pagefile usage). Unsafe, allow-listed. |
| `process_linux.rs` | module | `libc::getrusage(RUSAGE_SELF)` and `VmHWM` from `/proc/self/status`. Unsafe, allow-listed. |
| `platform.rs` | module | `RendererInfo::from_frame` (renderer, backend, adapter, driver, window system from the raw window handle) and `is_wsl()`. Unsafe only in `gl_string`, the GL string query. |

## Entry points
- `recorder::BenchRecorder::new(plan, started, selection)`, `record_fallback`, `on_ui`, `on_logic`.
  `lib.rs` creates the recorder once, before the first renderer attempt, and lends it to each attempt,
  so a fallback neither drops it nor resets the watchdog, and `startup_ms` includes the fallback cost.
- `report::BenchRun`: the JSON contract shared with `xtask/src/bench/report.rs`.

Definitions (architecture §7):
- `startup_ms`: from the first statement of `main()` to the start of the first `App::logic` call
  after frame 1 was presented. Informational; the gating number is xtask's outer
  `spawn_to_first_frame_ms`.
- Idle window: starts `IDLE_SETTLE_MS` (3 s) after the first frame and lasts N s (default 10). The
  UI thread is never woken during the window. Idle CPU % is process CPU time over wall time, so 1 %
  means 1 % of one core.
- Exit codes: 0 when the line was printed; 1 when a bench thread cannot start or the line cannot be
  written; 4 when the watchdog fires first.

Unsafe code: these are the only three files of the workspace on `UNSAFE_ALLOWLIST`
(`xtask/src/layering.rs`). Every unsafe block carries a `// SAFETY:` comment, and every failed OS
query becomes `null` in the JSON line. See `SECURITY.md`, "Audited unsafe FFI in the bench".

## Dependencies
- Internal: `../cli.rs` (`BenchPlan`, `BenchScenario`, `Renderer`, `RendererChoice`), `../lib.rs`
  (exit codes, `report_error`). `lib.rs` uses `platform::is_wsl()` for the X11-under-WSL rule.
- External: `eframe` (egui context, frame, wgpu and glow handles), `raw-window-handle`,
  `serde` / `serde_json`, `windows-sys` (Windows only), `libc` (Linux only).

## Tests / usage
```bash
cargo test -p visual-assembler --test bench_process --test bench_recorder --test bench_report
```
A real measurement needs a display and opens the window for about a second (debug build, which
prints in any terminal):

```bash
cargo run -p visual-assembler -- --bench startup
```
Under WSL the app uses X11 by default, so no prefix is needed. The line names the renderer that ran
(`renderer`), the requested choice (`renderer_requested`) and any fallback (`fallback_from`,
`fallback_reason`).
