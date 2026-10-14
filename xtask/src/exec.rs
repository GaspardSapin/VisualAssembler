//! External commands: spawn them explicitly and turn every failure into an error with context.
//!
//! Commands are always spawned with an argument vector, never through a shell (architecture §9.5).

use std::process::{Command, ExitStatus, Stdio};

use anyhow::{Context as _, Result, bail};

/// Renders `cmd` as `program arg1 arg2 …` for messages (lossy for non-UTF-8 arguments).
pub fn describe(cmd: &Command) -> String {
    let mut text = cmd.get_program().to_string_lossy().into_owned();
    for arg in cmd.get_args() {
        text.push(' ');
        text.push_str(&arg.to_string_lossy());
    }
    text
}

/// Runs `cmd` with inherited stdio and waits for it.
///
/// Fails when the program cannot be started or exits with a non-zero status.
pub fn run(cmd: &mut Command) -> Result<()> {
    let status = status(cmd)?;
    if !status.success() {
        bail!("`{}` failed ({status})", describe(cmd));
    }
    Ok(())
}

/// Runs `cmd` with inherited stdio and returns its exit status, whatever it is.
///
/// Fails only when the program cannot be started.
pub fn status(cmd: &mut Command) -> Result<ExitStatus> {
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    cmd.status()
        .with_context(|| format!("cannot start `{}`", describe(cmd)))
}

/// Runs `cmd` with stdin closed and stdout/stderr captured, and returns its stdout.
///
/// Fails when the program cannot be started, exits with a non-zero status (the error then
/// carries its stderr), or prints something that is not UTF-8 on stdout.
pub fn stdout_of(cmd: &mut Command) -> Result<String> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = cmd
        .output()
        .with_context(|| format!("cannot start `{}`", describe(cmd)))?;
    if !output.status.success() {
        bail!(
            "`{}` failed ({}): {}",
            describe(cmd),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout)
        .with_context(|| format!("`{}` printed non-UTF-8 output", describe(cmd)))
}
