# xtask/src/bench

## Purpose
`cargo xtask bench`: build one release GUI binary per renderer, run it many times in bench mode,
aggregate the JSON lines and write a `va.bench.report/1` report (m0-foundations.md §6.1 and §6.2,
amended by §17.8). The measurement itself happens inside the GUI (`apps/visual-assembler/src/bench/`);
this folder only orchestrates, times from the outside, and computes statistics.

**Decided, not implemented yet** (m0-foundations.md §19.5, fix-wave row 19-3): build the shipped
binary once, with both renderers, and pass `--renderer auto|glow|wgpu` to it (`all` = glow, wgpu,
then auto; default `auto`), with the `va.bench.run/2` and `va.bench.report/2` schemas. This README
describes the current runner.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | `BenchArgs` (clap) and `run`: defaults (scenario `all`, renderer `wgpu`, 20 startup / 3 idle runs, 1 warm-up, 10 s idle), git sha and dirty flag, host detection (`is_wsl`), report writing and printing |
| `runner.rs` | module | Build with `cargo build --release -p visual-assembler --no-default-features --features renderer-<r> --message-format=json-render-diagnostics`, copy to `<target>/bench/<r>/`, then spawn each run: deadlines 75 s (startup) and N + 75 s (idle), output readers joined within 5 s, stderr tail of 64 KiB, stdout lines capped at 1 MiB |
| `stats.rs` | module | `summarize`: median, nearest-rank p95, min and max; `None` on empty input or any non-finite value |
| `report.rs` | module | The report structs in normative key order, the markdown table, `report_file_name` and `write_report` (`create_new`, never overwrites; `-2`, `-3`, … on collision) |
| `timestamp.rs` | module | `utc_compact` (`YYYYMMDDTHHMMSSZ`) and `utc_iso`, with no date crate |

## Entry points
- `mod.rs` → `run`, then `runner::build` and `runner::run_once`.
- `report::Report`: the document that `docs/perf/` reports cite.

Behaviour worth knowing:
- `spawn_to_first_frame_ms` (from `Command::spawn` to the receipt of the GUI's JSON line) is the
  gating startup number. It is `null` for idle results (§17.8).
- A failed build, a non-zero exit, a missing line or a timeout aborts the whole command with exit 1,
  and no report is written.
- The parent environment reaches the GUI unchanged: `WGPU_BACKEND=…` or `env -u WAYLAND_DISPLAY`
  select a backend or a window system. The report records `WGPU_BACKEND`, `WGPU_ADAPTER_NAME`,
  `WGPU_POWER_PREF`, `WAYLAND_DISPLAY` and `DISPLAY`.
- Reports go to `bench/results/<YYYYMMDDTHHMMSSZ>-<sha8>.json` (gitignored). `sha8` is `nogit`
  until the first commit.

## Dependencies
- Internal: `../exec.rs`, `../locate.rs`. No workspace crate (rule R5): the GUI is spawned as a process.
- External: `anyhow`, `clap`, `serde`, `serde_json`.

## Tests / usage
```bash
cargo test -p xtask --test bench_stats --test bench_timestamp --test bench_report --test bench_runner
make bench SCENARIO=startup RUNS=3
```
The second command needs a display, opens the window 4 times (1 warm-up + 3 runs) and writes a report.
