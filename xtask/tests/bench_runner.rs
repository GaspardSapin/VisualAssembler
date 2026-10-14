//! Tests for `src/bench/runner.rs` and the argument helpers of `src/bench/mod.rs`: the pure
//! parts of building and spawning (cargo arguments, artifact message parsing, GUI arguments,
//! timeouts, run-line detection, WSL detection, output caps), and, on Unix, the run engine
//! against fake GUIs written in `sh` (deadlines, pipes held by a descendant, stderr tail,
//! over-long lines; m0-foundations.md §17.8). Spawning a real GUI needs a display and is
//! covered by the manual protocol of m0-foundations.md §12.
#![allow(clippy::unwrap_used)]

use std::{
    io::{BufReader, Cursor},
    path::{Path, PathBuf},
    time::Duration,
};

use clap::Parser;
use serde_json::json;
use xtask::bench::{
    BenchArgs, Renderer, Scenario, milliseconds, osrelease_is_wsl,
    runner::{
        CappedLine, IDLE_TIMEOUT_EXTRA_SECS, MAX_LINE_BYTES, READER_JOIN_TIMEOUT,
        STARTUP_TIMEOUT_SECS, STDERR_TAIL_BYTES, StreamTail, bench_copy_path, build_args,
        gui_executable, parse_run_line, read_capped_line, run_args, run_timeout,
    },
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

#[test]
fn build_command_selects_one_renderer() {
    assert_eq!(
        build_args(Renderer::Glow),
        [
            "build",
            "--release",
            "-p",
            "visual-assembler",
            "--no-default-features",
            "--features",
            "renderer-glow",
            "--message-format=json-render-diagnostics",
        ]
    );
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

#[test]
fn bench_copy_lives_under_the_target_directory() {
    let path = bench_copy_path(Path::new("target"), Renderer::Wgpu);
    let expected = Path::new("target")
        .join("bench")
        .join("wgpu")
        .join(format!("visual-assembler{}", std::env::consts::EXE_SUFFIX));
    assert_eq!(path, expected);
}

#[test]
fn gui_arguments_pass_the_duration_to_idle_only() {
    assert_eq!(run_args(Scenario::Startup, None), ["--bench", "startup"]);
    assert_eq!(
        run_args(Scenario::Idle, Some(3)),
        ["--bench", "idle", "--duration-secs", "3"]
    );
    assert_eq!(run_args(Scenario::Startup, Some(3)), ["--bench", "startup"]);
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

#[test]
fn only_run_schema_lines_are_accepted() {
    assert!(parse_run_line(r#"{"schema":"va.bench.run/1","scenario":"startup"}"#).is_some());
    assert!(parse_run_line(r#"{"schema":"va.info/1"}"#).is_none());
    assert!(parse_run_line("hello").is_none());
    assert!(parse_run_line("").is_none());
}

#[test]
fn bench_defaults_follow_the_spec() {
    let args = parse(&[]).unwrap();
    assert_eq!(args.scenarios(), [Scenario::Startup, Scenario::Idle]);
    assert_eq!(args.renderers(), [Renderer::Wgpu]);
    assert_eq!(args.runs_for(Scenario::Startup), 20);
    assert_eq!(args.runs_for(Scenario::Idle), 3);
    assert_eq!(args.warmup, 1);
    assert_eq!(args.duration_for(Scenario::Idle), Some(10));
    assert_eq!(args.duration_for(Scenario::Startup), None);
    assert!(!args.json);
}

#[test]
fn runs_override_both_scenarios() {
    let args = parse(&["idle", "--renderer", "all", "--runs", "5", "--warmup", "0"]).unwrap();
    assert_eq!(args.scenarios(), [Scenario::Idle]);
    assert_eq!(args.renderers(), [Renderer::Wgpu, Renderer::Glow]);
    assert_eq!(args.runs_for(Scenario::Startup), 5);
    assert_eq!(args.runs_for(Scenario::Idle), 5);
    assert_eq!(args.warmup, 0);
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
        bench_copy_path(&target, Renderer::Glow),
        target
            .join("bench")
            .join("glow")
            .join(format!("visual-assembler{}", std::env::consts::EXE_SUFFIX))
    );
}

/// §17.8 changed the deadlines: 75 s for startup, N + 75 s for idle.
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
    assert_eq!(run_args(Scenario::Idle, None), ["--bench", "idle"]);
}

#[test]
fn run_line_detection_corner_cases() {
    assert!(parse_run_line("  {\"schema\":\"va.bench.run/1\"}\r\n").is_some());
    assert!(parse_run_line("\t{\"frames_painted\":0,\"schema\":\"va.bench.run/1\"} ").is_some());
    for line in [
        r#"{"schema":"va.bench.run/2"}"#,
        r#"{"schema":"VA.BENCH.RUN/1"}"#,
        r#"{"schema":1}"#,
        r#"{"schema":null}"#,
        r#"[{"schema":"va.bench.run/1"}]"#,
        r#"{"run":{"schema":"va.bench.run/1"}}"#,
        r#"{"schema":"va.bench.run/1"} {"schema":"va.bench.run/1"}"#,
        r#"{"schema":"va.bench.run/1""#,
        r#""va.bench.run/1""#,
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

/// The run engine against fake GUIs written in POSIX `sh` (Linux CI and WSL).
#[cfg(unix)]
mod fake_gui {
    use std::{
        path::PathBuf,
        process::Command,
        sync::atomic::{AtomicU32, Ordering},
        time::{Duration, Instant},
    };

    use xtask::bench::{
        Renderer, Scenario,
        runner::{PIPES_OPEN_MESSAGE, READER_JOIN_TIMEOUT, STDERR_TAIL_BYTES, run_command},
    };

    /// Returns a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (§17.9).
    fn temp_dir(tag: &str) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        dir
    }

    /// A complete `va.bench.run/1` line for `scenario` and `renderer`.
    fn run_line(scenario: &str, renderer: &str) -> String {
        format!(
            r#"{{"schema":"va.bench.run/1","scenario":"{scenario}","renderer":"{renderer}","backend":"gl","adapter":null,"driver":null,"window_system":"x11","wsl":false,"os":"linux","arch":"x86_64","version":"0.1.0","startup_ms":12.5,"frames_painted":1,"process_cpu_ms":null,"peak_rss_bytes":null,"peak_private_bytes":null,"settle_ms":null,"idle_window_ms":null}}"#
        )
    }

    /// Runs the `sh` script `body` as a measured run with `timeout`; returns the outcome (the
    /// error rendered with its context) and the elapsed time.
    fn run_script(
        tag: &str,
        body: &str,
        scenario: Scenario,
        timeout: Duration,
    ) -> (Result<xtask::bench::report::Sample, String>, Duration) {
        let dir = temp_dir(tag);
        let script = dir.join("fake-gui.sh");
        std::fs::write(&script, body).unwrap();
        let mut cmd = Command::new("sh");
        cmd.arg(&script);
        let started = Instant::now();
        let outcome = run_command(cmd, scenario, Renderer::Wgpu, timeout)
            .map_err(|error| format!("{error:#}"));
        let elapsed = started.elapsed();
        std::fs::remove_dir_all(&dir).unwrap();
        (outcome, elapsed)
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
        assert!(error.contains("printed no va.bench.run/1 line"), "{error}");
    }

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
        assert!(error.contains("expected `startup` and `wgpu`"), "{error}");
    }
}
