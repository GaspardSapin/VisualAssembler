//! Bench build and spawn: one release GUI binary per renderer, and one measured child process
//! per run (§6.1 steps 1 and 2, amended by §17.8).
//!
//! A run never waits unboundedly: the child gets a deadline that is strictly larger than the
//! GUI's own watchdog, and once it has exited or been killed the output readers get at most
//! [`READER_JOIN_TIMEOUT`] to finish. Readers still blocked after that (a descendant process
//! holds the pipes) are detached and the run fails. Kept output is bounded: the last
//! [`STDERR_TAIL_BYTES`] of stderr, and stdout lines of at most [`MAX_LINE_BYTES`].

use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader, Read as _},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result, anyhow, bail};
use serde_json::Value;

use super::{
    Renderer, Scenario,
    report::{BenchRun, RUN_SCHEMA, Sample},
};
use crate::exec;

/// Package and binary name of the GUI.
pub const GUI_PACKAGE: &str = "visual-assembler";

/// Run timeout of the startup scenario, in seconds. Strictly larger than the GUI's 60 s startup
/// watchdog, so the GUI's own `bench timed out` (exit 4) surfaces under xtask (§17.8).
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

/// Polling period while waiting for the child to exit.
const EXIT_POLL: Duration = Duration::from_millis(10);

/// Size of one stderr read.
const STDERR_CHUNK: usize = 8 * 1024;

/// A measured GUI binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltBinary {
    /// The private copy that is measured: `<target>/bench/<renderer>/visual-assembler[.exe]`.
    pub path: PathBuf,
    /// Its size in bytes.
    pub bytes: u64,
}

/// Returns the cargo arguments that build the GUI with `renderer` only.
pub fn build_args(renderer: Renderer) -> Vec<String> {
    vec![
        "build".to_owned(),
        "--release".to_owned(),
        "-p".to_owned(),
        GUI_PACKAGE.to_owned(),
        "--no-default-features".to_owned(),
        "--features".to_owned(),
        format!("renderer-{}", renderer.as_str()),
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

/// Returns `<target_dir>/bench/<renderer>/visual-assembler[.exe]`.
pub fn bench_copy_path(target_dir: &Path, renderer: Renderer) -> PathBuf {
    target_dir
        .join("bench")
        .join(renderer.as_str())
        .join(format!("{GUI_PACKAGE}{}", std::env::consts::EXE_SUFFIX))
}

/// Builds the GUI with `renderer` only, copies the binary to [`bench_copy_path`] and returns it.
///
/// Cargo's rendered diagnostics stream to stderr; its JSON messages are read from stdout.
pub fn build(
    root: &Path,
    cargo: &std::ffi::OsStr,
    renderer: Renderer,
    target_dir: &Path,
) -> Result<BuiltBinary> {
    let mut cmd = Command::new(cargo);
    cmd.current_dir(root)
        .args(build_args(renderer))
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
    let copy = bench_copy_path(target_dir, renderer);
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

/// Returns the run timeout: 75 s for startup, `duration + 75` s for idle (§17.8).
pub fn run_timeout(scenario: Scenario, duration_secs: Option<u64>) -> Duration {
    match scenario {
        Scenario::Startup => Duration::from_secs(STARTUP_TIMEOUT_SECS),
        Scenario::Idle => Duration::from_secs(
            duration_secs
                .unwrap_or(super::DEFAULT_DURATION_SECS)
                .saturating_add(IDLE_TIMEOUT_EXTRA_SECS),
        ),
    }
}

/// Returns the GUI arguments of one run: `--bench <scenario> [--duration-secs N]`.
pub fn run_args(scenario: Scenario, duration_secs: Option<u64>) -> Vec<String> {
    let mut args = vec!["--bench".to_owned(), scenario.as_str().to_owned()];
    if scenario == Scenario::Idle
        && let Some(seconds) = duration_secs
    {
        args.push("--duration-secs".to_owned());
        args.push(seconds.to_string());
    }
    args
}

/// Parses `line` as JSON and returns it when its `schema` is [`RUN_SCHEMA`].
pub fn parse_run_line(line: &str) -> Option<Value> {
    let value: Value = serde_json::from_str(line.trim()).ok()?;
    (value.get("schema").and_then(Value::as_str) == Some(RUN_SCHEMA)).then_some(value)
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

/// Runs the measured binary once and returns its sample.
///
/// Fails when the child cannot start, times out (it is then killed), exits non-zero, keeps its
/// output pipes open after exiting, prints no `va.bench.run/1` line, or prints one for another
/// scenario or renderer. The error carries the last 64 KiB of the child's stderr.
pub fn run_once(
    binary: &Path,
    scenario: Scenario,
    renderer: Renderer,
    duration_secs: Option<u64>,
) -> Result<Sample> {
    let mut cmd = Command::new(binary);
    cmd.args(run_args(scenario, duration_secs));
    run_command(
        cmd,
        scenario,
        renderer,
        run_timeout(scenario, duration_secs),
    )
}

/// Runs `cmd` as one measured run of `scenario`/`renderer` with the deadline `timeout` (the
/// engine of [`run_once`], which passes [`run_timeout`]).
///
/// stdin is null and stdout/stderr are piped. `spawn_to_first_frame_ms` is the receipt time of
/// the first run line minus the instant just before the spawn, for startup runs; it is `None`
/// for idle runs (§17.8).
pub fn run_command(
    mut cmd: Command,
    scenario: Scenario,
    renderer: Renderer,
    timeout: Duration,
) -> Result<Sample> {
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
    if run.scenario != scenario.as_str() || run.renderer != renderer.as_str() {
        return Err(failure(format!(
            "reported scenario `{}` and renderer `{}`, expected `{}` and `{}`",
            run.scenario,
            run.renderer,
            scenario.as_str(),
            renderer.as_str()
        )));
    }
    let spawn_to_first_frame_ms = match scenario {
        Scenario::Startup => Some(received.duration_since(t0).as_secs_f64() * 1000.0),
        Scenario::Idle => None,
    };
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
