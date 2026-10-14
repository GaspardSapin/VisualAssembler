//! End-to-end tests of the `va` binary: clap behaviour, exit codes, and `va info` human and JSON
//! output (docs/specs/m0-foundations.md §5 and §11).
#![allow(clippy::unwrap_used)]

use std::ffi::OsString;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

/// Exit code of a usage error (docs/specs/m0-foundations.md §5).
const EXIT_USAGE: i32 = 2;

/// Exit code of an I/O or environment error (docs/specs/m0-foundations.md §5).
const EXIT_IO: i32 = 3;

/// Version line printed by `va --version` and first line of `va info`.
fn version_line() -> String {
    format!("va {}", env!("CARGO_PKG_VERSION"))
}

/// Builds a command running the `va` binary under test with `args`.
///
/// `NO_COLOR` is set so that a `CLICOLOR_FORCE` inherited from the environment cannot add ANSI
/// escapes to clap's messages: assertions compare plain text.
fn va_command(args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_va"));
    command.args(args).env("NO_COLOR", "1").stdin(Stdio::null());
    command
}

/// Runs `va` with `args`, capturing stdout and stderr.
fn run_va(args: &[&str]) -> Output {
    va_command(args).output().unwrap()
}

/// Decodes a captured stream as UTF-8.
fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[test]
fn version_prints_and_exits_zero() {
    let output = run_va(&["--version"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(text(&output.stdout), format!("{}\n", version_line()));
    assert_eq!(text(&output.stderr), "");
}

#[test]
fn help_lists_info() {
    let output = run_va(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = text(&output.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(stdout.contains("info"), "stdout: {stdout}");
    assert!(stdout.contains("--json"), "stdout: {stdout}");
    assert_eq!(text(&output.stderr), "");
}

/// Output stream of the `va` process that a test inspects.
#[derive(Clone, Copy, Debug)]
enum Stream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// One `help_usage_line_is_os_independent` case: what to run and what it must print.
struct UsageCase {
    /// Arguments passed to `va`.
    args: &'static [&'static str],
    /// Expected exit code.
    code: i32,
    /// Stream that carries clap's message.
    stream: Stream,
    /// The only `Usage:` line expected in that stream.
    usage: &'static str,
}

impl UsageCase {
    /// Builds a case: `va <args>` exits with `code` and prints `usage` as its only usage line on
    /// `stream`.
    fn new(args: &'static [&'static str], code: i32, stream: Stream, usage: &'static str) -> Self {
        Self {
            args,
            code,
            stream,
            usage,
        }
    }

    /// Runs `command` and asserts the exit code and the single usage line of this case.
    fn check(&self, command: &mut Command) {
        let output = command.output().unwrap();
        let args = self.args;
        assert_eq!(output.status.code(), Some(self.code), "args: {args:?}");
        let printed = match self.stream {
            Stream::Stdout => text(&output.stdout),
            Stream::Stderr => text(&output.stderr),
        };
        let usage_lines: Vec<&str> = printed
            .lines()
            .filter(|line| line.starts_with("Usage:"))
            .collect();
        assert_eq!(
            usage_lines,
            [self.usage],
            "args: {args:?}, {:?}: {printed}",
            self.stream
        );
    }
}

/// D13 (docs/specs/m0-foundations.md §17.11): `bin_name = "va"` makes the usage line name the
/// program `va` on every OS, never the executable file name (`va.exe` on Windows). Covers help on
/// stdout, help printed as a usage error on stderr, a parse error, and a subcommand's help.
#[test]
fn help_usage_line_is_os_independent() {
    const TOP: &str = "Usage: va [OPTIONS] <COMMAND>";
    const INFO: &str = "Usage: va info [OPTIONS]";
    let cases = [
        UsageCase::new(&["--help"], 0, Stream::Stdout, TOP),
        UsageCase::new(&["-h"], 0, Stream::Stdout, TOP),
        UsageCase::new(&[], EXIT_USAGE, Stream::Stderr, TOP),
        UsageCase::new(&["--bogus"], EXIT_USAGE, Stream::Stderr, TOP),
        UsageCase::new(&["info", "--help"], 0, Stream::Stdout, INFO),
        UsageCase::new(&["help", "info"], 0, Stream::Stdout, INFO),
    ];
    for case in &cases {
        case.check(&mut va_command(case.args));

        // On Unix, an argv[0] ending in `.exe` reproduces the Windows file name, so the Linux and
        // macOS runs also prove that the usage line does not follow the executable name.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            let mut renamed = va_command(case.args);
            renamed.arg0("/some/folder/va.exe");
            case.check(&mut renamed);
        }
    }
}

/// `--help` and `-h` both open with the `about` text, then the usage line. Guards against clap's
/// derive turning a multi-paragraph doc comment on `Cli` into a `long_about` that `--help` would
/// print instead (developer text in user-facing help).
#[test]
fn help_starts_with_about_then_usage() {
    for flag in ["--help", "-h"] {
        let output = run_va(&[flag]);
        assert_eq!(output.status.code(), Some(0), "flag: {flag}");
        let stdout = text(&output.stdout);
        assert!(
            stdout.starts_with(
                "VisualAssembler command-line interface\n\nUsage: va [OPTIONS] <COMMAND>\n"
            ),
            "flag: {flag}, stdout: {stdout}"
        );
    }
}

#[test]
fn no_subcommand_is_usage_error() {
    let output = run_va(&[]);
    assert_eq!(output.status.code(), Some(EXIT_USAGE));
    assert_eq!(text(&output.stdout), "");
    let stderr = text(&output.stderr);
    assert!(stderr.contains("Usage:"), "stderr: {stderr}");
}

#[test]
fn unknown_flag_is_usage_error() {
    let output = run_va(&["--bogus"]);
    assert_eq!(output.status.code(), Some(EXIT_USAGE));
    assert_eq!(text(&output.stdout), "");
    let stderr = text(&output.stderr);
    assert!(stderr.starts_with("error:"), "stderr: {stderr}");
}

#[test]
fn json_flag_without_subcommand_is_usage_error() {
    let output = run_va(&["--json"]);
    assert_eq!(output.status.code(), Some(EXIT_USAGE));
    assert_eq!(text(&output.stdout), "");
    let stderr = text(&output.stderr);
    assert!(stderr.starts_with("error:"), "stderr: {stderr}");
}

#[test]
fn info_human() {
    let output = run_va(&["info"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = text(&output.stdout);
    assert!(stdout.starts_with(&version_line()), "stdout: {stdout}");
    let expected = format!(
        "{}\nos: {}\narch: {}\narchitectures: (none)\n",
        version_line(),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    assert_eq!(stdout, expected);
    assert_eq!(text(&output.stderr), "");
}

#[test]
fn info_json() {
    for args in [["info", "--json"], ["--json", "info"]] {
        let output = run_va(&args);
        assert_eq!(output.status.code(), Some(0), "args: {args:?}");
        assert_eq!(text(&output.stderr), "", "args: {args:?}");

        let stdout = text(&output.stdout);
        let line = stdout
            .strip_suffix('\n')
            .unwrap_or_else(|| panic!("{args:?}: stdout must end with one newline: {stdout:?}"));
        assert!(
            !line.contains('\n'),
            "{args:?}: more than one line: {stdout:?}"
        );

        let document: Value = serde_json::from_str(line).unwrap();
        assert_eq!(document["schema"], "va.info/1", "args: {args:?}");
        assert_eq!(
            document["version"],
            env!("CARGO_PKG_VERSION"),
            "args: {args:?}"
        );
        assert_eq!(document["os"], std::env::consts::OS, "args: {args:?}");
        assert_eq!(document["arch"], std::env::consts::ARCH, "args: {args:?}");
        assert!(document["version"].is_string(), "args: {args:?}");
        assert!(document["os"].is_string(), "args: {args:?}");
        assert!(document["arch"].is_string(), "args: {args:?}");
        assert_eq!(
            document["architectures"],
            Value::Array(Vec::new()),
            "args: {args:?}"
        );

        // Exact line, which also pins the key order of the spec.
        let expected = format!(
            "{{\"schema\":\"va.info/1\",\"version\":\"{}\",\"os\":\"{}\",\"arch\":\"{}\",\"architectures\":[]}}",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        assert_eq!(line, expected, "args: {args:?}");
    }
}

/// Writing to `/dev/full` always fails with ENOSPC, which makes the stdout failure path
/// deterministic. Windows has no equivalent device, so this test is Linux-only.
#[cfg(target_os = "linux")]
#[test]
fn unwritable_stdout_is_io_error() {
    for args in [
        &["info"][..],
        &["info", "--json"][..],
        &["--version"][..],
        &["--help"][..],
    ] {
        let full = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap();
        let output = va_command(args)
            .stdout(Stdio::from(full))
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(EXIT_IO), "args: {args:?}");
        let stderr = text(&output.stderr);
        assert!(
            stderr.starts_with("error:"),
            "args: {args:?}, stderr: {stderr}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (audit round 1): short flags, help paths, malformed command lines, non-Unicode
// arguments, and a closed stdout pipe, which makes the I/O error path testable on every OS.
// ---------------------------------------------------------------------------------------------

/// Returns an argument that is not valid Unicode on the current OS: an invalid UTF-8 byte on
/// Unix, an unpaired UTF-16 surrogate on Windows.
fn non_unicode_argument() -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt as _;
        OsString::from_vec(vec![b'i', 0xff, b'o'])
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt as _;
        OsString::from_wide(&[u16::from(b'i'), 0xD800, u16::from(b'o')])
    }
}

#[test]
fn short_flags_behave_like_long_flags() {
    let version = run_va(&["-V"]);
    assert_eq!(version.status.code(), Some(0));
    assert_eq!(text(&version.stdout), format!("{}\n", version_line()));
    assert_eq!(text(&version.stderr), "");

    let help = run_va(&["-h"]);
    assert_eq!(help.status.code(), Some(0));
    let stdout = text(&help.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(stdout.contains("info"), "stdout: {stdout}");
    assert_eq!(text(&help.stderr), "");
}

#[test]
fn help_subcommand_and_subcommand_help_exit_zero_on_stdout() {
    for args in [&["help"][..], &["help", "info"], &["info", "--help"]] {
        let output = run_va(args);
        assert_eq!(output.status.code(), Some(0), "args: {args:?}");
        let stdout = text(&output.stdout);
        assert!(
            stdout.contains("Usage:"),
            "args: {args:?}, stdout: {stdout}"
        );
        assert_eq!(text(&output.stderr), "", "args: {args:?}");
    }
}

#[test]
fn malformed_command_lines_are_usage_errors_with_empty_stdout() {
    for args in [
        &["bogus"][..],
        &[""],
        &["info", "extra"],
        &["info", "--bogus"],
        &["info", "--json=yes"],
        &["--json", "--json", "info"],
        &["info é 文 with spaces"],
        &["INFO"],
    ] {
        let output = run_va(args);
        assert_eq!(output.status.code(), Some(EXIT_USAGE), "args: {args:?}");
        assert_eq!(text(&output.stdout), "", "args: {args:?}");
        let stderr = text(&output.stderr);
        assert!(
            stderr.starts_with("error:"),
            "args: {args:?}, stderr: {stderr}"
        );
    }
}

#[test]
fn non_unicode_argument_is_a_usage_error_not_a_panic() {
    let output = va_command(&[])
        .arg(non_unicode_argument())
        .output()
        .unwrap();
    // 101 would mean a panic, which the conventions reserve for bugs.
    assert_eq!(output.status.code(), Some(EXIT_USAGE));
    assert_eq!(text(&output.stdout), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("error:"), "stderr: {stderr}");
}

#[test]
fn json_output_is_identical_whatever_the_flag_position() {
    let before = run_va(&["--json", "info"]);
    let after = run_va(&["info", "--json"]);
    assert_eq!(before.status.code(), Some(0));
    assert_eq!(after.status.code(), Some(0));
    assert_eq!(before.stdout, after.stdout);
    // Exactly one line: one trailing newline and no other.
    let stdout = text(&before.stdout);
    assert_eq!(stdout.matches('\n').count(), 1, "stdout: {stdout:?}");
    assert!(!stdout.contains('\r'), "stdout: {stdout:?}");
}

/// A pipe whose read end is closed before the child starts makes every write to stdout fail
/// deterministically (EPIPE on Unix, ERROR_NO_DATA on Windows), so the exit code 3 path is
/// covered on all four CI runners, not only on Linux through `/dev/full`.
#[test]
fn closed_stdout_pipe_is_io_error() {
    for args in [
        &["info"][..],
        &["info", "--json"][..],
        &["--version"][..],
        &["--help"][..],
    ] {
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let output = va_command(args)
            .stdout(Stdio::from(writer))
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(EXIT_IO), "args: {args:?}");
        let stderr = text(&output.stderr);
        assert!(
            stderr.starts_with("error:"),
            "args: {args:?}, stderr: {stderr}"
        );
    }
}

/// QA audit round 1 (continuation): §17.10 gives exit 3 when clap's output cannot be written,
/// error output included. A usage error whose message goes to a stderr pipe with no reader
/// therefore exits 3, and stdout stays empty.
#[test]
fn usage_error_on_a_closed_stderr_pipe_is_io_error() {
    for args in [&[][..], &["--bogus"][..], &["info", "extra"][..]] {
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let output = va_command(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::from(writer))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(EXIT_IO), "args: {args:?}");
        assert!(output.stdout.is_empty(), "args: {args:?}");
    }
}
