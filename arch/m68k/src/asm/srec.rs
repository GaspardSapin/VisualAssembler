//! Program outputs (m2-assembler.md §4.12): Motorola S-records byte-identical to `a68k -s`
//! ([`to_srec`]), their parser ([`parse_srec`], also used by `va disasm`) and the flat binary
//! ([`to_binary`]).
//!
//! [`parse_srec`] refuses its input with a [`SrecError`]: the line of the refused record and a
//! [`SrecReason`], whose [`SrecReason::id`] names it (§15.2 W3). The text shown to a user is
//! written by the caller (`va` words each id in English), not by this crate.
//!
//! **Addresses.** The 68000 has a 24-bit address bus: [`to_srec`] writes every address masked
//! to 24 bits, as a68k wraps them (§12.1, probe y2), and a record never crosses the end of that
//! space. The assembler never produces such an image (it reports `asm-address-overflow`), so
//! this only concerns hand-built images. [`parse_srec`] keeps the addresses the records give
//! (S3 and S7 carry 32 bits).

use std::collections::BTreeMap;

use va_core::image::{ProgramImage, Segment, SegmentKind};
use va_core::source::MAX_SOURCE_BYTES;

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

/// The highest address of the 68000 bus (24 bits).
const BUS_MASK: u64 = 0xFF_FFFF;

/// The data bytes of a full `S2` record, as a68k writes them.
const RECORD_DATA: usize = 16;

/// Uppercase hexadecimal digits.
const HEX: &[u8; 16] = b"0123456789ABCDEF";

/// Renders `image` as S-records: `S0030000FC`, `S2` records of 16 data bytes (each segment
/// starts a new record), then `S8` with the entry address or `000000`; uppercase hex, CRLF after
/// every record on every OS.
///
/// Addresses and the entry are masked to 24 bits; a segment that runs past `$FFFFFF` continues
/// in a new record at `$000000` (see the module documentation).
pub fn to_srec(image: &ProgramImage) -> String {
    let mut text = String::new();
    push_record(&mut text, b'0', 0, 2, &[]);
    for segment in image.segments() {
        let mut address = segment.address;
        let mut rest = segment.bytes.as_slice();
        while !rest.is_empty() {
            let bus = address & BUS_MASK;
            // The bytes left before the 24-bit address wraps to 0 (at most 2^24).
            let room = usize::try_from(BUS_MASK - bus + 1).unwrap_or(usize::MAX);
            let (data, tail) = rest.split_at(rest.len().min(RECORD_DATA).min(room));
            push_record(&mut text, b'2', bus, 3, data);
            address = address.wrapping_add(data.len() as u64);
            rest = tail;
        }
    }
    push_record(
        &mut text,
        b'8',
        image.entry().unwrap_or(0) & BUS_MASK,
        3,
        &[],
    );
    text
}

/// Appends one record of type `kind` (an ASCII digit) to `text`: the count, the low
/// `address_len` bytes of `address`, `data` and the checksum, then CRLF.
fn push_record(text: &mut String, kind: u8, address: u64, address_len: usize, data: &[u8]) {
    let mut bytes = Vec::with_capacity(address_len + data.len() + 2);
    // At most 4 + 16 + 1 bytes follow the count.
    bytes.push((address_len + data.len() + 1) as u8);
    bytes.extend_from_slice(&address.to_be_bytes()[8 - address_len..]);
    bytes.extend_from_slice(data);
    bytes.push(!byte_sum(&bytes));
    text.push('S');
    text.push(char::from(kind));
    for byte in bytes {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 0xF)]));
    }
    text.push_str("\r\n");
}

/// The low byte of the sum of `bytes`.
fn byte_sum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0, |sum, byte| sum.wrapping_add(*byte))
}

/// Parses S-records (S0–S3, S5, S7–S9; LF or CRLF) into an image.
///
/// The text is split at LF, one trailing CR is removed and empty lines are skipped. The data
/// bytes form segments that are the maximal runs of contiguous addresses, kind `Code`; the entry
/// is the S7–S9 address, `None` when it is 0 or when no termination record exists (§15.2 W3).
/// The S0 content and the data bytes of S5 and S7–S9 records are ignored.
///
/// # Errors
///
/// A [`SrecError`] for the first refused record: not a record, an unsupported type, bad hex, a
/// bad length or checksum, a wrong S5 count, a record after the end record, more than 16 MiB of
/// data, or a byte placed twice.
pub fn parse_srec(text: &str) -> Result<ProgramImage, SrecError> {
    let mut loader = Loader::default();
    let mut last = 0;
    for (index, line) in text.split('\n').enumerate() {
        let record = line.strip_suffix('\r').unwrap_or(line);
        if record.is_empty() {
            continue;
        }
        last = u32::try_from(index + 1).unwrap_or(u32::MAX);
        let refused = |reason| SrecError { line: last, reason };
        let record = Record::read(record).map_err(refused)?;
        loader.add(record).map_err(refused)?;
    }
    loader
        .finish()
        .map_err(|reason| SrecError { line: last, reason })
}

/// One record whose own checks passed.
struct Record {
    /// The type digit, 0–9 (never 4 or 6).
    kind: u8,
    /// The address field.
    address: u64,
    /// The data bytes, between the address field and the checksum.
    data: Vec<u8>,
}

impl Record {
    /// Reads `line` (without its line end), with the per-record checks of §15.2 W3 in order.
    fn read(line: &str) -> Result<Self, SrecReason> {
        let bytes = line.as_bytes();
        let kind = match bytes {
            [b'S', kind @ b'0'..=b'9', ..] => kind - b'0',
            _ => return Err(SrecReason::NotARecord),
        };
        if matches!(kind, 4 | 6) {
            return Err(SrecReason::UnsupportedType);
        }
        let (pairs, odd) = bytes[2..].as_chunks::<2>();
        if !odd.is_empty() {
            return Err(SrecReason::BadHex);
        }
        let decoded = pairs
            .iter()
            .map(|&[high, low]| Some(nibble(high)? << 4 | nibble(low)?))
            .collect::<Option<Vec<u8>>>()
            .ok_or(SrecReason::BadHex)?;
        let address_len = match kind {
            0 | 1 | 5 | 9 => 2,
            2 | 8 => 3,
            _ => 4,
        };
        let Some((&count, after)) = decoded.split_first() else {
            return Err(SrecReason::BadLength);
        };
        let count = usize::from(count);
        if count != after.len() || count < address_len + 1 {
            return Err(SrecReason::BadLength);
        }
        let (body, checksum) = decoded.split_at(decoded.len() - 1);
        if checksum[0] != !byte_sum(body) {
            return Err(SrecReason::BadChecksum);
        }
        let (address, data) = body[1..].split_at(address_len);
        Ok(Self {
            kind,
            address: address
                .iter()
                .fold(0, |value, byte| value << 8 | u64::from(*byte)),
            data: data.to_vec(),
        })
    }
}

/// The value of one hexadecimal digit, either case.
fn nibble(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    }
}

/// The state of [`parse_srec`] across records.
#[derive(Default)]
struct Loader {
    /// The S1–S3 records seen so far (an S5 must count them).
    data_records: u64,
    /// The data bytes seen so far.
    total: usize,
    /// Whether a termination record (S7–S9) was seen.
    ended: bool,
    /// The entry of the termination record.
    entry: Option<u64>,
    /// The placed bytes as merged runs: start address → end address (exclusive).
    placed: BTreeMap<u64, u64>,
    /// Each non-empty data record: its address and its bytes.
    pieces: Vec<(u64, Vec<u8>)>,
}

impl Loader {
    /// Applies the checks across records (§15.2 W3, in order) to `record` and keeps it.
    fn add(&mut self, record: Record) -> Result<(), SrecReason> {
        if record.kind == 5 && record.address != self.data_records {
            return Err(SrecReason::BadCount);
        }
        if self.ended {
            return Err(SrecReason::AfterEnd);
        }
        match record.kind {
            1..=3 => {
                self.data_records += 1;
                self.total += record.data.len();
                if self.total > MAX_SOURCE_BYTES {
                    return Err(SrecReason::TooLarge);
                }
                if !record.data.is_empty() {
                    self.place(record.address, record.data.len() as u64)?;
                    self.pieces.push((record.address, record.data));
                }
            }
            7..=9 => {
                self.ended = true;
                self.entry = (record.address != 0).then_some(record.address);
            }
            _ => {}
        }
        Ok(())
    }

    /// Records the `len` bytes at `start` as placed, merging them with the runs they touch.
    fn place(&mut self, start: u64, len: u64) -> Result<(), SrecReason> {
        let end = start + len;
        let (mut run_start, mut run_end) = (start, end);
        if let Some((&before, &before_end)) = self.placed.range(..=start).next_back() {
            if before_end > start {
                return Err(SrecReason::Overlap);
            }
            if before_end == start {
                run_start = before;
                self.placed.remove(&before);
            }
        }
        if let Some((&after, &after_end)) = self.placed.range(start..).next() {
            if after < end {
                return Err(SrecReason::Overlap);
            }
            if after == end {
                run_end = after_end;
                self.placed.remove(&after);
            }
        }
        self.placed.insert(run_start, run_end);
        Ok(())
    }

    /// Builds the image: the pieces ordered by address, contiguous ones merged.
    fn finish(mut self) -> Result<ProgramImage, SrecReason> {
        self.pieces.sort_by_key(|(address, _)| *address);
        let mut segments: Vec<Segment> = Vec::new();
        for (address, bytes) in self.pieces {
            match segments.last_mut() {
                Some(last) if last.address + last.bytes.len() as u64 == address => {
                    last.bytes.extend_from_slice(&bytes);
                }
                _ => segments.push(Segment {
                    address,
                    kind: SegmentKind::Code,
                    bytes,
                }),
            }
        }
        // The runs were checked for overlap and end below 2^33, so the image is valid.
        ProgramImage::new(segments, self.entry).map_err(|_| SrecReason::Overlap)
    }
}

/// Returns the lowest address of `image` and its bytes up to the highest one, gaps filled with
/// zeros; `(0, [])` for an empty image.
///
/// Empty segments hold no byte and are left out. An image from the assembler spans at most the
/// 16 MiB of the 68000 bus; a hand-built image spanning more than the memory of the machine
/// cannot be allocated.
pub fn to_binary(image: &ProgramImage) -> (u64, Vec<u8>) {
    let mut filled = image
        .segments()
        .iter()
        .filter(|segment| !segment.bytes.is_empty())
        .peekable();
    // Segments are sorted by address, so the first non-empty one holds the lowest byte.
    let Some(base) = filled.peek().map(|segment| segment.address) else {
        return (0, Vec::new());
    };
    let mut bytes = Vec::new();
    for segment in filled {
        let offset = usize::try_from(segment.address - base).unwrap_or(usize::MAX);
        let end = offset.saturating_add(segment.bytes.len());
        if bytes.len() < end {
            bytes.resize(end, 0);
        }
        bytes[offset..end].copy_from_slice(&segment.bytes);
    }
    (base, bytes)
}
