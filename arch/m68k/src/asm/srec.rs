//! Program outputs (m2-assembler.md §4.12): Motorola S-records byte-identical to `a68k -s`
//! ([`to_srec`]), their parser ([`parse_srec`], also used by `va disasm`) and the flat binary
//! ([`to_binary`]).
//!
//! [`parse_srec`] refuses its input with a [`SrecError`]: the line of the refused record and a
//! [`SrecReason`], whose [`SrecReason::id`] names it (§15.2 W3). The text shown to a user is
//! written by the caller (`va` words each id in English), not by this crate.
//!
//! WP0 scaffold, given its final types by WP-D's day-1 commit (§16.4 G10): the §4.12 signatures
//! and the §15.2 W3 error type; the three functions keep `todo!()` bodies until WP-D implements
//! them.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-D implements m2 §4.12"
)]

use va_core::image::ProgramImage;

/// Why [`parse_srec`] refused its input (§15.2 W3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SrecError {
    /// The 1-based line of the refused record.
    pub line: u32,
    /// Why the record was refused.
    pub reason: SrecReason,
}

/// The reason of a [`SrecError`], in the order the checks run (§15.2 W3): the checks of one
/// record (`NotARecord` to `BadChecksum`, the first failure wins), then the checks across
/// records (`BadCount` to `Overlap`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SrecReason {
    /// The line is not `S` followed by a digit.
    NotARecord,
    /// An `S4` or `S6` record.
    UnsupportedType,
    /// The rest of the record is not an even number of hexadecimal digits (either case).
    BadHex,
    /// The count byte differs from the number of bytes after it, or is below the address field
    /// (2 bytes for S0, S1, S5, S9; 3 for S2, S8; 4 for S3, S7) plus the checksum.
    BadLength,
    /// The checksum is not the ones' complement of the low byte of the sum of the count,
    /// address and data bytes.
    BadChecksum,
    /// An `S5` record whose value differs from the number of S1–S3 records before it.
    BadCount,
    /// A record after the first termination record (S7–S9).
    AfterEnd,
    /// The S1–S3 data passes 16 MiB (`va_core::source::MAX_SOURCE_BYTES`) at this record.
    TooLarge,
    /// The record places a byte that an earlier record already placed.
    Overlap,
}

impl SrecReason {
    /// Returns the kebab-case identifier of the reason: `"not-a-record"`,
    /// `"unsupported-type"`, `"bad-hex"`, `"bad-length"`, `"bad-checksum"`, `"bad-count"`,
    /// `"after-end"`, `"too-large"` or `"overlap"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::NotARecord => "not-a-record",
            Self::UnsupportedType => "unsupported-type",
            Self::BadHex => "bad-hex",
            Self::BadLength => "bad-length",
            Self::BadChecksum => "bad-checksum",
            Self::BadCount => "bad-count",
            Self::AfterEnd => "after-end",
            Self::TooLarge => "too-large",
            Self::Overlap => "overlap",
        }
    }
}

/// Renders `image` as S-records: `S0030000FC`, `S2` records of 16 data bytes (each segment
/// starts a new record), then `S8` with the entry address or `000000`; uppercase hex, CRLF after
/// every record on every OS.
pub fn to_srec(image: &ProgramImage) -> String {
    todo!()
}

/// Parses S-records (S0–S3, S5, S7–S9; LF or CRLF) into an image.
///
/// The text is split at LF, one trailing CR is removed and empty lines are skipped. The data
/// bytes form segments that are the maximal runs of contiguous addresses, kind `Code`; the entry
/// is the S7–S9 address, `None` when it is 0 or when no termination record exists (§15.2 W3).
///
/// # Errors
///
/// A [`SrecError`] for the first refused record: not a record, an unsupported type, bad hex, a
/// bad length or checksum, a wrong S5 count, a record after the end record, more than 16 MiB of
/// data, or a byte placed twice.
pub fn parse_srec(text: &str) -> Result<ProgramImage, SrecError> {
    todo!()
}

/// Returns the lowest address of `image` and its bytes up to the highest one, gaps filled with
/// zeros; `(0, [])` for an empty image.
pub fn to_binary(image: &ProgramImage) -> (u64, Vec<u8>) {
    todo!()
}
