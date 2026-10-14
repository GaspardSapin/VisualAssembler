# apps/visual-assembler/tests

## Purpose
Tests of the GUI package. None of them opens a window or needs a GPU, so they run on headless CI
runners. They go through the library's public API, or spawn the debug binary, which uses the
console subsystem on Windows.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `binary.rs` | test | Spawns `visual-assembler`: `--version`, `--help`, usage errors (exit 2), the exact `resolve` messages, non-Unicode arguments, exit 1 when clap's message cannot be written (closed pipes; `/dev/full` on Linux), exit 3 when no display exists (Linux) |
| `cli.rs` | test | `GuiArgs::try_parse_from` + `resolve`: default renderer per build, renderer not compiled in, idle duration rules (1 to 3600 s, default 10), repeated or malformed flags |
| `spike.rs` | test | `egui_kittest` over the AccessKit tree: the title and subtitle are shown, and the static UI schedules no repaint (reactive mode) |
| `bench_report.rs` | test | The `va.bench.run/1` line: one line, the 18 keys in order, `null` for `None`, the spec example reproduced byte for byte |
| `bench_process.rs` | test | `parse_vm_hwm` edge cases, and `ProcessStats::sample()` on the real process. This exercises the unsafe OS code on every CI runner. |
| `bench_recorder.rs` | test | Timing constants and `watchdog_limit` (60 s for startup; 60 + N + 3 s for idle) |
| `renderer_backend.rs` | test | Guards the Windows DX12 pin: eframe's default `WgpuSetup` is `CreateNew`, DX12 is compiled into the Windows build, and backend names stay stable |

## Entry points
- `binary.rs` for the command-line contract, `spike.rs` for the UI.

**Not automated** (m0-foundations.md §11). Each needs a display, a GPU or a human:
- the real window on screen (title readable; move, resize, minimize, restore, close; no flicker);
- the bench numbers;
- the WSLg behaviour.

They are covered by the renderer protocol (§12, §17.13) and the G2 checklist in
`docs/perf/M0-backend.md` §4, which is still pending a human.

m0-foundations.md §19.3 adds headless renderer-selection tests to `cli.rs`, `binary.rs` and
`bench_report.rs`, and a `fallback.rs` whose tests are `#[ignore]` because they need a display. None
of them is written yet (fix-wave row 19-2).

## Dependencies
- Internal: the `visual_assembler` library and the `visual-assembler` binary of this package.
- External: `egui_kittest` (dev-dependency, without its `wgpu` and `snapshot` features), `clap`,
  `serde_json`, `eframe`.

## Tests / usage
```bash
cargo test -p visual-assembler --all-features
cargo test -p visual-assembler --test spike
```
