//! Bench build and spawn: the shipped release GUI binary, built once, and one measured child
//! process per run (m0-foundations.md §6.1 steps 1 and 2, amended by §17.8 and §19.5;
//! m4-editor-shell.md §7.1, §7.2).
//!
//! Every run passes `--renderer <auto|glow|wgpu> --bench <scenario>`, plus `--bench-input <path>`
//! for the scenarios that open a generated input and `--duration-secs N` for idle. Its run line is
//! validated against that request ([`validate_run`]): a forced renderer must run as asked with no
//! fallback, while under `auto` any renderer is valid and a fallback is counted by the report, not
//! rejected; the line must name the requested input, and the editor scenarios must report their
//! open time, run without vsync and report every injected event.
//!
//! Every run gets `VA_CONFIG_DIR` and `VA_DATA_DIR` pointing to the `config` and `data` folders
//! of one [`BenchProfile`], created once per command under `<target>/bench/`, so the user's
//! settings never affect a report.
//!
//! A run never waits unboundedly: the child gets a deadline that is strictly larger than the
//! GUI's own watchdog, and once it has exited or been killed the output readers get at most
//! [`READER_JOIN_TIMEOUT`] to finish. Readers still blocked after that (a descendant process
//! holds the pipes) are detached and the run fails. Kept output is bounded: the last
//! [`STDERR_TAIL_BYTES`] of stderr, and stdout lines of at most [`MAX_LINE_BYTES`].

use std::{
    collections::VecDeque,
    ffi::OsString,
    fs,
    io::{self, BufRead, BufReader, Read as _},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime},
};

use anyhow::{Context as _, Result, anyhow, bail};
use serde_json::Value;

use super::{
    RendererChoice, Scenario,
    report::{BenchRun, RUN_SCHEMA, Sample},
    timestamp,
};
use crate::exec;

/// Package and binary name of the GUI.
pub const GUI_PACKAGE: &str = "visual-assembler";

/// Folder of the measured copy under `<target_directory>/bench/` (§19.5).
pub const SHIPPED_DIR: &str = "shipped";

/// Run timeout of the startup, open, scroll and type scenarios, in seconds. Strictly larger than
/// the GUI's 60 s startup watchdog, which those scenarios use (m4-editor-shell.md §6.8), so the
/// GUI's own `bench timed out` (exit 4) surfaces under xtask (§17.8).
pub const STARTUP_TIMEOUT_SECS: u64 = 75;

/// Added to the idle duration N to get the idle run timeout (N + 75 s), in seconds. Strictly
/// larger than the GUI's idle watchdog of N + 63 s (60 s + N + 3 s settle, §17.8).
pub const IDLE_TIMEOUT_EXTRA_SECS: u64 = 75;

/// Longest wait for both output readers once the child has exited or been killed (§17.8).
pub const READER_JOIN_TIMEOUT: Duration = Duration::from_secs(5);

/// Bytes of child stderr kept for error messages: the last 64 KiB (§17.8).
pub const STDERR_TAIL_BYTES: usize = 64 * 1024;

/// Longest stdout line kept, in bytes (line break excluded). A longer line is truncated and can
/// never count as the run line (§17.8).
pub const MAX_LINE_BYTES: usize = 1024 * 1024;

/// Failure of a run whose output pipes outlive the GUI (§17.8, exact text).
pub const PIPES_OPEN_MESSAGE: &str =
    "output pipes still open after the GUI exited (a descendant process may hold them)";

/// Environment variable that points the GUI to its configuration folder (m4-editor-shell.md §4.1).
pub const ENV_CONFIG_DIR: &str = "VA_CONFIG_DIR";

/// Environment variable that points the GUI to its data folder (m4-editor-shell.md §4.1).
pub const ENV_DATA_DIR: &str = "VA_DATA_DIR";

/// Prefix of a profile folder name under `<target>/bench/`: `profile-<utc>-<pid>`.
pub const PROFILE_PREFIX: &str = "profile-";

/// Polling period while waiting for the child to exit.
const EXIT_POLL: Duration = Duration::from_millis(10);

/// Size of one stderr read.
const STDERR_CHUNK: usize = 8 * 1024;

/// A measured GUI binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltBinary {
    /// The private copy that is measured: `<target>/bench/shipped/visual-assembler[.exe]`.
    pub path: PathBuf,
    /// Its size in bytes.
    pub bytes: u64,
}

/// Returns the cargo arguments that build the shipped GUI: release, default features, so both
/// renderers are compiled in (§19.5).
pub fn build_args() -> Vec<String> {
    vec![
        "build".to_owned(),
        "--release".to_owned(),
        "-p".to_owned(),
        GUI_PACKAGE.to_owned(),
        "--message-format=json-render-diagnostics".to_owned(),
    ]
}

/// Returns the executable path carried by one cargo JSON message, when that message is the
/// `compiler-artifact` of the `visual-assembler` binary target.
pub fn gui_executable(message: &Value) -> Option<PathBuf> {
    if message.get("reason").and_then(Value::as_str) != Some("compiler-artifact") {
        return None;
    }
    let target = message.get("target")?;
    if target.get("name").and_then(Value::as_str) != Some(GUI_PACKAGE) {
        return None;
    }
    let is_bin = target
        .get("kind")
        .and_then(Value::as_array)
        .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("bin")));
    if !is_bin {
        return None;
    }
    message
        .get("executable")
        .and_then(Value::as_str)
        .map(PathBuf::from)
}

/// Returns `<target_dir>/bench/shipped/visual-assembler[.exe]`.
pub fn bench_copy_path(target_dir: &Path) -> PathBuf {
    target_dir
        .join("bench")
        .join(SHIPPED_DIR)
        .join(format!("{GUI_PACKAGE}{}", std::env::consts::EXE_SUFFIX))
}

/// Builds the shipped GUI ([`build_args`]), copies the binary to [`bench_copy_path`] and returns
/// the copy with its size.
///
/// Cargo's rendered diagnostics stream to stderr; its JSON messages are read from stdout.
pub fn build(root: &Path, cargo: &std::ffi::OsStr, target_dir: &Path) -> Result<BuiltBinary> {
    let mut cmd = Command::new(cargo);
    cmd.current_dir(root)
        .args(build_args())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let description = exec::describe(&cmd);
    let mut child = cmd
        .spawn()
        .with_context(|| format!("cannot start `{description}`"))?;
    let stdout = child
        .stdout
        .take()
        .context("cargo stdout was not captured")?;
    let mut executable = None;
    let mut read_error = None;
    for line in BufReader::new(stdout).lines() {
        match line {
            Ok(line) => {
                // Non-JSON lines cannot be the artifact message: skip them.
                if let Ok(message) = serde_json::from_str::<Value>(&line)
                    && let Some(path) = gui_executable(&message)
                {
                    executable = Some(path);
                }
            }
            Err(error) => {
                read_error = Some(error);
                break;
            }
        }
    }
    if let Some(error) = read_error {
        kill_and_reap(&mut child);
        return Err(error).with_context(|| format!("cannot read the output of `{description}`"));
    }
    let status = child
        .wait()
        .with_context(|| format!("cannot wait for `{description}`"))?;
    if !status.success() {
        bail!("`{description}` failed ({status})");
    }
    let built = executable.with_context(|| {
        format!("`{description}` reported no executable for the {GUI_PACKAGE} binary")
    })?;
    let copy = bench_copy_path(target_dir);
    let dir = copy
        .parent()
        .with_context(|| format!("{} has no parent folder", copy.display()))?;
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    std::fs::copy(&built, &copy)
        .with_context(|| format!("cannot copy {} to {}", built.display(), copy.display()))?;
    let bytes = std::fs::metadata(&copy)
        .with_context(|| format!("cannot read the size of {}", copy.display()))?
        .len();
    Ok(BuiltBinary { path: copy, bytes })
}

/// The isolated profile of one bench command: `<target>/bench/profile-<utc>-<pid>/` with its
/// `config` and `data` folders (m4-editor-shell.md §7.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchProfile {
    /// The profile folder.
    pub root: PathBuf,
    /// `<root>/config`, passed as `VA_CONFIG_DIR`.
    pub config_dir: PathBuf,
    /// `<root>/data`, passed as `VA_DATA_DIR`.
    pub data_dir: PathBuf,
}

impl BenchProfile {
    /// Returns the profile folder of a command started at `t` by process `pid`:
    /// `<target_dir>/bench/profile-<YYYYMMDDTHHMMSSZ>-<pid>`.
    pub fn path_for(target_dir: &Path, t: SystemTime, pid: u32) -> PathBuf {
        target_dir.join("bench").join(format!(
            "{PROFILE_PREFIX}{}-{pid}",
            timestamp::utc_compact(t)
        ))
    }

    /// Describes the profile rooted at `root`, without touching the file system.
    pub fn at(root: PathBuf) -> Self {
        Self {
            config_dir: root.join("config"),
            data_dir: root.join("data"),
            root,
        }
    }

    /// Creates the profile of [`path_for`](Self::path_for): `<target_dir>/bench` if missing,
    /// then the profile folder and its `config` and `data` folders with `fs::create_dir`, which
    /// fails when a folder already exists, so a profile is never shared or reused.
    pub fn create(target_dir: &Path, t: SystemTime, pid: u32) -> Result<Self> {
        let profile = Self::at(Self::path_for(target_dir, t, pid));
        let bench_dir = target_dir.join("bench");
        fs::create_dir_all(&bench_dir)
            .with_context(|| format!("cannot create {}", bench_dir.display()))?;
        for dir in [&profile.root, &profile.config_dir, &profile.data_dir] {
            fs::create_dir(dir)
                .with_context(|| format!("cannot create the bench profile {}", dir.display()))?;
        }
        Ok(profile)
    }

    /// Returns the variables every GUI run gets: `VA_CONFIG_DIR` and `VA_DATA_DIR`.
    pub fn env(&self) -> [(&'static str, &Path); 2] {
        [
            (ENV_CONFIG_DIR, self.config_dir.as_path()),
            (ENV_DATA_DIR, self.data_dir.as_path()),
        ]
    }

    /// Sets [`env`](Self::env) on `cmd`; the rest of the environment is inherited unchanged.
    pub fn apply(&self, cmd: &mut Command) {
        for (name, dir) in self.env() {
            cmd.env(name, dir);
        }
    }
}

/// One run as asked for: its arguments come from it and its run line is validated against it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRequest {
    /// The `--bench` scenario.
    pub scenario: Scenario,
    /// The `--renderer` value.
    pub renderer: RendererChoice,
    /// The `--bench-input` path, for the scenarios that open a generated input.
    pub input: Option<PathBuf>,
    /// The `--duration-secs` value, passed to idle runs only.
    pub duration_secs: Option<u64>,
}

impl RunRequest {
    /// Returns the name the run line must report as `input`: the last path component of
    /// `--bench-input` (lossy UTF-8), or `None` without an input.
    pub fn input_name(&self) -> Option<String> {
        let input = self.input.as_deref()?;
        let name = input.file_name().unwrap_or(input.as_os_str());
        Some(name.to_string_lossy().into_owned())
    }
}

/// Returns the run timeout: 75 s for startup, open, scroll and type, `duration + 75` s for idle
/// (§17.8; m4-editor-shell.md §6.8).
pub fn run_timeout(scenario: Scenario, duration_secs: Option<u64>) -> Duration {
    match scenario {
        Scenario::Idle => Duration::from_secs(
            duration_secs
                .unwrap_or(super::DEFAULT_DURATION_SECS)
                .saturating_add(IDLE_TIMEOUT_EXTRA_SECS),
        ),
        Scenario::Startup | Scenario::Open | Scenario::Scroll | Scenario::Type => {
            Duration::from_secs(STARTUP_TIMEOUT_SECS)
        }
    }
}

/// Returns the GUI arguments of one run, in this order (m4-editor-shell.md §7.1):
/// `--renderer <auto|glow|wgpu> --bench <scenario> [--bench-input <path>] [--duration-secs N]`.
/// The duration goes to idle runs only.
pub fn run_args(request: &RunRequest) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![
        "--renderer".into(),
        request.renderer.as_str().into(),
        "--bench".into(),
        request.scenario.as_str().into(),
    ];
    if let Some(input) = &request.input {
        args.push("--bench-input".into());
        args.push(input.into());
    }
    if request.scenario == Scenario::Idle
        && let Some(seconds) = request.duration_secs
    {
        args.push("--duration-secs".into());
        args.push(seconds.to_string().into());
    }
    args
}

/// Returns the command of one measured run: `binary` with [`run_args`] and the profile
/// variables of [`BenchProfile::apply`].
pub fn gui_command(binary: &Path, request: &RunRequest, profile: &BenchProfile) -> Command {
    let mut cmd = Command::new(binary);
    cmd.args(run_args(request));
    profile.apply(&mut cmd);
    cmd
}

/// Parses `line` as JSON and returns it when its `schema` is [`RUN_SCHEMA`].
pub fn parse_run_line(line: &str) -> Option<Value> {
    let value: Value = serde_json::from_str(line.trim()).ok()?;
    (value.get("schema").and_then(Value::as_str) == Some(RUN_SCHEMA)).then_some(value)
}

/// Spells an optional run-line value for a validation message: the value, or `null`.
fn or_null(value: Option<impl ToString>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

/// Checks that a run line answers the run that was asked for (§17.11, §19.5;
/// m4-editor-shell.md §7.2) and returns a description of the first mismatch otherwise:
///
/// - `scenario` must equal the requested scenario;
/// - `renderer_requested` must equal the `--renderer` value passed;
/// - for a forced `glow` or `wgpu`, `fallback_from` must be `null` and `renderer` must equal it;
/// - for `auto`, any `renderer` is valid, and a fallback is accepted (the report counts it);
/// - `input` must equal the requested input's name ([`RunRequest::input_name`]), and be `null`
///   without an input;
/// - `open`, `scroll` and `type` need a non-null `open_ms` and `vsync == false`;
/// - `scroll` and `type` need `event_count` equal to 300 and 500, and as many `event_ms` samples.
pub fn validate_run(run: &BenchRun, request: &RunRequest) -> Result<(), String> {
    let RunRequest {
        scenario, renderer, ..
    } = *request;
    if run.scenario != scenario.as_str() {
        return Err(format!(
            "reported scenario `{}`, expected `{}`",
            run.scenario,
            scenario.as_str()
        ));
    }
    if run.renderer_requested != renderer.as_str() {
        return Err(format!(
            "reported renderer_requested `{}`, expected `{}`",
            run.renderer_requested,
            renderer.as_str()
        ));
    }
    if renderer.is_forced() {
        if let Some(from) = &run.fallback_from {
            return Err(format!(
                "fell back from `{from}` to `{}` although `--renderer {}` forbids a fallback",
                run.renderer,
                renderer.as_str()
            ));
        }
        if run.renderer != renderer.as_str() {
            return Err(format!(
                "reported renderer `{}`, expected `{}` (forced by --renderer)",
                run.renderer,
                renderer.as_str()
            ));
        }
    }
    let expected_input = request.input_name();
    if run.input != expected_input {
        return Err(format!(
            "reported input {}, expected {}",
            or_null(run.input.as_deref().map(|name| format!("`{name}`"))),
            or_null(expected_input.map(|name| format!("`{name}`")))
        ));
    }
    if scenario.measures_open() {
        if run.open_ms.is_none() {
            return Err(format!(
                "reported open_ms null, expected a number for the `{scenario}` scenario"
            ));
        }
        if run.vsync {
            return Err(format!(
                "reported vsync true, expected false for the `{scenario}` scenario"
            ));
        }
    }
    if let Some(expected) = scenario.expected_events() {
        if run.event_count != Some(expected) {
            return Err(format!(
                "reported event_count {}, expected {expected} for the `{scenario}` scenario",
                or_null(run.event_count)
            ));
        }
        let samples = run.event_ms.as_ref().map(Vec::len);
        if samples.and_then(|count| u64::try_from(count).ok()) != Some(expected) {
            return Err(format!(
                "reported {} event_ms samples, expected {expected}",
                or_null(samples)
            ));
        }
    }
    Ok(())
}

/// One line read by [`read_capped_line`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CappedLine {
    /// The line without its `\n`, cut to the cap.
    pub bytes: Vec<u8>,
    /// Whether bytes past the cap were discarded.
    pub truncated: bool,
}

/// Reads one line from `reader`, up to and including `\n` (or to EOF), keeping at most `cap`
/// bytes of it. The rest of an over-long line is read and discarded, so memory stays bounded.
///
/// Returns `None` at EOF when nothing was read. `Interrupted` reads are retried.
pub fn read_capped_line(reader: &mut impl BufRead, cap: usize) -> io::Result<Option<CappedLine>> {
    let mut line = CappedLine {
        bytes: Vec::new(),
        truncated: false,
    };
    let mut read_any = false;
    loop {
        let (used, complete) = {
            let available = match reader.fill_buf() {
                Ok(available) => available,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            if available.is_empty() {
                return Ok(read_any.then_some(line));
            }
            read_any = true;
            let (content, used, complete) = match available.iter().position(|&b| b == b'\n') {
                Some(newline) => (&available[..newline], newline + 1, true),
                None => (available, available.len(), false),
            };
            let room = cap.saturating_sub(line.bytes.len());
            if content.len() > room {
                line.bytes.extend_from_slice(&content[..room]);
                line.truncated = true;
            } else {
                line.bytes.extend_from_slice(content);
            }
            (used, complete)
        };
        reader.consume(used);
        if complete {
            return Ok(Some(line));
        }
    }
}

/// The last bytes of a stream, at most a fixed capacity (the child's stderr).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamTail {
    /// Kept bytes, oldest first.
    kept: VecDeque<u8>,
    /// Maximum number of kept bytes.
    capacity: usize,
    /// Bytes dropped from the front so far.
    dropped: u64,
}

impl StreamTail {
    /// Creates an empty tail keeping at most `capacity` bytes.
    pub fn new(capacity: usize) -> Self {
        Self {
            kept: VecDeque::new(),
            capacity,
            dropped: 0,
        }
    }

    /// Appends `bytes`, dropping the oldest bytes beyond the capacity.
    pub fn push(&mut self, bytes: &[u8]) {
        let skip = bytes.len().saturating_sub(self.capacity);
        self.dropped += skip as u64;
        self.kept.extend(&bytes[skip..]);
        let excess = self.kept.len().saturating_sub(self.capacity);
        self.kept.drain(..excess);
        self.dropped += excess as u64;
    }

    /// Returns the number of kept bytes.
    pub fn len(&self) -> usize {
        self.kept.len()
    }

    /// Tells whether nothing is kept.
    pub fn is_empty(&self) -> bool {
        self.kept.is_empty()
    }

    /// Returns the number of bytes dropped from the front.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Returns the kept bytes as text (lossy UTF-8), preceded by a note when bytes were dropped.
    pub fn text(&self) -> String {
        let bytes: Vec<u8> = self.kept.iter().copied().collect();
        let text = String::from_utf8_lossy(&bytes);
        if self.dropped == 0 {
            text.into_owned()
        } else {
            format!(
                "[… {} earlier bytes dropped, last {} kept …]\n{text}",
                self.dropped,
                self.kept.len()
            )
        }
    }
}

/// What a reader thread reports to the run loop.
enum ReaderEvent {
    /// One stdout line with its receipt time.
    Line(Instant, CappedLine),
    /// stdout reached EOF (`None`) or failed.
    StdoutEnd(Option<io::Error>),
    /// A chunk of stderr.
    Stderr(Vec<u8>),
    /// stderr reached EOF (`None`) or failed.
    StderrEnd(Option<io::Error>),
}

/// How waiting for a child ended.
enum Ending {
    /// The child exited with this status.
    Exited(ExitStatus),
    /// The deadline passed; the child was killed.
    TimedOut,
    /// Polling the child's status failed; the child was killed.
    WaitFailed(io::Error),
}

/// Everything the run loop learned from the readers.
struct Collected {
    /// First run line and its receipt time.
    first_line: Option<(Instant, Value)>,
    /// stdout read error, if any.
    read_error: Option<io::Error>,
    /// Last bytes of stderr.
    stderr: StreamTail,
    /// stderr read error, if any.
    stderr_error: Option<io::Error>,
    /// The stdout reader reported its end.
    stdout_done: bool,
    /// The stderr reader reported its end.
    stderr_done: bool,
}

impl Collected {
    /// Starts with nothing collected.
    fn new() -> Self {
        Self {
            first_line: None,
            read_error: None,
            stderr: StreamTail::new(STDERR_TAIL_BYTES),
            stderr_error: None,
            stdout_done: false,
            stderr_done: false,
        }
    }

    /// Records one reader event. A truncated line never counts as the run line.
    fn apply(&mut self, event: ReaderEvent) {
        match event {
            ReaderEvent::Line(received, line) => {
                if self.first_line.is_none()
                    && !line.truncated
                    && let Some(value) = parse_run_line(&String::from_utf8_lossy(&line.bytes))
                {
                    self.first_line = Some((received, value));
                }
            }
            ReaderEvent::StdoutEnd(error) => {
                self.stdout_done = true;
                self.read_error = error;
            }
            ReaderEvent::Stderr(chunk) => self.stderr.push(&chunk),
            ReaderEvent::StderrEnd(error) => {
                self.stderr_done = true;
                self.stderr_error = error;
            }
        }
    }

    /// Tells whether both readers reported their end.
    fn readers_done(&self) -> bool {
        self.stdout_done && self.stderr_done
    }

    /// Returns the kept stderr, with a note when it could not be read entirely.
    fn stderr_text(&self) -> String {
        let mut text = self.stderr.text();
        if let Some(error) = &self.stderr_error {
            text.push_str(&format!("\n(stderr could not be read: {error})"));
        }
        text
    }
}

/// Runs the measured binary once as `request` asks ([`gui_command`]) and returns its sample.
///
/// Fails when the child cannot start, times out (it is then killed), exits non-zero, keeps its
/// output pipes open after exiting, prints no `va.bench.run/3` line, or prints one that does not
/// answer the request ([`validate_run`]). The error carries the last 64 KiB of the child's stderr.
pub fn run_once(binary: &Path, request: &RunRequest, profile: &BenchProfile) -> Result<Sample> {
    run_command(
        gui_command(binary, request, profile),
        request,
        run_timeout(request.scenario, request.duration_secs),
    )
}

/// Runs `cmd` as the measured run `request`, with the deadline `timeout` (the engine of
/// [`run_once`], which passes [`gui_command`] and [`run_timeout`]).
///
/// stdin is null and stdout/stderr are piped. `spawn_to_first_frame_ms` is the receipt time of
/// the first run line minus the instant just before the spawn, for startup runs; it is `None`
/// for the other scenarios, whose line arrives after the measurement (§17.8).
pub fn run_command(mut cmd: Command, request: &RunRequest, timeout: Duration) -> Result<Sample> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let description = exec::describe(&cmd);

    let t0 = Instant::now();
    let deadline = t0
        .checked_add(timeout)
        .with_context(|| format!("run timeout of {} s is out of range", timeout.as_secs()))?;
    let mut child = cmd
        .spawn()
        .with_context(|| format!("cannot start `{description}`"))?;
    let readers = match start_readers(&mut child) {
        Ok(readers) => readers,
        Err(error) => {
            kill_and_reap(&mut child);
            return Err(error.context(format!("cannot capture the output of `{description}`")));
        }
    };

    let mut collected = Collected::new();
    let Readers { threads, events } = readers;
    let ending = wait_for_exit(&mut child, &events, deadline, &mut collected);
    drain_readers(&events, &mut collected);
    // A reader that ended without reporting it panicked; one still running is blocked on a
    // pipe that a descendant process holds open.
    let pipes_open =
        !collected.readers_done() && threads.iter().any(|thread| !thread.is_finished());
    let mut reader_panicked = false;
    if pipes_open {
        // Detach the readers: joining them would block this command for as long as the
        // descendant lives.
        drop(threads);
    } else {
        for thread in threads {
            reader_panicked |= thread.join().is_err();
        }
        while let Ok(event) = events.try_recv() {
            collected.apply(event);
        }
    }

    let stderr = collected.stderr_text();
    let failure = |what: String| {
        anyhow!(
            "`{description}` {what}\n--- child stderr ---\n{}\n--- end of child stderr ---",
            stderr.trim_end()
        )
    };
    let pipes_note = if pipes_open {
        format!("; {PIPES_OPEN_MESSAGE}")
    } else {
        String::new()
    };
    let status = match ending {
        Ending::TimedOut => {
            return Err(failure(format!(
                "timed out after {} s and was killed{pipes_note}",
                timeout.as_secs()
            )));
        }
        Ending::WaitFailed(error) => {
            return Err(failure(format!(
                "could not be waited for ({error}) and was killed{pipes_note}"
            )));
        }
        Ending::Exited(status) => status,
    };
    if pipes_open {
        return Err(failure(format!("exited ({status}): {PIPES_OPEN_MESSAGE}")));
    }
    if reader_panicked {
        bail!("an output reader of `{description}` panicked");
    }
    if let Some(error) = &collected.read_error {
        return Err(failure(format!("stdout could not be read: {error}")));
    }
    if !status.success() {
        return Err(failure(format!("exited with {status}")));
    }
    let Some((received, value)) = collected.first_line else {
        return Err(failure(format!("printed no {RUN_SCHEMA} line on stdout")));
    };
    let run: BenchRun = serde_json::from_value(value)
        .map_err(|error| failure(format!("printed an invalid {RUN_SCHEMA} line: {error}")))?;
    validate_run(&run, request).map_err(failure)?;
    let spawn_to_first_frame_ms = request
        .scenario
        .reports_at_first_frame()
        .then(|| received.duration_since(t0).as_secs_f64() * 1000.0);
    Ok(Sample {
        spawn_to_first_frame_ms,
        run,
    })
}

/// The threads that drain a child's stdout and stderr, and their event channel.
struct Readers {
    /// The stdout and stderr reader threads.
    threads: [JoinHandle<()>; 2],
    /// Receives every reader event.
    events: Receiver<ReaderEvent>,
}

/// Starts the stdout line forwarder and the stderr forwarder of `child`.
fn start_readers(child: &mut Child) -> Result<Readers> {
    let stdout = child.stdout.take().context("stdout was not captured")?;
    let stderr = child.stderr.take().context("stderr was not captured")?;
    let (sender, events) = mpsc::channel();
    let stderr_sender = sender.clone();
    let stdout_thread = thread::Builder::new()
        .name("va-bench-stdout".to_owned())
        .spawn(move || forward_stdout(stdout, &sender))
        .context("cannot start the stdout reader thread")?;
    let stderr_thread = thread::Builder::new()
        .name("va-bench-stderr".to_owned())
        .spawn(move || forward_stderr(stderr, &stderr_sender))
        .context("cannot start the stderr reader thread")?;
    Ok(Readers {
        threads: [stdout_thread, stderr_thread],
        events,
    })
}

/// Sends each stdout line (capped at [`MAX_LINE_BYTES`]) with its receipt time, then the end.
fn forward_stdout(stdout: ChildStdout, events: &Sender<ReaderEvent>) {
    let mut reader = BufReader::new(stdout);
    let end = loop {
        match read_capped_line(&mut reader, MAX_LINE_BYTES) {
            Ok(Some(line)) => {
                let received = Instant::now();
                // A send error means the run loop gave up on this run (timeout or detached
                // readers, reported there); keep draining so the writer never blocks.
                let _ = events.send(ReaderEvent::Line(received, line));
            }
            Ok(None) => break None,
            Err(error) => break Some(error),
        }
    };
    // Same as above: if the run loop is gone, the run is already being reported.
    let _ = events.send(ReaderEvent::StdoutEnd(end));
}

/// Sends stderr in chunks, then the end.
fn forward_stderr(mut stderr: ChildStderr, events: &Sender<ReaderEvent>) {
    let mut buffer = vec![0_u8; STDERR_CHUNK];
    let end = loop {
        match stderr.read(&mut buffer) {
            Ok(0) => break None,
            Ok(count) => {
                // See `forward_stdout`: a gone run loop already reports the run.
                let _ = events.send(ReaderEvent::Stderr(buffer[..count].to_vec()));
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => break Some(error),
        }
    };
    // See `forward_stdout`.
    let _ = events.send(ReaderEvent::StderrEnd(end));
}

/// Waits until the child exits or `deadline` passes (the child is then killed), recording the
/// reader events that arrive meanwhile.
fn wait_for_exit(
    child: &mut Child,
    events: &Receiver<ReaderEvent>,
    deadline: Instant,
    collected: &mut Collected,
) -> Ending {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ending::Exited(status),
            Ok(None) => {}
            Err(error) => {
                kill_and_reap(child);
                return Ending::WaitFailed(error);
            }
        }
        let Some(remaining) = deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
        else {
            kill_and_reap(child);
            return Ending::TimedOut;
        };
        let wait = remaining.min(EXIT_POLL);
        match events.recv_timeout(wait) {
            Ok(event) => collected.apply(event),
            Err(RecvTimeoutError::Timeout) => {}
            // Both readers are gone: only the exit is left to wait for.
            Err(RecvTimeoutError::Disconnected) => thread::sleep(wait),
        }
    }
}

/// Records reader events until both readers report their end, for at most
/// [`READER_JOIN_TIMEOUT`] (§17.8).
fn drain_readers(events: &Receiver<ReaderEvent>, collected: &mut Collected) {
    let limit = Instant::now() + READER_JOIN_TIMEOUT;
    while !collected.readers_done() {
        let Some(remaining) = limit.checked_duration_since(Instant::now()) else {
            return;
        };
        match events.recv_timeout(remaining) {
            Ok(event) => collected.apply(event),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return,
        }
    }
}

/// Kills `child` and waits for it. Failures are printed on stderr: the caller is already
/// reporting a failure and has nothing better to do with them.
fn kill_and_reap(child: &mut Child) {
    if let Err(error) = child.kill() {
        eprintln!("warning: cannot kill child process {}: {error}", child.id());
    }
    if let Err(error) = child.wait() {
        eprintln!(
            "warning: cannot wait for child process {}: {error}",
            child.id()
        );
    }
}
