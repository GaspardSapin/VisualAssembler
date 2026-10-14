//! Bench report: the `va.bench.report/1` document, its markdown table and its file name (§6.2).
//!
//! Every struct serializes its keys in the order of the specification.

use std::{
    fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};

use super::{
    Renderer, Scenario,
    stats::{Summary, summarize, summarize_optional},
    timestamp,
};

/// Schema id of the report document.
pub const REPORT_SCHEMA: &str = "va.bench.report/1";

/// Schema id of the JSON line printed by `visual-assembler --bench` (§4.6).
pub const RUN_SCHEMA: &str = "va.bench.run/1";

/// Cargo profile of the measured binaries.
pub const PROFILE: &str = "release";

/// Value of `backend`/`adapter`/`driver`/`window_system` when the samples disagree.
pub const MIXED: &str = "mixed";

/// Value of `backend`/`window_system` when there is no sample to read it from.
pub const UNKNOWN: &str = "unknown";

/// Environment variables recorded in each result, in report order.
pub const ENV_VARS: [&str; 5] = [
    "WGPU_BACKEND",
    "WGPU_ADAPTER_NAME",
    "WGPU_POWER_PREF",
    "WAYLAND_DISPLAY",
    "DISPLAY",
];

/// Header line of the markdown table (§6.2, exact text).
pub const MARKDOWN_HEADER: &str = "| scenario | renderer | backend | window | spawn→frame p50/p95 ms | startup p50/p95 ms | idle CPU p50/p95 % | frames p50/max | peak RSS p50 MiB | binary MiB |";

/// Separator line that follows [`MARKDOWN_HEADER`].
pub const MARKDOWN_SEPARATOR: &str = "|---|---|---|---|---|---|---|---|---|---|";

/// Bytes per MiB.
const MIB: f64 = 1_048_576.0;

/// The JSON line printed by one GUI bench run, schema `va.bench.run/1` (§4.6, keys in order).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchRun {
    /// Always [`RUN_SCHEMA`].
    pub schema: String,
    /// `startup` or `idle`.
    pub scenario: String,
    /// `wgpu` or `glow`.
    pub renderer: String,
    /// Graphics backend, e.g. `dx12`, `vulkan`, `gl`.
    pub backend: String,
    /// GPU or adapter name.
    pub adapter: Option<String>,
    /// Driver or GL version string.
    pub driver: Option<String>,
    /// `win32`, `wayland`, `xlib`, `xcb` or `unknown`.
    pub window_system: String,
    /// Whether the GUI ran under WSL.
    pub wsl: bool,
    /// `std::env::consts::OS` of the GUI.
    pub os: String,
    /// `std::env::consts::ARCH` of the GUI.
    pub arch: String,
    /// GUI package version.
    pub version: String,
    /// `main()` to first frame presented, in ms.
    pub startup_ms: f64,
    /// Frames started until the measurement (startup) or during the idle window (idle).
    pub frames_painted: u64,
    /// Process CPU time in ms (since creation, or during the idle window).
    pub process_cpu_ms: Option<f64>,
    /// Lifetime peak resident set, in bytes.
    pub peak_rss_bytes: Option<u64>,
    /// Lifetime peak private bytes (Windows only).
    pub peak_private_bytes: Option<u64>,
    /// Settle delay before the idle window, in ms (idle only).
    pub settle_ms: Option<u64>,
    /// Measured idle window length, in ms (idle only).
    pub idle_window_ms: Option<f64>,
}

/// One measured run as recorded in the report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sample {
    /// From `Command::spawn` in xtask to the receipt of the run line, in ms. `null` for idle
    /// runs (§17.8): an idle run reports after its idle window, not after its first frame.
    pub spawn_to_first_frame_ms: Option<f64>,
    /// The run line printed by the GUI.
    pub run: BenchRun,
}

/// Environment variables that select backends and window systems, as seen by the runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct EnvSnapshot {
    /// `WGPU_BACKEND`.
    #[serde(rename = "WGPU_BACKEND")]
    pub wgpu_backend: Option<String>,
    /// `WGPU_ADAPTER_NAME`.
    #[serde(rename = "WGPU_ADAPTER_NAME")]
    pub wgpu_adapter_name: Option<String>,
    /// `WGPU_POWER_PREF`.
    #[serde(rename = "WGPU_POWER_PREF")]
    pub wgpu_power_pref: Option<String>,
    /// `WAYLAND_DISPLAY`.
    #[serde(rename = "WAYLAND_DISPLAY")]
    pub wayland_display: Option<String>,
    /// `DISPLAY`.
    #[serde(rename = "DISPLAY")]
    pub display: Option<String>,
}

impl EnvSnapshot {
    /// Builds a snapshot by asking `lookup` for each name of [`ENV_VARS`].
    pub fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Self {
        let [backend, adapter, power, wayland, display] = ENV_VARS;
        Self {
            wgpu_backend: lookup(backend),
            wgpu_adapter_name: lookup(adapter),
            wgpu_power_pref: lookup(power),
            wayland_display: lookup(wayland),
            display: lookup(display),
        }
    }

    /// Reads the variables from the current process environment (non-UTF-8 values are lossy).
    pub fn capture() -> Self {
        Self::from_lookup(|name| {
            std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
        })
    }
}

/// Git state of the measured source tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitInfo {
    /// `git rev-parse --short=8 HEAD`, or `nogit` when that fails.
    pub sha: String,
    /// Whether tracked files have uncommitted changes.
    pub dirty: bool,
}

/// The machine xtask ran on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostInfo {
    /// `std::env::consts::OS`.
    pub os: String,
    /// `std::env::consts::ARCH`.
    pub arch: String,
    /// Same rule as the GUI's `is_wsl`.
    pub wsl: bool,
    /// `std::thread::available_parallelism`, or null when the OS cannot tell.
    pub logical_cpus: Option<usize>,
}

/// How one result was measured (everything except what the samples provide).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultConfig {
    /// Measured scenario.
    pub scenario: Scenario,
    /// Renderer compiled into the measured binary.
    pub renderer: Renderer,
    /// Size of the measured binary, in bytes.
    pub binary_bytes: u64,
    /// Discarded warm-up runs.
    pub warmup: u32,
    /// Measured runs.
    pub runs: u32,
    /// Idle window length passed to the GUI; `None` for startup.
    pub duration_secs: Option<u64>,
    /// Environment seen by the runs.
    pub env: EnvSnapshot,
}

/// Per-metric summaries of one result (`null` when a sample lacks the metric).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResultSummary {
    /// Outer startup time, the gating number; always `null` for idle results (§17.8).
    pub spawn_to_first_frame_ms: Option<Summary>,
    /// Inner startup time reported by the GUI.
    pub startup_ms: Option<Summary>,
    /// Process CPU time.
    pub process_cpu_ms: Option<Summary>,
    /// Peak resident set.
    pub peak_rss_bytes: Option<Summary>,
    /// Peak private bytes (null on Linux).
    pub peak_private_bytes: Option<Summary>,
    /// Frames painted.
    pub frames_painted: Option<Summary>,
    /// Idle results only: per sample `process_cpu_ms / idle_window_ms × 100`.
    pub idle_cpu_percent: Option<Summary>,
}

impl ResultSummary {
    /// Summarizes `samples` of `scenario`. `spawn_to_first_frame_ms` is `null` for idle results
    /// whatever the samples hold (§17.8).
    pub fn from_samples(scenario: Scenario, samples: &[Sample]) -> Self {
        let spawn_to_first_frame_ms = match scenario {
            Scenario::Startup => {
                summarize_optional(samples.iter().map(|s| s.spawn_to_first_frame_ms))
            }
            Scenario::Idle => None,
        };
        let startup: Vec<f64> = samples.iter().map(|s| s.run.startup_ms).collect();
        let frames: Vec<f64> = samples
            .iter()
            .map(|s| s.run.frames_painted as f64)
            .collect();
        let idle_cpu_percent = match scenario {
            Scenario::Startup => None,
            Scenario::Idle => summarize_optional(samples.iter().map(|s| {
                let cpu = s.run.process_cpu_ms?;
                let window = s.run.idle_window_ms?;
                Some(cpu / window * 100.0)
            })),
        };
        Self {
            spawn_to_first_frame_ms,
            startup_ms: summarize(&startup),
            process_cpu_ms: summarize_optional(samples.iter().map(|s| s.run.process_cpu_ms)),
            peak_rss_bytes: summarize_optional(
                samples
                    .iter()
                    .map(|s| s.run.peak_rss_bytes.map(|bytes| bytes as f64)),
            ),
            peak_private_bytes: summarize_optional(
                samples
                    .iter()
                    .map(|s| s.run.peak_private_bytes.map(|bytes| bytes as f64)),
            ),
            frames_painted: summarize(&frames),
            idle_cpu_percent,
        }
    }
}

/// One (scenario, renderer) result of the report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BenchResult {
    /// Measured scenario.
    pub scenario: Scenario,
    /// Renderer compiled into the measured binary.
    pub renderer: Renderer,
    /// Size of the measured binary, in bytes.
    pub binary_bytes: u64,
    /// Discarded warm-up runs.
    pub warmup: u32,
    /// Measured runs.
    pub runs: u32,
    /// Idle window length; null for startup.
    pub duration_secs: Option<u64>,
    /// Environment seen by the runs.
    pub env: EnvSnapshot,
    /// Backend of the samples, or `mixed`.
    pub backend: String,
    /// Adapter of the samples, or `mixed`.
    pub adapter: Option<String>,
    /// Driver of the samples, or `mixed`.
    pub driver: Option<String>,
    /// Window system of the samples, or `mixed`.
    pub window_system: String,
    /// Per-metric summaries.
    pub summary: ResultSummary,
    /// Every measured run (warm-up runs are not recorded).
    pub samples: Vec<Sample>,
}

impl BenchResult {
    /// Builds a result: the descriptive fields and the summary are derived from `samples`.
    ///
    /// For idle results, every sample's `spawn_to_first_frame_ms` is set to `null` (§17.8).
    pub fn new(config: ResultConfig, mut samples: Vec<Sample>) -> Self {
        if config.scenario == Scenario::Idle {
            for sample in &mut samples {
                sample.spawn_to_first_frame_ms = None;
            }
        }
        let summary = ResultSummary::from_samples(config.scenario, &samples);
        Self {
            scenario: config.scenario,
            renderer: config.renderer,
            binary_bytes: config.binary_bytes,
            warmup: config.warmup,
            runs: config.runs,
            duration_secs: config.duration_secs,
            env: config.env,
            backend: agreed(samples.iter().map(|s| Some(s.run.backend.as_str())))
                .unwrap_or_else(|| UNKNOWN.to_owned()),
            adapter: agreed(samples.iter().map(|s| s.run.adapter.as_deref())),
            driver: agreed(samples.iter().map(|s| s.run.driver.as_deref())),
            window_system: agreed(samples.iter().map(|s| Some(s.run.window_system.as_str())))
                .unwrap_or_else(|| UNKNOWN.to_owned()),
            summary,
            samples,
        }
    }

    /// Renders this result as one markdown table row.
    pub fn markdown_row(&self) -> String {
        let s = &self.summary;
        format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.1} |",
            self.scenario.as_str(),
            self.renderer.as_str(),
            self.backend,
            self.window_system,
            median_p95(s.spawn_to_first_frame_ms.as_ref(), 1),
            median_p95(s.startup_ms.as_ref(), 1),
            median_p95(s.idle_cpu_percent.as_ref(), 2),
            s.frames_painted
                .as_ref()
                .map_or_else(not_available, |f| format!("{}/{}", f.median, f.max)),
            s.peak_rss_bytes
                .as_ref()
                .map_or_else(not_available, |r| format!("{:.1}", r.median / MIB)),
            self.binary_bytes as f64 / MIB,
        )
    }
}

/// Returns the common value of `values`, [`MIXED`] when they differ, `None` when they are all
/// `None` or there is none.
fn agreed<'a>(mut values: impl Iterator<Item = Option<&'a str>>) -> Option<String> {
    let first = values.next()?;
    if values.all(|value| value == first) {
        first.map(str::to_owned)
    } else {
        Some(MIXED.to_owned())
    }
}

/// Formats `median/p95` with `decimals` decimals, or `n/a`.
fn median_p95(summary: Option<&Summary>, decimals: usize) -> String {
    summary.map_or_else(not_available, |s| {
        format!("{:.*}/{:.*}", decimals, s.median, decimals, s.p95)
    })
}

/// The table cell of a missing value.
fn not_available() -> String {
    "n/a".to_owned()
}

/// The `va.bench.report/1` document.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    /// Always [`REPORT_SCHEMA`].
    pub schema: &'static str,
    /// Creation time, `YYYY-MM-DDTHH:MM:SSZ`.
    pub created_utc: String,
    /// Git state of the measured tree.
    pub git: GitInfo,
    /// Machine description.
    pub host: HostInfo,
    /// Always [`PROFILE`].
    pub profile: &'static str,
    /// One entry per (renderer, scenario), renderers in the order wgpu, glow.
    pub results: Vec<BenchResult>,
}

impl Report {
    /// Builds a report created at `created`.
    pub fn new(
        created: SystemTime,
        git: GitInfo,
        host: HostInfo,
        results: Vec<BenchResult>,
    ) -> Self {
        Self {
            schema: REPORT_SCHEMA,
            created_utc: timestamp::utc_iso(created),
            git,
            host,
            profile: PROFILE,
            results,
        }
    }

    /// Serializes the report as pretty-printed JSON, without a trailing newline.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("cannot serialize the bench report")
    }

    /// Renders the markdown table: header, separator, one row per result, each line ending
    /// with `\n`.
    pub fn markdown_table(&self) -> String {
        let mut table = format!("{MARKDOWN_HEADER}\n{MARKDOWN_SEPARATOR}\n");
        for result in &self.results {
            table.push_str(&result.markdown_row());
            table.push('\n');
        }
        table
    }
}

/// Returns the report file name `<YYYYMMDDTHHMMSSZ>-<sha8>.json`.
pub fn report_file_name(t: SystemTime, sha8: &str) -> String {
    format!("{}-{sha8}.json", timestamp::utc_compact(t))
}

/// Maximum number of `-2`, `-3`, … suffixes tried before giving up.
const MAX_NAME_ATTEMPTS: u32 = 1000;

/// Writes `report` into `dir` (created if missing) as [`report_file_name`], adding `-2`, `-3`,
/// … before `.json` when the name is taken. Never overwrites a file. Returns the written path.
pub fn write_report(dir: &Path, created: SystemTime, report: &Report) -> Result<PathBuf> {
    fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let mut text = report.to_json()?;
    text.push('\n');
    let base = report_file_name(created, &report.git.sha);
    let stem = base.trim_end_matches(".json");
    for attempt in 1..=MAX_NAME_ATTEMPTS {
        let name = if attempt == 1 {
            base.clone()
        } else {
            format!("{stem}-{attempt}.json")
        };
        let path = dir.join(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                file.write_all(text.as_bytes())
                    .and_then(|()| file.flush())
                    .with_context(|| format!("cannot write {}", path.display()))?;
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error).with_context(|| format!("cannot create {}", path.display()));
            }
        }
    }
    bail!(
        "{MAX_NAME_ATTEMPTS} report files named {stem}*.json already exist in {}",
        dir.display()
    );
}
