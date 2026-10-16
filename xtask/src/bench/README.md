# xtask/src/bench

## Purpose
`cargo xtask bench`: build the shipped GUI binary once, run it many times in bench mode with each
requested renderer choice, aggregate the JSON lines and write a `va.bench.report/2` report
(m0-foundations.md §6.1 and §6.2, amended by §17.8, §19.5 and §19.9 WP4-1…WP4-6). The measurement
itself happens inside the GUI (`apps/visual-assembler/src/bench/`); this folder only orchestrates,
times from the outside, and computes statistics.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | `BenchArgs` (clap) and `run`: defaults (scenario `all`, renderer `auto`, 20 startup / 3 idle runs, 1 warm-up, 10 s idle), `RendererChoice` / `RendererSelection` (`all` = glow, wgpu, then auto), git sha and dirty flag, host detection (`is_wsl`), progress lines that name the used renderer and any fallback, report writing and printing |
| `runner.rs` | module | Builds the shipped GUI with `cargo build --release -p visual-assembler --message-format=json-render-diagnostics` (default features, both renderers), copies it to `<target>/bench/shipped/`, then spawns each run with `--renderer <r> --bench <scenario> [--duration-secs N]`: deadlines 75 s (startup) and N + 75 s (idle), output readers joined within 5 s, stderr tail of 64 KiB, stdout lines capped at 1 MiB. `validate_run` checks each line against the run that was asked for. |
| `stats.rs` | module | `summarize`: median, nearest-rank p95, min and max; `None` on empty input or any non-finite value |
| `report.rs` | module | The report structs in normative key order (`REPORT_SCHEMA = "va.bench.report/2"`, `RUN_SCHEMA = "va.bench.run/2"`), the markdown table, `report_file_name` and `write_report` (`create_new`, never overwrites; `-2`, `-3`, … on collision) |
| `timestamp.rs` | module | `utc_compact` (`YYYYMMDDTHHMMSSZ`) and `utc_iso`, with no date crate |

## Entry points
- `mod.rs` → `run`, then `runner::build` and `runner::run_once`.
- `report::Report`: the document that `docs/perf/` reports cite.

Behaviour worth knowing:
- `spawn_to_first_frame_ms` (from `Command::spawn` to the receipt of the GUI's JSON line) is the
  gating startup number. It is `null` for idle results (§17.8).
- Run-line validation: `scenario` and `renderer_requested` must match the run. A forced `glow` or
  `wgpu` must report that renderer and `fallback_from: null`. Under `auto` any renderer is valid, and
  a fallback is counted, not rejected.
- Each result has `renderer_requested`, `renderer` (the one the samples used, `mixed` when they
  differ, `unknown` with no sample), `fallbacks` (samples with a non-null `fallback_from`) and
  `"binary": "shipped"` with `binary_bytes`.
- A failed build, a non-zero exit, a missing or invalid line or a timeout aborts the whole command
  with exit 1, and no report is written.
- The parent environment reaches the GUI unchanged: `WGPU_BACKEND=…`, `VA_WAYLAND=1`,
  `VA_TEST_FAIL_RENDERER=glow` or `env -u WAYLAND_DISPLAY` select a backend, a window system or the
  fallback path. The report's `env` map records `WGPU_BACKEND`, `WGPU_ADAPTER_NAME`,
  `WGPU_POWER_PREF`, `WAYLAND_DISPLAY`, `DISPLAY`, `VA_RENDERER`, `VA_TEST_FAIL_RENDERER` and
  `VA_WAYLAND`. xtask does not warn about `VA_RENDERER`: the `--renderer` it always passes overrides it.
- Under WSL the GUI uses X11 by default; never combine `VA_WAYLAND=1` with a wgpu run (WSLg
  compositor crash).
- Reports go to `bench/results/<YYYYMMDDTHHMMSSZ>-<sha8>.json` (gitignored). `sha8` is `nogit`
  until the first commit.
- Copies left under `<target>/bench/wgpu/` and `<target>/bench/glow/` by the pre-§19 runner are
  never read; `make clean` removes them.

## Dependencies
- Internal: `../exec.rs`, `../locate.rs`. No workspace crate (rule R5): the GUI is spawned as a process.
- External: `anyhow`, `clap`, `serde`, `serde_json`.

## Tests / usage
```bash
cargo test -p xtask --test bench_stats --test bench_timestamp --test bench_report --test bench_runner
make bench SCENARIO=startup RUNS=3
```
The second command needs a display, opens the window 4 times (1 warm-up + 3 runs) with `auto`, and
writes a report.
