# bench

## Purpose
Local inputs and outputs of the performance bench. Only this README is committed: everything the
bench writes is machine-specific, and lives in gitignored sub-folders.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `results/` | folder (gitignored) | `va.bench.report/1` reports written by `cargo xtask bench`, named `<YYYYMMDDTHHMMSSZ>-<sha8>.json` (`nogit` before the first commit). Reports never overwrite each other. |
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
cargo xtask bench all --renderer wgpu
```
Both need a display and open the window repeatedly. Under WSL, export
`CARGO_TARGET_DIR=$HOME/.cache/visualassembler/target` first, and prefix the command with
`env -u WAYLAND_DISPLAY` until the X11-under-WSL rule of m0 §19.1 lands (see
`apps/visual-assembler/README.md`).
