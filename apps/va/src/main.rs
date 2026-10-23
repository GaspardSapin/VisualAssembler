//! `va`: the headless VisualAssembler command-line interface.
//!
//! The command line is parsed by clap ([`cli::Cli`]) and dispatched to one module per subcommand
//! ([`commands`]). Exit codes (docs/specs/m0-foundations.md §5, m1-core-isa-lexer.md §7.5): 0
//! success, including `--help` and `--version`; 1 the command ran and found problems; 2 usage
//! error; 3 I/O or environment error; 101 panic (a bug). Human errors go to stderr, prefixed
//! `error: `; in `--json` mode a failure also prints a `va.error/1` document on stdout
//! ([`report`]). This binary never depends on a GUI crate (architecture rule R4), so it builds and
//! runs on headless machines.

mod arch_registry;
mod cli;
mod commands;
mod input;
mod report;

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;

use crate::cli::Cli;
use crate::report::{EXIT_IO, EXIT_USAGE};

/// Parses the command line, runs the selected subcommand and returns the process exit code.
fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => return print_parse_outcome(&error),
    };
    match commands::run(&cli) {
        Ok(outcome) => ExitCode::from(outcome.exit_code()),
        Err(error) => ExitCode::from(report::report_failure(&error, cli.json)),
    }
}

/// Prints clap's message for a parse outcome that is not a successful parse, and returns the exit
/// code to use.
///
/// `--help` and `--version` are printed on stdout with exit code 0; every other outcome is a usage
/// error printed on stderr with exit code [`EXIT_USAGE`] (clap's `Error::exit_code`), through
/// [`print_usage_error`]. If the message cannot be written, that failure is reported on stderr and
/// the exit code is [`EXIT_IO`].
fn print_parse_outcome(error: &clap::Error) -> ExitCode {
    let printed = if error.use_stderr() {
        print_usage_error(error)
    } else {
        error.print().and_then(|()| io::stdout().flush())
    };
    match printed {
        // clap only produces 0 or 2; a value outside u8 would be a clap bug and is treated as a
        // usage error rather than a success.
        Ok(()) => ExitCode::from(u8::try_from(error.exit_code()).unwrap_or(EXIT_USAGE)),
        Err(io_error) => {
            report::print_error_line(&format!(
                "cannot print the command-line message: {}",
                report::english_io(&io_error)
            ));
            ExitCode::from(EXIT_IO)
        }
    }
}

/// Writes clap's usage error on stderr, each line passed through [`report::terminal_safe`]
/// (Security M1-S8): the message quotes the offending arguments, which may hold ESC, CR, C1 or
/// bidi characters. The text is rendered without colour. A line feed inside an argument still
/// starts a new line (accepted limitation).
fn print_usage_error(error: &clap::Error) -> io::Result<()> {
    let text = error.render().to_string();
    let safe = text
        .split('\n')
        .map(report::terminal_safe)
        .collect::<Vec<_>>()
        .join("\n");
    let mut stderr = io::stderr().lock();
    stderr.write_all(safe.as_bytes())?;
    stderr.flush()
}
