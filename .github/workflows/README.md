# .github/workflows

## Purpose
GitHub Actions workflows. `ci.yml` checks every push to `main`, every pull request, and manual runs.
The check list itself lives in `cargo xtask ci`, so the workflow never duplicates cargo commands.
`release.yml` (packaging for x86_64 and aarch64) arrives in M8.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `ci.yml` | workflow | Jobs `fmt`, `docs`, `deny` and `check` (4 runners). Normative content: m0-foundations.md §8, amended by §17.5 and §18.7. |

| Job | Runner | What it runs |
|-----|--------|--------------|
| `fmt` | `ubuntu-latest` | `cargo fmt --all -- --check` |
| `docs` | `ubuntu-latest` | `python3 scripts/check_project_docs.py .` with `fetch-depth: 50`, so the AI co-author check sees the last 50 commits |
| `deny` | `ubuntu-latest` | Builds cargo-deny 0.20.2 from crates.io (`--locked`) in the runner's temp folder, outside the checkout, with no cache. Then runs `cargo deny --locked --manifest-path Cargo.toml --config .cargo/deny.toml check`. |
| `check (<os>)` | `windows-latest`, `windows-11-arm`, `ubuntu-latest`, `ubuntu-24.04-arm` | `cargo xtask ci --no-fmt`, then `cargo build --release --locked -p visual-assembler -p va`, then (Windows only) the GUI startup smoke step |

## Entry points
- `ci.yml`. Rules for editing it:
  - **Pin every action to a full commit SHA**, with the tag in a comment. A bump means re-resolving the
    SHA upstream, for example
    `git ls-remote https://github.com/Swatinem/rust-cache refs/tags/v2.9.2 'refs/tags/v2.9.2^{}'`.
    For an annotated tag the commit is the `^{}` line; a lightweight tag (such as checkout's
    `v7.0.1`) prints only one line, which is the commit.
  - **Never set `RUSTFLAGS`** (or any `RUSTFLAGS`-type variable): it replaces `.cargo/config.toml` and
    drops `+crt-static`. `cargo xtask ci` step 0 fails the job if one is set.
  - Keep `permissions: contents: read`, `persist-credentials: false`, and no `${{ … }}` inside `run:`
    scripts.
  - The rust cache is saved by `main` runs only, and the `deny` job never uses it.
  - Linux runners install no apt package in M0. If a dependency ever needs system libraries, add one
    `apt-get install --no-install-recommends` step and document it in the spec.
- The **GUI startup smoke** step is informational: `continue-on-error`, a 3-minute timeout, and
  `bash`, because PowerShell does not wait for GUI-subsystem executables. Hosted runners have no GPU,
  so it may fail. It never counts as evidence for a support tier (architecture §9.1).
  m0-foundations.md §19.3 adds a second informational smoke step on the Windows runners, run with
  `VA_TEST_FAIL_RENDERER=glow`, to document the renderer fallback. It is not in `ci.yml` yet (fix-wave
  row 19-4).

## Dependencies
- Internal: `cargo xtask ci`, `scripts/check_project_docs.py`, `.cargo/deny.toml`.
- External actions: `actions/checkout` v7.0.1, `dtolnay/rust-toolchain` (master, 2026-10-13),
  `Swatinem/rust-cache` v2.9.2, each pinned by SHA in `ci.yml`.

## Tests / usage
The workflow has **not run yet**: the GitHub repository does not exist until M0 is committed and
pushed, so exit criterion E2 is not observable yet. Branch protection with the required checks
`fmt`, `docs`, `deny` and the four `check (…)` jobs is planned after the push, with the user's
consent. Until then every CI check is advisory.

Reading runs from the dev machine, once the repository is pushed. `gh` is not on `PATH` in agent
shells, so call it by its full path:

```bash
"C:/Program Files/GitHub CLI/gh.exe" run list --workflow ci.yml --limit 5
"C:/Program Files/GitHub CLI/gh.exe" run watch <run-id>
"C:/Program Files/GitHub CLI/gh.exe" run view <run-id> --log-failed
```
Locally, `make ci` (`cargo xtask ci`) runs the checks of the `check` job's first step, plus the fmt
check. It does not run the release build or the smoke step.
