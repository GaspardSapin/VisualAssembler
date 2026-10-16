# bench

## Purpose
Local inputs and outputs of the performance bench. Only this README is committed: everything the
bench writes is machine-specific, and lives in gitignored sub-folders.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `results/` | folder (gitignored) | `va.bench.report/2` reports written by `cargo xtask bench` (shipped binary; requested and used renderer, fallback count), named `<YYYYMMDDTHHMMSSZ>-<sha8>.json` (`nogit` before the first commit). Reports never overwrite each other. |
| `generated/` | folder (gitignored, from M4) | Generated 5,000- and 100,000-line projects for the memory and large-file benches |

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
Both need a display and open the window repeatedly. The second one is run N1 of
`docs/perf/M0-backend.md` §11 (glow, wgpu, then the shipped `auto`). Under WSL, export
`CARGO_TARGET_DIR=$HOME/.cache/visualassembler/target` first; the GUI then uses X11 by default, and
`VA_WAYLAND=1` must never be combined with a wgpu run (see `apps/visual-assembler/README.md`).
