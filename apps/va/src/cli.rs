//! Command-line grammar of `va`, declared with clap's derive API.
//!
//! `va [--json] [--lang en|fr] info | lex <FILE|-> | isa <MNEMONIC[.SIZE]> | isa --list`
//! (m1-core-isa-lexer.md §7.1). clap generates `--help`, `--version` and the usage errors;
//! `main.rs` prints them and maps them to exit codes (docs/specs/m0-foundations.md §5).

use std::ffi::OsString;

use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{ArgGroup, Parser, Subcommand};
use va_workspace::i18n::Lang;

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

    /// Language of diagnostics and instruction documentation (default: VA_LANG, else the locale)
    #[arg(
        long,
        global = true,
        value_name = "LANG",
        value_parser = PossibleValuesParser::new(LANG_CODES).map(lang_from_code)
    )]
    pub lang: Option<Lang>,

    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Subcommands of `va` (architecture §2 lists the later ones).
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print build information.
    Info,
    /// Split a source file into tokens and report the lexer's diagnostics.
    Lex {
        /// Source file to lex, or - for standard input
        #[arg(value_name = "FILE|-")]
        file: OsString,
    },
    /// Describe an instruction: forms, addressing modes, flags and cycle counts.
    // Exactly one of MNEMONIC and --list: both or neither is a usage error (m1 §7.1).
    #[command(group(ArgGroup::new("target").required(true).args(["mnemonic", "list"])))]
    Isa {
        /// Instruction to describe, with an optional size suffix (e.g. move.l)
        #[arg(value_name = "MNEMONIC[.SIZE]")]
        mnemonic: Option<String>,
        /// List every mnemonic with its summary
        #[arg(long)]
        list: bool,
    },
}

/// The values of `--lang`: exactly the shipped languages. `auto` is the absence of the flag
/// (m1 §7.1, §6.2).
const LANG_CODES: [&str; 2] = [Lang::En.code(), Lang::Fr.code()];

/// Maps a value already checked against [`LANG_CODES`] to its language.
fn lang_from_code(code: String) -> Lang {
    // clap has checked the value, so the fallback is never taken.
    Lang::parse(&code).unwrap_or(Lang::En)
}
