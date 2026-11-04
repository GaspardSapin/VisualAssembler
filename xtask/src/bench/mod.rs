//! `cargo xtask bench`: build the shipped GUI binary once, measure it per renderer choice, write
//! a report (m0-foundations.md §6.1, amended by §17.8 and §19.5; m4-editor-shell.md §7).
//!
//! The measured binary is the shipped one: default features, so both renderers are compiled in
//! (m0-foundations.md §19.1). Each run passes `--renderer auto|glow|wgpu` to it; `--renderer all`
//! measures glow, then wgpu, then auto. The scenarios are `startup`, `idle` (with the generated
//! 5,000-line project), `open`, `scroll` and `type` (with the generated 100,000-line file); `all`
//! runs them in that order. The inputs are generated under `bench/generated/` when missing or out
//! of date ([`generate`]), and only when a selected scenario opens one. Every GUI run gets
//! `VA_CONFIG_DIR` and `VA_DATA_DIR` pointing to a fresh profile folder under
//! `<target>/bench/`, so the user's settings never affect a report.
//!
//! Progress goes to stderr. stdout carries only the markdown table and `report: <path>`, or the
//! report JSON with `--json`. Any failed generation, build or run aborts the command (exit code 1)
//! and no report is written. Apart from the two profile variables, the parent environment passes
//! through unchanged to the GUI, so `WGPU_BACKEND=gl`, `VA_WAYLAND=1`,
//! `VA_TEST_FAIL_RENDERER=glow` or `env -u WAYLAND_DISPLAY` select backends, window systems and
//! the fallback path; the report records those variables.

pub mod generate;
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
use generate::{BenchInputs, InputsStatus};
use report::{BenchResult, BenchRun, EnvSnapshot, GitInfo, HostInfo, Report, ResultConfig, Sample};
use runner::{BenchProfile, RunRequest};

/// Measured runs of the startup scenario when `--runs` is absent.
pub const DEFAULT_STARTUP_RUNS: u32 = 20;

/// Measured runs of the idle scenario when `--runs` is absent.
pub const DEFAULT_IDLE_RUNS: u32 = 3;

/// Measured runs of the open scenario when `--runs` is absent (m4-editor-shell.md §7.1).
pub const DEFAULT_OPEN_RUNS: u32 = 5;

/// Measured runs of the scroll scenario when `--runs` is absent (m4-editor-shell.md §7.1).
pub const DEFAULT_SCROLL_RUNS: u32 = 3;

/// Measured runs of the type scenario when `--runs` is absent (m4-editor-shell.md §7.1).
pub const DEFAULT_TYPE_RUNS: u32 = 3;

/// `PageDown` presses injected by one scroll run (m4-editor-shell.md §6.8).
pub const SCROLL_EVENTS: u64 = 300;

/// Script events injected by one type run: 20 cycles of 25 (m4-editor-shell.md §6.8).
pub const TYPE_EVENTS: u64 = 500;

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
    /// CPU and frames during an idle window, with the generated project open.
    Idle,
    /// Opening the generated large file.
    Open,
    /// Opening the large file, then one `PageDown` per frame.
    Scroll,
    /// Opening the large file, then one typing event per frame in its middle.
    Type,
}

/// The generated input a scenario opens with `--bench-input` (m4-editor-shell.md §7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputKind {
    /// The 5,000-line project folder `bench/generated/5k`.
    Project,
    /// The 100,000-line file `bench/generated/100k.asm`.
    Large,
}

impl InputKind {
    /// Returns the path of this input among `inputs`.
    pub fn path(self, inputs: &BenchInputs) -> &Path {
        match self {
            Self::Project => &inputs.project,
            Self::Large => &inputs.large,
        }
    }
}

impl Scenario {
    /// Every scenario, in the run order of `all` (m4-editor-shell.md §7.1).
    pub const ALL: [Self; 5] = [
        Self::Startup,
        Self::Idle,
        Self::Open,
        Self::Scroll,
        Self::Type,
    ];

    /// Returns the command-line and JSON spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Idle => "idle",
            Self::Open => "open",
            Self::Scroll => "scroll",
            Self::Type => "type",
        }
    }

    /// Returns the measured run count when `--runs` is absent: 20, 3, 5, 3, 3.
    pub fn default_runs(self) -> u32 {
        match self {
            Self::Startup => DEFAULT_STARTUP_RUNS,
            Self::Idle => DEFAULT_IDLE_RUNS,
            Self::Open => DEFAULT_OPEN_RUNS,
            Self::Scroll => DEFAULT_SCROLL_RUNS,
            Self::Type => DEFAULT_TYPE_RUNS,
        }
    }

    /// Returns the generated input the scenario opens: none for startup, the project for idle,
    /// the large file for open, scroll and type.
    pub fn input(self) -> Option<InputKind> {
        match self {
            Self::Startup => None,
            Self::Idle => Some(InputKind::Project),
            Self::Open | Self::Scroll | Self::Type => Some(InputKind::Large),
        }
    }

    /// Tells whether the scenario measures the open time, so its run line needs a non-null
    /// `open_ms` and `vsync == false` (m4-editor-shell.md §7.2).
    pub fn measures_open(self) -> bool {
        matches!(self, Self::Open | Self::Scroll | Self::Type)
    }

    /// Returns the number of events a run injects: 300 for scroll, 500 for type, none otherwise.
    pub fn expected_events(self) -> Option<u64> {
        match self {
            Self::Scroll => Some(SCROLL_EVENTS),
            Self::Type => Some(TYPE_EVENTS),
            Self::Startup | Self::Idle | Self::Open => None,
        }
    }

    /// Tells whether the GUI prints the run line at its first frame, so that its receipt time
    /// gives `spawn_to_first_frame_ms`: startup only. The other run lines arrive after their
    /// measurement (§17.8).
    pub fn reports_at_first_frame(self) -> bool {
        self == Self::Startup
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
    /// Idle only, with the generated 5,000-line project open.
    Idle,
    /// Open the generated 100,000-line file.
    Open,
    /// Open the 100,000-line file, then 300 PageDown presses.
    Scroll,
    /// Open the 100,000-line file, then type 500 events in its middle.
    Type,
    /// Startup, idle, open, scroll, then type.
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
    /// Measured runs per scenario [default: 20 for startup, 3 for idle, 5 for open, 3 for scroll
    /// and type].
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
    /// Returns the selected scenarios in run order (`all`: startup, idle, open, scroll, type).
    pub fn scenarios(&self) -> Vec<Scenario> {
        match self.scenario {
            ScenarioSelection::Startup => vec![Scenario::Startup],
            ScenarioSelection::Idle => vec![Scenario::Idle],
            ScenarioSelection::Open => vec![Scenario::Open],
            ScenarioSelection::Scroll => vec![Scenario::Scroll],
            ScenarioSelection::Type => vec![Scenario::Type],
            ScenarioSelection::All => Scenario::ALL.to_vec(),
        }
    }

    /// Tells whether a selected scenario opens a generated input, so the inputs must exist.
    pub fn needs_inputs(&self) -> bool {
        self.scenarios()
            .iter()
            .any(|scenario| scenario.input().is_some())
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

    /// Returns the measured run count of `scenario` (`--runs` overrides every default).
    pub fn runs_for(&self, scenario: Scenario) -> u32 {
        self.runs.unwrap_or_else(|| scenario.default_runs())
    }

    /// Returns the `--duration-secs` value passed to the GUI: `Some` for idle, `None` otherwise.
    pub fn duration_for(&self, scenario: Scenario) -> Option<u64> {
        (scenario == Scenario::Idle).then_some(self.duration_secs)
    }

    /// Returns the run request of `scenario` with `renderer`: its input taken from `inputs`
    /// (`None` when the scenario opens none, or when no inputs were prepared) and its duration.
    pub fn request(
        &self,
        scenario: Scenario,
        renderer: RendererChoice,
        inputs: Option<&BenchInputs>,
    ) -> RunRequest {
        RunRequest {
            scenario,
            renderer,
            input: scenario
                .input()
                .zip(inputs)
                .map(|(kind, inputs)| kind.path(inputs).to_path_buf()),
            duration_secs: self.duration_for(scenario),
        }
    }
}

/// Runs the whole bench from `root` and prints its outcome on stdout.
///
/// Prepares the generated inputs when a selected scenario opens one, builds the shipped GUI once,
/// creates the isolated profile, then, for each renderer choice and each scenario, runs the
/// warm-up runs (discarded, but they must succeed) and the measured runs.
pub fn run(root: &Path, args: &BenchArgs) -> Result<()> {
    let cargo = locate::cargo();
    let target_dir = locate::target_directory(root)?;
    let git = git_info(root);
    let host = detect_host();
    let env = EnvSnapshot::capture();

    let inputs = if args.needs_inputs() {
        let (inputs, status) = generate::ensure_inputs_with_status(root)?;
        let state = match status {
            InputsStatus::Current => "kept",
            InputsStatus::Generated => "generated",
        };
        eprintln!(
            "==> bench inputs {} ({state}, generator version {})",
            generate::generated_dir(root).display(),
            generate::GENERATOR_VERSION
        );
        Some(inputs)
    } else {
        None
    };

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
    let profile = BenchProfile::create(&target_dir, SystemTime::now(), std::process::id())?;
    eprintln!(
        "==> isolated profile {} (VA_CONFIG_DIR, VA_DATA_DIR)",
        profile.root.display()
    );
    let mut results = Vec::new();
    for renderer in args.renderers() {
        for scenario in args.scenarios() {
            let request = args.request(scenario, renderer, inputs.as_ref());
            let runs = args.runs_for(scenario);
            for index in 1..=args.warmup {
                let sample =
                    runner::run_once(&binary.path, &request, &profile).with_context(|| {
                        format!("{scenario}/{renderer} warm-up run {index}/{}", args.warmup)
                    })?;
                eprintln!(
                    "    {scenario}/{renderer} warm-up {index}/{}: {} (discarded)",
                    args.warmup,
                    describe_sample(&sample)
                );
            }
            let mut samples = Vec::new();
            for index in 1..=runs {
                let sample = runner::run_once(&binary.path, &request, &profile)
                    .with_context(|| format!("{scenario}/{renderer} run {index}/{runs}"))?;
                eprintln!(
                    "    {scenario}/{renderer} run {index}/{runs}: {}",
                    describe_sample(&sample)
                );
                samples.push(sample);
            }
            let config = ResultConfig {
                scenario,
                renderer_requested: renderer,
                binary_bytes: binary.bytes,
                warmup: args.warmup,
                runs,
                duration_secs: request.duration_secs,
                input: request.input_name(),
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

/// Describes one sample for the progress lines: `spawn→frame 205.0 ms, frames 1, renderer glow`;
/// editor runs add `open 61.2 ms` and, with events, `event p50/p95/max 4.0/6.3/11.8 ms`.
pub fn describe_sample(sample: &Sample) -> String {
    let run = &sample.run;
    let mut parts = vec![format!(
        "spawn→frame {}",
        milliseconds(sample.spawn_to_first_frame_ms)
    )];
    if let Some(open_ms) = run.open_ms {
        parts.push(format!("open {}", milliseconds(Some(open_ms))));
    }
    if let Some(events) = &run.event_summary_ms {
        parts.push(format!(
            "event p50/p95/max {:.1}/{:.1}/{:.1} ms",
            events.median, events.p95, events.max
        ));
    }
    parts.push(format!("frames {}", run.frames_painted));
    parts.push(used_renderer(run));
    parts.join(", ")
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
