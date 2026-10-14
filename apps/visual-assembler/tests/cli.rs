//! Covers `src/cli.rs`: parsing with `GuiArgs::try_parse_from` and validation with `resolve`
//! (docs/specs/m0-foundations.md §4.3, §11).
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use clap::Parser as _;
use visual_assembler::cli::{
    self, BenchPlan, BenchScenario, GuiArgs, LaunchPlan, RendererChoice, UsageError,
};

/// Both renderers compiled in.
const BOTH: &[RendererChoice] = &[RendererChoice::Wgpu, RendererChoice::Glow];

/// Parses a command line (without the program name).
fn parse(args: &[&str]) -> GuiArgs {
    GuiArgs::try_parse_from(std::iter::once("visual-assembler").chain(args.iter().copied()))
        .unwrap()
}

#[test]
fn default_renderer_is_wgpu_when_compiled() {
    let plan = cli::resolve(&parse(&[]), BOTH).unwrap();
    assert_eq!(
        plan,
        LaunchPlan {
            renderer: RendererChoice::Wgpu,
            bench: None
        }
    );
}

#[test]
fn default_renderer_is_glow_without_wgpu() {
    let plan = cli::resolve(&parse(&[]), &[RendererChoice::Glow]).unwrap();
    assert_eq!(plan.renderer, RendererChoice::Glow);
}

#[test]
fn explicit_renderer_is_kept() {
    let plan = cli::resolve(&parse(&["--renderer", "glow"]), BOTH).unwrap();
    assert_eq!(plan.renderer, RendererChoice::Glow);
}

#[test]
fn renderer_not_compiled_is_an_error() {
    let err = cli::resolve(&parse(&["--renderer", "glow"]), &[RendererChoice::Wgpu]).unwrap_err();
    assert_eq!(err, UsageError::RendererNotCompiled(RendererChoice::Glow));
    let message = err.to_string();
    assert!(message.contains("not compiled"), "{message}");
    assert_eq!(
        message,
        "renderer 'glow' is not compiled into this binary (build with --features renderer-glow)"
    );
}

#[test]
fn bench_idle_defaults_to_ten_seconds() {
    let plan = cli::resolve(&parse(&["--bench", "idle"]), BOTH).unwrap();
    assert_eq!(
        plan.bench,
        Some(BenchPlan {
            scenario: BenchScenario::Idle,
            idle_duration: Duration::from_secs(10)
        })
    );
}

#[test]
fn bench_idle_takes_duration() {
    let plan = cli::resolve(&parse(&["--bench", "idle", "--duration-secs", "3"]), BOTH).unwrap();
    assert_eq!(
        plan.bench,
        Some(BenchPlan {
            scenario: BenchScenario::Idle,
            idle_duration: Duration::from_secs(3)
        })
    );
}

#[test]
fn bench_startup_has_no_duration_flag() {
    let err = cli::resolve(
        &parse(&["--bench", "startup", "--duration-secs", "3"]),
        BOTH,
    )
    .unwrap_err();
    assert_eq!(err, UsageError::DurationWithoutIdle);
    assert_eq!(err.to_string(), "--duration-secs requires --bench idle");
}

#[test]
fn duration_without_bench_is_an_error() {
    let err = cli::resolve(&parse(&["--duration-secs", "3"]), BOTH).unwrap_err();
    assert_eq!(err, UsageError::DurationWithoutIdle);
}

#[test]
fn duration_zero_is_a_parse_error() {
    let parsed = GuiArgs::try_parse_from([
        "visual-assembler",
        "--bench",
        "idle",
        "--duration-secs",
        "0",
    ]);
    assert!(parsed.is_err());
}

#[test]
fn duration_above_one_hour_is_a_parse_error() {
    let parsed = GuiArgs::try_parse_from([
        "visual-assembler",
        "--bench",
        "idle",
        "--duration-secs",
        "3601",
    ]);
    assert!(parsed.is_err());
    let plan = cli::resolve(
        &parse(&["--bench", "idle", "--duration-secs", "3600"]),
        BOTH,
    )
    .unwrap();
    assert_eq!(plan.bench.unwrap().idle_duration, Duration::from_secs(3600));
}

#[test]
fn compiled_renderers_match_features() {
    let compiled = cli::compiled_renderers();
    assert_eq!(
        compiled.contains(&RendererChoice::Wgpu),
        cfg!(feature = "renderer-wgpu")
    );
    assert_eq!(
        compiled.contains(&RendererChoice::Glow),
        cfg!(feature = "renderer-glow")
    );
    assert!(!compiled.is_empty());
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (audit round 1).
// ---------------------------------------------------------------------------------------------

/// Parses a command line (without the program name), keeping clap's error.
fn try_parse(args: &[&str]) -> Result<GuiArgs, clap::Error> {
    GuiArgs::try_parse_from(std::iter::once("visual-assembler").chain(args.iter().copied()))
}

#[test]
fn wgpu_requested_in_a_glow_only_build_is_an_error() {
    let err = cli::resolve(&parse(&["--renderer", "wgpu"]), &[RendererChoice::Glow]).unwrap_err();
    assert_eq!(err, UsageError::RendererNotCompiled(RendererChoice::Wgpu));
    assert_eq!(
        err.to_string(),
        "renderer 'wgpu' is not compiled into this binary (build with --features renderer-wgpu)"
    );
}

#[test]
fn no_compiled_renderer_is_an_error_not_a_panic() {
    let err = cli::resolve(&parse(&[]), &[]).unwrap_err();
    assert!(matches!(err, UsageError::RendererNotCompiled(_)), "{err:?}");
    assert!(err.to_string().contains("not compiled"), "{err}");
}

#[test]
fn startup_bench_plan_has_the_startup_scenario() {
    let plan = cli::resolve(&parse(&["--bench", "startup"]), BOTH).unwrap();
    assert_eq!(plan.renderer, RendererChoice::Wgpu);
    assert_eq!(plan.bench.unwrap().scenario, BenchScenario::Startup);
}

#[test]
fn duration_lower_bound_is_inclusive() {
    let plan = cli::resolve(&parse(&["--bench", "idle", "--duration-secs", "1"]), BOTH).unwrap();
    assert_eq!(plan.bench.unwrap().idle_duration, Duration::from_secs(1));
}

#[test]
fn equals_syntax_and_any_flag_order_are_accepted() {
    let plan = cli::resolve(
        &parse(&["--duration-secs=7", "--renderer=glow", "--bench=idle"]),
        BOTH,
    )
    .unwrap();
    assert_eq!(
        plan,
        LaunchPlan {
            renderer: RendererChoice::Glow,
            bench: Some(BenchPlan {
                scenario: BenchScenario::Idle,
                idle_duration: Duration::from_secs(7)
            })
        }
    );
}

#[test]
fn malformed_values_are_usage_errors() {
    for bad in [
        &["--duration-secs", "-1"][..],
        &["--duration-secs", "abc"],
        &["--duration-secs", ""],
        &["--duration-secs", "1.5"],
        &["--duration-secs", " 5"],
        &["--duration-secs", "18446744073709551616"],
        &["--renderer", ""],
        &["--renderer", "opengl"],
        &["--bench", "startup "],
        &["--bench", "all"],
        &["extra"],
    ] {
        let err = try_parse(bad).unwrap_err();
        assert_eq!(err.exit_code(), 2, "{bad:?}");
    }
}

#[test]
fn repeated_flags_are_usage_errors() {
    for bad in [
        &["--bench", "startup", "--bench", "idle"][..],
        &["--renderer", "wgpu", "--renderer", "glow"],
        &[
            "--bench",
            "idle",
            "--duration-secs",
            "3",
            "--duration-secs",
            "4",
        ],
    ] {
        let err = try_parse(bad).unwrap_err();
        assert_eq!(err.exit_code(), 2, "{bad:?}");
    }
}

#[test]
fn help_and_version_are_display_outcomes_with_exit_code_zero() {
    let version = try_parse(&["--version"]).unwrap_err();
    assert_eq!(version.kind(), clap::error::ErrorKind::DisplayVersion);
    assert_eq!(version.exit_code(), 0);
    assert_eq!(version.to_string(), "visual-assembler 0.1.0\n");

    let help = try_parse(&["--help"]).unwrap_err();
    assert_eq!(help.kind(), clap::error::ErrorKind::DisplayHelp);
    assert_eq!(help.exit_code(), 0);
}
