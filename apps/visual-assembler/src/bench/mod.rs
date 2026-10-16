//! Bench instrumentation of the GUI (docs/specs/m0-foundations.md §4.4–§4.6, amended by §19.5).
//!
//! `visual-assembler --bench startup|idle` runs one measurement scenario inside the real window,
//! prints exactly one `va.bench.run/2` JSON line on stdout and exits. `cargo xtask bench`
//! orchestrates many runs and aggregates them.
//!
//! - [`recorder`]: the measurement algorithm (§4.5), hooked into `App::logic` / `App::ui`;
//! - [`report`]: the JSON line schema (§4.6, v2 in §19.5);
//! - [`process`]: process CPU time and peak memory, dispatched to the private OS modules
//!   `process_windows` / `process_linux` (the only files allowed to use `unsafe`, with
//!   `platform`);
//! - [`platform`]: renderer, GPU adapter and window-system detection, WSL detection.

pub mod platform;
pub mod process;
pub mod recorder;
pub mod report;

#[cfg(target_os = "linux")]
mod process_linux;
#[cfg(windows)]
mod process_windows;

/// Delay between the first presented frame and the start of the idle window, in milliseconds.
///
/// 3 s (docs/specs/m0-foundations.md §17.13): on the reference machine wgpu/Vulkan painted one
/// late frame about 2 s after the first one, so the idle window must start after the
/// post-map transients to measure steady state. Also reported as `settle_ms` in idle lines.
pub const IDLE_SETTLE_MS: u64 = 3000;

/// Default length of the idle window, in seconds (`--duration-secs`).
pub const DEFAULT_IDLE_SECS: u64 = 10;

/// Hard limit of the startup scenario, in seconds, after which the watchdog exits with code 4.
pub const STARTUP_WATCHDOG_SECS: u64 = 60;

/// Margin of the idle scenario, in seconds: the watchdog exits with code 4 after
/// `IDLE_WATCHDOG_EXTRA_SECS` + idle duration + [`IDLE_SETTLE_MS`] (60 + N + 3 s, §17.13).
/// See [`recorder::watchdog_limit`].
pub const IDLE_WATCHDOG_EXTRA_SECS: u64 = 60;
