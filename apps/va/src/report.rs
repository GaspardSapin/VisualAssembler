//! Exit codes, failures and their reporting (m1-core-isa-lexer.md §7.5, m0-foundations.md §5 and
//! §17.10).
//!
//! A subcommand that cannot do its work returns a [`CommandError`]. [`report_failure`] prints it:
//! the human line `error: <message>` on stderr and, in `--json` mode, the document
//! `{"schema":"va.error/1","code":"<code>","message":"<english>"}` on stdout, unless stdout is the
//! stream that failed. CLI messages are English (m1 §7.5; localizing the CLI chrome is M4), and
//! every I/O error is rendered in English by [`english_io`] (m1 §12.8 F-1). Diagnostics found in
//! a source file are not failures: they are the result of `va lex`.
//!
//! Text written for a terminal passes through [`terminal_safe`] (Security M1-S1, S3, S4): control
//! characters, C1 controls and bidirectional or line-separator format characters coming from the
//! source, a file name, the query or `VA_LANG` cannot reach the terminal raw. JSON documents are
//! not rewritten.

use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::io::{self, Write};

use serde::Serialize;

/// Exit code of a command that ran and found problems: `va lex` with an error diagnostic, `va isa`
/// with an unknown mnemonic or a size the instruction does not accept.
pub const EXIT_PROBLEMS: u8 = 1;

/// Exit code of a usage error (unknown flag, missing subcommand, bad value), as clap reports it.
pub const EXIT_USAGE: u8 = 2;

/// Exit code of an I/O or environment error: unreadable or refused input, an output stream that
/// cannot be written, a build without architecture or with invalid embedded resources.
pub const EXIT_IO: u8 = 3;

/// Schema identifier of the failure document printed on stdout in `--json` mode.
pub const ERROR_SCHEMA: &str = "va.error/1";

/// How a successful command ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing to report: exit code 0.
    Clean,
    /// The command ran and its result holds at least one error (`va lex`): exit code
    /// [`EXIT_PROBLEMS`]. The result document is printed, not a `va.error/1` document.
    ProblemsFound,
}

impl Outcome {
    /// Process exit code of this outcome: 0 or [`EXIT_PROBLEMS`].
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Clean => 0,
            Self::ProblemsFound => EXIT_PROBLEMS,
        }
    }
}

/// Stable kebab-case code of a reported failure: the `code` of the `va.error/1` document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// The input path cannot be opened or read (missing file, directory, permission, stdin error).
    ReadFailed,
    /// The input is larger than `va_core::source::MAX_SOURCE_BYTES` (16 MiB).
    InputTooLarge,
    /// The input is not valid UTF-8.
    InputNotUtf8,
    /// The binary was built without any `arch-*` feature, so no CPU can lex or describe anything.
    NoArchitecture,
    /// The embedded Fluent resources do not load: a build defect.
    LocaleInvalid,
    /// `va isa` was given a spelling that is not an instruction of the CPU.
    IsaUnknownMnemonic,
    /// `va isa` was given a size suffix that no form of the instruction accepts.
    IsaSizeNotAccepted,
}

impl ErrorCode {
    /// Returns the kebab-case identifier of the code, e.g. `"read-failed"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::ReadFailed => "read-failed",
            Self::InputTooLarge => "input-too-large",
            Self::InputNotUtf8 => "input-not-utf8",
            Self::NoArchitecture => "no-architecture",
            Self::LocaleInvalid => "locale-invalid",
            Self::IsaUnknownMnemonic => "isa-unknown-mnemonic",
            Self::IsaSizeNotAccepted => "isa-size-not-accepted",
        }
    }

    /// Returns the process exit code of a failure with this code (m1 §7.5): [`EXIT_PROBLEMS`]
    /// for the `isa-*` lookups, [`EXIT_IO`] for everything else.
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::IsaUnknownMnemonic | Self::IsaSizeNotAccepted => EXIT_PROBLEMS,
            Self::ReadFailed
            | Self::InputTooLarge
            | Self::InputNotUtf8
            | Self::NoArchitecture
            | Self::LocaleInvalid => EXIT_IO,
        }
    }
}

/// Failure of a subcommand after the command line was parsed successfully.
#[derive(Debug)]
pub enum CommandError {
    /// The command's output could not be written to stdout (closed pipe, full disk, ...): exit
    /// code [`EXIT_IO`].
    ///
    /// No `va.error/1` document is printed in `--json` mode for this failure: stdout is the very
    /// stream that failed, so the human line on stderr and the exit code are the only channels.
    Stdout(io::Error),
    /// The diagnostics of `va lex` (text mode) could not be written to stderr: exit code
    /// [`EXIT_IO`]. They are the command's result, so losing them is not silent. Text mode only,
    /// so no `va.error/1` document either.
    Stderr(io::Error),
    /// A failure with a stable code, reported as a `va.error/1` document in `--json` mode.
    Reported {
        /// What failed.
        code: ErrorCode,
        /// The English message, without the `error: ` prefix.
        message: String,
    },
}

impl CommandError {
    /// Builds a [`CommandError::Reported`] failure.
    pub fn reported(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Reported {
            code,
            message: message.into(),
        }
    }

    /// Process exit code of this failure (m1 §7.5).
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Stdout(_) | Self::Stderr(_) => EXIT_IO,
            Self::Reported { code, .. } => code.exit_code(),
        }
    }

    /// The code of the `va.error/1` document for this failure, or `None` when no document is
    /// printed (an output stream failed).
    pub fn code(&self) -> Option<ErrorCode> {
        match self {
            Self::Stdout(_) | Self::Stderr(_) => None,
            Self::Reported { code, .. } => Some(*code),
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stdout(error) => write!(f, "cannot write to stdout: {}", english_io(error)),
            Self::Stderr(error) => write!(f, "cannot write to stderr: {}", english_io(error)),
            Self::Reported { message, .. } => f.write_str(message),
        }
    }
}

impl Error for CommandError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Stdout(error) | Self::Stderr(error) => Some(error),
            Self::Reported { .. } => None,
        }
    }
}

/// The `va.error/1` document. Field order is the JSON key order.
#[derive(Serialize)]
struct ErrorDocument<'a> {
    /// Always [`ERROR_SCHEMA`].
    schema: &'static str,
    /// The failure's [`ErrorCode::id`].
    code: &'static str,
    /// The English message, identical to the human line without its `error: ` prefix.
    message: &'a str,
}

/// Reports `error` and returns the process exit code to use.
///
/// The human line `error: <message>` always goes to stderr. With `json`, a failure that has a
/// code also prints its `va.error/1` line on stdout; if that line cannot be written, the stdout
/// failure is reported too and the exit code becomes [`EXIT_IO`].
pub fn report_failure(error: &CommandError, json: bool) -> u8 {
    let message = error.to_string();
    if json && let Some(code) = error.code() {
        let document = ErrorDocument {
            schema: ERROR_SCHEMA,
            code: code.id(),
            message: &message,
        };
        if let Err(io_error) = write_json_line(&document) {
            print_error_line(&message);
            print_error_line(&format!(
                "cannot write to stdout: {}",
                english_io(&io_error)
            ));
            return EXIT_IO;
        }
    }
    print_error_line(&message);
    error.exit_code()
}

/// Writes `value` as one JSON line on stdout and flushes it.
fn write_json_line<T: Serialize>(value: &T) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    // serde_json converts its error into io::Error, keeping the original I/O error if any.
    serde_json::to_writer(&mut stdout, value)?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

/// Writes `error: <message>` and a newline on stderr, the message passed through
/// [`terminal_safe`] (it may echo a file name or a query).
///
/// stderr is the last reporting channel. If it cannot be written either, nothing else can carry
/// the message, so the write result is dropped on purpose: the caller's non-zero exit code still
/// signals the failure.
pub fn print_error_line(message: &str) {
    let mut stderr = io::stderr().lock();
    let _ = writeln!(stderr, "error: {}", terminal_safe(message));
}

/// Writes `warning: <message>` and a newline on stderr, the message passed through
/// [`terminal_safe`] (it may echo the value of `VA_LANG`).
///
/// A warning never changes the exit code, so a failure to write it is dropped on purpose, like
/// [`print_error_line`].
pub fn print_warning_line(message: &str) {
    let mut stderr = io::stderr().lock();
    let _ = writeln!(stderr, "warning: {}", terminal_safe(message));
}

/// Describes an I/O error in English (m1 §7.5, §12.7, §12.8 F-1): an OS error becomes its kind
/// and number, e.g. `broken pipe (os error 232)` or `entity not found (os error 2)`, because the
/// OS formats its own messages in the user's language (Windows); any other error keeps the
/// standard library's English text.
pub fn english_io(error: &io::Error) -> String {
    match error.raw_os_error() {
        Some(code) => format!("{} (os error {code})", error.kind()),
        None => error.to_string(),
    }
}

/// Tells whether [`terminal_safe`] rewrites `c`: the C0 controls U+0000–U+001F (tab included),
/// DEL U+007F, the C1 controls U+0080–U+009F, the marks U+200E/U+200F, the separators
/// U+2028/U+2029, and the bidirectional embeddings, overrides and isolates U+202A–U+202E and
/// U+2066–U+2069 (Security M1-S1).
fn is_unsafe_for_terminal(c: char) -> bool {
    matches!(
        c,
        '\u{0}'..='\u{1F}'
            | '\u{7F}'..='\u{9F}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2066}'..='\u{2069}'
    )
}

/// Returns `text` with every character of [`is_unsafe_for_terminal`] rewritten in its
/// `char::escape_debug` form (`\t`, `\n`, `\0`, `\u{1b}`, `\u{9b}`, `\u{202e}`, ...); other
/// characters, backslashes included, are kept. Borrows `text` when nothing needs rewriting.
///
/// Used for every human line that carries outside text: the diagnostics, fixed lines, file prefix
/// and token lines of `va lex`, and the `error:` and `warning:` lines. Escape sequences (ESC, BEL,
/// OSC 52, CSI as U+009B) and bidi overrides (U+202E) therefore never reach the terminal.
pub fn terminal_safe(text: &str) -> Cow<'_, str> {
    if !text.chars().any(is_unsafe_for_terminal) {
        return Cow::Borrowed(text);
    }
    let mut safe = String::with_capacity(text.len() + 16);
    for c in text.chars() {
        if is_unsafe_for_terminal(c) {
            let escaped = c.escape_debug();
            // escape_debug keeps a char it deems printable as is; every char of the set is
            // escaped anyway, in the same `\u{…}` notation.
            if escaped.len() > 1 {
                safe.extend(escaped);
            } else {
                safe.extend(c.escape_unicode());
            }
        } else {
            safe.push(c);
        }
    }
    Cow::Owned(safe)
}

/// Returns `quoted`, a JSON string literal (from serde_json), with every character of
/// [`is_unsafe_for_terminal`] written as the JSON escape `\uXXXX` (4 lowercase hex digits, e.g.
/// `\u009b`, `\u202e`, `\u007f`). Borrows `quoted` when nothing needs rewriting.
///
/// serde_json already escapes C0 controls but leaves DEL, C1, the bidi and separator characters
/// raw; after this pass the literal is terminal-safe and still a valid JSON string that decodes
/// to the same text (every character of the set is in the BMP and none is a surrogate). Used for
/// the token lines of `va lex` (m1 §7.3 `<text as a JSON string>`, Security M1-S3).
pub fn terminal_safe_json(quoted: &str) -> Cow<'_, str> {
    if !quoted.chars().any(is_unsafe_for_terminal) {
        return Cow::Borrowed(quoted);
    }
    let mut safe = String::with_capacity(quoted.len() + 16);
    for c in quoted.chars() {
        if is_unsafe_for_terminal(c) {
            safe.push_str(&format!("\\u{:04x}", u32::from(c)));
        } else {
            safe.push(c);
        }
    }
    Cow::Owned(safe)
}
