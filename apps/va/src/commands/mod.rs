//! Subcommand implementations of `va`, one file per subcommand (`info` in M0).
//!
//! Each subcommand returns `Result<(), CommandError>`; `main.rs` turns an error into an
//! `error: ` line on stderr and the exit code given by [`CommandError::exit_code`].

pub mod info;

use std::error::Error;
use std::fmt;
use std::io;

use crate::EXIT_IO;
use crate::cli::{Cli, Command};

/// Failure of a subcommand after the command line was parsed successfully.
#[derive(Debug)]
pub enum CommandError {
    /// The command's output could not be written to stdout (closed pipe, full disk, ...).
    ///
    /// No `va.error/1` document is printed in `--json` mode for this failure: stdout is the very
    /// stream that failed, so the human line on stderr and the exit code are the only channels.
    Stdout(io::Error),
}

impl CommandError {
    /// Process exit code of this failure, following the conventions of
    /// docs/specs/m0-foundations.md §5 (3 = I/O or environment error).
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Stdout(_) => EXIT_IO,
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stdout(error) => write!(f, "cannot write to stdout: {error}"),
        }
    }
}

impl Error for CommandError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Stdout(error) => Some(error),
        }
    }
}

/// Runs the subcommand selected on the command line.
pub fn run(cli: &Cli) -> Result<(), CommandError> {
    match cli.command {
        Command::Info => info::run(cli.json),
    }
}
