//! Atomic writes, capped reads and disk stamps (m4-editor-shell.md §4.7, architecture §6).
//!
//! [`write_atomic`] is the only way the IDE writes a file: saves, `session.toml`, backups and
//! the default `settings.toml`. The new content goes to `<name>.<pid>.<n>.tmp` in the target's
//! folder (created with `create_new`, so no existing file is ever clobbered), is flushed and
//! synced, then renamed over the target; a kill leaves either the old or the new content, plus
//! at most a stray temp file, never a torn target.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Writes the content of a file into the given writer.
pub type WriteFn = Box<dyn FnOnce(&mut dyn std::io::Write) -> std::io::Result<()> + Send>;

/// The per-process counter `n` of the temp names `<name>.<pid>.<n>.tmp`.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// How many temp names [`write_atomic`] tries when the previous ones already exist (a temp file
/// left by an earlier process that had the same pid).
const TEMP_ATTEMPTS: usize = 100;

/// Replaces `target` atomically with what `write` produces (temp file, sync, rename; the
/// original's permissions are kept on Unix and a symlink target is never replaced).
///
/// An existing `target` is resolved with `fs::canonicalize`, so a symlink is written through
/// and stays a symlink; a dangling symlink is an error (nothing is written). An existing target
/// whose permissions are read-only is refused with `PermissionDenied` before anything is
/// written, as the OS refuses to replace a read-only file on Windows. The temp file
/// `<name>.<pid>.<n>.tmp` is created with `create_new(true)` in the target's folder (a name
/// that already exists is skipped, never truncated); the writer runs into a `BufWriter`, then
/// the file is flushed and synced, the original's permissions are copied onto it (Unix) and it
/// is renamed over the target. On Unix the folder is then synced, best effort, so the rename
/// itself survives a power loss.
///
/// # Errors
/// Any I/O error; the temp file is then removed and `target` is unchanged. A panic of `write`
/// also removes the temp file.
pub fn write_atomic(target: &Path, write: WriteFn) -> std::io::Result<()> {
    let (resolved, original) = resolve_target(target)?;
    if let Some(metadata) = &original
        && metadata.permissions().readonly()
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the target file is read-only",
        ));
    }
    let name = resolved.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "the target has no file name")
    })?;
    let folder = resolved.parent().unwrap_or_else(|| Path::new(""));

    let (temp, file) = create_temp(folder, name)?;
    let mut writer = BufWriter::new(file);
    write(&mut writer)?;
    let file = writer
        .into_inner()
        .map_err(io::IntoInnerError::into_error)?;
    file.sync_all()?;
    #[cfg(unix)]
    if let Some(metadata) = &original {
        file.set_permissions(metadata.permissions())?;
    }
    drop(file);
    fs::rename(temp.path(), &resolved)?;
    temp.disarm();
    #[cfg(unix)]
    sync_folder(folder);
    Ok(())
}

/// The path to write and the metadata of the existing file there (`None` when it is new).
fn resolve_target(target: &Path) -> io::Result<(PathBuf, Option<fs::Metadata>)> {
    match fs::symlink_metadata(target) {
        // Something exists at `target` (a file, or a symlink, dangling or not): write through
        // its canonical path, whose error is returned for a dangling link.
        Ok(_) => {
            let resolved = fs::canonicalize(target)?;
            let metadata = fs::metadata(&resolved)?;
            Ok((resolved, Some(metadata)))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok((target.to_path_buf(), None)),
        Err(error) => Err(error),
    }
}

/// Creates the temp file `<name>.<pid>.<n>.tmp` in `folder` with `create_new(true)`, skipping
/// names that already exist.
fn create_temp(folder: &Path, name: &std::ffi::OsStr) -> io::Result<(TempFile, File)> {
    let pid = std::process::id();
    let mut last_error = None;
    for _ in 0..TEMP_ATTEMPTS {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut temp_name = name.to_os_string();
        temp_name.push(format!(".{pid}.{n}.tmp"));
        let path = folder.join(temp_name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((TempFile { path: Some(path) }, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => last_error = Some(error),
            Err(error) => return Err(error),
        }
    }
    Err(last_error
        .unwrap_or_else(|| io::Error::new(io::ErrorKind::AlreadyExists, "no free temp name")))
}

/// A temp file removed on drop unless [`TempFile::disarm`] ran (error and panic paths alike).
struct TempFile {
    /// The temp path; `None` once disarmed.
    path: Option<PathBuf>,
}

impl TempFile {
    /// The temp path.
    fn path(&self) -> &Path {
        self.path.as_deref().unwrap_or_else(|| Path::new(""))
    }

    /// Keeps the file: it was renamed over the target.
    fn disarm(mut self) {
        self.path = None;
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            // Best effort: the write already failed, and its error is what the caller sees.
            let _ = fs::remove_file(path);
        }
    }
}

/// Syncs `folder` so a rename into it is durable (Unix; errors are ignored, the write itself
/// already succeeded).
#[cfg(unix)]
fn sync_folder(folder: &Path) {
    let folder = if folder.as_os_str().is_empty() {
        Path::new(".")
    } else {
        folder
    };
    if let Ok(handle) = File::open(folder) {
        let _ = handle.sync_all();
    }
}

/// Reads at most `limit` bytes of `path`.
///
/// # Errors
/// Any I/O error, or `InvalidData` when the file holds more than `limit` bytes.
pub fn read_limited(path: &Path, limit: usize) -> std::io::Result<Vec<u8>> {
    let file = File::open(path)?;
    let cap = u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1);
    let hint = file
        .metadata()
        .map(|metadata| metadata.len().min(cap))
        .unwrap_or(0);
    let mut bytes = Vec::with_capacity(usize::try_from(hint).unwrap_or(0));
    file.take(cap).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("the file is larger than {limit} bytes"),
        ));
    }
    Ok(bytes)
}

/// Size and modification time of a file, to notice changes made by other programs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DiskStamp {
    /// Length in bytes.
    pub len: u64,
    /// Modification time, when the platform reports one.
    pub modified: Option<SystemTime>,
}

impl DiskStamp {
    /// Whether `self` and `recorded` have the same length and the same modification time to
    /// the millisecond, the precision of `backups/index.toml` (`disk_modified_ms`, m4 §4.8):
    /// compare a fresh [`stamp`] with a restored `BackupEntry::disk` through this, not `==`.
    pub fn matches(&self, recorded: &Self) -> bool {
        self.len == recorded.len
            && self.modified.map(system_time_to_millis)
                == recorded.modified.map(system_time_to_millis)
    }
}

/// The current [`DiskStamp`] of `path`.
///
/// # Errors
/// The error of `std::fs::metadata`.
pub fn stamp(path: &Path) -> std::io::Result<DiskStamp> {
    let metadata = fs::metadata(path)?;
    Ok(DiskStamp {
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

/// Milliseconds since the Unix epoch, negative before it (each side truncated toward the
/// epoch), saturated to the `i64` range.
pub(super) fn system_time_to_millis(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_millis()).unwrap_or(i64::MAX),
        Err(before) => i64::try_from(before.duration().as_millis())
            .map(|ms| -ms)
            .unwrap_or(i64::MIN),
    }
}

/// The time `ms` milliseconds from the Unix epoch, when the platform can represent it.
pub(super) fn millis_to_system_time(ms: i64) -> Option<SystemTime> {
    let offset = Duration::from_millis(ms.unsigned_abs());
    if ms >= 0 {
        UNIX_EPOCH.checked_add(offset)
    } else {
        UNIX_EPOCH.checked_sub(offset)
    }
}
