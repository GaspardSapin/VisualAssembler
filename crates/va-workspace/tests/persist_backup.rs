//! Tests of the hot-exit backups (m4-editor-shell.md §4.8, §10.1): the write order (text, then
//! index) and the removal order (index, then text), orphan adoption, missing texts, a newer
//! index schema disables the store, damaged files, the restore order, the index layout, ids and
//! timestamps.

#![allow(clippy::unwrap_used)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};

use va_workspace::paths::Paths;
use va_workspace::persist::{
    BackupEntry, BackupId, BackupProblem, BackupStore, DiskStamp, RestoreItem, Session, SessionTab,
    TabSource, WriteFn, restore_order, utc_timestamp,
};

/// Creates a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (CLAUDE.md test rule).
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    dir
}

/// Paths of a fresh temp folder (`config` and `data` not created yet).
fn fresh_paths(tag: &str) -> Paths {
    let root = temp_dir(tag);
    Paths {
        config_dir: root.join("config"),
        data_dir: root.join("data"),
        portable: false,
    }
}

/// A well-formed id.
fn id(text: &str) -> BackupId {
    BackupId::parse(text).unwrap()
}

/// An absolute path on this host.
fn abs(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fake-root")
        .join(name)
}

/// The entry of a file buffer.
fn file_entry(id_text: &str, file: &str) -> BackupEntry {
    BackupEntry {
        id: id(id_text),
        file: Some(abs(file)),
        title: None,
        encoding: "utf-8".to_owned(),
        disk: Some(DiskStamp {
            len: 42,
            modified: Some(UNIX_EPOCH + Duration::from_millis(1_730_000_000_123)),
        }),
        updated_utc: "2026-11-04T09:30:00Z".to_owned(),
    }
}

/// The entry of an untitled buffer.
fn untitled_entry(id_text: &str, title: &str) -> BackupEntry {
    BackupEntry {
        id: id(id_text),
        file: None,
        title: Some(title.to_owned()),
        encoding: "utf-8".to_owned(),
        disk: None,
        updated_utc: "2026-11-04T09:30:01Z".to_owned(),
    }
}

/// A writer closure that writes `text`.
fn text_of(text: &str) -> WriteFn {
    let text = text.to_owned();
    Box::new(move |out| out.write_all(text.as_bytes()))
}

/// Opens the store and fails on an I/O error.
fn open(paths: &Paths) -> (BackupStore, Vec<RestoreItem>, Vec<BackupProblem>) {
    BackupStore::open(paths).unwrap()
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

/// The id of a restore item.
fn item_id(item: &RestoreItem) -> &BackupId {
    match item {
        RestoreItem::Entry { entry, .. } => &entry.id,
        RestoreItem::Orphan { id, .. } => id,
    }
}

/// The ids of restore items, in order.
fn ids(items: &[RestoreItem]) -> Vec<&str> {
    items.iter().map(|item| item_id(item).0.as_str()).collect()
}

#[test]
fn opening_a_fresh_data_folder_creates_a_private_backups_folder() {
    let paths = fresh_paths("backup-fresh");
    let (store, items, problems) = open(&paths);
    assert!(items.is_empty() && problems.is_empty());
    assert!(store.entries().is_empty() && !store.is_disabled());
    assert!(paths.backups_dir().is_dir());
    assert!(
        names_in(&paths.backups_dir()).is_empty(),
        "open writes no file"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for dir in [&paths.data_dir, &paths.backups_dir()] {
            let mode = fs::metadata(dir).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode & 0o077, 0, "{} mode {mode:o}", dir.display());
        }
    }
}

#[test]
fn written_backups_are_restored_with_their_entries_and_texts() {
    let paths = fresh_paths("backup-restore");
    let (mut store, _, _) = open(&paths);
    let file = BackupEntry {
        encoding: "windows-1252".to_owned(),
        ..file_entry("b0000000000000010", "tp1/main.asm")
    };
    let untitled = untitled_entry("b0000000000000011", "Untitled-1");
    store
        .write(file.clone(), text_of("  move.l d0,d1\r\n"))
        .unwrap();
    store.write(untitled.clone(), text_of("é\n")).unwrap();
    assert_eq!(store.entries(), [file.clone(), untitled.clone()]);
    assert_eq!(
        names_in(&paths.backups_dir()),
        [
            "b0000000000000010.txt",
            "b0000000000000011.txt",
            "index.toml"
        ]
    );

    let (reopened, items, problems) = open(&paths);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(
        items,
        [
            RestoreItem::Entry {
                entry: file.clone(),
                text: "  move.l d0,d1\r\n".to_owned()
            },
            RestoreItem::Entry {
                entry: untitled.clone(),
                text: "é\n".to_owned()
            },
        ]
    );
    assert_eq!(reopened.entries(), [file, untitled]);
}

#[test]
fn the_disk_stamp_is_kept_to_the_millisecond() {
    let paths = fresh_paths("backup-stamp");
    let (mut store, _, _) = open(&paths);
    let exact = DiskStamp {
        len: 7,
        modified: Some(UNIX_EPOCH + Duration::from_nanos(1_730_000_000_123_456_789)),
    };
    let entry = BackupEntry {
        disk: Some(exact),
        ..file_entry("b0000000000000020", "a.asm")
    };
    store.write(entry, text_of("x")).unwrap();
    let (_, items, _) = open(&paths);
    let RestoreItem::Entry { entry, .. } = &items[0] else {
        panic!("{items:?}");
    };
    let recorded = entry.disk.unwrap();
    assert_eq!(
        recorded.modified,
        Some(UNIX_EPOCH + Duration::from_millis(1_730_000_000_123))
    );
    assert!(exact.matches(&recorded), "compare with matches, not ==");

    // A stamp without a modification time keeps only its length.
    let (mut store, _, _) = open(&paths);
    let entry = BackupEntry {
        disk: Some(DiskStamp {
            len: 9,
            modified: None,
        }),
        ..file_entry("b0000000000000020", "a.asm")
    };
    store.write(entry.clone(), text_of("y")).unwrap();
    let (reopened, _, _) = open(&paths);
    assert_eq!(reopened.entries(), [entry]);
}

#[test]
fn the_text_is_written_before_the_index() {
    let paths = fresh_paths("backup-write-order");
    let (mut store, _, _) = open(&paths);
    let first = untitled_entry("b0000000000000030", "Untitled-1");
    store.write(first.clone(), text_of("v1")).unwrap();

    // While the text of a new entry is written, the index does not list it and its final
    // `<id>.txt` does not exist yet.
    let seen = Arc::new(Mutex::new(None));
    let (index, text, slot) = (
        paths.backups_index(),
        paths.backups_dir().join("b0000000000000031.txt"),
        Arc::clone(&seen),
    );
    let second = untitled_entry("b0000000000000031", "Untitled-2");
    store
        .write(
            second,
            Box::new(move |out| {
                let index_text = fs::read_to_string(&index).unwrap();
                *slot.lock().unwrap() = Some((index_text, text.exists()));
                out.write_all(b"other")
            }),
        )
        .unwrap();
    let (index_during, text_existed) = seen.lock().unwrap().take().unwrap();
    assert!(index_during.contains("b0000000000000030"));
    assert!(
        !index_during.contains("b0000000000000031"),
        "{index_during}"
    );
    assert!(!text_existed);
    let index_after = fs::read_to_string(paths.backups_index()).unwrap();
    assert!(index_after.contains("b0000000000000031"));

    // An update replaces the text while the index still holds the previous entry.
    let updated = BackupEntry {
        updated_utc: "2026-11-04T10:00:00Z".to_owned(),
        ..first
    };
    let (index, slot) = (paths.backups_index(), Arc::clone(&seen));
    store
        .write(
            updated,
            Box::new(move |out| {
                *slot.lock().unwrap() = Some((fs::read_to_string(&index).unwrap(), true));
                out.write_all(b"v2")
            }),
        )
        .unwrap();
    let (index_during, _) = seen.lock().unwrap().take().unwrap();
    assert!(index_during.contains("2026-11-04T09:30:01Z"));
    assert!(!index_during.contains("2026-11-04T10:00:00Z"));
    assert!(
        fs::read_to_string(paths.backups_index())
            .unwrap()
            .contains("2026-11-04T10:00:00Z")
    );
}

#[test]
fn a_failed_text_write_leaves_the_index_and_the_old_text() {
    let paths = fresh_paths("backup-text-fails");
    let (mut store, _, _) = open(&paths);
    let entry = untitled_entry("b0000000000000040", "Untitled-1");
    store.write(entry.clone(), text_of("v1")).unwrap();
    let index_before = fs::read(paths.backups_index()).unwrap();

    let failing: WriteFn = Box::new(|out| {
        out.write_all(b"half of v2")?;
        Err(io::Error::other("encoding failed"))
    });
    store
        .write(
            BackupEntry {
                updated_utc: "later".to_owned(),
                ..entry.clone()
            },
            failing,
        )
        .unwrap_err();
    store
        .write(
            untitled_entry("b0000000000000041", "Untitled-2"),
            Box::new(|_| Err(io::Error::other("no"))),
        )
        .unwrap_err();
    assert_eq!(fs::read(paths.backups_index()).unwrap(), index_before);
    assert_eq!(store.entries(), [entry]);
    assert_eq!(
        names_in(&paths.backups_dir()),
        ["b0000000000000040.txt", "index.toml"]
    );
    let (_, items, _) = open(&paths);
    assert!(matches!(&items[..], [RestoreItem::Entry { text, .. }] if text == "v1"));
}

/// Puts a non-empty folder where `index.toml` is, so every index write fails on every OS (no
/// rename replaces a non-empty folder); returns the saved index for [`unblock_index`].
fn block_index(paths: &Paths) -> Vec<u8> {
    let saved = fs::read(paths.backups_index()).unwrap_or_default();
    let _ = fs::remove_file(paths.backups_index());
    fs::create_dir(paths.backups_index()).unwrap();
    fs::write(paths.backups_index().join("blocker"), "x").unwrap();
    saved
}

/// Undoes [`block_index`].
fn unblock_index(paths: &Paths, saved: &[u8]) {
    fs::remove_file(paths.backups_index().join("blocker")).unwrap();
    fs::remove_dir(paths.backups_index()).unwrap();
    fs::write(paths.backups_index(), saved).unwrap();
}

#[test]
fn the_index_is_written_before_the_text_is_removed() {
    let paths = fresh_paths("backup-remove-order");
    let (mut store, _, _) = open(&paths);
    let keep = untitled_entry("b0000000000000050", "Untitled-1");
    let gone = untitled_entry("b0000000000000051", "Untitled-2");
    store.write(keep.clone(), text_of("keep")).unwrap();
    store.write(gone.clone(), text_of("gone")).unwrap();
    let text = paths.backups_dir().join("b0000000000000051.txt");

    // The index write fails: the entry and its text both stay.
    let saved = block_index(&paths);
    store.remove(&gone.id).unwrap_err();
    assert!(text.is_file(), "the text is removed only after the index");
    assert_eq!(store.entries(), [keep.clone(), gone.clone()]);
    unblock_index(&paths, &saved);

    store.remove(&gone.id).unwrap();
    assert!(!text.exists());
    assert_eq!(store.entries(), [keep]);
    let index = fs::read_to_string(paths.backups_index()).unwrap();
    assert!(!index.contains("b0000000000000051") && index.contains("b0000000000000050"));
    let (_, items, problems) = open(&paths);
    assert!(problems.is_empty());
    assert_eq!(ids(&items), ["b0000000000000050"]);

    // Removing an id that is already gone is not an error.
    store.remove(&gone.id).unwrap();
}

#[test]
fn a_failed_index_write_after_the_text_leaves_an_orphan() {
    let paths = fresh_paths("backup-index-fails");
    let (mut store, _, _) = open(&paths);
    store
        .write(
            untitled_entry("b0000000000000060", "Untitled-1"),
            text_of("one"),
        )
        .unwrap();
    let saved = block_index(&paths);
    store
        .write(
            untitled_entry("b0000000000000061", "Untitled-2"),
            text_of("two"),
        )
        .unwrap_err();
    unblock_index(&paths, &saved);
    let (_, items, problems) = open(&paths);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(
        items[1],
        RestoreItem::Orphan {
            id: id("b0000000000000061"),
            text: "two".to_owned()
        }
    );
    assert_eq!(ids(&items), ["b0000000000000060", "b0000000000000061"]);
}

#[test]
fn an_orphan_text_is_restored_and_adopted_by_a_write() {
    let paths = fresh_paths("backup-orphan");
    paths.create_backups_dir().unwrap();
    fs::write(
        paths.backups_dir().join("b00000000000000a1.txt"),
        "lost work",
    )
    .unwrap();
    fs::write(paths.backups_dir().join("b00000000000000a2.txt"), "more").unwrap();
    let (mut store, items, problems) = open(&paths);
    assert!(problems.is_empty());
    assert_eq!(
        items,
        [
            RestoreItem::Orphan {
                id: id("b00000000000000a1"),
                text: "lost work".to_owned()
            },
            RestoreItem::Orphan {
                id: id("b00000000000000a2"),
                text: "more".to_owned()
            },
        ]
    );
    assert!(store.entries().is_empty());

    // Writing an entry with the orphan's id adopts it into the index.
    let adopted = untitled_entry("b00000000000000a1", "Recovered");
    store
        .write(adopted.clone(), text_of("lost work, edited"))
        .unwrap();
    // Removing the other orphan deletes its text.
    store.remove(&id("b00000000000000a2")).unwrap();
    assert_eq!(
        names_in(&paths.backups_dir()),
        ["b00000000000000a1.txt", "index.toml"]
    );
    let (_, items, problems) = open(&paths);
    assert!(problems.is_empty());
    assert_eq!(
        items,
        [RestoreItem::Entry {
            entry: adopted,
            text: "lost work, edited".to_owned()
        }]
    );
}

#[test]
fn a_missing_text_is_reported_and_dropped_from_the_index() {
    let paths = fresh_paths("backup-missing");
    let (mut store, _, _) = open(&paths);
    store
        .write(file_entry("b0000000000000070", "a.asm"), text_of("a"))
        .unwrap();
    store
        .write(
            untitled_entry("b0000000000000071", "Untitled-1"),
            text_of("b"),
        )
        .unwrap();
    fs::remove_file(paths.backups_dir().join("b0000000000000070.txt")).unwrap();
    let (mut store, items, problems) = open(&paths);
    assert_eq!(
        problems,
        [BackupProblem::MissingText(id("b0000000000000070"))]
    );
    assert_eq!(ids(&items), ["b0000000000000071"]);
    assert_eq!(store.entries().len(), 1);
    store
        .write(
            untitled_entry("b0000000000000072", "Untitled-2"),
            text_of("c"),
        )
        .unwrap();
    let index = fs::read_to_string(paths.backups_index()).unwrap();
    assert!(!index.contains("b0000000000000070"), "{index}");
    let (_, items, problems) = open(&paths);
    assert!(problems.is_empty());
    assert_eq!(ids(&items), ["b0000000000000071", "b0000000000000072"]);
}

#[test]
fn a_newer_index_schema_disables_the_store_and_is_never_rewritten() {
    let paths = fresh_paths("backup-newer");
    paths.create_backups_dir().unwrap();
    let index = "schema_version = 2\n\n[[backup]]\nid = \"b0000000000000080\"\nshape = \"new\"\n";
    fs::write(paths.backups_index(), index).unwrap();
    fs::write(paths.backups_dir().join("b0000000000000080.txt"), "future").unwrap();
    let (mut store, items, problems) = open(&paths);
    assert_eq!(problems, [BackupProblem::NewerSchema]);
    assert!(items.is_empty(), "nothing is restored from a newer index");
    assert!(store.is_disabled());
    let error = store
        .write(
            untitled_entry("b0000000000000081", "Untitled-1"),
            text_of("x"),
        )
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    let error = store.remove(&id("b0000000000000080")).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert_eq!(fs::read_to_string(paths.backups_index()).unwrap(), index);
    assert_eq!(
        names_in(&paths.backups_dir()),
        ["b0000000000000080.txt", "index.toml"]
    );
}

#[test]
fn a_damaged_index_is_reported_and_its_texts_come_back_as_orphans() {
    let paths = fresh_paths("backup-damaged");
    paths.create_backups_dir().unwrap();
    fs::write(paths.backups_dir().join("b0000000000000090.txt"), "kept").unwrap();
    for (index, problem_part) in [
        ("schema_version = 1\n[[backup]\n", "not valid TOML"),
        ("schema_version = \"1\"\n", "invalid schema_version"),
        ("schema_version = 0\n", "invalid schema_version"),
        ("backup = 3\n", "not an array of tables"),
    ] {
        fs::write(paths.backups_index(), index).unwrap();
        let (store, items, problems) = open(&paths);
        assert!(!store.is_disabled(), "{index:?}");
        assert!(
            matches!(&problems[..], [BackupProblem::Unreadable(detail)] if detail.contains(problem_part)),
            "{index:?}: {problems:?}"
        );
        assert_eq!(
            items,
            [RestoreItem::Orphan {
                id: id("b0000000000000090"),
                text: "kept".to_owned()
            }],
            "{index:?}"
        );
    }
    fs::write(paths.backups_index(), [0xff_u8, 0xfe]).unwrap();
    let (mut store, items, problems) = open(&paths);
    assert!(
        matches!(&problems[..], [BackupProblem::Unreadable(detail)] if detail.contains("not UTF-8"))
    );
    assert_eq!(ids(&items), ["b0000000000000090"]);
    // The store rewrites a damaged index.
    store
        .write(
            untitled_entry("b0000000000000090", "Recovered"),
            text_of("kept"),
        )
        .unwrap();
    let (_, items, problems) = open(&paths);
    assert!(problems.is_empty());
    assert!(matches!(&items[..], [RestoreItem::Entry { .. }]));
}

#[test]
fn bad_index_entries_are_skipped_and_their_neighbours_kept() {
    let paths = fresh_paths("backup-bad-entries");
    paths.create_backups_dir().unwrap();
    for name in ["b00000000000000b1", "b00000000000000b2"] {
        fs::write(paths.backups_dir().join(format!("{name}.txt")), name).unwrap();
    }
    let index = "schema_version = 1\n\
                 [[backup]]\nid = \"B00000000000000B1\"\n\
                 [[backup]]\nid = \"b00000000000000b2\"\nencoding = 5\ndisk_len = -3\n\
                 [[backup]]\ntitle = \"no id\"\n\
                 [[backup]]\nid = \"b00000000000000b2\"\ntitle = \"twice\"\n";
    fs::write(paths.backups_index(), index).unwrap();
    let (store, items, problems) = open(&paths);
    assert_eq!(problems.len(), 3, "{problems:?}");
    assert!(
        problems
            .iter()
            .all(|problem| matches!(problem, BackupProblem::Unreadable(_)))
    );
    assert_eq!(
        store.entries(),
        [BackupEntry {
            id: id("b00000000000000b2"),
            file: None,
            title: None,
            encoding: "utf-8".to_owned(),
            disk: None,
            updated_utc: String::new(),
        }],
        "wrong-typed optional fields are absent; a missing encoding is utf-8"
    );
    assert_eq!(ids(&items), ["b00000000000000b2", "b00000000000000b1"]);
    assert!(matches!(items[1], RestoreItem::Orphan { .. }));
}

#[test]
fn an_unreadable_text_is_reported_kept_and_not_restored() {
    let paths = fresh_paths("backup-not-utf8");
    let (mut store, _, _) = open(&paths);
    let entry = file_entry("b00000000000000c0", "a.asm");
    store.write(entry.clone(), text_of("fine")).unwrap();
    fs::write(
        paths.backups_dir().join("b00000000000000c0.txt"),
        b"ok\xff\xfe",
    )
    .unwrap();
    let (store, items, problems) = open(&paths);
    assert!(items.is_empty());
    assert_eq!(
        problems,
        [BackupProblem::Unreadable(
            "backups/b00000000000000c0.txt is not UTF-8 (byte 2)".to_owned()
        )]
    );
    assert_eq!(store.entries(), [entry], "the entry is kept for later");
}

#[test]
fn only_backup_text_files_are_read() {
    let paths = fresh_paths("backup-names");
    let (mut store, _, _) = open(&paths);
    store
        .write(
            untitled_entry("b00000000000000d0", "Untitled-1"),
            text_of("real"),
        )
        .unwrap();
    let dir = paths.backups_dir();
    let pid = std::process::id();
    for decoy in [
        format!("b00000000000000d0.txt.{pid}.0.tmp"),
        format!("b00000000000000d1.txt.{pid}.7.tmp"),
        format!("index.toml.{pid}.3.tmp"),
        "B00000000000000D2.txt".to_owned(),
        "b00000000000000d3.TXT".to_owned(),
        "b0000000000000d4.txt".to_owned(),
        "notes.txt".to_owned(),
        "b00000000000000d5.txt.bak".to_owned(),
    ] {
        fs::write(dir.join(decoy), "torn").unwrap();
    }
    fs::create_dir(dir.join("b00000000000000d6.txt")).unwrap();
    let (_, items, problems) = open(&paths);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(
        items,
        [RestoreItem::Entry {
            entry: untitled_entry("b00000000000000d0", "Untitled-1"),
            text: "real".to_owned()
        }]
    );
}

#[test]
fn items_come_in_index_order_then_orphans_and_follow_the_session_tabs() {
    let paths = fresh_paths("backup-order");
    let (mut store, _, _) = open(&paths);
    store
        .write(file_entry("b00000000000000e5", "z.asm"), text_of("z"))
        .unwrap();
    store
        .write(
            untitled_entry("b00000000000000e1", "Untitled-1"),
            text_of("u1"),
        )
        .unwrap();
    store
        .write(file_entry("b00000000000000e3", "a.asm"), text_of("a"))
        .unwrap();
    // Rewriting an entry keeps its place in the index.
    store
        .write(file_entry("b00000000000000e5", "z.asm"), text_of("z2"))
        .unwrap();
    for orphan in ["b00000000000000e9", "b00000000000000e2"] {
        fs::write(paths.backups_dir().join(format!("{orphan}.txt")), orphan).unwrap();
    }
    let (_, items, _) = open(&paths);
    assert_eq!(
        ids(&items),
        [
            "b00000000000000e5",
            "b00000000000000e1",
            "b00000000000000e3",
            "b00000000000000e2",
            "b00000000000000e9",
        ]
    );

    let tab = |source: TabSource| SessionTab {
        source,
        line: 1,
        column: 1,
        top_line: 1,
        active: false,
    };
    let session = Session {
        tabs: vec![
            tab(TabSource::File(abs("clean.asm"))),
            tab(TabSource::Backup(id("b00000000000000e9"))),
            tab(TabSource::File(abs("a.asm"))),
            tab(TabSource::Backup(id("b00000000000000e1"))),
            tab(TabSource::Backup(id("b00000000000000e3"))),
        ],
        ..Session::default()
    };
    let ordered = restore_order(items.clone(), &session);
    assert_eq!(
        ids(&ordered),
        [
            "b00000000000000e9",
            "b00000000000000e3",
            "b00000000000000e1",
            "b00000000000000e5",
            "b00000000000000e2",
        ]
    );
    assert_eq!(restore_order(items.clone(), &Session::default()), items);
}

#[test]
fn the_index_layout_is_fixed() {
    let paths = fresh_paths("backup-layout");
    let (mut store, _, _) = open(&paths);
    store
        .write(
            BackupEntry {
                encoding: "windows-1252".to_owned(),
                ..file_entry("b0000000000000100", "tp1/main.asm")
            },
            text_of("x"),
        )
        .unwrap();
    store
        .write(
            untitled_entry("b0000000000000101", "Untitled \"1\""),
            text_of("y"),
        )
        .unwrap();
    let file = abs("tp1/main.asm");
    let file = file.to_str().unwrap().replace('\\', "\\\\");
    assert_eq!(
        fs::read_to_string(paths.backups_index()).unwrap(),
        format!(
            "schema_version = 1\n\
             \n\
             [[backup]]\n\
             id = \"b0000000000000100\"\n\
             file = \"{file}\"\n\
             encoding = \"windows-1252\"\n\
             disk_len = 42\n\
             disk_modified_ms = 1730000000123\n\
             updated_utc = \"2026-11-04T09:30:00Z\"\n\
             \n\
             [[backup]]\n\
             id = \"b0000000000000101\"\n\
             title = \"Untitled \\\"1\\\"\"\n\
             encoding = \"utf-8\"\n\
             updated_utc = \"2026-11-04T09:30:01Z\"\n"
        )
    );
}

#[test]
fn ids_that_are_not_well_formed_are_refused() {
    let paths = fresh_paths("backup-bad-id");
    let (mut store, _, _) = open(&paths);
    for bad in [
        "../escape",
        "b0123",
        "",
        "b0123456789ABCDEF",
        "b0123456789abcdef/x",
    ] {
        let entry = BackupEntry {
            id: BackupId(bad.to_owned()),
            ..untitled_entry("b0000000000000001", "x")
        };
        let error = store.write(entry, text_of("x")).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput, "{bad:?}");
        let error = store.remove(&BackupId(bad.to_owned())).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput, "{bad:?}");
    }
    assert!(names_in(&paths.backups_dir()).is_empty());
    assert_eq!(names_in(&paths.data_dir), ["backups"]);
}

#[cfg(unix)]
#[test]
fn a_file_path_that_is_not_unicode_keeps_its_text_as_untitled() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let paths = fresh_paths("backup-not-unicode");
    let (mut store, _, _) = open(&paths);
    let path = abs("tp1").join(OsStr::from_bytes(b"caf\xe9.asm"));
    let entry = BackupEntry {
        file: Some(path.clone()),
        ..file_entry("b0000000000000110", "unused")
    };
    store.write(entry, text_of("text")).unwrap();
    let (_, items, problems) = open(&paths);
    assert!(problems.is_empty());
    let [RestoreItem::Entry { entry, text }] = &items[..] else {
        panic!("{items:?}");
    };
    assert_eq!(text, "text");
    assert_eq!(entry.file, None);
    assert_eq!(entry.disk, None);
    assert_eq!(
        entry.title.as_deref(),
        Some(path.to_string_lossy().as_ref())
    );
}

#[test]
fn backup_ids_parse_only_when_well_formed() {
    assert_eq!(
        BackupId::parse("b0123456789abcdef"),
        Some(BackupId("b0123456789abcdef".to_owned()))
    );
    for bad in [
        "",
        "b",
        "b0123456789abcde",
        "b0123456789abcdef0",
        "B0123456789abcdef",
        "b0123456789ABCDEF",
        "b0123456789abcdeg",
        " b0123456789abcdef",
        "b0123456789abcdé",
    ] {
        assert_eq!(BackupId::parse(bad), None, "{bad:?}");
        assert!(!BackupId(bad.to_owned()).is_well_formed());
    }
}

#[test]
fn generated_ids_are_well_formed_increasing_and_above_the_ids_seen() {
    let mut previous = BackupId::generate();
    for _ in 0..10_000 {
        let next = BackupId::generate();
        assert!(next.is_well_formed(), "{next:?}");
        assert!(next > previous, "{next:?} > {previous:?}");
        previous = next;
    }
    // An id far in the future, found by open, still gets a larger next id.
    let paths = fresh_paths("backup-generate");
    paths.create_backups_dir().unwrap();
    fs::write(paths.backups_dir().join("bfff0000000000000.txt"), "far").unwrap();
    let _ = open(&paths);
    let next = BackupId::generate();
    assert!(next > id("bfff0000000000000"), "{next:?}");
    assert!(next.is_well_formed());
}

#[test]
fn timestamps_are_rfc3339_utc_to_the_second() {
    let at = |seconds: u64| utc_timestamp(UNIX_EPOCH + Duration::from_secs(seconds));
    assert_eq!(at(0), "1970-01-01T00:00:00Z");
    assert_eq!(at(951_782_400), "2000-02-29T00:00:00Z");
    assert_eq!(at(1_700_000_000), "2023-11-14T22:13:20Z");
    assert_eq!(at(1_793_871_000), "2026-11-05T09:30:00Z");
    assert_eq!(at(4_107_542_399), "2100-02-28T23:59:59Z");
    assert_eq!(
        utc_timestamp(UNIX_EPOCH + Duration::from_millis(1_999)),
        "1970-01-01T00:00:01Z"
    );
    assert_eq!(
        utc_timestamp(UNIX_EPOCH - Duration::from_secs(1)),
        "1969-12-31T23:59:59Z"
    );
}
