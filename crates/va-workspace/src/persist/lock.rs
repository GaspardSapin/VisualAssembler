//! `session.lock` (m4-editor-shell.md §4.7, architecture §6): one instance at a time writes the
//! session and the backups.
//!
//! The lock is `File::try_lock` (std; `flock` on Unix, `LockFileEx` on Windows) on
//! `<data>/session.lock`, held for the life of the process. A lock belongs to one open file, so
//! a second [`try_lock_session`] fails even in the same process. Without the lock (another
//! instance, an unwritable data folder) the session is read-only: no `session.toml`, backup or
//! default-settings write.

use std::fs::{File, OpenOptions, TryLockError};

use crate::paths::Paths;

/// The held `session.lock`; released on drop.
#[derive(Debug)]
pub struct SessionLock {
    /// The locked file.
    file: File,
}

impl Drop for SessionLock {
    /// Releases the lock at once (closing the file would too, a little later on Windows).
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// Takes `session.lock`; `Ok(None)` when another instance holds it.
///
/// The data folder is created first if missing ([`Paths::create_data_dir`], mode `0o700` on
/// Unix). The lock file is opened for writing without truncation and never written: its content
/// does not matter, only the lock does.
///
/// # Errors
/// The lock file cannot be created or opened (the data folder is not writable), or the lock call
/// fails for another reason than "held elsewhere".
pub fn try_lock_session(paths: &Paths) -> std::io::Result<Option<SessionLock>> {
    paths.create_data_dir()?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(paths.session_lock())?;
    match file.try_lock() {
        Ok(()) => Ok(Some(SessionLock { file })),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error),
    }
}
