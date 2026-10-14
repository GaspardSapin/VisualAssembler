//! Command-line grammar of `va`, declared with clap's derive API.
//!
//! clap generates `--help`, `--version` and the usage errors. `main.rs` prints them and maps them
//! to exit codes (docs/specs/m0-foundations.md §5).

use clap::{Parser, Subcommand};

/// Parsed command line of `va`.
// Keep the doc comment above to one paragraph: clap's derive turns a multi-paragraph doc comment
// into `long_about`, which `--help` would print instead of `about`.
#[derive(Debug, Parser)]
#[command(
    name = "va",
    // Fixed program name for usage lines: `Usage: va [OPTIONS] <COMMAND>` on every OS, not the
    // executable file name (`va.exe` on Windows); docs/specs/m0-foundations.md §17.11 D13.
    bin_name = "va",
    version,
    about = "VisualAssembler command-line interface",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Print one JSON document on stdout instead of human-readable text
    #[arg(long, global = true)]
    pub json: bool,

    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Subcommands of `va`. M0 has only `info`; the M1+ commands are listed in architecture §2.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print build information.
    Info,
}
