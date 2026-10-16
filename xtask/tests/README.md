# xtask/tests

## Purpose
Tests of the task runner, one file per source module. They call the `xtask` library, or spawn
`cargo`, `git`, `sh` or Python where the behaviour is about those tools.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `bench_stats.rs` | test | Median, nearest-rank p95, min and max; empty and non-finite inputs |
| `bench_timestamp.rs` | test | `utc_compact` / `utc_iso`, including leap days and the year 2100 |
| `bench_report.rs` | test | Report file name, markdown header, the `va.bench.report/2` document and its key order; `renderer` taken from the samples (`mixed`, `unknown`), the `fallbacks` count, `"binary": "shipped"`, the eight-variable `env` map; idle `spawn_to_first_frame_ms` is `null` |
| `bench_runner.rs` | test | Shipped-binary cargo arguments, artifact parsing, GUI arguments (`--renderer <r>` first), `--renderer` values and defaults, deadlines, run-line detection and validation (forced runs never fall back; `auto` accepts any renderer), output caps; on Unix, the run engine against fake GUIs written in `sh` |
| `ci.rs` | test | The step order (step 0 first; glow-only and wgpu-only clippy once each), `check_build_env` (a variable set even to an empty value fails), the canonical `.cargo/config.toml`, `cargo xtask ci` stopping at step 0, the Windows parking path; in `ci.yml`, the renderer fallback smoke step equals the m0 §19.9 block, no `${{` sits in a `run:` value, and every action is SHA-pinned |
| `docs.rs` | test | `cargo xtask docs` and `scripts/gen_agencement.py` (LF output, git-aware walk), run in fresh git repositories; `check_project_docs.py` manifest checks: implicit members, `[patch]`/`[replace]`, and the reverse workspace-dependency check (an unused pin, a pin used only in a target table, an unused path pin, sorted once per key, root package tables, skipped when a member table is unusable); the AGENCEMENT annotation warning with its paths (at most 10) and the `--strict` exit codes |
| `hooks.rs` | test | `cargo xtask hooks [--force]` in temporary git repositories, and the `commit-msg` hook run through `sh` and `git hook run`, under several UTF-8 locales |
| `layering.rs` | test | Layering rules and GUI families on synthetic `cargo metadata`; source rules S1, S3–S9 on synthetic trees, one regression test per known bypass |

## Entry points
- `layering.rs` and `ci.rs` hold the security-relevant regression tests.

Rules for every test here (architecture §8, m0-foundations.md §17.9):
- Temporary folders are `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>`, created with `std::fs::create_dir`.
  Never use `std::env::temp_dir` or `create_dir_all`.
- `git` must be installed. On Windows, the hook-behaviour tests print `skipped: sh not found` when no
  `sh` is on `PATH`; on Unix that is a failure.
- `layering.rs` has 3 ignored tests. `live_workspace_satisfies_layering_and_source_rules` checks the
  real workspace (`cargo xtask ci` runs the same check). The two others record accepted tripwire
  limits (§18.1) and **fail by design**, so always run the ignored live test by name.

## Dependencies
- Internal: the `xtask` library.
- External: `clap` (`bench_runner.rs`), `serde` (`bench_report.rs`), `serde_json`; tools `git`,
  `sh`, Python 3.11+.

## Tests / usage
```bash
cargo test -p xtask
cargo test -p xtask --test hooks
cargo test -p xtask --test layering -- --ignored live_workspace --nocapture
```
