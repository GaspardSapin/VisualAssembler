# xtask/src/bench

## Purpose
`cargo xtask bench`: build the shipped GUI binary once, run it many times in bench mode with each
requested renderer choice, aggregate the JSON lines and write a `va.bench.report/3` report
(m0-foundations.md §6.1 and §6.2, amended by §17.8, §19.5 and §19.9 WP4-1…WP4-6;
m4-editor-shell.md §7). The measurement itself happens inside the GUI
(`apps/visual-assembler/src/bench/`); this folder only generates the inputs, orchestrates, times
from the outside, and computes statistics.

**Version 3 (M4 WP-H, commit `8f60ccf`)** adds the editor scenarios `open`, `scroll` and `type`,
the generated inputs and the isolated profile, and expects `va.bench.run/3` lines. Decided, not
implemented yet: the GUI side (`--bench open|scroll|type`, `--bench-input`, the event recorder of
m4 §6.8) is M4 WP-G. Until then the GUI still prints `va.bench.run/2` lines, so every
`cargo xtask bench` run, `startup` included, aborts with "printed no va.bench.run/3 line".

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | `BenchArgs` (clap) and `run`: `Scenario` (`startup`, `idle`, `open`, `scroll`, `type`; `all` runs them in that order) and its input, defaults (scenario `all`, renderer `auto`, 20 startup / 3 idle / 5 open / 3 scroll / 3 type runs, 1 warm-up, 10 s idle; 300 scroll and 500 type events), `RendererChoice` / `RendererSelection` (`all` = glow, wgpu, then auto), git sha and dirty flag, host detection (`is_wsl`), progress lines that name the used renderer and any fallback, report writing and printing |
| `generate.rs` | module | The generated inputs under `bench/generated/`: `100k.asm` (100,000 lines, for `open`, `scroll`, `type`) and the `5k/` project (`main.asm` plus 9 parts of 500 lines, and `.visualasm/project.toml`, for `idle`), from 20 template lines of our own; LF only, deterministic; `ensure_inputs` regenerates them when `VERSION` differs or a file is missing |
| `runner.rs` | module | Builds the shipped GUI with `cargo build --release -p visual-assembler --message-format=json-render-diagnostics` (default features, both renderers), copies it to `<target>/bench/shipped/`, creates one `BenchProfile` per command (`<target>/bench/profile-<utc>-<pid>/`, passed as `VA_CONFIG_DIR` and `VA_DATA_DIR`), then spawns each run with `--renderer <r> --bench <scenario> [--bench-input <path>] [--duration-secs N]`: deadlines 75 s (startup, open, scroll, type) and N + 75 s (idle), output readers joined within 5 s, stderr tail of 64 KiB, stdout lines capped at 1 MiB. `validate_run` checks each line against the run that was asked for. |
| `stats.rs` | module | `summarize`: median, nearest-rank p95, min and max; `None` on empty input or any non-finite value. `summarize_pooled`: median, p95, max and count of the pooled event samples |
| `report.rs` | module | The report structs in normative key order (`REPORT_SCHEMA = "va.bench.report/3"`, `RUN_SCHEMA = "va.bench.run/3"`; each result names its input, and the summary adds `open_ms` and the pooled `event_ms`), the markdown table, `report_file_name` and `write_report` (`create_new`, never overwrites; `-2`, `-3`, … on collision) |
| `timestamp.rs` | module | `utc_compact` (`YYYYMMDDTHHMMSSZ`) and `utc_iso`, with no date crate |

## Entry points
- `mod.rs` → `run`, then `runner::build` and `runner::run_once`.
- `report::Report`: the document that `docs/perf/` reports cite.

Behaviour worth knowing:
- `spawn_to_first_frame_ms` (from `Command::spawn` to the receipt of the GUI's JSON line) is the
  gating startup number. It is `null` for every scenario but startup (§17.8).
- Run-line validation: `scenario` and `renderer_requested` must match the run. A forced `glow` or
  `wgpu` must report that renderer and `fallback_from: null`. Under `auto` any renderer is valid, and
  a fallback is counted, not rejected. `input` must name the requested input; `open`, `scroll` and
  `type` need an `open_ms` and `vsync: false`; `scroll` and `type` need 300 and 500 events.
- Each result has `renderer_requested`, `renderer` (the one the samples used, `mixed` when they
  differ, `unknown` with no sample), `fallbacks` (samples with a non-null `fallback_from`) and
  `"binary": "shipped"` with `binary_bytes`.
- A failed generation or build, a non-zero exit, a missing or invalid line or a timeout aborts the whole command
  with exit 1, and no report is written.
- Apart from `VA_CONFIG_DIR` and `VA_DATA_DIR` (the isolated profile, so user settings never
  affect a report), the parent environment reaches the GUI unchanged: `WGPU_BACKEND=…`, `VA_WAYLAND=1`,
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
cargo test -p xtask --test bench_stats --test bench_timestamp --test bench_report --test bench_runner --test bench_generate
make bench SCENARIO=startup RUNS=3
```
The second command needs a display and opens the window 4 times (1 warm-up + 3 runs) with `auto`.
Until M4 WP-G it stops at the warm-up run with exit 1 and writes no report (see above).
