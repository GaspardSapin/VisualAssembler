# scripts

## Purpose
Project-documentation tooling of the agentic-project framework, and the git hooks. These scripts
check the repository's structure and pilot files. They never build or test Rust code: that belongs
to `xtask/`.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `gen_agencement.py` | script | Writes `AGENCEMENT.md`, the annotated file tree. Hand-written annotations survive regeneration, and new entries appear as `<!-- annotate -->`. Writes LF line endings on every OS. `--check` writes nothing and exits 1 when the file is stale. |
| `check_project_docs.py` | script | Verifies the closing ritual, below. Exits 1 on any problem; `--strict` also fails on warnings. |
| `git-hooks/` | folder | The `commit-msg` hook. See its README. |

`check_project_docs.py` checks:
1. the root pilot files exist;
2. every folder with a file of its own has a `README.md`, and every parent README names its child folders.
   Folders whose name starts with a dot are exempt from both checks: `.cargo/` (two configuration
   files) needs no README, and the root README is not required to name `.cargo/` or `.github/`. A
   folder below a dot-folder is still checked, so `.github/workflows/` has its README;
3. every external crate of the root `[workspace.dependencies]` appears in `DEPENDANCE.md` and `IMPORT.md`;
4. `[deps]`: every member dependency uses `workspace = true` (rule R6); every package below the root
   is listed in `[workspace].members`; the root has no `[patch]` or `[replace]` table;
5. `[lints]`: every member has exactly `[lints] workspace = true`, and the root keeps
   `unsafe_code = "deny"` (rule R8);
6. `AGENCEMENT.md` matches the real tree;
7. recommended `.gitignore` entries are present;
8. no folder is overloaded (warning above 15 files, error above 25);
9. none of the last 50 commits credits an AI tool; a missing `commit-msg` hook is a warning.

It also warns while `AGENCEMENT.md` still holds `<!-- annotate -->` entries. **Known defect:** the
generator's own header line contains that marker, so the warning `[layout] some entries are still
unannotated` appears even when every entry is annotated (the footer then reads `0 to annotate`).
The default run still exits 0; `--strict` exits 1 until the check is fixed.

## Entry points
- `check_project_docs.py` `main()`; it imports the tree walk from `gen_agencement.py`.

Both scripts skip paths ignored by git, using only the committed `.gitignore` files
(`git ls-files --others --ignored --exclude-per-directory=.gitignore --directory`). The result is
therefore the same on every machine. The Cargo checks need Python 3.11+ (`tomllib`); older Pythons
skip them with a warning.

## Dependencies
- Internal: read by `cargo xtask docs` (`xtask/src/docs.rs`) and by the CI `docs` job.
- External: Python 3.11+ standard library only; `git` on `PATH`.

## Tests / usage
```bash
python scripts/gen_agencement.py .
python scripts/gen_agencement.py . --check
python scripts/check_project_docs.py .
python scripts/check_project_docs.py . --strict
make docs
```
On Linux, call `python3`. `make docs` (`cargo xtask docs`) picks the interpreter itself. Before the
first commit, the check prints a `note:` that commit messages cannot be checked yet.
