//! Local renderer-fallback tests (docs/specs/m0-foundations.md §19.3): each one spawns the GUI
//! binary with `--bench startup`, so a real window opens and closes by itself. They need a
//! display and are `#[ignore]`d on CI; QA runs them on Windows and under WSLg X11 with
//! `cargo test -p visual-assembler --test fallback -- --ignored`.
//!
//! Every run removes `VA_RENDERER`, `VA_TEST_FAIL_RENDERER` and `VA_WAYLAND` from the inherited
//! environment, so under WSL the app always takes its X11 default (wgpu Wayland clients crash the
//! WSLg 1.0.73 compositor). Under WSL the runs also drop `WAYLAND_DISPLAY`, a second guard that
//! keeps every window on X11 even if the X11 default regressed.
//!
//! Only meaningful in the shipped build with both renderers: single-renderer builds (CI clippy
//! steps) compile this file to an empty test binary.
#![cfg(all(feature = "renderer-glow", feature = "renderer-wgpu"))]
#![allow(clippy::unwrap_used)]

use std::process::{Command, Output};

/// The text of the injected failure (`fallback_reason` and every §19.4 message).
const INJECTED: &str = "injected renderer failure (VA_TEST_FAIL_RENDERER)";

/// The order of `auto` on this target (§19.1).
fn auto_order() -> [&'static str; 2] {
    if cfg!(all(windows, target_arch = "aarch64")) {
        ["wgpu", "glow"]
    } else {
        ["glow", "wgpu"]
    }
}

/// True under Windows Subsystem for Linux (same rule as `bench::platform::is_wsl`).
fn is_wsl() -> bool {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|release| {
            let release = release.to_lowercase();
            release.contains("microsoft") || release.contains("wsl")
        })
        .unwrap_or(false)
}

/// Runs `visual-assembler --bench startup <args>` with a clean selection environment plus `env`.
fn bench_startup(args: &[&str], env: &[(&str, &str)]) -> Output {
    gui_command(&[&["--bench", "startup"][..], args].concat(), env)
        .output()
        .unwrap()
}

/// Builds `visual-assembler <args>` with a clean selection environment plus `env` (and, under
/// WSL, without `WAYLAND_DISPLAY`).
fn gui_command(args: &[&str], env: &[(&str, &str)]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_visual-assembler"));
    command.args(args);
    for variable in ["VA_RENDERER", "VA_TEST_FAIL_RENDERER", "VA_WAYLAND"] {
        command.env_remove(variable);
    }
    if is_wsl() {
        command.env_remove("WAYLAND_DISPLAY");
    }
    command.envs(env.iter().copied());
    command
}

/// Decodes a captured stream as UTF-8 (lossy: native drivers may write their own bytes).
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Asserts that `expected` appears in `stderr` as whole lines, in this order. Other lines may sit
/// in between: native drivers write their own (Mesa under WSLg even prints lines that start with
/// `warning: `), so the app's lines are matched exactly but not counted.
fn assert_lines_in_order(stderr: &str, expected: &[String]) {
    let mut lines = stderr.lines();
    for wanted in expected {
        assert!(
            lines.any(|line| line == wanted),
            "missing or out of order: {wanted:?}\nstderr: {stderr}"
        );
    }
}

/// The app's error lines (`error: `); drivers do not write this prefix.
fn error_lines(stderr: &str) -> Vec<&str> {
    stderr
        .lines()
        .filter(|line| line.starts_with("error: "))
        .collect()
}

/// Parses the single `va.bench.run/2` line of a successful run.
fn bench_line(output: &Output) -> serde_json::Value {
    let stdout = text(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout: {stdout}");
    let value: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(value["schema"], "va.bench.run/2", "{stdout}");
    value
}

/// Asserts a successful auto run that fell back from `from` to `to` after injecting `from`.
fn assert_injected_fallback(from: &str, to: &str) {
    let output = bench_startup(&[], &[("VA_TEST_FAIL_RENDERER", from)]);
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "stderr: {stderr}");
    let line = bench_line(&output);
    assert_eq!(line["renderer_requested"], "auto");
    assert_eq!(line["renderer"], to);
    assert_eq!(line["fallback_from"], from);
    assert_eq!(line["fallback_reason"], INJECTED);
    assert_eq!(line["injected_failure"], from);
    assert_eq!(line["frames_painted"], 1);
    assert_lines_in_order(
        &stderr,
        &[
            format!(
                "warning: VA_TEST_FAIL_RENDERER={from} is set: simulating a renderer initialization failure (test hook)"
            ),
            format!(
                "warning: the {from} renderer failed to initialize: {INJECTED}; falling back to {to}"
            ),
        ],
    );
    assert!(error_lines(&stderr).is_empty(), "stderr: {stderr}");
    if is_wsl() {
        assert_eq!(line["window_system"], "xlib", "WSL must default to X11");
    }
}

#[cfg(not(all(windows, target_arch = "aarch64")))]
#[test]
#[ignore = "needs a display; run with cargo test -p visual-assembler --test fallback -- --ignored"]
fn injected_glow_failure_falls_back_to_wgpu() {
    assert_injected_fallback("glow", "wgpu");
}

/// The Windows aarch64 counterpart: `auto` starts with wgpu there.
#[cfg(all(windows, target_arch = "aarch64"))]
#[test]
#[ignore = "needs a display; run with cargo test -p visual-assembler --test fallback -- --ignored"]
fn injected_wgpu_failure_falls_back_to_glow() {
    assert_injected_fallback("wgpu", "glow");
}

#[test]
#[ignore = "needs a display; run with cargo test -p visual-assembler --test fallback -- --ignored"]
fn both_failures_exit_3() {
    let [first, second] = auto_order();
    let output = bench_startup(&[], &[("VA_TEST_FAIL_RENDERER", "glow,wgpu")]);
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(3), "stderr: {stderr}");
    assert!(output.stdout.is_empty(), "stdout: {}", text(&output.stdout));
    let error = format!(
        "error: no renderer could be initialized (tried {first}, then {second}): {INJECTED}"
    );
    assert_lines_in_order(
        &stderr,
        &[
            "warning: VA_TEST_FAIL_RENDERER=glow,wgpu is set: simulating a renderer initialization failure (test hook)".to_owned(),
            format!("warning: the {first} renderer failed to initialize: {INJECTED}; falling back to {second}"),
            error.clone(),
            "note: see the README section \"Renderer selection\"".to_owned(),
        ],
    );
    assert_eq!(error_lines(&stderr), [error.as_str()], "stderr: {stderr}");
    assert_eq!(
        stderr.matches("falling back").count(),
        1,
        "at most one fallback: {stderr}"
    );
}

#[test]
#[ignore = "needs a display; run with cargo test -p visual-assembler --test fallback -- --ignored"]
fn forced_renderer_never_falls_back() {
    let output = bench_startup(
        &["--renderer", "glow"],
        &[("VA_TEST_FAIL_RENDERER", "glow")],
    );
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(3), "stderr: {stderr}");
    assert!(output.stdout.is_empty());
    assert!(!stderr.contains("falling back"), "stderr: {stderr}");
    assert_eq!(
        error_lines(&stderr),
        [format!(
            "error: the glow renderer failed to initialize: {INJECTED} (forced by --renderer; try --renderer auto)"
        )],
        "stderr: {stderr}"
    );

    // Forced through the variable: same rule, the message names VA_RENDERER.
    let output = bench_startup(
        &[],
        &[("VA_RENDERER", "glow"), ("VA_TEST_FAIL_RENDERER", "glow")],
    );
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(3), "stderr: {stderr}");
    assert!(output.stdout.is_empty());
    assert!(!stderr.contains("falling back"), "stderr: {stderr}");
    assert_eq!(
        error_lines(&stderr),
        [format!(
            "error: the glow renderer failed to initialize: {INJECTED} (forced by VA_RENDERER; try --renderer auto)"
        )],
        "stderr: {stderr}"
    );
}

#[cfg(not(all(windows, target_arch = "aarch64")))]
#[test]
#[ignore = "needs a display; run with cargo test -p visual-assembler --test fallback -- --ignored"]
fn auto_uses_glow_first() {
    let output = bench_startup(&[], &[]);
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "stderr: {stderr}");
    assert!(error_lines(&stderr).is_empty(), "stderr: {stderr}");
    assert!(!stderr.contains("falling back"), "stderr: {stderr}");
    assert!(
        !stderr.contains("VA_"),
        "no selection warning expected: {stderr}"
    );
    let line = bench_line(&output);
    assert_eq!(line["renderer_requested"], "auto");
    assert_eq!(line["renderer"], "glow");
    assert!(line["fallback_from"].is_null());
    assert!(line["fallback_reason"].is_null());
    assert!(line["injected_failure"].is_null());
    if is_wsl() {
        assert_eq!(line["window_system"], "xlib", "WSL must default to X11");
    }
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (§19 audit round 1). Same rules as above: every run closes its window by itself
// (bench mode) or is killed by the test (the interactive run).
// ---------------------------------------------------------------------------------------------

/// §19.2 "the bench recorder and its watchdog are created once": the idle scenario survives a
/// fallback too and reports its settle delay and window with the fallback keys.
#[cfg(not(all(windows, target_arch = "aarch64")))]
#[test]
#[ignore = "needs a display; run with cargo test -p visual-assembler --test fallback -- --ignored"]
fn injected_glow_failure_falls_back_in_the_idle_scenario() {
    let output = gui_command(
        &["--bench", "idle", "--duration-secs", "1"],
        &[("VA_TEST_FAIL_RENDERER", "glow")],
    )
    .output()
    .unwrap();
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "stderr: {stderr}");
    let line = bench_line(&output);
    assert_eq!(line["scenario"], "idle");
    assert_eq!(line["renderer_requested"], "auto");
    assert_eq!(line["renderer"], "wgpu");
    assert_eq!(line["fallback_from"], "glow");
    assert_eq!(line["fallback_reason"], INJECTED);
    assert_eq!(line["injected_failure"], "glow");
    assert_eq!(line["settle_ms"], 3000);
    assert!(line["idle_window_ms"].as_f64().unwrap() >= 1000.0, "{line}");
    assert_eq!(
        stderr.matches("falling back").count(),
        1,
        "stderr: {stderr}"
    );
}

/// A forced renderer ignores an injection that names the other renderer: no fallback, the hook
/// is still reported raw (§19.1 "Forced", §19.5 `injected_failure`).
#[test]
#[ignore = "needs a display; run with cargo test -p visual-assembler --test fallback -- --ignored"]
fn forced_renderer_ignores_an_injection_of_the_other_renderer() {
    for (args, env, renderer, injected) in [
        (
            &["--renderer", "wgpu"][..],
            &[("VA_TEST_FAIL_RENDERER", "glow")][..],
            "wgpu",
            "glow",
        ),
        (
            &[],
            &[("VA_RENDERER", "glow"), ("VA_TEST_FAIL_RENDERER", "wgpu")],
            "glow",
            "wgpu",
        ),
    ] {
        let output = bench_startup(args, env);
        let stderr = text(&output.stderr);
        assert_eq!(output.status.code(), Some(0), "{args:?} {env:?}: {stderr}");
        let line = bench_line(&output);
        assert_eq!(line["renderer_requested"], renderer, "{args:?} {env:?}");
        assert_eq!(line["renderer"], renderer, "{args:?} {env:?}");
        assert!(line["fallback_from"].is_null(), "{args:?} {env:?}");
        assert!(line["fallback_reason"].is_null(), "{args:?} {env:?}");
        assert_eq!(line["injected_failure"], injected, "{args:?} {env:?}");
        assert!(!stderr.contains("falling back"), "stderr: {stderr}");
        assert!(error_lines(&stderr).is_empty(), "stderr: {stderr}");
    }
}

/// §19.3 "unknown values produce a warning and are ignored": the run is a plain `auto` run.
#[cfg(not(all(windows, target_arch = "aarch64")))]
#[test]
#[ignore = "needs a display; run with cargo test -p visual-assembler --test fallback -- --ignored"]
fn unknown_injection_values_change_nothing() {
    let raw = "GLOW;wgpu,vulkan,";
    let output = bench_startup(&[], &[("VA_TEST_FAIL_RENDERER", raw)]);
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "stderr: {stderr}");
    let line = bench_line(&output);
    assert_eq!(line["renderer"], "glow");
    assert!(line["fallback_from"].is_null());
    assert_eq!(line["injected_failure"], raw);
    assert_lines_in_order(
        &stderr,
        &[
            "warning: ignoring unknown VA_TEST_FAIL_RENDERER value 'GLOW;wgpu' (expected glow or wgpu)".to_owned(),
            "warning: ignoring unknown VA_TEST_FAIL_RENDERER value 'vulkan' (expected glow or wgpu)".to_owned(),
        ],
    );
    assert!(!stderr.contains("simulating"), "stderr: {stderr}");
    assert!(!stderr.contains("falling back"), "stderr: {stderr}");
}

/// §19.1 under WSL: X11 is the default for every renderer even when `WAYLAND_DISPLAY` is set,
/// and `VA_WAYLAND=1` restores Wayland. Unlike the other tests, this one keeps `WAYLAND_DISPLAY`,
/// so it forces glow: a glow Wayland client never crashes the WSLg compositor (only wgpu does).
/// Skipped (passes) outside WSL or without a Wayland display.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "needs a display; run with cargo test -p visual-assembler --test fallback -- --ignored"]
fn wsl_defaults_to_x11_with_wayland_display_present() {
    if !is_wsl() || std::env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!("skipped: needs WSLg with WAYLAND_DISPLAY set");
        return;
    }
    for (va_wayland, expected) in [(None, "xlib"), (Some("0"), "xlib"), (Some("1"), "wayland")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_visual-assembler"));
        command
            .args(["--renderer", "glow", "--bench", "startup"])
            .env_remove("VA_RENDERER")
            .env_remove("VA_TEST_FAIL_RENDERER")
            .env_remove("VA_WAYLAND");
        if let Some(value) = va_wayland {
            command.env("VA_WAYLAND", value);
        }
        let output = command.output().unwrap();
        let stderr = text(&output.stderr);
        assert_eq!(output.status.code(), Some(0), "{va_wayland:?}: {stderr}");
        let line = bench_line(&output);
        assert_eq!(line["renderer"], "glow", "{va_wayland:?}");
        assert_eq!(line["window_system"], expected, "{va_wayland:?}");
        assert!(
            !stderr.contains("can crash the WSLg compositor"),
            "forced glow never warns: {stderr}"
        );
    }
}

/// Kills and reaps the child when dropped, so a failing assertion never leaves a window open.
struct KillOnDrop(std::process::Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        // Already exited, or the kill races its exit: either way only the wait matters.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// §19.2 outside the bench: after an interactive (non-bench) fallback, the process keeps running
/// with the next renderer's window instead of exiting. The test kills it after 2 s.
#[cfg(not(all(windows, target_arch = "aarch64")))]
#[test]
#[ignore = "needs a display; run with cargo test -p visual-assembler --test fallback -- --ignored"]
fn interactive_fallback_keeps_the_window_open() {
    use std::io::BufRead as _;
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    let mut command = gui_command(&[], &[("VA_TEST_FAIL_RENDERER", "glow")]);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = KillOnDrop(command.spawn().unwrap());
    let stderr = child.0.stderr.take().unwrap();
    let (sender, lines) = std::sync::mpsc::channel::<String>();
    let reader = std::thread::spawn(move || {
        for line in std::io::BufReader::new(stderr).lines() {
            let Ok(line) = line else { break };
            if sender.send(line).is_err() {
                break;
            }
        }
    });

    let expected = format!(
        "warning: the glow renderer failed to initialize: {INJECTED}; falling back to wgpu"
    );
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut seen = Vec::new();
    while !seen.contains(&expected) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match lines.recv_timeout(remaining) {
            Ok(line) => seen.push(line),
            Err(error) => panic!("no fallback warning ({error}); stderr so far: {seen:?}"),
        }
    }
    std::thread::sleep(Duration::from_secs(2));
    assert!(
        child.0.try_wait().unwrap().is_none(),
        "the GUI exited after the fallback; stderr: {seen:?}"
    );
    drop(child);
    reader.join().unwrap();
    let rest: Vec<String> = lines.try_iter().collect();
    assert!(
        !seen
            .iter()
            .chain(&rest)
            .any(|line| line.starts_with("error: ")),
        "stderr: {seen:?} {rest:?}"
    );
}
