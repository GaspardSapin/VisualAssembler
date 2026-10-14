//! Process statistics for the bench: CPU time and peak memory of the current process.
//!
//! [`ProcessStats::sample`] dispatches to the OS-specific private modules (`process_windows`,
//! `process_linux`); on other operating systems every value is `None`. Each value is `None` when
//! the OS query fails, which the bench JSON line reports as `null`.

/// One sample of the current process statistics. `None` means unavailable on this OS, or the OS
/// query failed.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ProcessStats {
    /// CPU time used since process creation (user + kernel, all threads), in milliseconds.
    pub cpu_ms: Option<f64>,
    /// Lifetime peak resident memory in bytes (Windows `PeakWorkingSetSize`, Linux `VmHWM`).
    pub peak_rss_bytes: Option<u64>,
    /// Lifetime peak private bytes (Windows `PeakPagefileUsage`); always `None` on Linux.
    pub peak_private_bytes: Option<u64>,
}

impl ProcessStats {
    /// Samples the current process. Cheap enough to call from a frame (a few syscalls).
    pub fn sample() -> Self {
        #[cfg(windows)]
        {
            super::process_windows::sample()
        }
        #[cfg(target_os = "linux")]
        {
            super::process_linux::sample()
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            Self::default()
        }
    }
}

/// Parses the `VmHWM` line of a Linux `/proc/<pid>/status` text and returns it in bytes.
///
/// Returns `None` when the line is missing, its value is not a decimal integer, its unit is not
/// `kB`, or the conversion to bytes overflows. Portable: usable and testable on every OS.
pub fn parse_vm_hwm(proc_status: &str) -> Option<u64> {
    let value = proc_status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))?;
    let mut fields = value.split_whitespace();
    let kibibytes: u64 = fields.next()?.parse().ok()?;
    match (fields.next(), fields.next()) {
        (Some("kB"), None) => kibibytes.checked_mul(1024),
        _ => None,
    }
}
