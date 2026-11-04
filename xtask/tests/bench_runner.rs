//! Tests for `src/bench/runner.rs` and the argument helpers of `src/bench/mod.rs`: the pure
//! parts of building and spawning (shipped-binary cargo arguments, artifact message parsing, GUI
//! arguments with `--renderer` and `--bench-input`, the isolated profile, timeouts, run-line
//! detection and validation against the request, WSL detection, output caps; m0-foundations.md
//! §17.8, §19.5; m4-editor-shell.md §7.1, §7.2), and, on Unix, the run engine against fake GUIs
//! written in `sh` (deadlines, pipes held by a descendant, stderr tail, over-long lines, forced
//! and auto run lines, editor scenarios, profile variables). Spawning a real GUI needs a display
//! and the recorder of m4-editor-shell.md §6.8, and is covered by `cargo xtask bench` itself.
#![allow(clippy::unwrap_used)]

use std::{
    ffi::OsStr,
    io::{BufReader, Cursor},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use serde_json::json;
use xtask::bench::{
    BenchArgs, DEFAULT_IDLE_RUNS, DEFAULT_OPEN_RUNS, DEFAULT_SCROLL_RUNS, DEFAULT_STARTUP_RUNS,
    DEFAULT_TYPE_RUNS, InputKind, RendererChoice, SCROLL_EVENTS, Scenario, TYPE_EVENTS,
    describe_sample,
    generate::BenchInputs,
    milliseconds, osrelease_is_wsl,
    report::{BenchRun, EventSummary, Sample},
    runner::{
        BenchProfile, CappedLine, ENV_CONFIG_DIR, ENV_DATA_DIR, IDLE_TIMEOUT_EXTRA_SECS,
        MAX_LINE_BYTES, PROFILE_PREFIX, READER_JOIN_TIMEOUT, RunRequest, STARTUP_TIMEOUT_SECS,
        STDERR_TAIL_BYTES, StreamTail, bench_copy_path, build_args, gui_command, gui_executable,
        parse_run_line, read_capped_line, run_args, run_timeout, validate_run,
    },
    used_renderer,
};

/// Minimal parser wrapping [`BenchArgs`] the way `cargo xtask bench` does.
#[derive(Debug, Parser)]
struct Cli {
    /// The bench arguments.
    #[command(flatten)]
    bench: BenchArgs,
}

/// Parses `cargo xtask bench <args>`.
fn parse(args: &[&str]) -> Result<BenchArgs, clap::Error> {
    Cli::try_parse_from(std::iter::once("bench").chain(args.iter().copied())).map(|cli| cli.bench)
}

/// A run request without input.
fn request(scenario: Scenario, renderer: RendererChoice, duration_secs: Option<u64>) -> RunRequest {
    RunRequest {
        scenario,
        renderer,
        input: None,
        duration_secs,
    }
}

/// A run request with the input `input`.
fn request_with_input(
    scenario: Scenario,
    renderer: RendererChoice,
    input: &str,
    duration_secs: Option<u64>,
) -> RunRequest {
    RunRequest {
        input: Some(PathBuf::from(input)),
        ..request(scenario, renderer, duration_secs)
    }
}

/// Returns a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (§17.9).
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    dir
}

/// §19.5 replaced the single-renderer builds: the bench builds the shipped binary once, with
/// default features (both renderers).
#[test]
fn build_command_builds_the_shipped_binary_with_default_features() {
    let args = build_args();
    assert_eq!(
        args,
        [
            "build",
            "--release",
            "-p",
            "visual-assembler",
            "--message-format=json-render-diagnostics",
        ]
    );
    assert!(!args.iter().any(|arg| arg.contains("features")), "{args:?}");
}

#[test]
fn artifact_message_of_the_gui_binary_gives_its_executable() {
    let bin = json!({
        "reason": "compiler-artifact",
        "target": { "name": "visual-assembler", "kind": ["bin"] },
        "executable": "/t/release/visual-assembler",
    });
    assert_eq!(
        gui_executable(&bin),
        Some(PathBuf::from("/t/release/visual-assembler"))
    );
    let lib = json!({
        "reason": "compiler-artifact",
        "target": { "name": "visual_assembler", "kind": ["lib"] },
        "executable": null,
    });
    assert_eq!(gui_executable(&lib), None);
    let other = json!({
        "reason": "compiler-artifact",
        "target": { "name": "va", "kind": ["bin"] },
        "executable": "/t/release/va",
    });
    assert_eq!(gui_executable(&other), None);
    assert_eq!(gui_executable(&json!({ "reason": "build-finished" })), None);
}

/// §19.5: the measured copy is `<target_directory>/bench/shipped/visual-assembler[.exe]`.
#[test]
fn bench_copy_lives_under_the_target_directory() {
    let path = bench_copy_path(Path::new("target"));
    let expected = Path::new("target")
        .join("bench")
        .join("shipped")
        .join(format!("visual-assembler{}", std::env::consts::EXE_SUFFIX));
    assert_eq!(path, expected);
}

/// §19.5: every run passes `--renderer <r>`; the duration still goes to idle runs only.
#[test]
fn gui_arguments_pass_the_renderer_and_the_duration_to_idle_only() {
    assert_eq!(
        run_args(&request(Scenario::Startup, RendererChoice::Auto, None)),
        ["--renderer", "auto", "--bench", "startup"]
    );
    assert_eq!(
        run_args(&request(Scenario::Idle, RendererChoice::Glow, Some(3))),
        [
            "--renderer",
            "glow",
            "--bench",
            "idle",
            "--duration-secs",
            "3"
        ]
    );
    assert_eq!(
        run_args(&request(Scenario::Startup, RendererChoice::Wgpu, Some(3))),
        ["--renderer", "wgpu", "--bench", "startup"]
    );
}

/// §17.8 changed the deadlines: 75 s for startup, N + 75 s for idle.
#[test]
fn timeouts_follow_the_spec() {
    assert_eq!(
        run_timeout(Scenario::Startup, None),
        Duration::from_secs(75)
    );
    assert_eq!(
        run_timeout(Scenario::Idle, Some(10)),
        Duration::from_secs(85)
    );
}

/// m4 §7.3 moved the run line to `va.bench.run/3`; a `/2` or `/1` line (an older binary) no
/// longer counts as the run line.
#[test]
fn only_run_schema_lines_are_accepted() {
    assert!(parse_run_line(r#"{"schema":"va.bench.run/3","scenario":"startup"}"#).is_some());
    assert!(parse_run_line(r#"{"schema":"va.bench.run/2","scenario":"startup"}"#).is_none());
    assert!(parse_run_line(r#"{"schema":"va.bench.run/1","scenario":"startup"}"#).is_none());
    assert!(parse_run_line(r#"{"schema":"va.info/1"}"#).is_none());
    assert!(parse_run_line("hello").is_none());
    assert!(parse_run_line("").is_none());
}

/// §19.5 changed the renderer default from `wgpu` to `auto`, what ships. m4 §7.1 extended
/// `all` to the five scenarios, with their own default run counts.
#[test]
fn bench_defaults_follow_the_spec() {
    let args = parse(&[]).unwrap();
    assert_eq!(
        args.scenarios(),
        [
            Scenario::Startup,
            Scenario::Idle,
            Scenario::Open,
            Scenario::Scroll,
            Scenario::Type
        ]
    );
    assert_eq!(args.renderers(), [RendererChoice::Auto]);
    assert_eq!(args.runs_for(Scenario::Startup), 20);
    assert_eq!(args.runs_for(Scenario::Idle), 3);
    assert_eq!(args.runs_for(Scenario::Open), 5);
    assert_eq!(args.runs_for(Scenario::Scroll), 3);
    assert_eq!(args.runs_for(Scenario::Type), 3);
    assert_eq!(args.warmup, 1);
    assert_eq!(args.duration_for(Scenario::Idle), Some(10));
    assert_eq!(args.duration_for(Scenario::Startup), None);
    assert!(!args.json);
}

/// §19.5 changed `--renderer all`: glow, then wgpu, then auto.
#[test]
fn runs_override_both_scenarios() {
    let args = parse(&["idle", "--renderer", "all", "--runs", "5", "--warmup", "0"]).unwrap();
    assert_eq!(args.scenarios(), [Scenario::Idle]);
    assert_eq!(
        args.renderers(),
        [
            RendererChoice::Glow,
            RendererChoice::Wgpu,
            RendererChoice::Auto
        ]
    );
    for scenario in Scenario::ALL {
        assert_eq!(args.runs_for(scenario), 5, "{scenario}");
    }
    assert_eq!(args.warmup, 0);
}

/// §19.5: `--renderer auto|glow|wgpu|all`, each giving its choices in run order.
#[test]
fn renderer_values_parse() {
    for (value, expected) in [
        ("auto", &[RendererChoice::Auto][..]),
        ("glow", &[RendererChoice::Glow]),
        ("wgpu", &[RendererChoice::Wgpu]),
        (
            "all",
            &[
                RendererChoice::Glow,
                RendererChoice::Wgpu,
                RendererChoice::Auto,
            ],
        ),
    ] {
        let args = parse(&["startup", "--renderer", value]).unwrap();
        assert_eq!(args.renderers(), expected, "{value}");
    }
    for bad in ["Auto", "GLOW", "gl", "dx12", "both"] {
        let error = parse(&["--renderer", bad]).unwrap_err();
        assert_eq!(error.exit_code(), 2, "{bad}");
    }
}

#[test]
fn renderer_choices_have_their_cli_spelling_and_only_auto_is_unforced() {
    for (choice, text, forced) in [
        (RendererChoice::Auto, "auto", false),
        (RendererChoice::Glow, "glow", true),
        (RendererChoice::Wgpu, "wgpu", true),
    ] {
        assert_eq!(choice.as_str(), text);
        assert_eq!(choice.to_string(), text);
        assert_eq!(serde_json::to_value(choice).unwrap(), text);
        assert_eq!(choice.is_forced(), forced, "{text}");
    }
}

#[test]
fn invalid_bench_arguments_are_usage_errors() {
    for bad in [
        &["bogus"][..],
        &["--renderer", "vulkan"],
        &["--runs", "0"],
        &["--duration-secs", "0"],
        &["--duration-secs", "3601"],
    ] {
        let error = parse(bad).unwrap_err();
        assert_eq!(error.exit_code(), 2, "{bad:?}");
    }
}

#[test]
fn wsl_kernel_release_is_detected() {
    assert!(osrelease_is_wsl("6.6.87.2-microsoft-standard-WSL2\n"));
    assert!(osrelease_is_wsl("5.10.16.3-Microsoft"));
    assert!(osrelease_is_wsl("6.1.0-wsl"));
    assert!(!osrelease_is_wsl("6.8.0-45-generic"));
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (audit round 1): malformed cargo messages, unicode paths with spaces, timeout
// extremes, run-line detection corner cases, argument bounds.
// ---------------------------------------------------------------------------------------------

#[test]
fn artifact_messages_without_a_usable_executable_are_ignored() {
    for message in [
        json!({ "reason": "compiler-artifact", "target": { "name": "visual-assembler", "kind": ["bin"] }, "executable": null }),
        json!({ "reason": "compiler-artifact", "target": { "name": "visual-assembler", "kind": ["bin"] } }),
        json!({ "reason": "compiler-artifact", "target": { "name": "visual-assembler" }, "executable": "/t/x" }),
        json!({ "reason": "compiler-artifact", "target": { "name": "visual-assembler", "kind": "bin" }, "executable": "/t/x" }),
        json!({ "reason": "compiler-artifact", "executable": "/t/x" }),
        json!({ "reason": "compiler-artifact", "target": { "name": "visual_assembler", "kind": ["bin"] }, "executable": "/t/x" }),
        json!({ "reason": "compiler-message", "target": { "name": "visual-assembler", "kind": ["bin"] }, "executable": "/t/x" }),
        json!({ "reason": "compiler-artifact", "target": { "name": "visual-assembler", "kind": ["bin"] }, "executable": 42 }),
        json!(["compiler-artifact"]),
        json!(null),
    ] {
        assert_eq!(gui_executable(&message), None, "{message}");
    }
}

#[test]
fn artifact_paths_with_spaces_and_unicode_are_kept_verbatim() {
    for path in [
        r"C:\Users\José Q\Code Info\target\release\visual-assembler.exe",
        "/home/zoë/my projects/文件/target/release/visual-assembler",
    ] {
        let message = json!({
            "reason": "compiler-artifact",
            "target": { "name": "visual-assembler", "kind": ["bin"] },
            "executable": path,
        });
        assert_eq!(gui_executable(&message), Some(PathBuf::from(path)));
    }
}

#[test]
fn bench_copy_path_under_a_unicode_target_with_spaces() {
    let target = Path::new("tärget dir").join("文件 ✓");
    assert_eq!(
        bench_copy_path(&target),
        target
            .join("bench")
            .join("shipped")
            .join(format!("visual-assembler{}", std::env::consts::EXE_SUFFIX))
    );
}

/// §17.8 changed the deadlines: 75 s for startup, N + 75 s for idle. §19.5 added `--renderer`
/// to the GUI arguments.
#[test]
fn timeouts_cover_defaults_bounds_and_overflow() {
    assert_eq!(run_timeout(Scenario::Idle, None), Duration::from_secs(85));
    assert_eq!(
        run_timeout(Scenario::Idle, Some(1)),
        Duration::from_secs(76)
    );
    assert_eq!(
        run_timeout(Scenario::Idle, Some(3600)),
        Duration::from_secs(3675)
    );
    assert_eq!(
        run_timeout(Scenario::Idle, Some(u64::MAX)),
        Duration::from_secs(u64::MAX)
    );
    assert_eq!(
        run_timeout(Scenario::Startup, Some(3600)),
        Duration::from_secs(75)
    );
    assert_eq!(
        run_args(&request(Scenario::Idle, RendererChoice::Auto, None)),
        ["--renderer", "auto", "--bench", "idle"]
    );
}

/// §19.5 moved the accepted schema from `va.bench.run/1` to `va.bench.run/2`, and m4 §7.3 to
/// `va.bench.run/3`; the corner cases are the same, one version up, and `/2` is now rejected.
#[test]
fn run_line_detection_corner_cases() {
    assert!(parse_run_line("  {\"schema\":\"va.bench.run/3\"}\r\n").is_some());
    assert!(parse_run_line("\t{\"frames_painted\":0,\"schema\":\"va.bench.run/3\"} ").is_some());
    for line in [
        r#"{"schema":"va.bench.run/1"}"#,
        r#"{"schema":"va.bench.run/2"}"#,
        r#"{"schema":"va.bench.run/4"}"#,
        r#"{"schema":"VA.BENCH.RUN/3"}"#,
        r#"{"schema":1}"#,
        r#"{"schema":null}"#,
        r#"[{"schema":"va.bench.run/3"}]"#,
        r#"{"run":{"schema":"va.bench.run/3"}}"#,
        r#"{"schema":"va.bench.run/3"} {"schema":"va.bench.run/3"}"#,
        r#"{"schema":"va.bench.run/3""#,
        r#""va.bench.run/3""#,
        "null",
        "error: bench timed out",
    ] {
        assert!(parse_run_line(line).is_none(), "{line}");
    }
}

#[test]
fn more_invalid_bench_arguments_are_usage_errors() {
    for bad in [
        &["startup", "idle"][..],
        &["--runs", "-1"],
        &["--runs", "abc"],
        &["--runs", "4294967296"],
        &["--warmup", "-1"],
        &["--warmup", "x"],
        &["--duration-secs", ""],
        &["--duration-secs", "1.5"],
        &["--json", "--json"],
        &["--renderer", ""],
        &["--renderer", "wgpu", "--renderer", "glow"],
    ] {
        let error = parse(bad).unwrap_err();
        assert_eq!(error.exit_code(), 2, "{bad:?}");
    }
}

#[test]
fn bench_argument_bounds_are_inclusive() {
    let low = parse(&[
        "idle",
        "--runs",
        "1",
        "--duration-secs",
        "1",
        "--warmup",
        "0",
    ])
    .unwrap();
    assert_eq!(low.runs_for(Scenario::Idle), 1);
    assert_eq!(low.duration_for(Scenario::Idle), Some(1));
    assert_eq!(low.warmup, 0);
    let high = parse(&["--runs", "4294967295", "--duration-secs", "3600", "--json"]).unwrap();
    assert_eq!(high.runs_for(Scenario::Startup), u32::MAX);
    assert_eq!(high.duration_for(Scenario::Idle), Some(3600));
    assert!(high.json);
}

#[test]
fn wsl_detection_edge_cases() {
    assert!(!osrelease_is_wsl(""));
    assert!(!osrelease_is_wsl("\n"));
    assert!(osrelease_is_wsl("MICROSOFT"));
    assert!(osrelease_is_wsl("Wsl"));
    assert!(!osrelease_is_wsl("6.8.0-microsof"));
}

// ---------------------------------------------------------------------------------------------
// Fix wave (§17.8): deadlines above the GUI watchdogs, bounded output, and the run engine.
// ---------------------------------------------------------------------------------------------

/// The GUI's startup watchdog (§17.8).
const GUI_STARTUP_WATCHDOG_SECS: u64 = 60;

/// The GUI's idle watchdog beyond N: 60 s plus the 3 s settle (§17.8, §17.13).
const GUI_IDLE_WATCHDOG_EXTRA_SECS: u64 = 60 + 3;

#[test]
fn deadlines_are_strictly_larger_than_the_gui_watchdogs() {
    assert_eq!(STARTUP_TIMEOUT_SECS, 75);
    assert_eq!(IDLE_TIMEOUT_EXTRA_SECS, 75);
    assert!(run_timeout(Scenario::Startup, None).as_secs() > GUI_STARTUP_WATCHDOG_SECS);
    for seconds in [1, 10, 3600] {
        assert!(
            run_timeout(Scenario::Idle, Some(seconds)).as_secs()
                > seconds + GUI_IDLE_WATCHDOG_EXTRA_SECS,
            "N = {seconds}"
        );
    }
}

#[test]
fn output_bounds_follow_the_spec() {
    assert_eq!(READER_JOIN_TIMEOUT, Duration::from_secs(5));
    assert_eq!(STDERR_TAIL_BYTES, 64 * 1024);
    assert_eq!(MAX_LINE_BYTES, 1024 * 1024);
}

/// Reads every line of `input` with `cap`, through a 4-byte buffer so lines span refills.
fn capped_lines(input: &[u8], cap: usize) -> Vec<CappedLine> {
    let mut reader = BufReader::with_capacity(4, Cursor::new(input.to_vec()));
    let mut lines = Vec::new();
    while let Some(line) = read_capped_line(&mut reader, cap).unwrap() {
        lines.push(line);
    }
    lines
}

/// A kept line.
fn line(bytes: &[u8], truncated: bool) -> CappedLine {
    CappedLine {
        bytes: bytes.to_vec(),
        truncated,
    }
}

#[test]
fn capped_lines_keep_at_most_the_cap_and_flag_the_rest() {
    assert_eq!(
        capped_lines(b"short\nexactly8\nmuch longer line\r\n\nlast", 8),
        [
            line(b"short", false),
            line(b"exactly8", false),
            line(b"much lon", true),
            line(b"", false),
            line(b"last", false),
        ]
    );
    assert_eq!(capped_lines(b"", 8), Vec::<CappedLine>::new());
    assert_eq!(capped_lines(b"123456789", 8), [line(b"12345678", true)]);
}

#[test]
fn stream_tail_keeps_the_last_bytes_and_counts_the_dropped_ones() {
    let mut tail = StreamTail::new(8);
    assert!(tail.is_empty());
    tail.push(b"abc");
    assert_eq!(tail.text(), "abc");
    assert_eq!(tail.dropped(), 0);
    tail.push(b"defghij");
    assert_eq!(tail.len(), 8);
    assert_eq!(tail.dropped(), 2);
    assert_eq!(
        tail.text(),
        "[… 2 earlier bytes dropped, last 8 kept …]\ncdefghij"
    );
    tail.push(&[b'z'; 20]);
    assert_eq!(tail.len(), 8);
    assert_eq!(tail.dropped(), 22);
    assert!(tail.text().ends_with("zzzzzzzz"));
}

#[test]
fn progress_lines_show_n_a_for_idle_runs() {
    assert_eq!(milliseconds(Some(205.04)), "205.0 ms");
    assert_eq!(milliseconds(None), "n/a");
}

// ---------------------------------------------------------------------------------------------
// Renderer decision (§19.5): run-line validation against the requested renderer.
// ---------------------------------------------------------------------------------------------

/// A startup run line requested with `requested`, running `renderer`, after a fallback from
/// `fallback_from` when it is `Some`.
fn v3_run(requested: &str, renderer: &str, fallback_from: Option<&str>) -> BenchRun {
    let value = serde_json::json!({
        "schema": "va.bench.run/3",
        "scenario": "startup",
        "renderer_requested": requested,
        "renderer": renderer,
        "fallback_from": fallback_from,
        "fallback_reason": fallback_from.map(|_| "injected renderer failure (VA_TEST_FAIL_RENDERER)"),
        "injected_failure": fallback_from,
        "backend": if renderer == "glow" { "gl" } else { "dx12" },
        "adapter": "Test Adapter",
        "driver": "1.0",
        "window_system": "win32",
        "wsl": false,
        "os": "windows",
        "arch": "x86_64",
        "version": "0.1.0",
        "startup_ms": 150.0,
        "frames_painted": 1,
        "process_cpu_ms": 120.0,
        "peak_rss_bytes": 104857600,
        "peak_private_bytes": 52428800,
        "settle_ms": null,
        "idle_window_ms": null,
        "input": null,
        "input_lines": null,
        "vsync": true,
        "open_ms": null,
        "event_count": null,
        "event_ms": null,
        "event_summary_ms": null,
    });
    serde_json::from_value(value).unwrap()
}

#[test]
fn forced_runs_must_use_the_forced_renderer_without_fallback() {
    for (choice, name) in [
        (RendererChoice::Glow, "glow"),
        (RendererChoice::Wgpu, "wgpu"),
    ] {
        assert_eq!(
            validate_run(
                &v3_run(name, name, None),
                &request(Scenario::Startup, choice, None)
            ),
            Ok(())
        );
    }
    let error = validate_run(
        &v3_run("glow", "wgpu", None),
        &request(Scenario::Startup, RendererChoice::Glow, None),
    )
    .unwrap_err();
    assert_eq!(
        error,
        "reported renderer `wgpu`, expected `glow` (forced by --renderer)"
    );
    let error = validate_run(
        &v3_run("wgpu", "wgpu", Some("glow")),
        &request(Scenario::Startup, RendererChoice::Wgpu, None),
    )
    .unwrap_err();
    assert_eq!(
        error,
        "fell back from `glow` to `wgpu` although `--renderer wgpu` forbids a fallback"
    );
    // A fallback is rejected even when it lands on the forced renderer's name.
    assert!(
        validate_run(
            &v3_run("glow", "glow", Some("wgpu")),
            &request(Scenario::Startup, RendererChoice::Glow, None)
        )
        .is_err()
    );
}

#[test]
fn auto_runs_accept_any_renderer_and_a_fallback() {
    for (renderer, fallback_from) in [
        ("glow", None),
        ("wgpu", None),
        ("wgpu", Some("glow")),
        ("glow", Some("wgpu")),
    ] {
        assert_eq!(
            validate_run(
                &v3_run("auto", renderer, fallback_from),
                &request(Scenario::Startup, RendererChoice::Auto, None)
            ),
            Ok(()),
            "{renderer} {fallback_from:?}"
        );
    }
}

#[test]
fn renderer_requested_must_echo_the_flag() {
    for (choice, line_requested) in [
        (RendererChoice::Auto, "glow"),
        (RendererChoice::Glow, "auto"),
        (RendererChoice::Wgpu, "glow"),
        (RendererChoice::Auto, "AUTO"),
        (RendererChoice::Auto, ""),
    ] {
        let error = validate_run(
            &v3_run(line_requested, "glow", None),
            &request(Scenario::Startup, choice, None),
        )
        .unwrap_err();
        assert_eq!(
            error,
            format!(
                "reported renderer_requested `{line_requested}`, expected `{}`",
                choice.as_str()
            )
        );
    }
}

#[test]
fn a_run_line_for_another_scenario_is_rejected_first() {
    let error = validate_run(
        &v3_run("glow", "wgpu", Some("glow")),
        &request(Scenario::Idle, RendererChoice::Glow, None),
    )
    .unwrap_err();
    assert_eq!(error, "reported scenario `startup`, expected `idle`");
}

#[test]
fn progress_lines_name_the_used_renderer_and_its_fallback() {
    assert_eq!(
        used_renderer(&v3_run("auto", "glow", None)),
        "renderer glow"
    );
    assert_eq!(
        used_renderer(&v3_run("auto", "wgpu", Some("glow"))),
        "renderer wgpu (fallback from glow)"
    );
}

// ---------------------------------------------------------------------------------------------
// Editor scenarios (m4-editor-shell.md §7.1, §7.2): scenarios, default runs, inputs, arguments,
// the isolated profile and the v3 validation rules.
// ---------------------------------------------------------------------------------------------

/// The instant `seconds` after the Unix epoch.
fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

/// Generated inputs under `/g`.
fn inputs() -> BenchInputs {
    BenchInputs {
        large: PathBuf::from("/g").join("100k.asm"),
        project: PathBuf::from("/g").join("5k"),
    }
}

/// A run line of `scenario` with auto on `input`, as the v3 GUI prints it: open time and no
/// vsync for the editor scenarios, `events` samples of 4.0 ms.
fn editor_run(scenario: Scenario, input: Option<&str>, events: Option<u64>) -> BenchRun {
    let editor = scenario.measures_open();
    BenchRun {
        scenario: scenario.as_str().to_owned(),
        input: input.map(str::to_owned),
        input_lines: input.map(|_| 100_000),
        vsync: !editor,
        open_ms: editor.then_some(61.2),
        event_count: events,
        event_ms: events.map(|count| vec![4.0; usize::try_from(count).unwrap()]),
        event_summary_ms: events.map(|_| EventSummary {
            median: 4.0,
            p95: 4.0,
            max: 4.0,
        }),
        ..v3_run("auto", "glow", None)
    }
}

/// Validates `run` against an auto request of `scenario` on `input`.
fn validate_editor(run: &BenchRun, scenario: Scenario, input: Option<&str>) -> Result<(), String> {
    let request = match input {
        Some(input) => request_with_input(scenario, RendererChoice::Auto, input, None),
        None => request(scenario, RendererChoice::Auto, None),
    };
    validate_run(run, &request)
}

#[test]
fn scenario_values_parse_and_all_runs_the_five_in_order() {
    for (value, expected) in [
        ("startup", Scenario::Startup),
        ("idle", Scenario::Idle),
        ("open", Scenario::Open),
        ("scroll", Scenario::Scroll),
        ("type", Scenario::Type),
    ] {
        assert_eq!(parse(&[value]).unwrap().scenarios(), [expected], "{value}");
        assert_eq!(expected.as_str(), value);
        assert_eq!(expected.to_string(), value);
        assert_eq!(serde_json::to_value(expected).unwrap(), value);
    }
    assert_eq!(parse(&["all"]).unwrap().scenarios(), Scenario::ALL);
    assert_eq!(
        Scenario::ALL,
        [
            Scenario::Startup,
            Scenario::Idle,
            Scenario::Open,
            Scenario::Scroll,
            Scenario::Type
        ]
    );
    for bad in ["Open", "TYPE", "typing", "large"] {
        assert_eq!(parse(&[bad]).unwrap_err().exit_code(), 2, "{bad}");
    }
}

#[test]
fn default_runs_follow_the_spec_and_runs_overrides_them_all() {
    assert_eq!(
        [
            DEFAULT_STARTUP_RUNS,
            DEFAULT_IDLE_RUNS,
            DEFAULT_OPEN_RUNS,
            DEFAULT_SCROLL_RUNS,
            DEFAULT_TYPE_RUNS
        ],
        [20, 3, 5, 3, 3]
    );
    let defaults = parse(&["all"]).unwrap();
    let overridden = parse(&["all", "--runs", "2"]).unwrap();
    for (scenario, runs) in Scenario::ALL.into_iter().zip([20, 3, 5, 3, 3]) {
        assert_eq!(scenario.default_runs(), runs, "{scenario}");
        assert_eq!(defaults.runs_for(scenario), runs, "{scenario}");
        assert_eq!(overridden.runs_for(scenario), 2, "{scenario}");
    }
}

#[test]
fn scenario_properties_follow_the_spec() {
    assert_eq!(SCROLL_EVENTS, 300);
    assert_eq!(TYPE_EVENTS, 500);
    for (scenario, input, measures_open, events, first_frame) in [
        (Scenario::Startup, None, false, None, true),
        (Scenario::Idle, Some(InputKind::Project), false, None, false),
        (Scenario::Open, Some(InputKind::Large), true, None, false),
        (
            Scenario::Scroll,
            Some(InputKind::Large),
            true,
            Some(300),
            false,
        ),
        (
            Scenario::Type,
            Some(InputKind::Large),
            true,
            Some(500),
            false,
        ),
    ] {
        assert_eq!(scenario.input(), input, "{scenario}");
        assert_eq!(scenario.measures_open(), measures_open, "{scenario}");
        assert_eq!(scenario.expected_events(), events, "{scenario}");
        assert_eq!(scenario.reports_at_first_frame(), first_frame, "{scenario}");
    }
    let inputs = inputs();
    assert_eq!(InputKind::Large.path(&inputs), inputs.large);
    assert_eq!(InputKind::Project.path(&inputs), inputs.project);
}

#[test]
fn only_scenarios_that_open_an_input_need_the_generated_inputs() {
    assert!(!parse(&["startup"]).unwrap().needs_inputs());
    for value in ["idle", "open", "scroll", "type", "all"] {
        assert!(parse(&[value]).unwrap().needs_inputs(), "{value}");
    }
}

#[test]
fn requests_take_the_input_and_the_duration_of_their_scenario() {
    let args = parse(&["all", "--duration-secs", "4"]).unwrap();
    let inputs = inputs();
    let startup = args.request(Scenario::Startup, RendererChoice::Glow, Some(&inputs));
    assert_eq!(
        startup,
        request(Scenario::Startup, RendererChoice::Glow, None)
    );
    assert_eq!(
        args.request(Scenario::Idle, RendererChoice::Auto, Some(&inputs)),
        RunRequest {
            input: Some(inputs.project.clone()),
            ..request(Scenario::Idle, RendererChoice::Auto, Some(4))
        }
    );
    for scenario in [Scenario::Open, Scenario::Scroll, Scenario::Type] {
        assert_eq!(
            args.request(scenario, RendererChoice::Wgpu, Some(&inputs)),
            RunRequest {
                input: Some(inputs.large.clone()),
                ..request(scenario, RendererChoice::Wgpu, None)
            }
        );
    }
    // Without prepared inputs (only startup was selected), no request carries one.
    assert_eq!(
        args.request(Scenario::Open, RendererChoice::Auto, None)
            .input,
        None
    );
}

/// m4 §7.1: `--renderer <r> --bench <scenario> [--bench-input <path>] [--duration-secs N]`.
#[test]
fn gui_arguments_per_scenario_with_the_bench_input() {
    let inputs = inputs();
    let args = parse(&["all"]).unwrap();
    let large = inputs.large.as_os_str();
    let project = inputs.project.as_os_str();
    let expected: [(Scenario, Vec<&OsStr>); 5] = [
        (
            Scenario::Startup,
            vec!["--renderer", "auto", "--bench", "startup"]
                .into_iter()
                .map(OsStr::new)
                .collect(),
        ),
        (
            Scenario::Idle,
            ["--renderer", "auto", "--bench", "idle", "--bench-input"]
                .into_iter()
                .map(OsStr::new)
                .chain([project, OsStr::new("--duration-secs"), OsStr::new("10")])
                .collect(),
        ),
        (
            Scenario::Open,
            ["--renderer", "auto", "--bench", "open", "--bench-input"]
                .into_iter()
                .map(OsStr::new)
                .chain([large])
                .collect(),
        ),
        (
            Scenario::Scroll,
            ["--renderer", "auto", "--bench", "scroll", "--bench-input"]
                .into_iter()
                .map(OsStr::new)
                .chain([large])
                .collect(),
        ),
        (
            Scenario::Type,
            ["--renderer", "auto", "--bench", "type", "--bench-input"]
                .into_iter()
                .map(OsStr::new)
                .chain([large])
                .collect(),
        ),
    ];
    for (scenario, expected) in expected {
        let request = args.request(scenario, RendererChoice::Auto, Some(&inputs));
        assert_eq!(run_args(&request), expected, "{scenario}");
    }
}

/// m4 §6.8: open, scroll and type use the GUI's 60 s startup watchdog, so their deadline is the
/// startup one (75 s), whatever the idle duration.
#[test]
fn editor_scenarios_use_the_startup_deadline() {
    for scenario in [Scenario::Open, Scenario::Scroll, Scenario::Type] {
        for duration in [None, Some(1), Some(3600)] {
            assert_eq!(
                run_timeout(scenario, duration),
                Duration::from_secs(STARTUP_TIMEOUT_SECS),
                "{scenario} {duration:?}"
            );
        }
    }
}

#[test]
fn the_input_name_is_the_last_path_component() {
    for (input, name) in [
        ("/g/bench/generated/100k.asm", "100k.asm"),
        ("/g/bench/generated/5k", "5k"),
        ("/g/bench/generated/5k/", "5k"),
        ("100k.asm", "100k.asm"),
        ("C:/Users/José Q/bench/generated/5k", "5k"),
    ] {
        let request = request_with_input(Scenario::Open, RendererChoice::Auto, input, None);
        assert_eq!(request.input_name().as_deref(), Some(name), "{input}");
    }
    assert_eq!(
        request(Scenario::Startup, RendererChoice::Auto, None).input_name(),
        None
    );
}

/// m4 §7.1: the profile is `<target>/bench/profile-<utc>-<pid>/{config,data}`.
#[test]
fn the_profile_lives_under_the_target_bench_folder() {
    let target = Path::new("tärget dir");
    let root = BenchProfile::path_for(target, at(1_700_000_000), 4242);
    assert_eq!(
        root,
        target.join("bench").join("profile-20231114T221320Z-4242")
    );
    assert_eq!(PROFILE_PREFIX, "profile-");
    let profile = BenchProfile::at(root.clone());
    assert_eq!(profile.root, root);
    assert_eq!(profile.config_dir, root.join("config"));
    assert_eq!(profile.data_dir, root.join("data"));
    assert_eq!(ENV_CONFIG_DIR, "VA_CONFIG_DIR");
    assert_eq!(ENV_DATA_DIR, "VA_DATA_DIR");
    assert_eq!(
        profile.env(),
        [
            ("VA_CONFIG_DIR", profile.config_dir.as_path()),
            ("VA_DATA_DIR", profile.data_dir.as_path()),
        ]
    );
}

/// m4 §7.1: created once per command with `fs::create_dir`, so an existing profile is an error.
#[test]
fn the_profile_is_created_once_and_never_reused() {
    let target = temp_dir("bench-profile");
    let profile = BenchProfile::create(&target, at(1_700_000_000), 7).unwrap();
    assert!(profile.root.starts_with(target.join("bench")));
    assert!(profile.config_dir.is_dir());
    assert!(profile.data_dir.is_dir());
    let error = format!(
        "{:#}",
        BenchProfile::create(&target, at(1_700_000_000), 7).unwrap_err()
    );
    assert!(error.contains("cannot create the bench profile"), "{error}");
    let other = BenchProfile::create(&target, at(1_700_000_000), 8).unwrap();
    assert_ne!(other.root, profile.root);
    std::fs::remove_dir_all(&target).unwrap();
}

#[test]
fn the_gui_command_passes_the_arguments_and_only_the_profile_variables() {
    let profile = BenchProfile::at(PathBuf::from("/t/bench/profile-x-1"));
    let request = request_with_input(Scenario::Type, RendererChoice::Wgpu, "/g/100k.asm", None);
    let cmd = gui_command(
        Path::new("/t/bench/shipped/visual-assembler"),
        &request,
        &profile,
    );
    assert_eq!(
        cmd.get_program(),
        OsStr::new("/t/bench/shipped/visual-assembler")
    );
    assert_eq!(cmd.get_args().collect::<Vec<_>>(), run_args(&request));
    let envs: Vec<(&OsStr, Option<&OsStr>)> = cmd.get_envs().collect();
    assert_eq!(
        envs,
        [
            (
                OsStr::new("VA_CONFIG_DIR"),
                Some(profile.config_dir.as_os_str())
            ),
            (
                OsStr::new("VA_DATA_DIR"),
                Some(profile.data_dir.as_os_str())
            ),
        ]
    );
}

#[test]
fn editor_run_lines_that_answer_their_request_are_valid() {
    assert_eq!(
        validate_editor(
            &editor_run(Scenario::Idle, Some("5k"), None),
            Scenario::Idle,
            Some("/g/5k")
        ),
        Ok(())
    );
    assert_eq!(
        validate_editor(
            &editor_run(Scenario::Open, Some("100k.asm"), None),
            Scenario::Open,
            Some("/g/100k.asm")
        ),
        Ok(())
    );
    for (scenario, events) in [(Scenario::Scroll, 300), (Scenario::Type, 500)] {
        assert_eq!(
            validate_editor(
                &editor_run(scenario, Some("100k.asm"), Some(events)),
                scenario,
                Some("/g/100k.asm")
            ),
            Ok(()),
            "{scenario}"
        );
    }
    // Startup and idle may report vsync; startup has no input.
    assert_eq!(
        validate_editor(
            &editor_run(Scenario::Startup, None, None),
            Scenario::Startup,
            None
        ),
        Ok(())
    );
}

/// m4 §7.2: `input` must equal the requested input's name, and be null without one.
#[test]
fn the_run_line_must_name_the_requested_input() {
    for (reported, requested, message) in [
        (
            None,
            Some("/g/100k.asm"),
            "reported input null, expected `100k.asm`",
        ),
        (
            Some("5k"),
            Some("/g/100k.asm"),
            "reported input `5k`, expected `100k.asm`",
        ),
        (
            Some("100K.ASM"),
            Some("/g/100k.asm"),
            "reported input `100K.ASM`, expected `100k.asm`",
        ),
    ] {
        let run = editor_run(Scenario::Open, reported, None);
        assert_eq!(
            validate_editor(&run, Scenario::Open, requested),
            Err(message.to_owned())
        );
    }
    let startup = BenchRun {
        input: Some("100k.asm".to_owned()),
        ..editor_run(Scenario::Startup, None, None)
    };
    assert_eq!(
        validate_editor(&startup, Scenario::Startup, None),
        Err("reported input `100k.asm`, expected null".to_owned())
    );
}

/// m4 §7.2: open, scroll and type need a non-null `open_ms` and `vsync == false`.
#[test]
fn editor_scenarios_need_an_open_time_and_no_vsync() {
    for (scenario, events) in [
        (Scenario::Open, None),
        (Scenario::Scroll, Some(300)),
        (Scenario::Type, Some(500)),
    ] {
        let no_open = BenchRun {
            open_ms: None,
            ..editor_run(scenario, Some("100k.asm"), events)
        };
        assert_eq!(
            validate_editor(&no_open, scenario, Some("/g/100k.asm")),
            Err(format!(
                "reported open_ms null, expected a number for the `{scenario}` scenario"
            ))
        );
        let vsync = BenchRun {
            vsync: true,
            ..editor_run(scenario, Some("100k.asm"), events)
        };
        assert_eq!(
            validate_editor(&vsync, scenario, Some("/g/100k.asm")),
            Err(format!(
                "reported vsync true, expected false for the `{scenario}` scenario"
            ))
        );
    }
}

/// m4 §7.2: scroll and type need `event_count` equal to 300 and 500, and as many samples.
#[test]
fn event_scenarios_need_every_event_sample() {
    for (scenario, expected) in [(Scenario::Scroll, 300), (Scenario::Type, 500)] {
        for count in [
            None,
            Some(0),
            Some(expected - 1),
            Some(expected + 1),
            Some(25),
        ] {
            let run = editor_run(scenario, Some("100k.asm"), count);
            let count_text = count.map_or_else(|| "null".to_owned(), |n| n.to_string());
            assert_eq!(
                validate_editor(&run, scenario, Some("/g/100k.asm")),
                Err(format!(
                    "reported event_count {count_text}, expected {expected} for the `{scenario}` scenario"
                )),
                "{scenario} {count:?}"
            );
        }
        let short = BenchRun {
            event_ms: Some(vec![4.0; usize::try_from(expected).unwrap() - 1]),
            ..editor_run(scenario, Some("100k.asm"), Some(expected))
        };
        assert_eq!(
            validate_editor(&short, scenario, Some("/g/100k.asm")),
            Err(format!(
                "reported {} event_ms samples, expected {expected}",
                expected - 1
            ))
        );
        let missing = BenchRun {
            event_ms: None,
            ..editor_run(scenario, Some("100k.asm"), Some(expected))
        };
        assert_eq!(
            validate_editor(&missing, scenario, Some("/g/100k.asm")),
            Err(format!(
                "reported null event_ms samples, expected {expected}"
            ))
        );
    }
    // A type count on a scroll run is wrong, and open runs need no event at all.
    assert!(
        validate_editor(
            &editor_run(Scenario::Scroll, Some("100k.asm"), Some(500)),
            Scenario::Scroll,
            Some("/g/100k.asm")
        )
        .is_err()
    );
    assert_eq!(
        validate_editor(
            &editor_run(Scenario::Open, Some("100k.asm"), Some(500)),
            Scenario::Open,
            Some("/g/100k.asm")
        ),
        Ok(())
    );
}

/// The renderer rules of §19.5 still come before the v3 rules.
#[test]
fn renderer_rules_apply_to_editor_scenarios_too() {
    let run = BenchRun {
        renderer_requested: "glow".to_owned(),
        renderer: "wgpu".to_owned(),
        fallback_from: Some("glow".to_owned()),
        open_ms: None,
        ..editor_run(Scenario::Type, Some("100k.asm"), Some(500))
    };
    let request = request_with_input(Scenario::Type, RendererChoice::Glow, "/g/100k.asm", None);
    assert_eq!(
        validate_run(&run, &request),
        Err(
            "fell back from `glow` to `wgpu` although `--renderer glow` forbids a fallback"
                .to_owned()
        )
    );
}

#[test]
fn progress_lines_describe_each_scenario() {
    let startup = Sample {
        spawn_to_first_frame_ms: Some(205.04),
        run: v3_run("auto", "glow", None),
    };
    assert_eq!(
        describe_sample(&startup),
        "spawn→frame 205.0 ms, frames 1, renderer glow"
    );
    let open = Sample {
        spawn_to_first_frame_ms: None,
        run: editor_run(Scenario::Open, Some("100k.asm"), None),
    };
    assert_eq!(
        describe_sample(&open),
        "spawn→frame n/a, open 61.2 ms, frames 1, renderer glow"
    );
    let mut typed = editor_run(Scenario::Type, Some("100k.asm"), Some(500));
    typed.frames_painted = 503;
    typed.event_summary_ms = Some(EventSummary {
        median: 4.0,
        p95: 6.34,
        max: 11.8,
    });
    typed.renderer = "wgpu".to_owned();
    typed.fallback_from = Some("glow".to_owned());
    let typed = Sample {
        spawn_to_first_frame_ms: None,
        run: typed,
    };
    assert_eq!(
        describe_sample(&typed),
        "spawn→frame n/a, open 61.2 ms, event p50/p95/max 4.0/6.3/11.8 ms, frames 503, renderer wgpu (fallback from glow)"
    );
}

/// The run engine against fake GUIs written in POSIX `sh` (Linux CI and WSL).
#[cfg(unix)]
mod fake_gui {
    use std::{
        path::PathBuf,
        process::Command,
        time::{Duration, Instant},
    };

    use xtask::bench::{
        RendererChoice, Scenario,
        runner::{
            BenchProfile, PIPES_OPEN_MESSAGE, READER_JOIN_TIMEOUT, RunRequest, STDERR_TAIL_BYTES,
            run_command,
        },
    };

    use super::temp_dir;

    /// The seven `va.bench.run/3` keys of a run without input (startup), as a JSON fragment.
    const NO_EDITOR_KEYS: &str = r#""input":null,"input_lines":null,"vsync":true,"open_ms":null,"event_count":null,"event_ms":null,"event_summary_ms":null"#;

    /// A complete `va.bench.run/3` line for `scenario`, requested with `requested`, running
    /// `renderer` after a fallback from `fallback_from` (a JSON value: `null` or `"glow"`), with
    /// `editor` as its last seven keys.
    fn run_line_with(
        scenario: &str,
        requested: &str,
        renderer: &str,
        fallback_from: &str,
        editor: &str,
    ) -> String {
        format!(
            r#"{{"schema":"va.bench.run/3","scenario":"{scenario}","renderer_requested":"{requested}","renderer":"{renderer}","fallback_from":{fallback_from},"fallback_reason":null,"injected_failure":null,"backend":"gl","adapter":null,"driver":null,"window_system":"x11","wsl":false,"os":"linux","arch":"x86_64","version":"0.1.0","startup_ms":12.5,"frames_painted":1,"process_cpu_ms":null,"peak_rss_bytes":null,"peak_private_bytes":null,"settle_ms":null,"idle_window_ms":null,{editor}}}"#
        )
    }

    /// A complete `va.bench.run/3` line without input, with a fallback from `fallback_from`.
    fn run_line_v3(scenario: &str, requested: &str, renderer: &str, fallback_from: &str) -> String {
        run_line_with(scenario, requested, renderer, fallback_from, NO_EDITOR_KEYS)
    }

    /// A complete `va.bench.run/3` line for `scenario`, forced to `renderer` (no fallback).
    fn run_line(scenario: &str, renderer: &str) -> String {
        run_line_v3(scenario, renderer, renderer, "null")
    }

    /// The last seven keys of an editor run line on `input` with `events` samples of 4.0 ms
    /// (none for `open`).
    fn editor_keys(input: &str, events: Option<usize>) -> String {
        let (count, samples, summary) = match events {
            Some(count) => (
                count.to_string(),
                format!("[{}]", vec!["4.0"; count].join(",")),
                r#"{"median":4.0,"p95":4.0,"max":4.0}"#.to_owned(),
            ),
            None => ("null".to_owned(), "null".to_owned(), "null".to_owned()),
        };
        format!(
            r#""input":"{input}","input_lines":100000,"vsync":false,"open_ms":61.2,"event_count":{count},"event_ms":{samples},"event_summary_ms":{summary}"#
        )
    }

    /// Runs the `sh` script `body` as a measured run requested with wgpu, with `timeout`;
    /// returns the outcome (the error rendered with its context) and the elapsed time.
    fn run_script(
        tag: &str,
        body: &str,
        scenario: Scenario,
        timeout: Duration,
    ) -> (Result<xtask::bench::report::Sample, String>, Duration) {
        run_script_as(tag, body, scenario, RendererChoice::Wgpu, timeout)
    }

    /// Runs the `sh` script `body` as a measured run requested with `renderer`, with `timeout`.
    fn run_script_as(
        tag: &str,
        body: &str,
        scenario: Scenario,
        renderer: RendererChoice,
        timeout: Duration,
    ) -> (Result<xtask::bench::report::Sample, String>, Duration) {
        let dir = temp_dir(tag);
        let script = dir.join("fake-gui.sh");
        std::fs::write(&script, body).unwrap();
        let mut cmd = Command::new("sh");
        cmd.arg(&script);
        let request = RunRequest {
            scenario,
            renderer,
            input: None,
            duration_secs: None,
        };
        let started = Instant::now();
        let outcome = run_command(cmd, &request, timeout).map_err(|error| format!("{error:#}"));
        let elapsed = started.elapsed();
        std::fs::remove_dir_all(&dir).unwrap();
        (outcome, elapsed)
    }

    /// Runs the `sh` script `body` as the measured run `request`, with the profile variables of
    /// a fresh profile; returns the outcome and the profile.
    fn run_script_for(
        tag: &str,
        body: &str,
        request: &RunRequest,
    ) -> (Result<xtask::bench::report::Sample, String>, BenchProfile) {
        let dir = temp_dir(tag);
        let script = dir.join("fake-gui.sh");
        std::fs::write(&script, body).unwrap();
        let profile = BenchProfile::create(&dir, std::time::SystemTime::now(), 1).unwrap();
        let mut cmd = Command::new("sh");
        cmd.arg(&script);
        profile.apply(&mut cmd);
        let outcome = run_command(cmd, request, Duration::from_secs(30))
            .map_err(|error| format!("{error:#}"));
        std::fs::remove_dir_all(&dir).unwrap();
        (outcome, profile)
    }

    /// A request of `scenario` with auto and the input `input`.
    fn editor_request(scenario: Scenario, input: &str) -> RunRequest {
        RunRequest {
            scenario,
            renderer: RendererChoice::Auto,
            input: Some(PathBuf::from("/bench/generated").join(input)),
            duration_secs: None,
        }
    }

    #[test]
    fn startup_run_gives_a_numeric_spawn_time() {
        let body = format!(
            "printf '%s\\n' 'starting'\nprintf '%s\\n' '{}'\nexit 0\n",
            run_line("startup", "wgpu")
        );
        let (outcome, _) = run_script(
            "fake-startup",
            &body,
            Scenario::Startup,
            Duration::from_secs(30),
        );
        let sample = outcome.unwrap();
        let ms = sample.spawn_to_first_frame_ms.unwrap();
        assert!(ms > 0.0 && ms < 30_000.0, "{ms}");
        assert_eq!(sample.run.scenario, "startup");
    }

    #[test]
    fn idle_run_gives_no_spawn_time() {
        let body = format!("printf '%s\\n' '{}'\nexit 0\n", run_line("idle", "wgpu"));
        let (outcome, _) = run_script("fake-idle", &body, Scenario::Idle, Duration::from_secs(30));
        let sample = outcome.unwrap();
        assert_eq!(sample.spawn_to_first_frame_ms, None);
        assert_eq!(sample.run.startup_ms, 12.5);
    }

    #[test]
    fn pipes_held_by_a_descendant_fail_the_run_after_the_join_timeout() {
        let body = format!(
            "printf '%s\\n' '{}'\nsleep 20 &\nexit 0\n",
            run_line("startup", "wgpu")
        );
        let (outcome, elapsed) = run_script(
            "fake-descendant",
            &body,
            Scenario::Startup,
            Duration::from_secs(60),
        );
        let error = outcome.unwrap_err();
        assert!(error.contains(PIPES_OPEN_MESSAGE), "{error}");
        assert!(
            elapsed >= READER_JOIN_TIMEOUT && elapsed < READER_JOIN_TIMEOUT * 3,
            "{elapsed:?}"
        );
    }

    #[test]
    fn deadline_kills_the_child_and_fails_the_run() {
        let (outcome, elapsed) = run_script(
            "fake-hang",
            "printf 'still starting\\n' >&2\nexec sleep 30\n",
            Scenario::Startup,
            Duration::from_secs(1),
        );
        let error = outcome.unwrap_err();
        assert!(
            error.contains("timed out after 1 s and was killed"),
            "{error}"
        );
        assert!(error.contains("still starting"), "{error}");
        assert!(elapsed < Duration::from_secs(10), "{elapsed:?}");
    }

    #[test]
    fn stderr_is_capped_to_its_last_64_kib() {
        let body = "printf 'FIRST-MARKER\\n' >&2\nhead -c 200000 /dev/zero | tr '\\000' 'x' >&2\nprintf '\\nLAST-MARKER\\n' >&2\nexit 3\n";
        let (outcome, _) = run_script(
            "fake-stderr",
            body,
            Scenario::Startup,
            Duration::from_secs(30),
        );
        let error = outcome.unwrap_err();
        assert!(
            error.contains("exited with"),
            "{}",
            error.chars().take(200).collect::<String>()
        );
        assert!(error.contains("LAST-MARKER"));
        assert!(!error.contains("FIRST-MARKER"));
        assert!(error.contains("earlier bytes dropped"));
        assert!(error.len() < STDERR_TAIL_BYTES + 2048, "{}", error.len());
    }

    #[test]
    fn an_over_long_stdout_line_never_counts_as_the_run_line() {
        let padded = |spaces: u32| {
            format!(
                "printf '%s' '{}'\nhead -c {spaces} /dev/zero | tr '\\000' ' '\nprintf '\\n'\nexit 0\n",
                run_line("startup", "wgpu")
            )
        };
        let (short, _) = run_script(
            "fake-padded",
            &padded(1000),
            Scenario::Startup,
            Duration::from_secs(30),
        );
        assert!(short.is_ok(), "{short:?}");
        let (long, _) = run_script(
            "fake-over-long",
            &padded(1_100_000),
            Scenario::Startup,
            Duration::from_secs(30),
        );
        let error = long.unwrap_err();
        assert!(error.contains("printed no va.bench.run/3 line"), "{error}");
    }

    /// §19.5: a forced run whose line names another requested renderer fails.
    #[test]
    fn a_run_line_for_another_renderer_fails_the_run() {
        let body = format!("printf '%s\\n' '{}'\nexit 0\n", run_line("startup", "glow"));
        let (outcome, _) = run_script(
            "fake-renderer",
            &body,
            Scenario::Startup,
            Duration::from_secs(30),
        );
        let error = outcome.unwrap_err();
        assert!(
            error.contains("reported renderer_requested `glow`, expected `wgpu`"),
            "{error}"
        );
    }

    /// §19.5: a forced run that fell back fails, with the child's stderr attached.
    #[test]
    fn a_forced_run_that_fell_back_fails_the_run() {
        let body = format!(
            "printf 'warning: the wgpu renderer failed\\n' >&2\nprintf '%s\\n' '{}'\nexit 0\n",
            run_line_v3("startup", "wgpu", "glow", r#""wgpu""#)
        );
        let (outcome, _) = run_script(
            "fake-forced-fallback",
            &body,
            Scenario::Startup,
            Duration::from_secs(30),
        );
        let error = outcome.unwrap_err();
        assert!(
            error.contains("fell back from `wgpu` to `glow` although `--renderer wgpu`"),
            "{error}"
        );
        assert!(
            error.contains("warning: the wgpu renderer failed"),
            "{error}"
        );
    }

    /// §19.5: under auto, a fallback run is a valid sample.
    #[test]
    fn an_auto_run_that_fell_back_is_a_valid_sample() {
        let body = format!(
            "printf '%s\\n' '{}'\nexit 0\n",
            run_line_v3("startup", "auto", "wgpu", r#""glow""#)
        );
        let (outcome, _) = run_script_as(
            "fake-auto-fallback",
            &body,
            Scenario::Startup,
            RendererChoice::Auto,
            Duration::from_secs(30),
        );
        let sample = outcome.unwrap();
        assert_eq!(sample.run.renderer, "wgpu");
        assert_eq!(sample.run.fallback_from.as_deref(), Some("glow"));
        assert!(sample.spawn_to_first_frame_ms.is_some());
    }

    /// m4 §7.3: a binary printing `va.bench.run/2` (M0) gives no run line.
    #[test]
    fn a_v2_run_line_is_not_a_run_line() {
        let v2 = r#"{"schema":"va.bench.run/2","scenario":"startup","renderer_requested":"auto","renderer":"glow","fallback_from":null,"fallback_reason":null,"injected_failure":null,"backend":"gl","adapter":null,"driver":null,"window_system":"x11","wsl":false,"os":"linux","arch":"x86_64","version":"0.1.0","startup_ms":12.5,"frames_painted":1,"process_cpu_ms":null,"peak_rss_bytes":null,"peak_private_bytes":null,"settle_ms":null,"idle_window_ms":null}"#;
        let body = format!("printf '%s\\n' '{v2}'\nexit 0\n");
        let (outcome, _) = run_script_as(
            "fake-v2",
            &body,
            Scenario::Startup,
            RendererChoice::Auto,
            Duration::from_secs(30),
        );
        let error = outcome.unwrap_err();
        assert!(error.contains("printed no va.bench.run/3 line"), "{error}");
    }

    /// §19.5: an old binary printing `va.bench.run/1` gives no run line.
    #[test]
    fn a_v1_run_line_is_not_a_run_line() {
        let v1 = r#"{"schema":"va.bench.run/1","scenario":"startup","renderer":"glow","backend":"gl","adapter":null,"driver":null,"window_system":"x11","wsl":false,"os":"linux","arch":"x86_64","version":"0.1.0","startup_ms":12.5,"frames_painted":1,"process_cpu_ms":null,"peak_rss_bytes":null,"peak_private_bytes":null,"settle_ms":null,"idle_window_ms":null}"#;
        let body = format!("printf '%s\\n' '{v1}'\nexit 0\n");
        let (outcome, _) = run_script_as(
            "fake-v1",
            &body,
            Scenario::Startup,
            RendererChoice::Auto,
            Duration::from_secs(30),
        );
        let error = outcome.unwrap_err();
        assert!(error.contains("printed no va.bench.run/3 line"), "{error}");
    }

    /// m4 §7.1: every run gets `VA_CONFIG_DIR` and `VA_DATA_DIR` of the profile; the fake GUI
    /// exits 7 when they are not the profile's folders.
    #[test]
    fn the_profile_variables_reach_the_gui() {
        let body = format!(
            "case \"$VA_CONFIG_DIR\" in */profile-*-1/config) ;; *) exit 7 ;; esac\n\
             case \"$VA_DATA_DIR\" in */profile-*-1/data) ;; *) exit 7 ;; esac\n\
             [ -d \"$VA_CONFIG_DIR\" ] && [ -d \"$VA_DATA_DIR\" ] || exit 7\n\
             printf '%s\\n' '{}'\nexit 0\n",
            run_line("startup", "auto")
        );
        let request = RunRequest {
            scenario: Scenario::Startup,
            renderer: RendererChoice::Auto,
            input: None,
            duration_secs: None,
        };
        let (outcome, profile) = run_script_for("fake-profile", &body, &request);
        let sample = outcome.unwrap();
        assert!(sample.spawn_to_first_frame_ms.is_some());
        assert!(profile.config_dir.ends_with("config"));
    }

    /// m4 §7.2: an open run line with its input, its open time and no vsync is a valid sample,
    /// with no spawn time (its line arrives after the measurement).
    #[test]
    fn an_open_run_gives_its_open_time_and_no_spawn_time() {
        let line = run_line_with(
            "open",
            "auto",
            "glow",
            "null",
            &editor_keys("100k.asm", None),
        );
        let body = format!("printf '%s\\n' '{line}'\nexit 0\n");
        let (outcome, _) = run_script_for(
            "fake-open",
            &body,
            &editor_request(Scenario::Open, "100k.asm"),
        );
        let sample = outcome.unwrap();
        assert_eq!(sample.spawn_to_first_frame_ms, None);
        assert_eq!(sample.run.open_ms, Some(61.2));
        assert_eq!(sample.run.input.as_deref(), Some("100k.asm"));
    }

    /// m4 §7.2: a type run line must carry its 500 samples.
    #[test]
    fn a_type_run_needs_its_500_samples() {
        let valid = run_line_with(
            "type",
            "auto",
            "glow",
            "null",
            &editor_keys("100k.asm", Some(500)),
        );
        let body = format!("printf '%s\\n' '{valid}'\nexit 0\n");
        let (outcome, _) = run_script_for(
            "fake-type",
            &body,
            &editor_request(Scenario::Type, "100k.asm"),
        );
        let sample = outcome.unwrap();
        assert_eq!(sample.run.event_ms.unwrap().len(), 500);
        assert_eq!(sample.spawn_to_first_frame_ms, None);

        let short = run_line_with(
            "type",
            "auto",
            "glow",
            "null",
            &editor_keys("100k.asm", Some(499)),
        );
        let body = format!("printf '%s\\n' '{short}'\nexit 0\n");
        let (outcome, _) = run_script_for(
            "fake-type-short",
            &body,
            &editor_request(Scenario::Type, "100k.asm"),
        );
        let error = outcome.unwrap_err();
        assert!(
            error.contains("reported event_count 499, expected 500"),
            "{error}"
        );
    }

    /// m4 §7.2: a scroll run line that names another input fails the run.
    #[test]
    fn a_run_line_for_another_input_fails_the_run() {
        let line = run_line_with(
            "scroll",
            "auto",
            "glow",
            "null",
            &editor_keys("other.asm", Some(300)),
        );
        let body = format!("printf '%s\\n' '{line}'\nexit 0\n");
        let (outcome, _) = run_script_for(
            "fake-scroll-input",
            &body,
            &editor_request(Scenario::Scroll, "100k.asm"),
        );
        let error = outcome.unwrap_err();
        assert!(
            error.contains("reported input `other.asm`, expected `100k.asm`"),
            "{error}"
        );
    }
}
