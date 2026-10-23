# docs/perf

## Purpose
Measurement reports. A performance claim anywhere in the repository must cite a report in this
folder (architecture §7). Reports give the exact commands, the machine and power state, the numbers
and the decision taken. Raw bench outputs stay in `bench/results/` (gitignored) and are cited by file
name only.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `M0-backend.md` | report | The M0 renderer measurements (protocol m0-foundations.md §12 and §17.13): glow and wgpu on Windows, where the measured rule chose wgpu pinned to DX12; under WSLg, wgpu works on X11 while wgpu on Wayland crashed the compositor (outcome W proposed). **That decision is superseded** by the user's decision of 2026-10-14 (m0 §19, architecture §9.8: glow by default, automatic fallback to wgpu, X11 under WSL); the measurements stay valid. Also holds the gate table, startup analysis, cold-start and G2 checklists (pending a human), and the E3/E4 toolchain evidence. §11 records the decision and round 3 (runs N1–N6) on the shipped dual-renderer binary: `auto` runs glow, the fallback cost, a genuine OpenGL failure under WSLg, and exit 3 when both renderers fail. |
| `2026-10-23-m1-lexer.md` | report | M1 exit criterion E7: `make bench-lexer` on 100,000 generated lines at `46a0559`, on mains with "Best performance": median 7.157 ms (463.2 MB/s) against the §5.6 budget of ≤ 50 ms, 6.99× headroom (pass); also the WSL figure (7.104 ms), the first-run-after-idle spread and a rough M4 keystroke estimate. |

## Entry points
- `M0-backend.md` §11 (the decision and the shipped-binary numbers), then §5 (gate table) and §3
  (single-renderer results of rounds 1–2). §6 records the superseded measured decision; the current
  decision is m0-foundations.md §19.
- `2026-10-23-m1-lexer.md`: the verdict table at the top, then §3 (runs) and §7 (how to reproduce).

Naming: milestone decision reports are `M<n>-<topic>.md`. Optimization reports are
`YYYY-MM-DD-<topic>.md`, with before/after numbers and the exact command (architecture §7).

## Dependencies
- Internal: numbers come from `cargo xtask bench` (`xtask/src/bench/`), the GUI's `--bench` mode
  (`apps/visual-assembler/src/bench/`) and the ignored lexer bench
  (`arch/m68k/tests/syntax_bench.rs`, run by `make bench-lexer`).
- External: none.

## Tests / usage
Reports are not executable. To reproduce run N1 of `M0-backend.md` §11 (shipped binary: glow, wgpu,
then `auto`), and the M1 lexer measurement:

```bash
cargo xtask bench all --renderer all
make bench-lexer
```
Measure on mains power in "Best performance" mode: the M0 report shows about +33 % startup time on
battery.
