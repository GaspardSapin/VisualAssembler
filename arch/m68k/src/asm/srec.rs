//! Program outputs (m2-assembler.md §4.12): Motorola S-records byte-identical to `a68k -s`
//! ([`to_srec`]), their parser ([`parse_srec`], also used by `va disasm`) and the flat binary
//! ([`to_binary`]).
//!
//! WP0 scaffold: the §4.12 signatures with `todo!()` bodies; WP-D implements them. The fields of
//! [`SrecError`] are not frozen by §4.2: WP-D owns them (WP-G reads them for the
//! `input-not-srecord` message).

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-D implements m2 §4.12"
)]

use va_core::image::ProgramImage;

/// Why [`parse_srec`] refused its input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SrecError {
    /// The 1-based line of the offending record.
    pub line: u32,
    /// The kebab-case id of the reason (e.g. `"bad-checksum"`).
    pub reason: &'static str,
}

/// Renders `image` as S-records: `S0030000FC`, `S2` records of 16 data bytes (each segment
/// starts a new record), then `S8` with the entry address or `000000`; uppercase hex, CRLF after
/// every record on every OS.
pub fn to_srec(image: &ProgramImage) -> String {
    todo!()
}

/// Parses S-records (S0–S3, S5, S7–S9; LF or CRLF) into an image.
///
/// # Errors
///
/// A [`SrecError`] for a bad record (checksum, length, hex), more than 16 MiB of data, or
/// overlapping records.
pub fn parse_srec(text: &str) -> Result<ProgramImage, SrecError> {
    todo!()
}

/// Returns the lowest address of `image` and its bytes up to the highest one, gaps filled with
/// zeros; `(0, [])` for an empty image.
pub fn to_binary(image: &ProgramImage) -> (u64, Vec<u8>) {
    todo!()
}
