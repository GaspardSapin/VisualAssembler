//! Covers `src/bench/process.rs` and, through `ProcessStats::sample`, the unsafe OS code of
//! `process_windows.rs` / `process_linux.rs` (docs/specs/m0-foundations.md §4.4, §11).
#![allow(clippy::unwrap_used)]

use visual_assembler::bench::process::{self, ProcessStats};

#[test]
fn parse_vm_hwm() {
    let status = "VmPeak:\t 2048 kB\nVmHWM:\t    1024 kB\n";
    assert_eq!(process::parse_vm_hwm(status), Some(1_048_576));
    // Missing line.
    assert_eq!(process::parse_vm_hwm("VmPeak:\t 2048 kB\n"), None);
    // Non-numeric value.
    assert_eq!(process::parse_vm_hwm("VmHWM:\t lots kB\n"), None);
}

#[test]
fn parse_vm_hwm_edge_cases() {
    assert_eq!(process::parse_vm_hwm(""), None);
    assert_eq!(process::parse_vm_hwm("VmHWM:\n"), None);
    assert_eq!(process::parse_vm_hwm("VmHWM:\t -5 kB\n"), None);
    // The unit must be `kB`, as in every Linux `/proc/<pid>/status`.
    assert_eq!(process::parse_vm_hwm("VmHWM:\t 1024 MB\n"), None);
    assert_eq!(process::parse_vm_hwm("VmHWM:\t 1024\n"), None);
    // Overflow of the conversion to bytes.
    assert_eq!(
        process::parse_vm_hwm(&format!("VmHWM:\t {} kB\n", u64::MAX)),
        None
    );
}

#[test]
fn sample_reports_real_values() {
    let stats = ProcessStats::sample();
    if cfg!(any(windows, target_os = "linux")) {
        let cpu_ms = stats.cpu_ms.unwrap();
        assert!(cpu_ms >= 0.0, "cpu_ms = {cpu_ms}");
        assert!(stats.peak_rss_bytes.unwrap() > 0);
    }
    if cfg!(windows) {
        assert!(stats.peak_private_bytes.unwrap() > 0);
    }
    if cfg!(target_os = "linux") {
        assert_eq!(stats.peak_private_bytes, None);
    }
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (audit round 1): realistic `/proc/<pid>/status` text, line endings, integer
// boundaries, and proof that the unsafe OS queries track real CPU and memory growth.
// ---------------------------------------------------------------------------------------------

/// A complete `/proc/self/status` as printed by Linux 6.x (values shortened).
const PROC_STATUS: &str = "Name:\tbench_process\nUmask:\t0022\nState:\tR (running)\n\
Tgid:\t4242\nPid:\t4242\nVmPeak:\t  998872 kB\nVmSize:\t  998868 kB\nVmLck:\t       0 kB\n\
VmPin:\t       0 kB\nVmHWM:\t  123456 kB\nVmRSS:\t  120000 kB\nRssAnon:\t   80000 kB\n\
Threads:\t3\nvoluntary_ctxt_switches:\t12\n";

#[test]
fn parse_vm_hwm_reads_a_full_status_file() {
    assert_eq!(process::parse_vm_hwm(PROC_STATUS), Some(123_456 * 1024));
    let crlf = PROC_STATUS.replace('\n', "\r\n");
    assert_eq!(process::parse_vm_hwm(&crlf), Some(123_456 * 1024));
    let no_final_newline = "VmHWM:\t42 kB";
    assert_eq!(process::parse_vm_hwm(no_final_newline), Some(42 * 1024));
}

#[test]
fn parse_vm_hwm_integer_boundaries() {
    assert_eq!(process::parse_vm_hwm("VmHWM:\t0 kB\n"), Some(0));
    let largest = u64::MAX / 1024;
    assert_eq!(
        process::parse_vm_hwm(&format!("VmHWM:\t{largest} kB\n")),
        Some(largest * 1024)
    );
    assert_eq!(
        process::parse_vm_hwm(&format!("VmHWM:\t{} kB\n", largest + 1)),
        None
    );
}

#[test]
fn parse_vm_hwm_rejects_lookalike_lines_and_malformed_values() {
    // Only a line starting with exactly `VmHWM:` counts.
    assert_eq!(process::parse_vm_hwm("VmHWMx:\t12 kB\n"), None);
    assert_eq!(
        process::parse_vm_hwm("XVmHWM:\t12 kB\nVmHWM:\t7 kB\n"),
        Some(7 * 1024)
    );
    for bad in [
        "VmHWM:\t12 kB extra\n",
        "VmHWM:\t12 KB\n",
        "VmHWM:\t12kB\n",
        "VmHWM:\t1.5 kB\n",
        "VmHWM:\t0x10 kB\n",
        "VmHWM:\t١٢ kB\n",
    ] {
        assert_eq!(process::parse_vm_hwm(bad), None, "{bad:?}");
    }
}

/// Spins on the current thread until the process CPU time grew by `min_ms`, or `limit` passed.
fn burn_cpu(min_ms: f64, limit: std::time::Duration) -> f64 {
    let start = ProcessStats::sample().cpu_ms.unwrap();
    let deadline = std::time::Instant::now() + limit;
    let mut accumulator = 0_u64;
    loop {
        for i in 0..100_000_u64 {
            accumulator = accumulator
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(i);
        }
        std::hint::black_box(accumulator);
        let now = ProcessStats::sample().cpu_ms.unwrap();
        if now - start >= min_ms || std::time::Instant::now() >= deadline {
            return now - start;
        }
    }
}

#[test]
fn sample_tracks_cpu_time_and_peak_memory_growth() {
    if !cfg!(any(windows, target_os = "linux")) {
        return;
    }
    const MIB: u64 = 1_048_576;
    let before = ProcessStats::sample();

    let burned = burn_cpu(50.0, std::time::Duration::from_secs(10));
    assert!(burned >= 50.0, "CPU time grew by only {burned} ms");

    // Every byte is written, so every page becomes resident.
    let block = vec![1_u8; (128 * MIB) as usize];
    std::hint::black_box(&block);
    let after = ProcessStats::sample();
    drop(block);

    assert!(after.cpu_ms.unwrap() >= before.cpu_ms.unwrap());
    let rss_before = before.peak_rss_bytes.unwrap();
    let rss_after = after.peak_rss_bytes.unwrap();
    assert!(
        rss_after >= rss_before + 64 * MIB,
        "peak RSS {rss_before} -> {rss_after} after touching 128 MiB"
    );
    if cfg!(windows) {
        let private_before = before.peak_private_bytes.unwrap();
        let private_after = after.peak_private_bytes.unwrap();
        assert!(
            private_after >= private_before + 64 * MIB,
            "peak private bytes {private_before} -> {private_after} after allocating 128 MiB"
        );
    }
}
