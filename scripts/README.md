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

`check_project_docs.py` checks, numbered as in its docstring:
- **1** the root pilot files exist;
- **2** every folder with a file of its own has a `README.md`, and every parent README names its child
  folders. Folders whose name starts with a dot are exempt from both checks: `.cargo/` (two
  configuration files) needs no README, and the root README is not required to name `.cargo/` or
  `.github/`. A folder below a dot-folder is still checked, so `.github/workflows/` has its README;
- **3** every external crate of the root `[workspace.dependencies]` appears in `DEPENDANCE.md` and
  `IMPORT.md`;
- **3b** `[deps]`: every member dependency uses `workspace = true` (rule R6). In reverse, it also
  reports every root `[workspace.dependencies]` key that no member declares with `workspace = true`,
  path entries and target tables included:
  `[deps] Cargo.toml: workspace dependency '<name>' is not declared by any member (rule R6: remove the pin or declare it with workspace = true)`.
  cargo-deny's `unused` rule cannot prove this (m0 §19.9 S19-3);
- **3c** `[lints]`: every member has exactly `[lints] workspace = true`, and the root keeps
  `unsafe_code = "deny"` (rule R8);
- **3d** every package below the root is listed in `[workspace].members`;
- **3e** the root has no `[patch]` or `[replace]` table;
- **4** `AGENCEMENT.md` matches the real tree. Then it warns
  `[layout] some entries are still unannotated (<n>): <paths>` (at most 10 paths, then
  `and <k> more`) for tree entries whose annotation is empty or still holds `<!-- annotate -->`. The
  header line never counts, and `--strict` exits 1 only when such entries exist;
- **5** recommended `.gitignore` entries are present;
- **6** no folder is overloaded (warning above 15 files, error above 25);
- **7** none of the last 50 commits credits an AI tool; a missing `commit-msg` hook is a warning.

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
