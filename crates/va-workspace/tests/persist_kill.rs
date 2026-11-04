//! The kill test of exit criterion E4 (m4-editor-shell.md §4.8, §10.1): a child process writes
//! backup versions in a loop and is killed 20 times; after each kill the store reopens, every
//! restored text is one complete version, versions never go backwards, no buffer goes missing,
//! and no `*.tmp` file is ever read.
//!
//! The child is this test binary run again (`std::env::current_exe()` with `--exact
//! kill_helper --ignored`) with `VA_KILL_HELPER_DIR` naming the test folder. It keeps two
//! buffers: `A`, a file buffer under a fixed id rewritten in place, and `B`, an untitled buffer
//! that moves to a fresh id at each version (write the new id, then remove the old one), so a
//! kill can also land between the text and the index writes (an orphan) and inside a removal.
//! Each version is 64 KiB to 1 MiB of lines that all hold its number. The kill delays come from
//! a fixed-seed xorshift in 5–200 ms, counted from the child's ready line, so each kill lands in
//! the write loop rather than in the process start.

#![allow(clippy::unwrap_used)]

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

use va_workspace::paths::Paths;
use va_workspace::persist::{
    BackupEntry, BackupId, BackupStore, DiskStamp, RestoreItem, WriteFn, utc_timestamp,
};

/// The variable that turns [`kill_helper`] on and names the test folder.
const HELPER_DIR_VAR: &str = "VA_KILL_HELPER_DIR";
/// The line the helper prints once its store is open.
const READY: &str = "va-kill-helper-ready";
/// Number of kills.
const KILLS: usize = 20;
/// Seed of the kill delays.
const DELAY_SEED: u64 = 0x5EED_F00D_4B11_7E57;
/// Shortest and longest kill delay, in milliseconds.
const DELAY_MS: (u64, u64) = (5, 200);
/// The fixed id of buffer `A`.
const FILE_ID: &str = "b0000000000000a00";
/// Smallest and largest version text, in bytes.
const VERSION_BYTES: (u64, u64) = (64 * 1024, 1024 * 1024);
/// Length of one version line, newline included.
const LINE_BYTES: u64 = 64;
/// How long a helper writes at most, should no kill come (a stray child never runs forever).
const HELPER_LIFETIME: Duration = Duration::from_secs(60);
/// How long the parent waits for the ready line.
const READY_TIMEOUT: Duration = Duration::from_secs(60);
/// The content of the planted temp files, which must never come back as a backup.
const DECOY: &str = "v9999999999 torn temp file";

/// One step of xorshift64.
fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// The text of `version`: 64 KiB to 1 MiB of 64-byte lines that each start with `v<version>`.
fn version_text(version: u64) -> String {
    let mut state = version.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let span = VERSION_BYTES.1 - VERSION_BYTES.0 + 1;
    let size = VERSION_BYTES.0 + xorshift(&mut state) % span;
    let lines = size / LINE_BYTES;
    let mut text = String::with_capacity(usize::try_from(lines * LINE_BYTES).unwrap());
    for line in 0..lines {
        let start = text.len();
        text.push_str(&format!("v{version:010} line {line:08} "));
        let pad = usize::try_from(LINE_BYTES).unwrap() - 1 - (text.len() - start);
        text.extend(std::iter::repeat_n('.', pad));
        text.push('\n');
    }
    text
}

/// The version of `text` when it is exactly one complete version, else `None`.
fn complete_version(text: &str) -> Option<u64> {
    let version: u64 = text.strip_prefix('v')?.get(..10)?.parse().ok()?;
    (text == version_text(version)).then_some(version)
}

/// The folders of the test folder `dir`.
fn paths_in(dir: &Path) -> Paths {
    Paths {
        config_dir: dir.join("config"),
        data_dir: dir.join("data"),
        portable: false,
    }
}

/// A writer closure for `text`.
fn writer(text: String) -> WriteFn {
    Box::new(move |out| out.write_all(text.as_bytes()))
}

/// The entry of buffer `A`.
fn file_entry(dir: &Path, len: usize) -> BackupEntry {
    BackupEntry {
        id: BackupId::parse(FILE_ID).unwrap(),
        file: Some(dir.join("project").join("main.asm")),
        title: None,
        encoding: "utf-8".to_owned(),
        disk: Some(DiskStamp {
            len: u64::try_from(len).unwrap(),
            modified: None,
        }),
        updated_utc: utc_timestamp(SystemTime::now()),
    }
}

/// The entry of buffer `B` under `id`.
fn untitled_entry(id: BackupId) -> BackupEntry {
    BackupEntry {
        id,
        file: None,
        title: Some("Untitled-1".to_owned()),
        encoding: "utf-8".to_owned(),
        disk: None,
        updated_utc: utc_timestamp(SystemTime::now()),
    }
}

/// The id and text of a restore item.
fn parts(item: &RestoreItem) -> (&BackupId, &str) {
    match item {
        RestoreItem::Entry { entry, text } => (&entry.id, text),
        RestoreItem::Orphan { id, text } => (id, text),
    }
}

/// The helper process: writes versions of `A` and `B` until killed. Without
/// `VA_KILL_HELPER_DIR` (a plain `cargo test -- --ignored`) it does nothing.
#[test]
#[ignore = "helper of kill_restores_one_complete_earlier_version; runs in a child process"]
fn kill_helper() {
    let Some(dir) = std::env::var_os(HELPER_DIR_VAR) else {
        return;
    };
    let dir = PathBuf::from(dir);
    let paths = paths_in(&dir);
    let (mut store, items, _) = BackupStore::open(&paths).unwrap();
    let file_id = BackupId::parse(FILE_ID).unwrap();
    let mut version = 0;
    let mut untitled: Vec<(BackupId, u64)> = Vec::new();
    for item in &items {
        let (id, text) = parts(item);
        let found = complete_version(text).unwrap_or(0);
        version = version.max(found);
        if *id != file_id {
            untitled.push((id.clone(), found));
        }
    }
    {
        let mut out = std::io::stdout().lock();
        writeln!(out, "{READY}").unwrap();
        out.flush().unwrap();
    }
    // Keep the newest copy of `B` and discard the others (a kill can leave two).
    untitled.sort_by_key(|(_, found)| *found);
    let mut current = match untitled.pop() {
        Some((id, _)) => id,
        None => BackupId::generate(),
    };
    for (id, _) in untitled {
        store.remove(&id).unwrap();
    }
    let started = Instant::now();
    while started.elapsed() < HELPER_LIFETIME {
        version += 1;
        let text = version_text(version);
        store
            .write(file_entry(&dir, text.len()), writer(text.clone()))
            .unwrap();
        let next = BackupId::generate();
        store
            .write(untitled_entry(next.clone()), writer(text))
            .unwrap();
        store.remove(&current).unwrap();
        current = next;
    }
}

/// A running helper, killed on drop so a failing assertion never leaves it behind.
struct Helper {
    /// The child process.
    child: Child,
    /// Its stderr log.
    log: PathBuf,
}

impl Helper {
    /// Starts the helper on `dir` and waits for its ready line.
    fn start(dir: &Path, round: usize) -> Self {
        let log = dir.join(format!("helper-{round}.log"));
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "kill_helper",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(HELPER_DIR_VAR, dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(File::create(&log).unwrap())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (ready, is_ready) = mpsc::channel();
        // Reads every line until the child dies, so its pipe never fills.
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if line.contains(READY) {
                    let _ = ready.send(());
                }
            }
        });
        let mut helper = Self { child, log };
        if is_ready.recv_timeout(READY_TIMEOUT).is_err() {
            helper.kill();
            panic!(
                "round {round}: the helper never got ready; its stderr:\n{}",
                helper.stderr()
            );
        }
        helper
    }

    /// The helper's stderr so far.
    fn stderr(&self) -> String {
        fs::read_to_string(&self.log).unwrap_or_default()
    }

    /// Kills the helper and waits for it; fails if it had already exited (a write error).
    fn kill_running(&mut self, round: usize) {
        if let Some(status) = self.child.try_wait().unwrap() {
            panic!(
                "round {round}: the helper exited by itself ({status}); its stderr:\n{}",
                self.stderr()
            );
        }
        self.kill();
    }

    /// Kills the helper (SIGKILL on Unix, TerminateProcess on Windows) and reaps it.
    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Creates a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (CLAUDE.md test rule).
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    dir
}

/// The names in `dir` that end with `suffix`.
fn names_ending(dir: &Path, suffix: &str) -> Vec<String> {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(suffix))
        .collect()
}

#[test]
fn version_texts_are_complete_and_sized() {
    for version in [0, 1, 2, 77, 123_456, 9_999_999_999] {
        let text = version_text(version);
        let len = u64::try_from(text.len()).unwrap();
        assert!((VERSION_BYTES.0 - LINE_BYTES..=VERSION_BYTES.1).contains(&len));
        assert_eq!(len % LINE_BYTES, 0);
        assert_eq!(complete_version(&text), Some(version));
        assert!(
            text.lines()
                .all(|line| line.starts_with(&format!("v{version:010} ")))
        );
        let torn = &text[..text.len() - 1];
        assert_eq!(complete_version(torn), None);
        assert_eq!(complete_version(&text[..text.len() / 2]), None);
    }
    assert_eq!(complete_version(DECOY), None);
}

#[test]
fn kill_restores_one_complete_earlier_version() {
    let dir = temp_dir("persist-kill");
    let paths = paths_in(&dir);
    let file_id = BackupId::parse(FILE_ID).unwrap();

    // Version 0 of both buffers exists before the first kill, so a missing buffer is a failure.
    let (mut store, _, _) = BackupStore::open(&paths).unwrap();
    let seed = version_text(0);
    store
        .write(file_entry(&dir, seed.len()), writer(seed.clone()))
        .unwrap();
    store
        .write(untitled_entry(BackupId::generate()), writer(seed))
        .unwrap();
    drop(store);
    // Temp files as a kill would leave them, beside real names: they must never be read.
    let backups = paths.backups_dir();
    let decoys = [
        format!("{FILE_ID}.txt.4242424242.0.tmp"),
        "index.toml.4242424242.1.tmp".to_owned(),
    ];
    for decoy in &decoys {
        fs::write(backups.join(decoy), DECOY).unwrap();
    }

    let mut delays = DELAY_SEED;
    let (mut last_file, mut last_untitled) = (0, 0);
    let mut temps_left = 0;
    let started = Instant::now();
    for round in 0..KILLS {
        let delay = DELAY_MS.0 + xorshift(&mut delays) % (DELAY_MS.1 - DELAY_MS.0 + 1);
        let mut helper = Helper::start(&dir, round);
        std::thread::sleep(Duration::from_millis(delay));
        helper.kill_running(round);
        drop(helper);

        // 1. `open` succeeds after the kill.
        let (_, items, problems) = BackupStore::open(&paths)
            .unwrap_or_else(|error| panic!("round {round}: open failed: {error}"));
        assert!(problems.is_empty(), "round {round}: {problems:?}");
        let texts = names_ending(&backups, ".txt");
        // 4. Every item comes from one `<id>.txt`; the temp files are never read.
        assert_eq!(items.len(), texts.len(), "round {round}: {texts:?}");
        temps_left = names_ending(&backups, ".tmp").len() - decoys.len();
        for decoy in &decoys {
            assert_eq!(fs::read_to_string(backups.join(decoy)).unwrap(), DECOY);
        }
        let (mut file_version, mut untitled_version) = (None, None);
        for item in &items {
            let (id, text) = parts(item);
            assert!(
                texts.contains(&format!("{}.txt", id.0)),
                "round {round}: {id:?}"
            );
            // 2. Every restored text is one complete version.
            let version = complete_version(text).unwrap_or_else(|| {
                panic!(
                    "round {round}: {} is torn ({} bytes, starts {:?})",
                    id.0,
                    text.len(),
                    text.get(..24)
                )
            });
            if *id == file_id {
                file_version = Some(version);
            } else {
                untitled_version = untitled_version.max(Some(version));
            }
        }
        // Never missing, and 3. never backwards.
        let file_version =
            file_version.unwrap_or_else(|| panic!("round {round}: the file buffer is missing"));
        let untitled_version = untitled_version
            .unwrap_or_else(|| panic!("round {round}: the untitled buffer is missing"));
        assert!(file_version >= last_file, "round {round}: file went back");
        assert!(
            untitled_version >= last_untitled,
            "round {round}: untitled went back"
        );
        last_file = file_version;
        last_untitled = untitled_version;
    }
    assert!(
        last_file >= 1,
        "the helpers never completed a version: the kills tested nothing"
    );
    println!(
        "{KILLS} kills in {:?}: file buffer at v{last_file}, untitled at v{last_untitled}, \
         {temps_left} temp files left by the kills (never read)",
        started.elapsed()
    );
}
