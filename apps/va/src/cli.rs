//! Command-line grammar of `va`, declared with clap's derive API.
//!
//! `va [--json] [--lang en|fr] info | lex <FILE|-> | isa <MNEMONIC[.SIZE]> | isa --list`
//! (m1-core-isa-lexer.md §7.1), and `asm <FILE|-> [-o <OUT|->] [--format srec|bin]
//! [--listing[=<PATH>]] [--map[=<PATH>]] [-I <DIR>]... [--no-opt]` and `disasm <FILE|->
//! [--format srec|bin] [--org <ADDR>]` (m2-assembler.md §8.1). clap generates `--help`,
//! `--version` and the usage errors; `main.rs` prints them and maps them to exit codes
//! (docs/specs/m0-foundations.md §5).

use std::ffi::OsString;

use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};
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
    /// Assemble a source file into S-records or a binary image.
    Asm(AsmArgs),
    /// Disassemble S-records or a binary image.
    Disasm(DisasmArgs),
}

/// The arguments of `va asm` (m2-assembler.md §8.1).
///
/// WP0 shell: the grammar of §8.1 as declared here; WP-G gives the arguments their meaning
/// (default output paths, `output-required`, `output-is-input`, `stdout-conflict`).
#[derive(Debug, Args)]
#[expect(
    dead_code,
    reason = "WP0 shell: WP-G implements va asm (m2 §8) and reads every field"
)]
pub struct AsmArgs {
    /// Source file to assemble, or - for standard input
    #[arg(value_name = "FILE|-")]
    pub file: OsString,
    /// Output file, or - for standard output (default: FILE with the extension .hex or .bin)
    #[arg(short = 'o', value_name = "OUT|-")]
    pub output: Option<OsString>,
    /// Output format
    #[arg(long, value_enum, value_name = "FORMAT", default_value_t = ImageFormat::Srec)]
    pub format: ImageFormat,
    /// Also write the listing (default: OUT with the extension .lst)
    #[arg(long, value_name = "PATH", num_args = 0..=1, require_equals = true)]
    pub listing: Option<Option<OsString>>,
    /// Also write the source map as JSON (default: OUT with the extension .map)
    #[arg(long, value_name = "PATH", num_args = 0..=1, require_equals = true)]
    pub map: Option<Option<OsString>>,
    /// Add a folder where INCLUDE and INCBIN look for files, after the folder of FILE
    #[arg(short = 'I', value_name = "DIR")]
    pub include: Vec<OsString>,
    /// Switch every optimization off (as a68k -n)
    #[arg(long)]
    pub no_opt: bool,
}

/// The arguments of `va disasm` (m2-assembler.md §8.1).
///
/// WP0 shell: the grammar of §8.1 as declared here; WP-G gives the arguments their meaning
/// (reading `--org` as `$1000`, `0x1000` or decimal, for binary input only).
#[derive(Debug, Args)]
#[expect(
    dead_code,
    reason = "WP0 shell: WP-G implements va disasm (m2 §8) and reads every field"
)]
pub struct DisasmArgs {
    /// S-record or binary file to disassemble, or - for standard input
    #[arg(value_name = "FILE|-")]
    pub file: OsString,
    /// Input format
    #[arg(long, value_enum, value_name = "FORMAT", default_value_t = ImageFormat::Srec)]
    pub format: ImageFormat,
    /// Load address of a binary input (default: 0)
    #[arg(long, value_name = "ADDR")]
    pub org: Option<String>,
}

/// The file format of a program image (`--format` of `va asm` and `va disasm`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub enum ImageFormat {
    /// Motorola S-records (S0, S2, S8), as a68k -s writes them
    Srec,
    /// Raw bytes from the lowest address to the highest, gaps filled with zeros
    Bin,
}

/// The values of `--lang`: exactly the shipped languages. `auto` is the absence of the flag
/// (m1 §7.1, §6.2).
const LANG_CODES: [&str; 2] = [Lang::En.code(), Lang::Fr.code()];

/// Maps a value already checked against [`LANG_CODES`] to its language.
fn lang_from_code(code: String) -> Lang {
    // clap has checked the value, so the fallback is never taken.
    Lang::parse(&code).unwrap_or(Lang::En)
}
