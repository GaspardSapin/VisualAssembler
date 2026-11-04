//! Tests of the atomic writes, capped reads, disk stamps and the `va-io` worker of
//! `va_workspace::persist` (m4-editor-shell.md §4.7, §10.1): temp naming and `create_new`, a
//! failure or a panic removes the temp, the target is all-or-nothing, symlinks and permissions
//! are kept; jobs run in order on `va-io`, the waker slot, `flush` and a panicking job.

#![allow(clippy::unwrap_used)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};

use va_workspace::persist::{DiskStamp, IoWorker, WriteFn, read_limited, stamp, write_atomic};

/// Creates a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (CLAUDE.md test rule).
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    dir
}

/// A writer closure that writes `bytes`.
fn bytes_of(bytes: &[u8]) -> WriteFn {
    let bytes = bytes.to_vec();
    Box::new(move |out| out.write_all(&bytes))
}

/// The names in `dir`, sorted.
fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Whether `name` is a temp name `<target>.<this pid>.<n>.tmp`; returns `n`.
fn temp_counter(name: &str, target: &str) -> Option<u64> {
    let rest = name.strip_prefix(target)?.strip_prefix('.')?;
    let rest = rest.strip_suffix(".tmp")?;
    let (pid, n) = rest.split_once('.')?;
    (pid == std::process::id().to_string()).then_some(())?;
    n.parse().ok()
}

#[test]
fn a_new_file_is_created_with_the_content_and_no_temp_is_left() {
    let dir = temp_dir("atomic-new");
    let target = dir.join("main.asm");
    write_atomic(&target, bytes_of(b"  move.l d0,d1\n")).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"  move.l d0,d1\n");
    assert_eq!(names_in(&dir), ["main.asm"]);
}

#[test]
fn an_existing_file_is_replaced_as_a_whole() {
    let dir = temp_dir("atomic-replace");
    let target = dir.join("long.txt");
    fs::write(&target, "a much longer original content\n".repeat(100)).unwrap();
    write_atomic(&target, bytes_of(b"short")).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"short");
    write_atomic(&target, bytes_of(b"")).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"");
    assert_eq!(names_in(&dir), ["long.txt"]);
}

#[test]
fn the_temp_file_is_named_after_the_target_with_the_pid_and_a_counter() {
    let dir = temp_dir("atomic-naming");
    let target = dir.join("session.toml");
    let seen = Arc::new(Mutex::new(Vec::new()));
    for _ in 0..3 {
        let (folder, seen) = (dir.clone(), Arc::clone(&seen));
        write_atomic(
            &target,
            Box::new(move |out| {
                seen.lock().unwrap().push(names_in(&folder));
                out.write_all(b"x")
            }),
        )
        .unwrap();
    }
    let seen = seen.lock().unwrap();
    let mut counters = Vec::new();
    for (round, names) in seen.iter().enumerate() {
        let temps: Vec<u64> = names
            .iter()
            .filter_map(|name| temp_counter(name, "session.toml"))
            .collect();
        assert_eq!(
            temps.len(),
            1,
            "one temp file while writing, round {round}: {names:?}"
        );
        assert_eq!(names.len(), if round == 0 { 1 } else { 2 }, "{names:?}");
        counters.push(temps[0]);
    }
    assert!(
        counters.windows(2).all(|pair| pair[0] < pair[1]),
        "the counter grows: {counters:?}"
    );
    assert_eq!(names_in(&dir), ["session.toml"]);
}

#[test]
fn an_existing_temp_name_is_never_clobbered() {
    let dir = temp_dir("atomic-create-new");
    let target = dir.join("notes.txt");
    let observed = Arc::new(Mutex::new(None));
    let (folder, slot) = (dir.clone(), Arc::clone(&observed));
    write_atomic(
        &target,
        Box::new(move |out| {
            let names = names_in(&folder);
            *slot.lock().unwrap() = names
                .iter()
                .find_map(|name| temp_counter(name, "notes.txt"));
            out.write_all(b"first")
        }),
    )
    .unwrap();
    let n = observed.lock().unwrap().unwrap();
    // Plant files at the next temp names, as an earlier process with the same pid could have.
    let pid = std::process::id();
    let planted: Vec<PathBuf> = (n + 1..=n + 8)
        .map(|k| dir.join(format!("notes.txt.{pid}.{k}.tmp")))
        .collect();
    for path in &planted {
        fs::write(path, "user data").unwrap();
    }
    write_atomic(&target, bytes_of(b"second")).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"second");
    for path in &planted {
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "user data",
            "{}",
            path.display()
        );
    }
}

#[test]
fn a_failing_writer_removes_the_temp_and_keeps_the_target() {
    let dir = temp_dir("atomic-fail");
    let target = dir.join("main.asm");
    fs::write(&target, "original").unwrap();
    let error = write_atomic(
        &target,
        Box::new(|out| {
            out.write_all(&[b'x'; 100_000])?;
            Err(io::Error::other("encoding failed"))
        }),
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "encoding failed");
    assert_eq!(fs::read_to_string(&target).unwrap(), "original");
    assert_eq!(names_in(&dir), ["main.asm"]);

    // Same for a new file: nothing is left at all.
    let fresh = dir.join("fresh.asm");
    write_atomic(&fresh, Box::new(|_| Err(io::Error::other("no")))).unwrap_err();
    assert!(!fresh.exists());
    assert_eq!(names_in(&dir), ["main.asm"]);
}

#[test]
fn a_panicking_writer_removes_the_temp_and_keeps_the_target() {
    let dir = temp_dir("atomic-panic");
    let target = dir.join("main.asm");
    fs::write(&target, "original").unwrap();
    let target_for_write = target.clone();
    let outcome = std::panic::catch_unwind(move || {
        write_atomic(
            &target_for_write,
            Box::new(|out| {
                out.write_all(b"half").unwrap();
                panic!("writer bug");
            }),
        )
    });
    assert!(outcome.is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "original");
    assert_eq!(names_in(&dir), ["main.asm"]);
}

#[test]
fn a_missing_folder_is_an_error_and_creates_nothing() {
    let dir = temp_dir("atomic-missing");
    let target = dir.join("absent").join("file.txt");
    let error = write_atomic(&target, bytes_of(b"x")).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(names_in(&dir).is_empty());
}

#[test]
fn a_read_only_target_is_refused_and_unchanged() {
    let dir = temp_dir("atomic-read-only");
    let target = dir.join("locked.asm");
    fs::write(&target, "original").unwrap();
    let mut permissions = fs::metadata(&target).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&target, permissions).unwrap();
    let error = write_atomic(&target, bytes_of(b"new")).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(fs::read_to_string(&target).unwrap(), "original");
    assert_eq!(names_in(&dir), ["locked.asm"]);
}

/// Writes through a symlink and checks that the link stays a link and its target changed.
fn check_symlink_written_through(dir: &Path, link: &Path, real: &Path) {
    write_atomic(link, bytes_of(b"through the link")).unwrap();
    assert!(
        fs::symlink_metadata(link).unwrap().file_type().is_symlink(),
        "the link is never replaced by a regular file"
    );
    assert_eq!(fs::read(real).unwrap(), b"through the link");
    assert_eq!(fs::read(link).unwrap(), b"through the link");
    assert!(
        names_in(dir).iter().all(|name| !name.ends_with(".tmp")),
        "{:?}",
        names_in(dir)
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_target_is_written_through_and_preserved() {
    let dir = temp_dir("atomic-symlink");
    let real_dir = dir.join("real");
    fs::create_dir(&real_dir).unwrap();
    let real = real_dir.join("main.asm");
    fs::write(&real, "original").unwrap();
    let link = dir.join("link.asm");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    check_symlink_written_through(&real_dir, &link, &real);
    assert_eq!(names_in(&dir), ["link.asm", "real"]);
}

#[cfg(windows)]
#[test]
fn a_symlink_target_is_written_through_and_preserved() {
    let dir = temp_dir("atomic-symlink");
    let real_dir = dir.join("real");
    fs::create_dir(&real_dir).unwrap();
    let real = real_dir.join("main.asm");
    fs::write(&real, "original").unwrap();
    let link = dir.join("link.asm");
    if let Err(error) = std::os::windows::fs::symlink_file(&real, &link) {
        println!("note: symlinks are not permitted here ({error}); the symlink check is skipped");
        return;
    }
    check_symlink_written_through(&real_dir, &link, &real);
}

#[cfg(unix)]
#[test]
fn a_dangling_symlink_is_an_error_and_stays_a_link() {
    let dir = temp_dir("atomic-dangling");
    let link = dir.join("link.asm");
    std::os::unix::fs::symlink(dir.join("gone.asm"), &link).unwrap();
    write_atomic(&link, bytes_of(b"x")).unwrap_err();
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(names_in(&dir), ["link.asm"]);
}

#[cfg(unix)]
#[test]
fn the_permissions_of_the_original_are_kept() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir("atomic-mode");
    let target = dir.join("script.sh");
    fs::write(&target, "#!/bin/sh\n").unwrap();
    for mode in [0o640, 0o755, 0o600] {
        fs::set_permissions(&target, fs::Permissions::from_mode(mode)).unwrap();
        write_atomic(&target, bytes_of(b"echo hi\n")).unwrap();
        let kept = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(kept, mode, "mode {mode:o}");
    }
}

#[test]
fn read_limited_reads_up_to_the_limit_and_refuses_more() {
    let dir = temp_dir("atomic-read");
    let path = dir.join("data.bin");
    fs::write(&path, [7_u8; 1000]).unwrap();
    assert_eq!(read_limited(&path, 1000).unwrap(), vec![7_u8; 1000]);
    assert_eq!(read_limited(&path, 5000).unwrap().len(), 1000);
    let error = read_limited(&path, 999).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    let error = read_limited(&dir.join("absent"), 10).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    fs::write(&path, b"").unwrap();
    assert!(read_limited(&path, 0).unwrap().is_empty());
}

#[test]
fn stamp_reports_the_length_and_the_modification_time() {
    let dir = temp_dir("atomic-stamp");
    let path = dir.join("main.asm");
    fs::write(&path, "12345").unwrap();
    let first = stamp(&path).unwrap();
    assert_eq!(first.len, 5);
    assert!(
        first.modified.is_some(),
        "Linux and Windows report a modification time"
    );
    assert_eq!(
        stamp(&path).unwrap(),
        first,
        "an unchanged file keeps its stamp"
    );
    write_atomic(&path, bytes_of(b"123456789")).unwrap();
    let second = stamp(&path).unwrap();
    assert_eq!(second.len, 9);
    assert_ne!(second, first);
    assert_eq!(
        stamp(&dir.join("absent")).unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
}

#[test]
fn disk_stamps_match_to_the_millisecond() {
    let base = UNIX_EPOCH + Duration::from_millis(1_730_000_000_123);
    let exact = DiskStamp {
        len: 10,
        modified: Some(base + Duration::from_nanos(456_789)),
    };
    let recorded = DiskStamp {
        len: 10,
        modified: Some(base),
    };
    assert_ne!(exact, recorded);
    assert!(exact.matches(&recorded) && recorded.matches(&exact));
    let later = DiskStamp {
        modified: Some(base + Duration::from_millis(1)),
        ..recorded
    };
    assert!(!exact.matches(&later));
    let longer = DiskStamp {
        len: 11,
        ..recorded
    };
    assert!(!exact.matches(&longer));
    let unknown = DiskStamp {
        len: 10,
        modified: None,
    };
    assert!(unknown.matches(&unknown) && !unknown.matches(&recorded));
    let before_epoch = DiskStamp {
        len: 1,
        modified: UNIX_EPOCH.checked_sub(Duration::from_micros(1500)),
    };
    assert!(before_epoch.matches(&before_epoch));
}

// -------------------------------------------------------------------------------------------
// The va-io worker
// -------------------------------------------------------------------------------------------

/// Waits on `receiver` for at most 5 s.
fn recv<T>(receiver: &mpsc::Receiver<T>) -> T {
    receiver.recv_timeout(Duration::from_secs(5)).unwrap()
}

#[test]
fn jobs_run_in_order_on_the_va_io_thread() {
    let worker = IoWorker::spawn().unwrap();
    let (sender, results) = mpsc::channel();
    for index in 0..200 {
        let sender = sender.clone();
        worker.submit(Box::new(move || {
            let name = std::thread::current().name().map(str::to_owned);
            sender.send((index, name)).unwrap();
        }));
    }
    assert!(worker.flush(Duration::from_secs(5)));
    let received: Vec<(i32, Option<String>)> = results.try_iter().collect();
    assert_eq!(received.len(), 200, "flush waited for every queued job");
    for (expected, (index, name)) in received.into_iter().enumerate() {
        assert_eq!(index, i32::try_from(expected).unwrap());
        assert_eq!(name.as_deref(), Some("va-io"));
    }
}

#[test]
fn clones_share_one_worker_and_its_order() {
    let worker = IoWorker::spawn().unwrap();
    let other = worker.clone();
    let (sender, results) = mpsc::channel();
    for index in 0..50 {
        let sender = sender.clone();
        let handle = if index % 2 == 0 { &worker } else { &other };
        handle.submit(Box::new(move || {
            sender.send((index, std::thread::current().id())).unwrap();
        }));
    }
    assert!(other.flush(Duration::from_secs(5)));
    let received: Vec<_> = results.try_iter().collect();
    assert_eq!(
        received.iter().map(|(index, _)| *index).collect::<Vec<_>>(),
        (0..50).collect::<Vec<_>>()
    );
    assert!(received.iter().all(|(_, thread)| *thread == received[0].1));
    assert_ne!(received[0].1, std::thread::current().id());
}

#[test]
fn the_waker_calls_the_slot_and_is_a_no_op_before() {
    let worker = IoWorker::spawn().unwrap();
    let early = worker.waker();
    early(); // nothing set yet: no effect, no panic
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    worker.set_waker(Arc::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    }));
    early();
    worker.waker()();
    worker.clone().waker()();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "a waker taken before set_waker calls the new one"
    );

    // A job wakes the UI after sending its result.
    let (sender, results) = mpsc::channel();
    let waker = worker.waker();
    worker.submit(Box::new(move || {
        sender.send(42).unwrap();
        waker();
    }));
    assert_eq!(recv(&results), 42);
    assert!(worker.flush(Duration::from_secs(5)));
    assert_eq!(calls.load(Ordering::SeqCst), 4);

    // set_waker replaces the previous waker.
    let replaced = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&replaced);
    worker.set_waker(Arc::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    }));
    early();
    assert_eq!(calls.load(Ordering::SeqCst), 4);
    assert_eq!(replaced.load(Ordering::SeqCst), 1);
}

#[test]
fn flush_times_out_behind_a_blocked_job_then_succeeds() {
    let worker = IoWorker::spawn().unwrap();
    let (release, blocked) = mpsc::channel::<()>();
    worker.submit(Box::new(move || {
        let _ = blocked.recv_timeout(Duration::from_secs(10));
    }));
    assert!(!worker.flush(Duration::from_millis(50)));
    release.send(()).unwrap();
    assert!(worker.flush(Duration::from_secs(5)));
}

#[test]
fn a_panicking_job_does_not_stop_the_worker() {
    let worker = IoWorker::spawn().unwrap();
    let (sender, results) = mpsc::channel::<u8>();
    let dropped = sender.clone();
    worker.submit(Box::new(move || {
        let _keep = dropped;
        panic!("job bug");
    }));
    worker.submit(Box::new(move || sender.send(7).unwrap()));
    assert!(worker.flush(Duration::from_secs(5)));
    assert_eq!(recv(&results), 7);
    // The panicking job's sender was dropped with it: once the second one is gone too, the
    // receiver sees the disconnection.
    assert!(results.recv_timeout(Duration::from_secs(5)).is_err());
}

#[test]
fn the_worker_and_its_waker_cross_threads() {
    fn send_sync<T: Send + Sync>(_: &T) {}
    let worker = IoWorker::spawn().unwrap();
    send_sync(&worker);
    send_sync(&worker.waker());
    let (sender, results) = mpsc::channel();
    let from_thread = worker.clone();
    std::thread::spawn(move || {
        from_thread.submit(Box::new(move || {
            sender.send("from another thread").unwrap()
        }));
    })
    .join()
    .unwrap();
    assert_eq!(recv(&results), "from another thread");
}

#[test]
fn the_worker_writes_atomically_off_the_calling_thread() {
    let dir = temp_dir("atomic-worker");
    let target = dir.join("session.toml");
    let worker = IoWorker::spawn().unwrap();
    let (sender, results) = mpsc::channel();
    let path = target.clone();
    worker.submit(Box::new(move || {
        sender
            .send(write_atomic(&path, bytes_of(b"schema_version = 1\n")).is_ok())
            .unwrap();
    }));
    assert!(recv(&results));
    assert_eq!(fs::read(&target).unwrap(), b"schema_version = 1\n");
}
