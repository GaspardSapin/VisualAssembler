# docs/perf

## Purpose
Measurement reports. A performance claim anywhere in the repository must cite a report in this
folder (architecture §7). Reports give the exact commands, the machine and power state, the numbers
and the decision taken. Raw bench outputs stay in `bench/results/` (gitignored) and are cited by file
name only.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `M0-backend.md` | report | The M0 renderer measurements (protocol m0-foundations.md §12 and §17.13): glow and wgpu on Windows, where the measured rule chose wgpu pinned to DX12; under WSLg, wgpu works on X11 while wgpu on Wayland crashed the compositor (outcome W proposed). **That decision is superseded** by the user's decision of 2026-10-14 (m0 §19, architecture §9.8: glow by default, automatic fallback to wgpu, X11 under WSL); the measurements stay valid. Also holds the gate table, startup analysis, cold-start and G2 checklists (pending a human), and the E3/E4 toolchain evidence. m0 §19.7 asks for a new §11 with runs N1–N6 on the shipped dual-renderer binary; it is not written yet. |

## Entry points
- `M0-backend.md` §5 (gate table) and §3 (results). §6 records the superseded measured decision; the
  current decision is m0-foundations.md §19.

Naming: milestone decision reports are `M<n>-<topic>.md`. Optimization reports are
`YYYY-MM-DD-<topic>.md`, with before/after numbers and the exact command (architecture §7).

## Dependencies
- Internal: numbers come from `cargo xtask bench` (`xtask/src/bench/`) and the GUI's
  `--bench` mode (`apps/visual-assembler/src/bench/`).
- External: none.

## Tests / usage
Reports are not executable. To reproduce one configuration of `M0-backend.md` (Windows, plain wgpu):

```bash
cargo xtask bench all --renderer wgpu
```
Measure on mains power in "Best performance" mode: the M0 report shows about +33 % startup time on
battery.
