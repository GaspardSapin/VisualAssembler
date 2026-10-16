//! `cargo xtask bench`: build the shipped GUI binary once, measure it per renderer choice, write
//! a report (§6.1, amended by §17.8 and §19.5).
//!
//! The measured binary is the shipped one: default features, so both renderers are compiled in
//! (m0-foundations.md §19.1). Each run passes `--renderer auto|glow|wgpu` to it; `--renderer all`
//! measures glow, then wgpu, then auto. Progress goes to stderr. stdout carries only the markdown
//! table and `report: <path>`, or the report JSON with `--json`. Any failed build or run aborts the
//! command (exit code 1) and no report is written. The parent environment passes through unchanged
//! to the GUI, so `WGPU_BACKEND=gl`, `VA_WAYLAND=1`, `VA_TEST_FAIL_RENDERER=glow` or
//! `env -u WAYLAND_DISPLAY` select backends, window systems and the fallback path; the report
//! records those variables.

pub mod report;
pub mod runner;
pub mod stats;
pub mod timestamp;

use std::{
    fmt,
    io::Write as _,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use anyhow::{Context as _, Result};
use serde::Serialize;

use crate::{exec, locate};
use report::{BenchResult, BenchRun, EnvSnapshot, GitInfo, HostInfo, Report, ResultConfig};

/// Measured runs of the startup scenario when `--runs` is absent.
pub const DEFAULT_STARTUP_RUNS: u32 = 20;

/// Measured runs of the idle scenario when `--runs` is absent.
pub const DEFAULT_IDLE_RUNS: u32 = 3;

/// Discarded warm-up runs when `--warmup` is absent.
pub const DEFAULT_WARMUP: u32 = 1;

/// Idle window length when `--duration-secs` is absent (same default as the GUI).
pub const DEFAULT_DURATION_SECS: u64 = 10;

/// Returns the reports folder `<root>/bench/results` (gitignored), with native separators.
pub fn results_dir(root: &Path) -> PathBuf {
    root.join("bench").join("results")
}

/// `git.sha` when `git rev-parse` fails.
pub const NO_GIT_SHA: &str = "nogit";

/// A measurement scenario of the GUI (`--bench` value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scenario {
    /// Process start to first frame presented.
    Startup,
    /// CPU and frames during an idle window after startup.
    Idle,
}

impl Scenario {
    /// Returns the command-line and JSON spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Idle => "idle",
        }
    }
}

impl fmt::Display for Scenario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A `--renderer` value passed to the shipped GUI, recorded as `renderer_requested` in its run
/// line (m0-foundations.md §19.1, §19.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RendererChoice {
    /// The shipped default: the target's order (glow then wgpu, or wgpu then glow on Windows
    /// aarch64) with an in-process fallback when the first renderer fails to initialize.
    Auto,
    /// glow (OpenGL through WGL, EGL or GLX), forced: no fallback.
    Glow,
    /// wgpu (DX12, Vulkan, GL, Metal), forced: no fallback.
    Wgpu,
}

impl RendererChoice {
    /// Returns the command-line and JSON spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Glow => "glow",
            Self::Wgpu => "wgpu",
        }
    }

    /// Tells whether this choice forces one renderer, so the GUI must neither fall back nor run
    /// another renderer (§19.1, §19.5).
    pub fn is_forced(self) -> bool {
        self != Self::Auto
    }
}

impl fmt::Display for RendererChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Scenario selection on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ScenarioSelection {
    /// Startup only.
    Startup,
    /// Idle only.
    Idle,
    /// Startup, then idle.
    All,
}

/// Renderer selection on the command line (§19.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum RendererSelection {
    /// The shipped default: glow first, wgpu as fallback (wgpu first on Windows aarch64).
    Auto,
    /// glow only, forced.
    Glow,
    /// wgpu only, forced.
    Wgpu,
    /// glow, then wgpu, then auto.
    All,
}

/// Arguments of `cargo xtask bench`.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
pub struct BenchArgs {
    /// Scenario to measure.
    #[arg(value_enum, default_value_t = ScenarioSelection::All)]
    pub scenario: ScenarioSelection,
    /// Renderer choice passed to the shipped GUI (`all` = glow, then wgpu, then auto).
    #[arg(long, value_enum, default_value_t = RendererSelection::Auto)]
    pub renderer: RendererSelection,
    /// Measured runs per scenario [default: 20 for startup, 3 for idle].
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub runs: Option<u32>,
    /// Discarded warm-up runs per scenario; they must succeed too.
    #[arg(long, default_value_t = DEFAULT_WARMUP)]
    pub warmup: u32,
    /// Idle window length in seconds (idle scenario only).
    #[arg(long, default_value_t = DEFAULT_DURATION_SECS,
          value_parser = clap::value_parser!(u64).range(1..=3600))]
    pub duration_secs: u64,
    /// Print the report JSON on stdout instead of the table and the report path.
    #[arg(long)]
    pub json: bool,
}

impl BenchArgs {
    /// Returns the selected scenarios in run order (startup, then idle).
    pub fn scenarios(&self) -> Vec<Scenario> {
        match self.scenario {
            ScenarioSelection::Startup => vec![Scenario::Startup],
            ScenarioSelection::Idle => vec![Scenario::Idle],
            ScenarioSelection::All => vec![Scenario::Startup, Scenario::Idle],
        }
    }

    /// Returns the selected renderer choices in run order (`all`: glow, then wgpu, then auto).
    pub fn renderers(&self) -> Vec<RendererChoice> {
        match self.renderer {
            RendererSelection::Auto => vec![RendererChoice::Auto],
            RendererSelection::Glow => vec![RendererChoice::Glow],
            RendererSelection::Wgpu => vec![RendererChoice::Wgpu],
            RendererSelection::All => vec![
                RendererChoice::Glow,
                RendererChoice::Wgpu,
                RendererChoice::Auto,
            ],
        }
    }

    /// Returns the measured run count of `scenario` (`--runs` overrides both defaults).
    pub fn runs_for(&self, scenario: Scenario) -> u32 {
        self.runs.unwrap_or(match scenario {
            Scenario::Startup => DEFAULT_STARTUP_RUNS,
            Scenario::Idle => DEFAULT_IDLE_RUNS,
        })
    }

    /// Returns the `--duration-secs` value passed to the GUI: `Some` for idle, `None` otherwise.
    pub fn duration_for(&self, scenario: Scenario) -> Option<u64> {
        (scenario == Scenario::Idle).then_some(self.duration_secs)
    }
}

/// Runs the whole bench from `root` and prints its outcome on stdout.
///
/// Builds the shipped GUI once, then, for each renderer choice and each scenario, runs the
/// warm-up runs (discarded, but they must succeed) and the measured runs.
pub fn run(root: &Path, args: &BenchArgs) -> Result<()> {
    let cargo = locate::cargo();
    let target_dir = locate::target_directory(root)?;
    let git = git_info(root);
    let host = detect_host();
    let env = EnvSnapshot::capture();

    eprintln!(
        "==> build {} (shipped: default features, release)",
        runner::GUI_PACKAGE
    );
    let binary = runner::build(root, &cargo, &target_dir)?;
    eprintln!(
        "==> measuring {} ({:.1} MiB)",
        binary.path.display(),
        binary.bytes as f64 / 1_048_576.0
    );
    let mut results = Vec::new();
    for renderer in args.renderers() {
        for scenario in args.scenarios() {
            let duration_secs = args.duration_for(scenario);
            let runs = args.runs_for(scenario);
            for index in 1..=args.warmup {
                let sample = runner::run_once(&binary.path, scenario, renderer, duration_secs)
                    .with_context(|| {
                        format!("{scenario}/{renderer} warm-up run {index}/{}", args.warmup)
                    })?;
                eprintln!(
                    "    {scenario}/{renderer} warm-up {index}/{}: spawn→frame {}, {} (discarded)",
                    args.warmup,
                    milliseconds(sample.spawn_to_first_frame_ms),
                    used_renderer(&sample.run)
                );
            }
            let mut samples = Vec::new();
            for index in 1..=runs {
                let sample = runner::run_once(&binary.path, scenario, renderer, duration_secs)
                    .with_context(|| format!("{scenario}/{renderer} run {index}/{runs}"))?;
                eprintln!(
                    "    {scenario}/{renderer} run {index}/{runs}: spawn→frame {}, frames {}, {}",
                    milliseconds(sample.spawn_to_first_frame_ms),
                    sample.run.frames_painted,
                    used_renderer(&sample.run)
                );
                samples.push(sample);
            }
            let config = ResultConfig {
                scenario,
                renderer_requested: renderer,
                binary_bytes: binary.bytes,
                warmup: args.warmup,
                runs,
                duration_secs,
                env: env.clone(),
            };
            results.push(BenchResult::new(config, samples));
        }
    }

    let created = SystemTime::now();
    let report = Report::new(created, git, host, results);
    let path = report::write_report(&results_dir(root), created, &report)?;
    let mut stdout = std::io::stdout().lock();
    if args.json {
        writeln!(stdout, "{}", report.to_json()?)
    } else {
        write!(stdout, "{}", report.markdown_table())
            .and_then(|()| writeln!(stdout, "report: {}", path.display()))
    }
    .context("cannot write the bench outcome on stdout")?;
    Ok(())
}

/// Formats an optional duration for the progress lines: `12.3 ms`, or `n/a` (idle runs).
pub fn milliseconds(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_owned(), |ms| format!("{ms:.1} ms"))
}

/// Describes the renderer a run used, for the progress lines: `renderer glow`, or
/// `renderer wgpu (fallback from glow)` (§19.5: a fallback under `auto` is counted, not rejected).
pub fn used_renderer(run: &BenchRun) -> String {
    match &run.fallback_from {
        Some(from) => format!("renderer {} (fallback from {from})", run.renderer),
        None => format!("renderer {}", run.renderer),
    }
}

/// Reads the git state of `root`: short sha (`nogit` on failure) and dirtiness of tracked files.
///
/// Failures are not fatal (the bench can run from an archive) but are reported on stderr.
pub fn git_info(root: &Path) -> GitInfo {
    let mut rev_parse = Command::new("git");
    rev_parse
        .current_dir(root)
        .args(["rev-parse", "--short=8", "HEAD"]);
    let sha = match exec::stdout_of(&mut rev_parse) {
        Ok(stdout) if !stdout.trim().is_empty() => stdout.trim().to_owned(),
        Ok(_) => {
            eprintln!("warning: `git rev-parse` printed no sha; recording `{NO_GIT_SHA}`");
            NO_GIT_SHA.to_owned()
        }
        Err(error) => {
            eprintln!("warning: {error:#}; recording sha `{NO_GIT_SHA}`");
            NO_GIT_SHA.to_owned()
        }
    };
    let mut status = Command::new("git");
    status
        .current_dir(root)
        .args(["status", "--porcelain", "--untracked-files=no"]);
    let dirty = match exec::stdout_of(&mut status) {
        Ok(stdout) => !stdout.trim().is_empty(),
        Err(error) => {
            eprintln!("warning: {error:#}; recording dirty = false");
            false
        }
    };
    GitInfo { sha, dirty }
}

/// Describes the host xtask runs on.
pub fn detect_host() -> HostInfo {
    let logical_cpus = match std::thread::available_parallelism() {
        Ok(count) => Some(count.get()),
        Err(error) => {
            eprintln!("warning: cannot count logical CPUs ({error}); recording null");
            None
        }
    };
    HostInfo {
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        wsl: is_wsl(),
        logical_cpus,
    }
}

/// Tells whether a `/proc/sys/kernel/osrelease` text denotes WSL: it contains `microsoft` or
/// `WSL`, ignoring case.
pub fn osrelease_is_wsl(osrelease: &str) -> bool {
    let lower = osrelease.to_ascii_lowercase();
    lower.contains("microsoft") || lower.contains("wsl")
}

/// Tells whether this process runs under WSL (same rule as the GUI's `platform::is_wsl`).
///
/// Always `false` outside Linux, and `false` when the kernel release cannot be read.
pub fn is_wsl() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    std::fs::read_to_string("/proc/sys/kernel/osrelease").is_ok_and(|text| osrelease_is_wsl(&text))
}
