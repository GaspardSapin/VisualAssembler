//! Reading a source input for `va lex` (m1-core-isa-lexer.md §7.2).
//!
//! The file, or stdin for `-`, is read through `Read::take(MAX_SOURCE_BYTES + 1)`, so an oversized
//! input is never read whole, then validated by `SourceText::from_bytes` (size, UTF-8, one BOM
//! stripped). Every failure exits 3: `read-failed` (unreadable path, directory, stdin error),
//! `input-too-large` (more than 16 MiB) or `input-not-utf8` (with the byte offset). Bytes are
//! never decoded lossily: encoding conversion is an M4 editor feature.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Read};

use va_core::source::{MAX_SOURCE_BYTES, SourceError, SourceText};

use crate::report::{CommandError, ErrorCode};

/// The argument that selects standard input instead of a file.
pub const STDIN_ARGUMENT: &str = "-";

/// One byte more than the limit: reading that many proves the input is too large without reading
/// the rest of it.
const READ_LIMIT: u64 = MAX_SOURCE_BYTES as u64 + 1;

/// A validated source input.
#[derive(Debug)]
pub struct Input {
    /// The argument as given, decoded lossily (`-` for stdin): the `file` of `va.lex/1` and the
    /// prefix of the GNU-style diagnostic lines.
    pub name: String,
    /// The validated text, without its BOM.
    pub text: SourceText,
}

/// Reads and validates the input named by `argument`: a path, or [`STDIN_ARGUMENT`] for stdin.
///
/// # Errors
///
/// A [`CommandError::Reported`] with [`ErrorCode::ReadFailed`], [`ErrorCode::InputTooLarge`] or
/// [`ErrorCode::InputNotUtf8`], all exit code 3.
pub fn read(argument: &OsStr) -> Result<Input, CommandError> {
    let name = argument.to_string_lossy().into_owned();
    let is_stdin = argument == STDIN_ARGUMENT;
    let described = if is_stdin {
        "standard input".to_owned()
    } else {
        format!("'{name}'")
    };
    let bytes = if is_stdin {
        read_limited(io::stdin().lock())
    } else {
        File::open(argument).and_then(read_limited)
    }
    .map_err(|error| {
        // Windows refuses to open a directory with "access denied" and Linux fails the read
        // with EISDIR: both are reported as what they are.
        let reason = if !is_stdin && fs::metadata(argument).is_ok_and(|meta| meta.is_dir()) {
            "it is a directory".to_owned()
        } else {
            english(&error)
        };
        CommandError::reported(
            ErrorCode::ReadFailed,
            format!("cannot read {described}: {reason}"),
        )
    })?;
    let text = SourceText::from_bytes(bytes).map_err(|error| match error {
        SourceError::TooLarge { .. } => CommandError::reported(
            ErrorCode::InputTooLarge,
            format!(
                "{described} is larger than the input limit of 16 MiB ({MAX_SOURCE_BYTES} bytes)"
            ),
        ),
        SourceError::NotUtf8 { offset } => CommandError::reported(
            ErrorCode::InputNotUtf8,
            format!(
                "{described} is not valid UTF-8: invalid byte at offset {offset}; save it as UTF-8"
            ),
        ),
    })?;
    Ok(Input { name, text })
}

/// Describes an I/O error in English: the OS formats its own messages in the user's language
/// (Windows), while CLI messages and `va.error/1` messages are English (m1 §7.5). An OS error
/// becomes its kind and number, e.g. `entity not found (os error 2)`; other errors keep the
/// standard library's English text.
fn english(error: &io::Error) -> String {
    match error.raw_os_error() {
        Some(code) => format!("{} (os error {code})", error.kind()),
        None => error.to_string(),
    }
}

/// Reads at most [`READ_LIMIT`] bytes from `reader`.
fn read_limited(reader: impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(READ_LIMIT).read_to_end(&mut bytes)?;
    Ok(bytes)
}
