# xtask/src

## Purpose
Sources of the task runner. `main.rs` only parses and dispatches; every command and check lives in
the `xtask` library so tests can call it directly.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `main.rs` | binary root | clap dispatch (`bin_name = "cargo xtask"`), switches to the workspace root, prints `error: {e:#}` and exits 1 on any error (`docs` forwards the check script's code) |
| `lib.rs` | library root | Declares the modules below |
| `ci.rs` | module | `cargo xtask ci`: the step list, step 0 (`check_build_env`, `check_cargo_config`, `CANONICAL_CARGO_CONFIG`), and parking the running `xtask.exe` on Windows |
| `docs.rs` | module | `cargo xtask docs [--check]`: runs `scripts/gen_agencement.py` (skipped with `--check`) then `scripts/check_project_docs.py`; picks the Python interpreter |
| `hooks.rs` | module | `cargo xtask hooks [--force]`: safe install of `scripts/git-hooks/commit-msg` |
| `layering.rs` | module | `check_layering` (crate map, GUI-crate families) and `scan_sources` (rules S1, S3–S9), plus `UNSAFE_ALLOWLIST`, `NET_ALLOWLIST` and `INCLUDE_ALLOWLIST` |
| `exec.rs` | module | Spawns external commands with an argument vector, never a shell; turns failures into errors with context |
| `locate.rs` | module | Workspace root, `cargo` program (`$CARGO`), `cargo metadata`, target directory |
| `bench/` | folder | `cargo xtask bench`. See its README. |

## Entry points
- `main.rs` for the command surface; `ci.rs::steps` for what CI checks; `layering.rs` for the
  architecture rules that tooling enforces.

The allow-lists in `layering.rs` are security-relevant: adding an entry needs Security's review and a
`SECURITY.md` record (architecture §9.3, §9.4).

## Dependencies
- Internal: none (rule R5).
- External: `anyhow`, `clap`, `serde`, `serde_json`.

## Tests / usage
```bash
cargo test -p xtask
cargo xtask --help
```
