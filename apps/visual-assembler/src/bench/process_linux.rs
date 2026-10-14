//! Linux implementation of [`ProcessStats::sample`].
//!
//! - CPU: `libc::getrusage(RUSAGE_SELF)`, `ru_utime + ru_stime` in milliseconds;
//! - memory: `VmHWM` read from `/proc/self/status` with `std::fs`, parsed by
//!   [`parse_vm_hwm`]; Linux has no peak-private-bytes counter, so that value is `None`.
//!
//! This file is on the unsafe allow-list (`UNSAFE_ALLOWLIST` in `xtask/src/layering.rs`).
#![allow(unsafe_code)]

use std::mem::MaybeUninit;

use super::process::{ProcessStats, parse_vm_hwm};

/// Path of the status file of the current process.
const PROC_SELF_STATUS: &str = "/proc/self/status";

/// Samples CPU time and peak RSS of the current process. A failed query yields `None`.
pub(super) fn sample() -> ProcessStats {
    ProcessStats {
        cpu_ms: cpu_ms(),
        peak_rss_bytes: peak_rss_bytes(),
        peak_private_bytes: None,
    }
}

/// User + system CPU time of the current process in milliseconds, or `None` if the call fails.
fn cpu_ms() -> Option<f64> {
    let mut usage = MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: `RUSAGE_SELF` is a valid `who` value and the pointer refers to writable memory
    // large enough for one `rusage`, which the kernel fills completely on success.
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status != 0 {
        return None;
    }
    // SAFETY: `getrusage` returned 0, so it initialized the whole structure.
    let usage = unsafe { usage.assume_init() };
    Some(timeval_ms(usage.ru_utime) + timeval_ms(usage.ru_stime))
}

/// Converts a `timeval` (seconds + microseconds) to milliseconds.
fn timeval_ms(time: libc::timeval) -> f64 {
    time.tv_sec as f64 * 1000.0 + time.tv_usec as f64 / 1000.0
}

/// `VmHWM` of the current process in bytes, or `None` if the file cannot be read or parsed.
fn peak_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string(PROC_SELF_STATUS).ok()?;
    parse_vm_hwm(&status)
}
