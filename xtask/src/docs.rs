//! `cargo xtask docs`: regenerate `AGENCEMENT.md`, then run the project documentation check.
//!
//! The Python interpreter is `$VA_PYTHON`, else `python` on Windows, else `python3`.
//! *Rejected:* trying `python3` first on Windows, where it can resolve to the Microsoft Store stub.

use std::{ffi::OsString, path::Path, process::Command};

use anyhow::{Context as _, Result};

use crate::exec;

/// Environment variable that overrides the Python interpreter.
pub const PYTHON_ENV: &str = "VA_PYTHON";

/// Script that regenerates `AGENCEMENT.md` (relative to the workspace root).
pub const GENERATE_SCRIPT: &str = "scripts/gen_agencement.py";

/// Script that verifies the project documentation (relative to the workspace root).
pub const CHECK_SCRIPT: &str = "scripts/check_project_docs.py";

/// Arguments of `cargo xtask docs`.
#[derive(Debug, Clone, Default, clap::Args)]
pub struct DocsArgs {
    /// Only verify: skip regenerating AGENCEMENT.md.
    #[arg(long)]
    pub check: bool,
}

/// Picks the Python interpreter from the value of `VA_PYTHON` (an empty value counts as unset).
pub fn python_program(override_value: Option<OsString>) -> OsString {
    match override_value {
        Some(value) if !value.is_empty() => value,
        _ if cfg!(windows) => OsString::from("python"),
        _ => OsString::from("python3"),
    }
}

/// Runs the generator (unless `--check`), then the check script, and returns the check
/// script's exit code.
///
/// Fails (exit code 1 in `main`) when Python cannot be started or the generator fails. A check
/// script killed without an exit code, or with a code outside 0..=255, maps to 1.
pub fn run(root: &Path, args: &DocsArgs) -> Result<u8> {
    let python = python_program(std::env::var_os(PYTHON_ENV));
    let hint = || {
        format!(
            "Python interpreter `{}` (set {PYTHON_ENV} to choose another one)",
            python.to_string_lossy()
        )
    };
    if !args.check {
        let mut generate = Command::new(&python);
        generate.current_dir(root).args([GENERATE_SCRIPT, "."]);
        exec::run(&mut generate).with_context(hint)?;
    }
    let mut check = Command::new(&python);
    check.current_dir(root).args([CHECK_SCRIPT, "."]);
    let status = exec::status(&mut check).with_context(hint)?;
    Ok(status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1))
}
