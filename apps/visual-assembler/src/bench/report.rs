//! The bench JSON line, schema `va.bench.run/2` (docs/specs/m0-foundations.md §19.5, which
//! extends the 18 keys of §4.6 with the renderer-selection keys).
//!
//! The field order of [`BenchRun`] is the normative key order of the line.

use serde::Serialize;

/// Schema identifier carried by every bench JSON line.
pub const RUN_SCHEMA: &str = "va.bench.run/2";

/// One bench result, serialized as a single JSON line by [`BenchRun::to_json_line`].
///
/// `None` fields serialize as `null`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BenchRun {
    /// Always [`RUN_SCHEMA`].
    pub schema: &'static str,
    /// `"startup"` or `"idle"`.
    pub scenario: &'static str,
    /// The resolved renderer choice: `"auto"`, `"glow"` or `"wgpu"`.
    pub renderer_requested: &'static str,
    /// The renderer actually running: `"wgpu"` or `"glow"` (`"unknown"` only if the frame
    /// exposes neither renderer).
    pub renderer: &'static str,
    /// The renderer that failed to initialize before the fallback: `"glow"`, `"wgpu"` or `null`.
    pub fallback_from: Option<&'static str>,
    /// First line of that renderer's error, or `null` without a fallback.
    pub fallback_reason: Option<String>,
    /// The raw `VA_TEST_FAIL_RENDERER` value when it is set, else `null`.
    pub injected_failure: Option<String>,
    /// Graphics backend: `dx12`, `vulkan`, `gl`, `metal`, `noop`, or `unknown`.
    pub backend: &'static str,
    /// GPU / adapter name (wgpu `AdapterInfo::name`, GL `GL_RENDERER`), if known.
    pub adapter: Option<String>,
    /// Driver description (wgpu driver + driver info, GL `GL_VERSION`), if known.
    pub driver: Option<String>,
    /// `win32`, `wayland`, `xlib`, `xcb` or `unknown`.
    pub window_system: &'static str,
    /// True when running under Windows Subsystem for Linux.
    pub wsl: bool,
    /// `std::env::consts::OS`.
    pub os: &'static str,
    /// `std::env::consts::ARCH`.
    pub arch: &'static str,
    /// Package version (`CARGO_PKG_VERSION`).
    pub version: &'static str,
    /// Milliseconds from `main()` entry to the first presented frame (a fallback is included).
    pub startup_ms: f64,
    /// Startup: frames started until the measurement (expected 1).
    /// Idle: frames started during the idle window (expected 0).
    pub frames_painted: u64,
    /// Startup: process CPU time since creation. Idle: process CPU time during the idle window.
    /// In milliseconds, user + kernel, all threads.
    pub process_cpu_ms: Option<f64>,
    /// Lifetime peak resident memory (Windows `PeakWorkingSetSize`, Linux `VmHWM`), in bytes.
    pub peak_rss_bytes: Option<u64>,
    /// Lifetime peak private bytes (Windows `PeakPagefileUsage`); `None` on Linux.
    pub peak_private_bytes: Option<u64>,
    /// Startup: `None`. Idle: settle delay before the window, in milliseconds
    /// ([`crate::bench::IDLE_SETTLE_MS`], 3000).
    pub settle_ms: Option<u64>,
    /// Startup: `None`. Idle: measured length of the idle window, in milliseconds.
    pub idle_window_ms: Option<f64>,
}

impl BenchRun {
    /// Serializes the run as one compact JSON line, without a trailing newline.
    ///
    /// # Panics
    /// Never in practice: `serde_json` only fails on maps with non-string keys or on failing
    /// custom `Serialize` implementations, and this struct has neither.
    pub fn to_json_line(&self) -> String {
        match serde_json::to_string(self) {
            Ok(line) => line,
            Err(err) => panic!("BenchRun serialization is infallible, yet failed: {err}"),
        }
    }
}
