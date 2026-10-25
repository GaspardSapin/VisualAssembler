# crates/va-workspace/src/persist

## Purpose
Everything the IDE writes or reads in the background (m4-editor-shell.md §4.7–§4.8):

- the `va-io` worker thread, which runs every file job in order and wakes the UI through a
  `Waker` when a result is ready (architecture T2);
- atomic writes: a `<name>.<pid>.<n>.tmp` file next to the target, synced, then renamed over it
  (architecture §6); a symlink target is never replaced;
- `session.toml` (window, zoom, theme, explorer, project root, open tabs) and `session.lock`, held
  for the life of the process; without the lock the session is read-only;
- hot exit: the backups of unsaved buffers in `<data>/backups/`, where `<id>.txt` is written before
  `index.toml` and removed after it, so a kill never leaves an index entry pointing at a torn file.

**WP0 scaffold (wave 1, commits `b4e7aae` and `570afb1`); implemented in wave 2 by WP-C.** The
session types of §4.7 (frozen 2026-10-24) and their `Default` impls are final; every function and
method (`IoWorker`, `write_atomic`, `read_limited`, `stamp`, `try_lock_session`, `parse_session`,
`session_to_toml`, `BackupStore`) is a `todo!()` stub and panics when called.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | Worker and writes: `Waker`, `IoWorker` (`spawn`, `set_waker`, `waker`, `submit`, `flush`), `WriteFn`, `write_atomic`, `read_limited`, `DiskStamp`, `stamp`. Session: `SessionLock`, `try_lock_session`, `Session` with `SessionWindow`, `SessionUi`, `SessionWorkspace`, `SessionTab` and `TabSource`, `parse_session`, `session_to_toml`. Backups (§4.8): `BackupId`, `BackupEntry`, `BackupStore` (`open`, `write`, `remove`), `RestoreItem`, `BackupProblem` |

WP-C splits the items into the private modules `worker.rs`, `atomic.rs`, `session.rs`, `lock.rs`
and `backup.rs` (m4 §2) by the end of its package, and re-exports each frozen item by name from
`mod.rs` (`pub use`, no glob), so the paths `va_workspace::persist::*` never move (m4 §15.1 M2).

## Entry points
- `IoWorker::spawn`, then `submit(job)`: the only way the shell touches the disk.
- `write_atomic(target, write)`: every save, session and backup write. `va asm` keeps its own
  private copy of the rule until a later convergence (m2 §8.1, §13.2).
- `BackupStore::open(paths)`: what to restore at startup.

`mod.rs` carries `#![expect(unused_variables, dead_code, clippy::todo, reason = "WP0 stub: …")]`;
a listed lint that stops firing raises `unfulfilled_lint_expectations`, which CI denies. WP-C
removes each lint in the commit that makes it stop firing and the attribute with the last
`todo!()`, never replacing it by `allow`; a new stub file carries its own expectation (m4 §15.1 M3).

## Dependencies
- Internal: `crate::paths` (`Paths`), `crate::config` (`ThemeKind`).
- External: `toml` and `serde` for `session.toml` and `backups/index.toml` (WP-C; not used by the
  scaffold).

## Tests / usage
```bash
cargo test -p va-workspace
```
In the scaffold this runs only the M1 `i18n` tests. WP-C adds `tests/persist_atomic.rs`,
`persist_session.rs`, `persist_backup.rs`, `persist_lock.rs` and `persist_kill.rs`, the kill test of
exit criterion E4 (20 kills while writing backups, every restore gives one complete earlier
version; m4 §4.8, §10.1).
