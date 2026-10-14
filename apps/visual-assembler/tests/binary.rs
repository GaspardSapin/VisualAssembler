//! Spawns the `visual-assembler` binary and checks `--version`, `--help` and usage errors
//! (docs/specs/m0-foundations.md §4.3, §11). None of these opens a window, so they are safe on
//! headless runners. Tests run the debug build, which uses the console subsystem on Windows.
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
        let output = Command::new(env!("CARGO_BIN_EXE_visual-assembler"))
            .args(args)
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("WAYLAND_SOCKET")
            .output()
            .unwrap();
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
