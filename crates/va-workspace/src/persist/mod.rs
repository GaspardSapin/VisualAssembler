//! The I/O worker, atomic writes, the session and its lock, and the hot-exit backups
//! (m4-editor-shell.md §4.7–§4.8, frozen).
//!
//! - One worker thread `va-io` runs every file job in order and wakes the UI through a
//!   [`Waker`] when a result is ready (architecture T2); a panicking job never stops it (T6).
//! - [`write_atomic`] writes a `<name>.<pid>.<n>.tmp` file next to the target, syncs it, then
//!   renames it over the target; a symlink is written through, never replaced.
//! - `session.lock` is held for the life of the process; without it the session is read-only.
//! - Backups live in the data folder only: `<id>.txt` is written before `index.toml`, and
//!   removed after it, so a kill never leaves an index entry pointing at a torn file.
//!
//! The items live in private files (`worker.rs`, `atomic.rs`, `session.rs`, `lock.rs`,
//! `backup.rs`) and are re-exported here by name, so their public paths are
//! `va_workspace::persist::*` (m4 §15.1 M2).

mod atomic;
mod backup;
mod lock;
mod session;
mod worker;

pub use atomic::{DiskStamp, WriteFn, read_limited, stamp, write_atomic};
pub use backup::{
    BackupEntry, BackupId, BackupProblem, BackupStore, RestoreItem, restore_order, utc_timestamp,
};
pub use lock::{SessionLock, try_lock_session};
pub use session::{
    Session, SessionTab, SessionUi, SessionWindow, SessionWorkspace, TabSource, parse_session,
    session_to_toml,
};
pub use worker::{IoWorker, Waker};
