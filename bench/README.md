# bench

## Purpose
Local inputs and outputs of the performance bench. Only this README is committed: everything the
bench writes is machine-specific, and lives in gitignored sub-folders.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `results/` | folder (gitignored) | Reports written by `cargo xtask bench` (shipped binary; requested and used renderer, fallback count), named `<YYYYMMDDTHHMMSSZ>-<sha8>.json` (`nogit` before the first commit): `va.bench.report/2` up to M4, `va.bench.report/3` from M4 WP-H. Reports never overwrite each other. |
| `generated/` | folder (gitignored, M4 WP-H) | Inputs written by `cargo xtask bench` when a selected scenario needs them (`xtask/src/bench/generate.rs`): `100k.asm` (100,000 lines, for `open`, `scroll`, `type`), the `5k/` project (5,000 lines, for `idle`) and `VERSION`; regenerated when the generator version changes or a file is missing |

Both folders are also hidden from agents by `.claudeignore`: reports are large and only cited by name.

## Entry points
- The newest file in `results/`. A report that supports a claim is summarized in `docs/perf/`, which
  cites the file name.

## Dependencies
- Internal: written by `xtask/src/bench/`.
- External: none.

## Tests / usage
```bash
make bench SCENARIO=startup RUNS=3
cargo xtask bench all --renderer all
```
Both need a display and open the window repeatedly. Until M4 WP-G both stop at their first run
with exit 1: the GUI still prints `va.bench.run/2` lines and the runner expects
`va.bench.run/3` (see `xtask/src/bench/README.md`). The second one is run N1 of
`docs/perf/M0-backend.md` §11 (glow, wgpu, then the shipped `auto`). Under WSL, export
`CARGO_TARGET_DIR=$HOME/.cache/visualassembler/target` first; the GUI then uses X11 by default, and
`VA_WAYLAND=1` must never be combined with a wgpu run (see `apps/visual-assembler/README.md`).
