//! Spawns the `visual-assembler` binary and checks `--version`, `--help`, usage errors and, on
//! Linux without a display, the window error and the renderer-selection warnings
//! (docs/specs/m0-foundations.md §4.3, §11, §19.3, §19.4). None of these opens a window, so they
//! are safe on headless runners. Tests run the debug build, which uses the console subsystem on
//! Windows.
#![allow(clippy::unwrap_used)]

use std::process::{Command, Output};

/// Runs the GUI binary with `args` and returns its captured output.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_visual-assembler"))
        .args(args)
        .output()
        .unwrap()
}

/// Decodes a captured stream as UTF-8.
fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[test]
fn version() {
    let output = run(&["--version"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        text(&output.stdout),
        format!("visual-assembler {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(text(&output.stdout), "visual-assembler 0.1.0\n");
}

/// The renderer-selection variables, removed from every spawned process whose stderr is checked
/// exactly, so a developer's own environment cannot change the outcome. Only the Linux tests
/// reach a window (and therefore read them).
#[cfg(target_os = "linux")]
const SELECTION_VARIABLES: [&str; 3] = ["VA_RENDERER", "VA_TEST_FAIL_RENDERER", "VA_WAYLAND"];

/// §19.1: `--renderer` offers `auto`, `glow` and `wgpu`, in that order, in the short help, and
/// the long help explains each value and the `VA_RENDERER` default without rustdoc markup.
#[test]
fn help_lists_renderer_values() {
    let short = run(&["-h"]);
    assert_eq!(short.status.code(), Some(0));
    let short = text(&short.stdout);
    assert!(
        short.contains("[possible values: auto, glow, wgpu]"),
        "-h does not list the renderer values:\n{short}"
    );

    let long = run(&["--help"]);
    assert_eq!(long.status.code(), Some(0));
    let long = text(&long.stdout);
    for line in [
        "[default: $VA_RENDERER, else auto]",
        "- auto: glow, then wgpu if glow fails to start (wgpu first on Windows ARM64)",
        "- glow: OpenGL through glow only, no fallback",
        "- wgpu: wgpu only (DX12 on Windows unless WGPU_BACKEND is set), no fallback",
    ] {
        assert!(long.contains(line), "--help lacks {line:?}:\n{long}");
    }
    assert!(
        !long.contains("[`"),
        "rustdoc link markup in --help:\n{long}"
    );
}

#[test]
fn help_lists_bench_flags() {
    let output = run(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = text(&output.stdout);
    for flag in ["--bench", "--renderer", "--duration-secs"] {
        assert!(
            stdout.contains(flag),
            "--help does not list {flag}:\n{stdout}"
        );
    }
}

/// The exit-code constants match the table of docs/specs/m0-foundations.md §17.10.
#[test]
fn exit_code_constants_match_the_spec_table() {
    assert_eq!(visual_assembler::EXIT_RUNTIME, 1);
    assert_eq!(visual_assembler::EXIT_USAGE, 2);
    assert_eq!(visual_assembler::EXIT_WINDOW, 3);
    assert_eq!(visual_assembler::EXIT_BENCH_TIMEOUT, 4);
}

/// §17.10: when clap's help or version text cannot be written, the binary exits 1
/// (`EXIT_RUNTIME`), not clap's 0, and says why on stderr. stdout is `/dev/full`, where every
/// write fails with ENOSPC. Linux only: `/dev/full` is a Linux device.
#[cfg(target_os = "linux")]
#[test]
fn help_and_version_on_full_stdout_exit_1() {
    for flag in ["--help", "-h", "--version", "-V"] {
        let full = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_visual-assembler"))
            .arg(flag)
            .stdout(full)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{flag}");
        let stderr = text(&output.stderr);
        assert!(
            stderr.starts_with("error: cannot print the command-line message"),
            "{flag}, stderr: {stderr}"
        );
    }
}

#[test]
fn bad_bench_value() {
    let output = run(&["--bench", "bogus"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(text(&output.stderr).starts_with("error:"));
}

#[test]
fn duration_without_idle() {
    let output = run(&["--duration-secs", "5"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = text(&output.stderr);
    assert!(stderr.starts_with("error:"), "stderr: {stderr}");
    assert!(stderr.contains("--duration-secs"), "stderr: {stderr}");
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (audit round 1). Every command line below fails in clap or in `cli::resolve`,
// before any window is created, so these tests stay safe on headless runners.
// ---------------------------------------------------------------------------------------------

/// Returns an argument that is not valid Unicode on the current OS: an invalid UTF-8 byte on
/// Unix, an unpaired UTF-16 surrogate on Windows.
fn non_unicode_argument() -> std::ffi::OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt as _;
        std::ffi::OsString::from_vec(vec![b'w', 0xff])
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt as _;
        std::ffi::OsString::from_wide(&[u16::from(b'w'), 0xD800])
    }
}

#[test]
fn short_flags_and_clean_streams() {
    let version = run(&["-V"]);
    assert_eq!(version.status.code(), Some(0));
    assert_eq!(text(&version.stdout), "visual-assembler 0.1.0\n");
    assert_eq!(text(&version.stderr), "");

    for flag in ["-h", "--help"] {
        let help = run(&[flag]);
        assert_eq!(help.status.code(), Some(0), "{flag}");
        assert!(text(&help.stdout).contains("Usage:"), "{flag}");
        assert_eq!(text(&help.stderr), "", "{flag}");
    }
}

#[test]
fn invalid_command_lines_exit_2_before_any_window() {
    for args in [
        &["--bogus"][..],
        &["positional"],
        &["--bench"],
        &["--renderer"],
        &["--renderer", "vulkan"],
        &["--renderer", ""],
        &["--bench", "idle", "--duration-secs", "0"],
        &["--bench", "idle", "--duration-secs", "3601"],
        &["--bench", "idle", "--duration-secs", "abc"],
        &["--bench", "idle", "--duration-secs", ""],
        &["--bench", "idle", "--duration-secs", "1.5"],
        &["--bench", "idle", "--duration-secs", "-1"],
        &["--bench", "idle", "--duration-secs", "18446744073709551616"],
        &["--bench", "startup", "--bench", "idle"],
        &["--bench", "startup", "--duration-secs", "5"],
        &["--renderer", "glow", "--duration-secs", "5"],
    ] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        assert!(output.stdout.is_empty(), "args: {args:?}");
        let stderr = text(&output.stderr);
        assert!(
            stderr.starts_with("error:"),
            "args: {args:?}, stderr: {stderr}"
        );
    }
}

#[test]
fn resolve_errors_print_the_exact_spec_message() {
    for args in [
        &["--duration-secs", "5"][..],
        &["--bench", "startup", "--duration-secs", "5"],
    ] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        assert_eq!(
            text(&output.stderr),
            "error: --duration-secs requires --bench idle\n",
            "args: {args:?}"
        );
    }
}

#[test]
fn non_unicode_argument_is_a_usage_error_not_a_panic() {
    let output = Command::new(env!("CARGO_BIN_EXE_visual-assembler"))
        .arg("--renderer")
        .arg(non_unicode_argument())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("error:"), "stderr: {stderr}");
}

/// Without any display variable, winit cannot connect to a window system, so
/// `eframe::run_native` fails and the binary must exit with code 3 at once, in bench mode too
/// (no JSON line, and no wait for the 60 s watchdog). Linux only: elsewhere a window would open.
#[cfg(target_os = "linux")]
#[test]
fn missing_display_is_a_window_error() {
    for args in [
        &[][..],
        &["--bench", "startup"],
        &["--bench", "idle", "--duration-secs", "1"],
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_visual-assembler"));
        command
            .args(args)
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("WAYLAND_SOCKET");
        for variable in SELECTION_VARIABLES {
            command.env_remove(variable);
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(3), "args: {args:?}");
        assert!(output.stdout.is_empty(), "args: {args:?}");
        let stderr = text(&output.stderr);
        assert!(
            stderr.starts_with("error: cannot create the window or the renderer"),
            "args: {args:?}, stderr: {stderr}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// QA audit round 1 (continuation): the §17.10 exit code 1 on every OS, not only through the
// Linux `/dev/full`. A pipe whose read end is closed before the child starts makes every write
// fail at once (EPIPE on Unix, where Rust ignores SIGPIPE; ERROR_NO_DATA on Windows), so there
// is no race with the child's first write. None of these command lines opens a window.
// ---------------------------------------------------------------------------------------------

/// Returns the write end of a pipe whose read end is already closed.
fn closed_pipe() -> std::process::Stdio {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    std::process::Stdio::from(writer)
}

#[test]
fn help_and_version_on_a_closed_stdout_pipe_exit_1() {
    for flag in ["--help", "-h", "--version", "-V"] {
        let output = Command::new(env!("CARGO_BIN_EXE_visual-assembler"))
            .arg(flag)
            .stdin(std::process::Stdio::null())
            .stdout(closed_pipe())
            .stderr(std::process::Stdio::piped())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(i32::from(visual_assembler::EXIT_RUNTIME)),
            "{flag}"
        );
        let stderr = text(&output.stderr);
        assert!(
            stderr.starts_with("error: cannot print the command-line message"),
            "{flag}, stderr: {stderr}"
        );
    }
}

/// A clap usage error whose message cannot be written on stderr is a runtime failure (§17.10:
/// clap's error text cannot be written), not a usage error: exit 1, nothing on stdout.
#[test]
fn clap_usage_error_on_a_closed_stderr_pipe_exits_1() {
    for args in [&["--bench", "bogus"][..], &["--bogus"][..], &["extra"][..]] {
        let output = Command::new(env!("CARGO_BIN_EXE_visual-assembler"))
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(closed_pipe())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(i32::from(visual_assembler::EXIT_RUNTIME)),
            "{args:?}"
        );
        assert!(output.stdout.is_empty(), "{args:?}");
    }
}

// ---------------------------------------------------------------------------------------------
// Renderer selection (docs/specs/m0-foundations.md §19.3, §19.4). Linux only: without any display
// variable winit cannot create its event loop, so no window can open.
// ---------------------------------------------------------------------------------------------

/// Runs the GUI binary on Linux with no display variable, no selection variable from the
/// caller's environment, and then the given `env` pairs.
#[cfg(target_os = "linux")]
fn run_without_display(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_visual-assembler"));
    command
        .args(args)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET");
    for variable in SELECTION_VARIABLES {
        command.env_remove(variable);
    }
    command.envs(env.iter().copied());
    command.output().unwrap()
}

/// §19.3 `platform_failure_never_falls_back`: a window-system failure (no display) is not a
/// renderer initialization error, so even an `auto` plan with an injected glow failure exits 3
/// with the unchanged window message and never falls back.
#[cfg(target_os = "linux")]
#[test]
fn platform_failure_never_falls_back() {
    let output = run_without_display(
        &["--bench", "startup"],
        &[("VA_TEST_FAIL_RENDERER", "glow")],
    );
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("cannot create the window"),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("falling back"), "stderr: {stderr}");
    assert!(
        stderr.contains("error: cannot create the window or the renderer: "),
        "stderr: {stderr}"
    );
}

/// The selection warnings of §19.1 and §19.3 are printed before any window, in this order: the
/// ignored `VA_RENDERER`, the ignored injection values, then the loud test-hook warning.
#[cfg(target_os = "linux")]
#[test]
fn selection_warnings_come_first_and_in_order() {
    let output = run_without_display(
        &["--bench", "startup"],
        &[
            ("VA_RENDERER", "vulkan"),
            ("VA_TEST_FAIL_RENDERER", "glow,metal"),
        ],
    );
    assert_eq!(output.status.code(), Some(3));
    let stderr = text(&output.stderr);
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 4, "stderr: {stderr}");
    assert_eq!(
        lines[..3],
        [
            "warning: ignoring VA_RENDERER='vulkan' (expected auto, glow or wgpu)",
            "warning: ignoring unknown VA_TEST_FAIL_RENDERER value 'metal' (expected glow or wgpu)",
            "warning: VA_TEST_FAIL_RENDERER=glow,metal is set: simulating a renderer initialization failure (test hook)",
        ],
        "stderr: {stderr}"
    );
    assert!(
        lines[3].starts_with("error: cannot create the window or the renderer: "),
        "stderr: {stderr}"
    );
}

/// A forced renderer meets the same platform failure: the window message, not the forced one,
/// because nothing was wrong with the renderer itself (§19.4 "Platform failure").
///
/// It forces both glow and wgpu, so it needs the shipped build with both renderers: in a
/// single-renderer build the other one is rightly a usage error (exit 2) before any window.
#[cfg(all(
    target_os = "linux",
    feature = "renderer-glow",
    feature = "renderer-wgpu"
))]
#[test]
fn forced_renderer_platform_failure_keeps_the_window_message() {
    let forced_by_flag = run_without_display(
        &["--renderer", "glow", "--bench", "startup"],
        &[("VA_TEST_FAIL_RENDERER", "glow")],
    );
    let forced_by_variable =
        run_without_display(&["--bench", "startup"], &[("VA_RENDERER", "wgpu")]);
    for output in [forced_by_flag, forced_by_variable] {
        assert_eq!(output.status.code(), Some(3));
        let stderr = text(&output.stderr);
        assert!(
            stderr.contains("error: cannot create the window or the renderer: "),
            "stderr: {stderr}"
        );
        assert!(!stderr.contains("forced by"), "stderr: {stderr}");
        assert!(!stderr.contains("falling back"), "stderr: {stderr}");
    }
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (§19 audit round 1). The first test runs on every OS: each command line fails in
// clap or in `cli::resolve`, before any window. The others are Linux-only (no display variable).
// ---------------------------------------------------------------------------------------------

/// The selection variables never turn a usage error into a renderer attempt: whatever
/// `VA_RENDERER`, `VA_TEST_FAIL_RENDERER` and `VA_WAYLAND` hold, a bad command line exits 2 with
/// nothing on stdout and no fallback or renderer failure on stderr.
#[test]
fn selection_variables_never_turn_a_usage_error_into_a_window() {
    for args in [
        &["--renderer", "vulkan"][..],
        &["--renderer", "Auto"],
        &["--duration-secs", "5"],
        &["--bench", "startup", "--duration-secs", "5"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_visual-assembler"))
            .args(args)
            .env("VA_RENDERER", "wgpu")
            .env("VA_TEST_FAIL_RENDERER", "glow,wgpu")
            .env("VA_WAYLAND", "1")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        assert!(output.stdout.is_empty(), "args: {args:?}");
        let stderr = text(&output.stderr);
        for forbidden in ["falling back", "failed to initialize", "no renderer could"] {
            assert!(
                !stderr.contains(forbidden),
                "args: {args:?}, stderr: {stderr}"
            );
        }
        assert!(
            stderr.lines().any(|line| line.starts_with("error: ")),
            "args: {args:?}, stderr: {stderr}"
        );
    }
}

/// True under Windows Subsystem for Linux (same rule as `bench::platform::is_wsl`).
#[cfg(all(
    target_os = "linux",
    feature = "renderer-glow",
    feature = "renderer-wgpu"
))]
fn is_wsl() -> bool {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|release| {
            let release = release.to_lowercase();
            release.contains("microsoft") || release.contains("wsl")
        })
        .unwrap_or(false)
}

/// §19.1: with `VA_WAYLAND=1` under WSL, the compositor warning is printed exactly when the plan
/// includes wgpu (auto, forced by the flag or by the variable), before the window error. Outside
/// WSL, or with any other `VA_WAYLAND` value, it is never printed.
///
/// Shipped build only: it forces both renderers, and a single-renderer `auto` plan never
/// includes wgpu (glow-only) or rejects `--renderer glow` (wgpu-only).
#[cfg(all(
    target_os = "linux",
    feature = "renderer-glow",
    feature = "renderer-wgpu"
))]
#[test]
fn va_wayland_warning_follows_the_plan() {
    const WARNING: &str =
        "warning: VA_WAYLAND=1 with the wgpu renderer can crash the WSLg compositor (weston 9)";
    let wsl = is_wsl();
    for (args, env, expected) in [
        (&["--bench", "startup"][..], &[("VA_WAYLAND", "1")][..], wsl),
        (
            &["--renderer", "wgpu", "--bench", "startup"],
            &[("VA_WAYLAND", "1")],
            wsl,
        ),
        (
            &["--bench", "startup"],
            &[("VA_WAYLAND", "1"), ("VA_RENDERER", "wgpu")],
            wsl,
        ),
        (
            &["--renderer", "auto", "--bench", "startup"],
            &[("VA_WAYLAND", "1"), ("VA_RENDERER", "glow")],
            wsl,
        ),
        (
            &["--renderer", "glow", "--bench", "startup"],
            &[("VA_WAYLAND", "1")],
            false,
        ),
        (
            &["--bench", "startup"],
            &[("VA_WAYLAND", "1"), ("VA_RENDERER", "glow")],
            false,
        ),
        (&["--bench", "startup"], &[("VA_WAYLAND", "0")], false),
        (&["--bench", "startup"], &[("VA_WAYLAND", " 1")], false),
    ] {
        let output = run_without_display(args, env);
        assert_eq!(output.status.code(), Some(3), "{args:?} {env:?}");
        let stderr = text(&output.stderr);
        let lines: Vec<&str> = stderr.lines().collect();
        if expected {
            assert_eq!(lines.len(), 2, "{args:?} {env:?}, stderr: {stderr}");
            assert_eq!(lines[0], WARNING, "{args:?} {env:?}");
        } else {
            assert_eq!(lines.len(), 1, "{args:?} {env:?}, stderr: {stderr}");
        }
        assert!(
            lines[lines.len() - 1].starts_with("error: cannot create the window or the renderer: "),
            "{args:?} {env:?}, stderr: {stderr}"
        );
    }
}

/// Empty selection variables: an empty `VA_TEST_FAIL_RENDERER` simulates nothing and prints
/// nothing; an empty `VA_RENDERER` is invalid and warns (§19.1, read literally).
#[cfg(target_os = "linux")]
#[test]
fn empty_selection_variables() {
    let empty_hook = run_without_display(&["--bench", "startup"], &[("VA_TEST_FAIL_RENDERER", "")]);
    assert_eq!(empty_hook.status.code(), Some(3));
    let stderr = text(&empty_hook.stderr);
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 1, "stderr: {stderr}");
    assert!(
        lines[0].starts_with("error: cannot create the window or the renderer: "),
        "stderr: {stderr}"
    );

    let empty_choice = run_without_display(&["--bench", "startup"], &[("VA_RENDERER", "")]);
    assert_eq!(empty_choice.status.code(), Some(3));
    let stderr = text(&empty_choice.stderr);
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 2, "stderr: {stderr}");
    assert_eq!(
        lines[0],
        "warning: ignoring VA_RENDERER='' (expected auto, glow or wgpu)"
    );
}

/// Non-Unicode selection variables are ignored with one lossy warning each, before any window.
#[cfg(target_os = "linux")]
#[test]
fn non_unicode_selection_variables_warn_before_any_window() {
    use std::os::unix::ffi::OsStringExt as _;
    let bad = || std::ffi::OsString::from_vec(vec![b'g', 0xff]);
    let mut command = Command::new(env!("CARGO_BIN_EXE_visual-assembler"));
    command
        .args(["--bench", "startup"])
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .env_remove("VA_WAYLAND")
        .env("VA_RENDERER", bad())
        .env("VA_TEST_FAIL_RENDERER", bad());
    let output = command.output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    let stderr = text(&output.stderr);
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 3, "stderr: {stderr}");
    assert_eq!(
        lines[..2],
        [
            "warning: ignoring VA_RENDERER='g\u{FFFD}' (expected auto, glow or wgpu)",
            "warning: ignoring VA_TEST_FAIL_RENDERER='g\u{FFFD}' (not valid Unicode; expected glow, wgpu or glow,wgpu)",
        ],
        "stderr: {stderr}"
    );
    assert!(
        lines[2].starts_with("error: cannot create the window or the renderer: "),
        "stderr: {stderr}"
    );
}

/// QA (§19.9 re-check, row 19-8 a, S19-2): control characters in the selection variables reach
/// stderr escaped. A CR, LF or escape sequence in `VA_RENDERER` and a forged `error:` line in
/// `VA_TEST_FAIL_RENDERER` stay inside their own warning lines, so stderr holds exactly the three
/// warnings and the window error, and no control character other than the line ends.
#[cfg(target_os = "linux")]
#[test]
fn control_characters_in_selection_variables_reach_stderr_escaped() {
    let output = run_without_display(
        &["--bench", "startup"],
        &[
            ("VA_RENDERER", "glow\r\n\u{1b}[2J"),
            ("VA_TEST_FAIL_RENDERER", "glow,bad\nerror: forged"),
        ],
    );
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    let stderr = text(&output.stderr);
    assert!(
        !stderr.chars().any(|c| c.is_control() && c != '\n'),
        "stderr: {stderr:?}"
    );
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 4, "stderr: {stderr:?}");
    assert_eq!(
        lines[..3],
        [
            r"warning: ignoring VA_RENDERER='glow\r\n\u{1b}[2J' (expected auto, glow or wgpu)",
            r"warning: ignoring unknown VA_TEST_FAIL_RENDERER value 'bad\nerror: forged' (expected glow or wgpu)",
            r"warning: VA_TEST_FAIL_RENDERER=glow,bad\nerror: forged is set: simulating a renderer initialization failure (test hook)",
        ],
        "stderr: {stderr:?}"
    );
    assert!(
        lines[3].starts_with("error: cannot create the window or the renderer: "),
        "stderr: {stderr:?}"
    );
}
