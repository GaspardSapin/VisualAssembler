# crates/va-workspace/src/persist

## Purpose
Everything the IDE writes or reads in the background (m4-editor-shell.md §4.7–§4.8):

- the `va-io` worker thread, which runs every file job in order and wakes the UI through a
  `Waker` when a result is ready (architecture T2); a panicking job never stops it (T6);
- atomic writes: a `<name>.<pid>.<n>.tmp` file next to the target, synced, then renamed over it
  (architecture §6); a symlink is written through, never replaced;
- `session.toml` (window, zoom, theme, explorer, project root, open tabs) and `session.lock`, held
  for the life of the process; without the lock the session is read-only;
- hot exit: the backups of unsaved buffers in `<data>/backups/`, where `<id>.txt` is written before
  `index.toml` and removed after it, so a kill never leaves an index entry pointing at a torn file.

**Implemented in wave 2 by WP-C (commit `0d17640`)** over the WP0 scaffold (wave 1, commits
`b4e7aae` and `570afb1`): the items moved into five private files and no `todo!()` is left. Nothing
calls them yet: the `va-ui` shell (WP-F) and the GUI wiring (WP-G) are not implemented yet.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | Module doc and the `pub use` by name (no glob) of every item below, so the paths `va_workspace::persist::*` never move (m4 §15.1 M2) |
| `worker.rs` | module | `Waker`, `IoWorker` (`spawn`, `set_waker`, `waker`, `submit`, `flush`): one `va-io` thread, jobs in submission order, a panicking job caught |
| `atomic.rs` | module | `WriteFn`, `write_atomic` (temp file with `create_new`, synced, renamed), `read_limited`, `DiskStamp`, `stamp` |
| `session.rs` | module | `Session` with `SessionWindow`, `SessionUi`, `SessionWorkspace`, `SessionTab` and `TabSource`; `parse_session` (never fails, a bad value takes its default) and `session_to_toml` (fixed layout) |
| `lock.rs` | module | `SessionLock`, `try_lock_session`: `File::try_lock` on `<data>/session.lock`, held for the life of the process |
| `backup.rs` | module | Backups (§4.8): `BackupId`, `BackupEntry`, `BackupStore` (`open`, `write`, `remove`; orphan adoption, a newer index schema disables it), `RestoreItem`, `BackupProblem`, `restore_order`, `utc_timestamp` |

## Entry points
- `IoWorker::spawn`, then `submit(job)`: the only way the shell touches the disk.
- `write_atomic(target, write)`: every save, session and backup write. `va asm` keeps its own
  private copy of the rule until a later convergence (m2 §8.1, §13.2).
- `BackupStore::open(paths)`: what to restore at startup.

## Dependencies
- Internal: `crate::paths` (`Paths`), `crate::config` (`ThemeKind`, `MAX_CONFIG_BYTES`).
- External: `toml` and `serde` for `session.toml` and `backups/index.toml`.

## Tests / usage
```bash
cargo test -p va-workspace
```
The `persist` tests are `tests/persist_atomic.rs`, `persist_session.rs`, `persist_backup.rs`,
`persist_lock.rs` and `persist_kill.rs`, the kill test of exit criterion E4 (20 kills while writing
backups, every restore gives one complete earlier version; m4 §4.8, §10.1). See
`crates/va-workspace/tests/README.md`.
