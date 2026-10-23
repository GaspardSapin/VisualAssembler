# .github/workflows

## Purpose
GitHub Actions workflows. `ci.yml` checks every push to `main`, every pull request, and manual runs.
The check list itself lives in `cargo xtask ci`, so the workflow never duplicates cargo commands.
`release.yml` (packaging for x86_64 and aarch64) arrives in M8.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `ci.yml` | workflow | Jobs `fmt`, `docs`, `deny` and `check` (4 runners). Normative content: m0-foundations.md §8, amended by §17.5, §18.7, §19.3 and §19.9 (row 19-9 a). |

| Job | Runner | What it runs |
|-----|--------|--------------|
| `fmt` | `ubuntu-latest` | `cargo fmt --all -- --check` |
| `docs` | `ubuntu-latest` | `python3 scripts/check_project_docs.py .` with `fetch-depth: 50`, so the AI co-author check sees the last 50 commits |
| `deny` | `ubuntu-latest` | Builds cargo-deny 0.20.2 from crates.io (`--locked`) in the runner's temp folder, outside the checkout, with no cache. Then runs `cargo deny --locked --manifest-path Cargo.toml --config .cargo/deny.toml check`. |
| `check (<os>)` | `windows-latest`, `windows-11-arm`, `ubuntu-latest`, `ubuntu-24.04-arm` | Step `clippy, layering, tests`: `cargo xtask ci --no-fmt`, the 8 non-fmt steps listed in `xtask/README.md` (since M1 they include clippy and the tests of `va` without any CPU, m1 §12.7). Then `cargo build --release --locked -p visual-assembler -p va`, then (Windows only) the two GUI smoke steps |

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
  - Linux runners install no apt package (M0, M1). If a dependency ever needs system libraries, add one
    `apt-get install --no-install-recommends` step and document it in the spec.
- The two **GUI smoke** steps are informational: `continue-on-error`, a 3-minute timeout, and
  `bash`, because PowerShell does not wait for GUI-subsystem executables. Hosted runners have no GPU,
  so they may fail. They never count as evidence for a support tier (architecture §9.1).
  - **GUI startup smoke** runs `./target/release/visual-assembler.exe --bench startup`, so its line
    shows what `auto` starts on the hosted image.
  - **GUI renderer fallback smoke** sets `VA_TEST_FAIL_RENDERER`, through the step's `env:`, to the
    first renderer of the runner's `auto` order: `glow` on `windows-latest`, `wgpu` on
    `windows-11-arm`. Its log shows a fallback (`"fallback_from":"glow"` on x64,
    `"fallback_from":"wgpu"` on ARM64). On ARM64, when the image has no usable OpenGL, it shows the
    `no renderer could be initialized (tried wgpu, then glow)` lines instead.
  - Neither step can gate a merge (`continue-on-error`). The real fallback is tested locally
    (`apps/visual-assembler/tests/README.md`).

## Dependencies
- Internal: `cargo xtask ci`, `scripts/check_project_docs.py`, `.cargo/deny.toml`.
- External actions: `actions/checkout` v7.0.1, `dtolnay/rust-toolchain` (master, 2026-10-13),
  `Swatinem/rust-cache` v2.9.2, each pinned by SHA in `ci.yml`.

## Tests / usage
Recorded runs (read with `gh run view` on 2026-10-16 and 2026-10-23; `SECURITY.md`,
`docs/perf/M0-backend.md`): run 38635672228 (2026-10-14, `c326afd`, before the §19 renderer work)
passed all 7 jobs; run 38968085878 (`646b6f7`) failed only `check (windows-11-arm)` on a dead-code
warning; run 38969165969 (2026-10-16, `6ed43c8`) passed all 7 jobs, the §19 smoke steps included.
The M1 commits (`cc827b0..8994d44`) were pushed to `main`. Run 40023018227 on `8994d44` failed only
`deny`, with `error[yanked]` on `clipboard-win` 5.4.1 (Windows only, through arboard 3.6.1,
egui-winit and eframe 0.36.2), yanked upstream on 2026-10-23 for an out-of-bounds read in
`raw::get_html`. Commit `5f45f9b` bumps it to 5.4.3 (`SECURITY.md`, M1-S10), and run 40027188396
(2026-10-23, `5f45f9b`) passed all 7 jobs. Its `check (windows-11-arm)` job still shows the
annotation "Process completed with exit code 3" from the informational GUI smoke steps, as the M0
run 38970636341 on `cc827b0` did; `continue-on-error` keeps the job green.
Branch protection on `main` requires `fmt`, `docs`, `deny` and the four `check (…)` jobs. It does not
include administrators (`enforce_admins` is off), so the owner's direct pushes are not gated.

Reading runs and the protection from the dev machine. `gh` is not on `PATH` in agent shells, so call
it by its full path:

```bash
"C:/Program Files/GitHub CLI/gh.exe" run list --workflow ci.yml --limit 5
"C:/Program Files/GitHub CLI/gh.exe" run watch <run-id>
"C:/Program Files/GitHub CLI/gh.exe" run view <run-id> --log-failed
"C:/Program Files/GitHub CLI/gh.exe" api repos/GaspardSapin/VisualAssembler/branches/main/protection
```
Locally, `make ci` (`cargo xtask ci`) runs the checks of the `check` job's first step, plus the fmt
check. It does not run the release build or the smoke steps. `cargo test -p xtask --test ci` checks
that the fallback smoke step is the spec block and that every action stays SHA-pinned.
