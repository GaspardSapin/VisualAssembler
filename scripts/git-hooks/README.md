# scripts/git-hooks

## Purpose
Git hooks shipped with the repository. Git does not install hooks by itself: `make hooks` copies them
into the repository's hooks folder. There is one hook today, and it enforces the project rule that no
commit credits an AI tool.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `commit-msg` | POSIX sh hook | Rejects (exit 1) a message containing `Co-authored-by:`, `Assisted-by:` or `Generated-by:` followed by an AI tool name, `generated with` followed by such a name, or the robot emoji. 12 names, ASCII case ignored. Normative content: m0-foundations.md §18.2. |

## Entry points
- `commit-msg`. It runs with `LC_ALL=C`, so `sed` and `grep` match bytes on every platform. Git for
  Windows' grep misses 4-byte UTF-8 characters under the UTF-8 locale git sets for hooks. The hook
  ignores comment lines and everything below git's scissors line, and it fails closed when the
  message file or a tool fails.

Keep the names and trailer keys in sync with `COAUTHOR_RE` in `scripts/check_project_docs.py`,
which re-checks the last 50 commits in the CI `docs` job.

Limits (details in `SECURITY.md`):
- client-side only: `--no-verify`, rebase and cherry-pick skip it;
- comment lines kept by `-m`/`-F`, `--cleanup=verbatim` or a custom `core.commentChar` pass locally
  (the CI log check catches them after the push);
- a trailer folded onto a continuation line, and an AI identity used as *author*, are not caught.

## Dependencies
- Internal: installed by `cargo xtask hooks` (`xtask/src/hooks.rs`).
- External: `sh`, `sed`, `grep` (Git for Windows provides them).

## Tests / usage
```bash
make hooks                                    # install, or print "already installed"
cargo xtask hooks --force                     # replace a different hook, keeping a backup
sh scripts/git-hooks/commit-msg <message-file> # 1 = rejected, 0 = accepted
cargo test -p xtask --test hooks
```
