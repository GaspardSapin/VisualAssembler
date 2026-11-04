//! Tests of `session.lock` (m4-editor-shell.md §4.7, §10.1, §16.2 B2): a second
//! `try_lock_session` gets `None` while the first lock is held, the lock is released on drop,
//! the data folder is created first (private on Unix), and an unwritable data folder is an
//! error.

#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use va_workspace::paths::Paths;
use va_workspace::persist::try_lock_session;

/// Creates a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (CLAUDE.md test rule).
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    dir
}

/// Paths whose data folder is `<root>/data` (not created yet).
fn paths_under(root: &std::path::Path) -> Paths {
    Paths {
        config_dir: root.join("config"),
        data_dir: root.join("data"),
        portable: false,
    }
}

#[test]
fn a_second_lock_is_refused_until_the_first_is_dropped() {
    let root = temp_dir("lock-second");
    let paths = paths_under(&root);
    let first = try_lock_session(&paths).unwrap();
    assert!(first.is_some(), "a free lock is taken");
    assert!(
        try_lock_session(&paths).unwrap().is_none(),
        "held elsewhere (even in this process): None"
    );
    assert!(try_lock_session(&paths).unwrap().is_none());
    drop(first);
    let again = try_lock_session(&paths).unwrap();
    assert!(again.is_some(), "released on drop");
    assert!(try_lock_session(&paths).unwrap().is_none());
}

#[test]
fn the_lock_file_lives_in_a_created_data_folder() {
    let root = temp_dir("lock-create");
    let paths = Paths {
        config_dir: root.join("config"),
        data_dir: root.join("deep").join("data"),
        portable: false,
    };
    assert!(!paths.data_dir.exists());
    let lock = try_lock_session(&paths).unwrap().unwrap();
    assert!(paths.data_dir.is_dir());
    assert!(paths.session_lock().is_file());
    assert!(
        !paths.config_dir.exists(),
        "only the data folder is created"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&paths.data_dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode & 0o077, 0, "data folder mode {mode:o} is private");
    }
    drop(lock);
    assert!(
        paths.session_lock().is_file(),
        "the file stays; only the lock is released"
    );
}

#[test]
fn an_existing_lock_file_is_reused_and_never_truncated() {
    let root = temp_dir("lock-existing");
    let paths = paths_under(&root);
    fs::create_dir(&paths.data_dir).unwrap();
    fs::write(paths.session_lock(), "left by an earlier run").unwrap();
    let lock = try_lock_session(&paths).unwrap();
    assert!(lock.is_some());
    drop(lock);
    assert_eq!(
        fs::read_to_string(paths.session_lock()).unwrap(),
        "left by an earlier run"
    );
}

#[test]
fn separate_data_folders_lock_independently() {
    let root = temp_dir("lock-separate");
    let one = paths_under(&root.join("one"));
    let two = paths_under(&root.join("two"));
    let first = try_lock_session(&one).unwrap();
    let second = try_lock_session(&two).unwrap();
    assert!(first.is_some() && second.is_some());
}

#[test]
fn an_unwritable_data_folder_is_an_error() {
    let root = temp_dir("lock-unwritable");
    let blocker = root.join("file");
    fs::write(
        &blocker,
        "a regular file where the data folder's parent should be",
    )
    .unwrap();
    let paths = Paths {
        config_dir: root.join("config"),
        data_dir: blocker.join("data"),
        portable: false,
    };
    assert!(try_lock_session(&paths).is_err());
}

#[test]
fn the_lock_moves_to_another_thread() {
    let root = temp_dir("lock-thread");
    let paths = paths_under(&root);
    let lock = try_lock_session(&paths).unwrap().unwrap();
    let checked = paths.clone();
    std::thread::spawn(move || {
        assert!(try_lock_session(&checked).unwrap().is_none());
        drop(lock);
        assert!(try_lock_session(&checked).unwrap().is_some());
    })
    .join()
    .unwrap();
}
