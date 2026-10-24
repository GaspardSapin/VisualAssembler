//! Subcommand implementations of `va`, one file per subcommand (`info`, `lex`, `isa`).
//!
//! Each subcommand returns `Result<Outcome, CommandError>` (see [`crate::report`]); `main.rs`
//! turns the outcome into exit code 0 or 1, and an error into its report and exit code.
//! [`Session`] is the shared set-up of the commands that render localized text.
//!
//! `asm` and `disasm` (m2-assembler.md §8) are WP0 shells: they parse and then report that they
//! are not implemented yet ([`not_implemented`]). WP-G adds `asm.rs` and `disasm.rs` and
//! replaces the shells.

pub mod info;
pub mod isa;
pub mod lex;

use std::io::{self, Write};

use serde::Serialize;
use va_core::arch::Architecture;
use va_workspace::i18n::{Lang, Localizer, resolve_lang};

use crate::arch_registry;
use crate::cli::{Cli, Command};
use crate::report::{self, CommandError, ERROR_SCHEMA, ErrorCode, Outcome};

/// What `va lex` and `va isa` need: the CPU and a localizer in the chosen language.
pub struct Session {
    /// The architecture that lexes and describes (the registry's default).
    pub arch: &'static dyn Architecture,
    /// The CPU's messages in the chosen language, English as the fallback.
    pub localizer: Localizer,
}

impl Session {
    /// Chooses the language, the architecture and builds the localizer, in that order.
    ///
    /// The language comes from `flag` (`--lang`), else `VA_LANG`, else the locale variables
    /// (m1 §6.2, §7.1). Variables are read with `var_os` and decoded lossily, so a non-Unicode
    /// `VA_LANG` is reported too; with a flag, no variable is read. A rejected `VA_LANG` prints
    /// `warning: ignoring VA_LANG=<value> (expected en, fr or auto)` on stderr, the value passed
    /// through `report::terminal_safe` (m1 §12.8 N-2, Security M1-S4), so it stays on one line
    /// and carries no escape sequence.
    ///
    /// # Errors
    ///
    /// `no-architecture` when the binary has no `arch-*` feature; `locale-invalid` when the
    /// embedded resources do not load (a build defect), with the localizer's English message.
    pub fn open(flag: Option<Lang>) -> Result<Self, CommandError> {
        let choice = resolve_lang(flag, &|name| {
            std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
        });
        if let Some(value) = &choice.ignored_va_lang {
            report::print_warning_line(&format!(
                "ignoring VA_LANG={value} (expected en, fr or auto)"
            ));
        }
        let arch = arch_registry::default_architecture().ok_or_else(|| {
            CommandError::reported(
                ErrorCode::NoArchitecture,
                "this build of va has no CPU architecture (it was built without an arch-* feature)",
            )
        })?;
        let localizer = Localizer::for_architecture(choice.lang, arch)
            .map_err(|error| CommandError::reported(ErrorCode::LocaleInvalid, error.to_string()))?;
        Ok(Self { arch, localizer })
    }
}

/// Runs the subcommand selected on the command line.
pub fn run(cli: &Cli) -> Result<Outcome, CommandError> {
    let lang = cli.lang;
    match &cli.command {
        Command::Info => info::run(cli.json).map(|()| Outcome::Clean),
        Command::Lex { file } => lex::run(&Session::open(lang)?, file, cli.json),
        // clap guarantees exactly one of MNEMONIC and --list.
        Command::Isa { mnemonic, .. } => {
            let session = Session::open(lang)?;
            match mnemonic {
                Some(query) => isa::run(&session, query, cli.json),
                None => isa::run_list(&session, cli.json),
            }
        }
        Command::Asm(_) => not_implemented("asm", cli.json),
        Command::Disasm(_) => not_implemented("disasm", cli.json),
    }
}

/// The `code` of the `va.error/1` document printed by [`not_implemented`]; it leaves with the
/// shells (WP-G).
const NOT_IMPLEMENTED_CODE: &str = "not-implemented";

/// The `va.error/1` document of [`not_implemented`]. Field order is the JSON key order.
#[derive(Serialize)]
struct NotImplementedDocument<'a> {
    /// Always [`ERROR_SCHEMA`].
    schema: &'static str,
    /// Always [`NOT_IMPLEMENTED_CODE`].
    code: &'static str,
    /// The English message, identical to the human line without its `error: ` prefix.
    message: &'a str,
}

/// WP0 shell of `va asm` and `va disasm` (m2-assembler.md §1): reports that `command` is not
/// implemented yet and ends with exit code 1, without reading any input.
///
/// stderr gets `error: va <command> is not implemented yet (…)`; with `json`, stdout also gets
/// a `va.error/1` document with the code [`NOT_IMPLEMENTED_CODE`]. The failure is not a usage
/// error: the command line was valid.
///
/// # Errors
///
/// A stdout write failure in `--json` mode (exit 3).
fn not_implemented(command: &str, json: bool) -> Result<Outcome, CommandError> {
    let message =
        format!("va {command} is not implemented yet (m2-assembler.md, work package WP-G)");
    if json {
        let document = NotImplementedDocument {
            schema: ERROR_SCHEMA,
            code: NOT_IMPLEMENTED_CODE,
            message: &message,
        };
        let mut stdout = io::stdout().lock();
        serde_json::to_writer(&mut stdout, &document)
            .map_err(io::Error::from)
            .and_then(|()| stdout.write_all(b"\n"))
            .and_then(|()| stdout.flush())
            .map_err(CommandError::Stdout)?;
    }
    report::print_error_line(&message);
    Ok(Outcome::ProblemsFound)
}
