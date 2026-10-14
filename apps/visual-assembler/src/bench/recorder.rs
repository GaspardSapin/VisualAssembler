//! The bench measurement algorithm (docs/specs/m0-foundations.md §4.5), hooked into the app.
//!
//! Definitions:
//! - the first frame presented is the start of the first `App::logic` call where
//!   `ctx.cumulative_frame_nr() >= 1`: eframe paints frame 1, presents it and makes the window
//!   visible before it runs frame 2;
//! - `frames` holds `cumulative_frame_nr() + 1` as of the last `App::ui` call, i.e. the number of
//!   frames started so far.
//!
//! Steps:
//! 1. [`BenchRecorder::new`] starts a watchdog thread that exits the process with code 4
//!    ([`crate::EXIT_BENCH_TIMEOUT`]) when no measurement is reported within the hard limit
//!    ([`watchdog_limit`]);
//! 2. [`BenchRecorder::on_ui`] updates `frames` and, on its first call, captures the
//!    [`RendererInfo`] and requests one repaint so that frame 2 exists;
//! 3. [`BenchRecorder::on_logic`] records the first presented frame. The startup scenario then
//!    reports at once; the idle scenario hands over to the `va-bench-idle` thread, which waits
//!    [`IDLE_SETTLE_MS`], samples, sleeps for the idle window, samples again and reports. The
//!    UI thread is never woken during the idle window.
//!
//! Reporting prints the JSON line on stdout, flushes, tells the watchdog, and asks the root
//! viewport to close. If the bench cannot start a thread or write its line, it prints
//! `error: …` on stderr and exits the process with code 1 ([`crate::EXIT_RUNTIME`], §17.10).

use std::fmt::Display;
use std::io::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{self, ViewportCommand, ViewportId};

use super::platform::{self, RendererInfo};
use super::process::ProcessStats;
use super::report::{BenchRun, RUN_SCHEMA};
use super::{IDLE_SETTLE_MS, IDLE_WATCHDOG_EXTRA_SECS, STARTUP_WATCHDOG_SECS};
use crate::cli::{BenchPlan, BenchScenario};

/// Name of the watchdog thread.
const WATCHDOG_THREAD: &str = "va-bench-watchdog";

/// Name of the idle measurement thread (normative).
const IDLE_THREAD: &str = "va-bench-idle";

/// Returns the watchdog's hard limit for `plan`, counted from the creation of the recorder.
///
/// - startup: [`STARTUP_WATCHDOG_SECS`] (60 s);
/// - idle: [`IDLE_WATCHDOG_EXTRA_SECS`] + idle duration + [`IDLE_SETTLE_MS`], i.e. 60 + N + 3 s
///   (docs/specs/m0-foundations.md §17.13). The settle delay is part of the limit because the
///   idle thread sleeps it before the window starts.
///
/// The arithmetic saturates, so an out-of-range duration yields a very long limit instead of a
/// panic (the command line caps N at 3600 s anyway).
pub fn watchdog_limit(plan: &BenchPlan) -> Duration {
    match plan.scenario {
        BenchScenario::Startup => Duration::from_secs(STARTUP_WATCHDOG_SECS),
        BenchScenario::Idle => Duration::from_secs(IDLE_WATCHDOG_EXTRA_SECS)
            .saturating_add(Duration::from_millis(IDLE_SETTLE_MS))
            .saturating_add(plan.idle_duration),
    }
}

/// Runs one bench scenario inside the app. Created only when `--bench` is given.
pub struct BenchRecorder {
    /// Scenario and idle window length.
    plan: BenchPlan,
    /// Instant taken by the first statement of `main`.
    started: Instant,
    /// Frames started so far (`cumulative_frame_nr() + 1` at the last `on_ui`).
    frames: Arc<AtomicU64>,
    /// Captured on the first `on_ui` call.
    renderer: Option<RendererInfo>,
    /// Set by the first `on_logic` call that sees a presented frame.
    first_frame: Option<Instant>,
    /// Tells the watchdog that the measurement was reported.
    reported: mpsc::Sender<()>,
}

impl BenchRecorder {
    /// Creates the recorder and starts the watchdog thread.
    ///
    /// The watchdog limit is [`watchdog_limit`]: 60 s for the startup scenario, and
    /// 60 s + idle duration + 3 s of settle for the idle scenario. When it fires before the
    /// measurement is reported, it prints `error: bench timed out` on stderr and exits the
    /// process with code 4. It stops silently once the result is reported, or when every handle
    /// to it is dropped (the app shut down before measuring: the window was closed).
    ///
    /// # Errors
    /// Returns the I/O error of the operating system if the watchdog thread cannot be created.
    pub fn new(plan: BenchPlan, started: Instant) -> std::io::Result<Self> {
        let limit = watchdog_limit(&plan);
        let (reported, watchdog) = mpsc::channel::<()>();
        thread::Builder::new()
            .name(WATCHDOG_THREAD.to_owned())
            .spawn(move || match watchdog.recv_timeout(limit) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => {}
                Err(RecvTimeoutError::Timeout) => {
                    crate::report_error("bench timed out");
                    std::process::exit(i32::from(crate::EXIT_BENCH_TIMEOUT));
                }
            })?;

        Ok(Self {
            plan,
            started,
            frames: Arc::new(AtomicU64::new(0)),
            renderer: None,
            first_frame: None,
            reported,
        })
    }

    /// Called from `App::ui` on every frame, after the UI is drawn.
    ///
    /// Stores `cumulative_frame_nr() + 1` in the frame counter. On the first call, captures the
    /// renderer information and requests one repaint so that frame 2 exists for the measurement.
    pub fn on_ui(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        self.frames
            .store(ctx.cumulative_frame_nr() + 1, Ordering::SeqCst);
        if self.renderer.is_none() {
            self.renderer = Some(RendererInfo::from_frame(frame));
            ctx.request_repaint();
        }
    }

    /// Called from `App::logic` before every frame (and while the window is hidden).
    ///
    /// On the first call that sees a presented frame (`cumulative_frame_nr() >= 1`), records the
    /// first-frame instant and runs the scenario: the startup scenario reports immediately, the
    /// idle scenario starts the `va-bench-idle` thread.
    pub fn on_logic(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        if self.first_frame.is_some() || ctx.cumulative_frame_nr() < 1 {
            return;
        }
        let first_frame = Instant::now();
        self.first_frame = Some(first_frame);
        let startup_ms = duration_ms(first_frame.saturating_duration_since(self.started));
        // `on_ui` always runs before a presented frame exists; this fallback only guards the
        // invariant.
        let renderer = match &self.renderer {
            Some(renderer) => renderer.clone(),
            None => RendererInfo::from_frame(frame),
        };

        match self.plan.scenario {
            BenchScenario::Startup => {
                let stats = ProcessStats::sample();
                let run = bench_run(
                    BenchScenario::Startup,
                    &renderer,
                    startup_ms,
                    self.frames.load(Ordering::SeqCst),
                    stats.cpu_ms,
                    &stats,
                    None,
                );
                report(&run, &self.reported, ctx);
            }
            BenchScenario::Idle => {
                let idle = IdleMeasurement {
                    ctx: ctx.clone(),
                    frames: Arc::clone(&self.frames),
                    startup_ms,
                    renderer,
                    duration: self.plan.idle_duration,
                    reported: self.reported.clone(),
                };
                let spawned = thread::Builder::new()
                    .name(IDLE_THREAD.to_owned())
                    .spawn(move || idle.run());
                if let Err(err) = spawned {
                    fail(format_args!(
                        "cannot start the idle measurement thread: {err}"
                    ));
                }
            }
        }
    }
}

/// State moved to the `va-bench-idle` thread.
struct IdleMeasurement {
    /// Used only to close the window at the end (it wakes the UI thread once).
    ctx: egui::Context,
    /// Shared frame counter, updated by `on_ui`.
    frames: Arc<AtomicU64>,
    /// Startup time measured by `on_logic`.
    startup_ms: f64,
    /// Renderer captured by the first `on_ui`.
    renderer: RendererInfo,
    /// Length of the idle window.
    duration: Duration,
    /// Tells the watchdog that the measurement was reported.
    reported: mpsc::Sender<()>,
}

impl IdleMeasurement {
    /// Settles, measures the idle window between two samples, and reports.
    fn run(self) {
        thread::sleep(Duration::from_millis(IDLE_SETTLE_MS));
        let (start, start_stats, start_frames) = sample(&self.frames);
        thread::sleep(self.duration);
        let (end, end_stats, end_frames) = sample(&self.frames);

        let cpu_ms = match (start_stats.cpu_ms, end_stats.cpu_ms) {
            (Some(start_cpu), Some(end_cpu)) => Some(end_cpu - start_cpu),
            _ => None,
        };
        let idle_window_ms = duration_ms(end.saturating_duration_since(start));
        let run = bench_run(
            BenchScenario::Idle,
            &self.renderer,
            self.startup_ms,
            end_frames.saturating_sub(start_frames),
            cpu_ms,
            &end_stats,
            Some(idle_window_ms),
        );
        report(&run, &self.reported, &self.ctx);
    }
}

/// Takes one idle sample: (instant, process statistics, frames started).
fn sample(frames: &AtomicU64) -> (Instant, ProcessStats, u64) {
    let now = Instant::now();
    let stats = ProcessStats::sample();
    let frames = frames.load(Ordering::SeqCst);
    (now, stats, frames)
}

/// Builds the JSON line content. `memory` provides the lifetime peaks; `idle_window_ms` is
/// `Some` exactly for the idle scenario, which also sets `settle_ms`.
fn bench_run(
    scenario: BenchScenario,
    renderer: &RendererInfo,
    startup_ms: f64,
    frames_painted: u64,
    process_cpu_ms: Option<f64>,
    memory: &ProcessStats,
    idle_window_ms: Option<f64>,
) -> BenchRun {
    BenchRun {
        schema: RUN_SCHEMA,
        scenario: scenario.as_str(),
        renderer: renderer.renderer,
        backend: renderer.backend,
        adapter: renderer.adapter.clone(),
        driver: renderer.driver.clone(),
        window_system: renderer.window_system,
        wsl: platform::is_wsl(),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        version: env!("CARGO_PKG_VERSION"),
        startup_ms,
        frames_painted,
        process_cpu_ms,
        peak_rss_bytes: memory.peak_rss_bytes,
        peak_private_bytes: memory.peak_private_bytes,
        settle_ms: idle_window_ms.map(|_| IDLE_SETTLE_MS),
        idle_window_ms,
    }
}

/// Prints the line on stdout, flushes, tells the watchdog, and closes the root viewport.
fn report(run: &BenchRun, reported: &mpsc::Sender<()>, ctx: &egui::Context) {
    let line = run.to_json_line();
    let mut stdout = std::io::stdout().lock();
    if let Err(err) = writeln!(stdout, "{line}").and_then(|()| stdout.flush()) {
        fail(format_args!(
            "cannot write the bench result to stdout: {err}"
        ));
    }
    // A send error only means the watchdog already stopped (it never stops before a report
    // unless the recorder was dropped), so there is nothing left to tell it.
    let _watchdog_gone = reported.send(());
    ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Close);
}

/// Reports a bench infrastructure failure on stderr and exits with [`crate::EXIT_RUNTIME`].
fn fail(message: impl Display) -> ! {
    crate::report_error(message);
    std::process::exit(i32::from(crate::EXIT_RUNTIME));
}

/// Converts a duration to fractional milliseconds.
fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
