//! Windows implementation of [`ProcessStats::sample`] through `windows-sys`.
//!
//! - CPU: `GetProcessTimes(GetCurrentProcess())`, kernel + user `FILETIME` (100 ns units)
//!   divided by 10,000 gives milliseconds;
//! - memory: `GetProcessMemoryInfo`, `PeakWorkingSetSize` (peak RSS) and `PeakPagefileUsage`
//!   (peak private bytes).
//!
//! This file is on the unsafe allow-list (`UNSAFE_ALLOWLIST` in `xtask/src/layering.rs`).
#![allow(unsafe_code)]

use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

use super::process::ProcessStats;

/// Number of `FILETIME` ticks (100 ns each) in one millisecond.
const FILETIME_TICKS_PER_MS: f64 = 10_000.0;

/// Samples CPU time and peak memory of the current process. A failed query yields `None` for
/// the values it provides.
pub(super) fn sample() -> ProcessStats {
    let (peak_rss_bytes, peak_private_bytes) = match peak_memory() {
        Some((rss, private)) => (Some(rss), Some(private)),
        None => (None, None),
    };
    ProcessStats {
        cpu_ms: cpu_ms(),
        peak_rss_bytes,
        peak_private_bytes,
    }
}

/// Kernel + user CPU time of the current process in milliseconds, or `None` if the call fails.
fn cpu_ms() -> Option<f64> {
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: `GetCurrentProcess` returns a pseudo-handle that is always valid for the calling
    // process and needs no closing. The four pointers come from exclusive borrows of live,
    // properly aligned `FILETIME` locals that the call only writes to.
    let ok = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };
    if ok == 0 {
        return None;
    }
    let ticks = filetime_ticks(kernel).saturating_add(filetime_ticks(user));
    Some(ticks as f64 / FILETIME_TICKS_PER_MS)
}

/// Peak working set and peak pagefile usage in bytes, or `None` if the call fails.
fn peak_memory() -> Option<(u64, u64)> {
    let size = u32::try_from(std::mem::size_of::<PROCESS_MEMORY_COUNTERS>()).ok()?;
    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: size,
        ..PROCESS_MEMORY_COUNTERS::default()
    };
    // SAFETY: `GetCurrentProcess` returns an always-valid pseudo-handle (see `cpu_ms`). The
    // pointer comes from an exclusive borrow of a live `PROCESS_MEMORY_COUNTERS`, and `size` is
    // exactly its size, so the call writes within bounds.
    let ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) };
    if ok == 0 {
        return None;
    }
    let rss = u64::try_from(counters.PeakWorkingSetSize).ok()?;
    let private = u64::try_from(counters.PeakPagefileUsage).ok()?;
    Some((rss, private))
}

/// Combines the two halves of a `FILETIME` into a 64-bit count of 100 ns ticks.
fn filetime_ticks(time: FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}
