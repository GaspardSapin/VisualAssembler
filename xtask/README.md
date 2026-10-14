# xtask

## Purpose
The Rust task runner behind `make` (package `xtask`, run as `cargo xtask <command>` through the alias
in `.cargo/config.toml`). It holds every project command that needs logic: bench, CI checks, docs,
hook installation. It depends on no workspace crate (rule R5): it drives cargo, git, Python and the
built binaries as child processes, never through a shell.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Dependencies `anyhow`, `clap`, `serde`, `serde_json` (all `workspace = true`) |
| `src/` | folder | Command implementations and checks. See its README. |
| `tests/` | folder | One test file per module. See its README. |

## Entry points
| Command | Behaviour | Exit |
|---------|-----------|------|
| `cargo xtask bench [startup\|idle\|all] [--renderer wgpu\|glow\|all] [--runs N] [--warmup N] [--duration-secs N] [--json]` | Builds one release GUI per renderer, measures, writes `bench/results/<time>-<sha8>.json` (see `src/bench/README.md`). m0 §19.5 replaces this with the shipped dual-renderer binary and `--renderer auto\|glow\|wgpu\|all` (default `auto`); not implemented yet. | 0 ok · 1 build/run failure or timeout · 2 usage |
| `cargo xtask ci [--no-fmt]` | The CI check list, below | 0 / 1 |
| `cargo xtask docs [--check]` | `python scripts/gen_agencement.py .` (skipped with `--check`), then `python scripts/check_project_docs.py .`. Python is `$VA_PYTHON`, else `python` on Windows, else `python3`. | the check script's code; 1 if Python is missing |
| `cargo xtask hooks [--force]` | Installs `scripts/git-hooks/commit-msg` into this repository's hooks folder | 0 / 1 |

Usage errors exit 2 (clap). Any other error prints `error: <context chain>` on stderr and exits 1.

**`cargo xtask ci` steps**, in order, stopping at the first failure. `==> <step>` is printed on stderr
before each one:

0. Build configuration (rule R8). Fails when `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`,
   `CARGO_BUILD_RUSTFLAGS` or any `CARGO_TARGET_*_RUSTFLAGS` is **set, even empty**. Also fails when
   `.cargo/config.toml` differs from `CANONICAL_CARGO_CONFIG` in `src/ci.rs` after blank lines,
   comments and whitespace are dropped, or when an extension-less `.cargo/config` exists. Always runs,
   also with `--no-fmt`.
1. `cargo fmt --all -- --check` (skipped with `--no-fmt`; CI runs fmt in its own job).
2. `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
3. `cargo clippy -p visual-assembler --all-targets --no-default-features --features renderer-glow --locked -- -D warnings`.
4. Layering and source checks, in process (`src/layering.rs`).
5. `cargo test --workspace --all-features --locked`.

m0-foundations.md §19.6 adds a step 3b, the same clippy run with `--features renderer-wgpu` only.
It is not implemented yet (fix-wave row 19-3).

On Windows, before step 5, the running `xtask.exe` is renamed to `xtask.exe.old` so cargo can relink
it (§17.11 D09). A second `cargo xtask ci` started from the same target folder while one is still
running fails at that rename.

**`cargo xtask hooks` safety** (§17.7): a `core.hooksPath` outside this repository is refused, even
with `--force`. An identical hook prints `already installed`. A different file is left untouched
unless `--force` is given; `--force` first renames it to `commit-msg.bak.<UTC time>`. `make hooks`
never passes `--force`.

**Threat model of the checks** (m0-foundations.md §18.1, binding). The source scan and step 0 are
*tripwires* against accidental or careless network code, unsafe code and build-flag overrides. They
are not a sandbox against deliberate obfuscation: renamed macros, raw identifiers, out-of-tree target
paths, crate aliasing. A bypass is a defect only if it is a realistic accidental pattern that compiles
and nothing catches. Deliberate ones are recorded as accepted limitations in `SECURITY.md`. Code
review, cargo-deny and branch protection cover them.

## Dependencies
- Internal: none (rule R5).
- External: `anyhow`, `clap`, `serde`, `serde_json`. Tools: `cargo`, `git`, Python 3.11+.

## Tests / usage
```bash
cargo test -p xtask
cargo xtask ci
cargo xtask docs --check
cargo xtask hooks
```
