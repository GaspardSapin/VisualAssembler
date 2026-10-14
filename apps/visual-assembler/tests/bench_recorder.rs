//! Covers `src/bench/recorder.rs` and the timing constants of `src/bench/mod.rs` that it uses:
//! the settle delay and the watchdog hard limits (docs/specs/m0-foundations.md §4.5, §17.13).
//! Pure arithmetic: no window, no thread, no GPU.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use visual_assembler::bench::recorder::watchdog_limit;
use visual_assembler::bench::{
    DEFAULT_IDLE_SECS, IDLE_SETTLE_MS, IDLE_WATCHDOG_EXTRA_SECS, STARTUP_WATCHDOG_SECS,
};
use visual_assembler::cli::{BenchPlan, BenchScenario};

/// A bench plan for `scenario` with an idle window of `idle_secs` seconds.
fn plan(scenario: BenchScenario, idle_secs: u64) -> BenchPlan {
    BenchPlan {
        scenario,
        idle_duration: Duration::from_secs(idle_secs),
    }
}

#[test]
fn timing_constants_match_the_spec() {
    assert_eq!(IDLE_SETTLE_MS, 3000);
    assert_eq!(DEFAULT_IDLE_SECS, 10);
    assert_eq!(STARTUP_WATCHDOG_SECS, 60);
    assert_eq!(IDLE_WATCHDOG_EXTRA_SECS, 60);
}

#[test]
fn startup_watchdog_is_60_s_whatever_the_idle_duration() {
    for idle_secs in [1, DEFAULT_IDLE_SECS, 3600] {
        assert_eq!(
            watchdog_limit(&plan(BenchScenario::Startup, idle_secs)),
            Duration::from_secs(60),
            "idle_secs: {idle_secs}"
        );
    }
}

#[test]
fn idle_watchdog_is_60_plus_n_plus_3_s() {
    for idle_secs in [1, DEFAULT_IDLE_SECS, 3600] {
        assert_eq!(
            watchdog_limit(&plan(BenchScenario::Idle, idle_secs)),
            Duration::from_secs(60 + idle_secs + 3),
            "idle_secs: {idle_secs}"
        );
    }
}

/// The idle limit must cover the settle delay, the window and the margin, so a correct run
/// (which reports right after settle + window) can never be killed by its own watchdog.
#[test]
fn idle_watchdog_covers_settle_and_window() {
    let idle = plan(BenchScenario::Idle, DEFAULT_IDLE_SECS);
    let busy_part = Duration::from_millis(IDLE_SETTLE_MS) + idle.idle_duration;
    assert!(watchdog_limit(&idle) > busy_part);
    assert_eq!(
        watchdog_limit(&idle) - busy_part,
        Duration::from_secs(IDLE_WATCHDOG_EXTRA_SECS)
    );
}

#[test]
fn idle_watchdog_saturates_instead_of_panicking() {
    let huge = BenchPlan {
        scenario: BenchScenario::Idle,
        idle_duration: Duration::MAX,
    };
    assert_eq!(watchdog_limit(&huge), Duration::MAX);
}
