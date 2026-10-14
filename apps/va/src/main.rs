//! `va`: the headless VisualAssembler command-line interface.
//!
//! The command line is parsed by clap ([`cli::Cli`]) and dispatched to one module per subcommand
//! ([`commands`]). Exit codes (docs/specs/m0-foundations.md §5): 0 success, including `--help` and
//! `--version`; 2 usage error; 3 I/O or environment error (output that cannot be written).
//! Human errors go to stderr, prefixed `error: `. This binary never depends on a GUI crate
//! (architecture rule R4), so it builds and runs on headless machines.

mod cli;
mod commands;

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;

use crate::cli::Cli;

/// Exit code of a usage error (unknown flag, missing subcommand, bad value), as clap reports it.
pub const EXIT_USAGE: u8 = 2;

/// Exit code of an I/O or environment error, such as an output stream that cannot be written.
pub const EXIT_IO: u8 = 3;

/// Parses the command line, runs the selected subcommand and returns the process exit code.
fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => return print_parse_outcome(&error),
    };
    match commands::run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            print_error_line(&error.to_string());
            ExitCode::from(error.exit_code())
        }
    }
}

/// Prints clap's message for a parse outcome that is not a successful parse, and returns the exit
/// code to use.
///
/// `--help` and `--version` are printed on stdout with exit code 0; every other outcome is a usage
/// error printed on stderr with exit code [`EXIT_USAGE`] (clap's `Error::exit_code`). If the
/// message cannot be written, that failure is reported on stderr and the exit code is [`EXIT_IO`].
fn print_parse_outcome(error: &clap::Error) -> ExitCode {
    match error.print().and_then(|()| io::stdout().flush()) {
        // clap only produces 0 or 2; a value outside u8 would be a clap bug and is treated as a
        // usage error rather than a success.
        Ok(()) => ExitCode::from(u8::try_from(error.exit_code()).unwrap_or(EXIT_USAGE)),
        Err(io_error) => {
            print_error_line(&format!(
                "cannot print the command-line message: {io_error}"
            ));
            ExitCode::from(EXIT_IO)
        }
    }
}

/// Writes `error: <message>` and a newline on stderr.
///
/// stderr is the last reporting channel. If it cannot be written either, nothing else can carry
/// the message, so the write result is dropped on purpose: the caller's non-zero exit code still
/// signals the failure.
fn print_error_line(message: &str) {
    let mut stderr = io::stderr().lock();
    let _ = writeln!(stderr, "error: {message}");
}
