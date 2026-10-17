//! Program image and source map (m1-core-isa-lexer.md §3.6): what the M2 assembler produces and
//! the M3 emulator loads. Both land in M1 so that M3 can start its adapter during M2.
//!
//! Addresses are `u64` for every CPU; each architecture masks them to its bus width.

use crate::source::{FileId, TextRange};

/// What a segment holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SegmentKind {
    /// Instructions.
    Code,
    /// Initialized data.
    Data,
    /// Zero-initialized data.
    Bss,
}

/// A contiguous run of bytes at an address.
///
/// Bss bytes are materialized as zeros: the producer fills them, so every consumer reads every
/// kind the same way.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Segment {
    /// Address of the first byte.
    pub address: u64,
    /// What the segment holds.
    pub kind: SegmentKind,
    /// The bytes (zeros for [`SegmentKind::Bss`]).
    pub bytes: Vec<u8>,
}

/// Number of addresses of the 64-bit address space, as a `u128` end bound.
const ADDRESS_SPACE: u128 = 1 << 64;

/// Returns the exclusive end address of `segment`, which may be exactly [`ADDRESS_SPACE`] (or
/// above it for a segment that overflows).
fn segment_end(segment: &Segment) -> u128 {
    u128::from(segment.address) + segment.bytes.len() as u128
}

/// A program ready to load: segments sorted by address, none overlapping, and an optional entry
/// point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramImage {
    /// Segments sorted by address (stable: equal addresses keep their input order).
    segments: Vec<Segment>,
    /// Address where execution starts, if the program defines one.
    entry: Option<u64>,
}

/// Why a set of segments is not a valid [`ProgramImage`]. Indexes are positions in the input
/// vector given to [`ProgramImage::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageError {
    /// Two segments share at least one byte address.
    Overlap {
        /// The smaller input index of the two.
        first: usize,
        /// The larger input index of the two.
        second: usize,
    },
    /// A segment extends past the end of the 64-bit address space.
    AddressOverflow {
        /// Input index of the segment.
        segment: usize,
    },
}

impl ProgramImage {
    /// Validates and sorts `segments` by address.
    ///
    /// Fails on the first segment (in input order) whose last byte is past `u64::MAX`, then on
    /// the first overlap found in address order. Empty segments hold no byte, so they never
    /// overlap; they are kept. `entry` is not checked against the segments.
    pub fn new(mut segments: Vec<Segment>, entry: Option<u64>) -> Result<Self, ImageError> {
        if let Some(segment) = segments.iter().position(|s| segment_end(s) > ADDRESS_SPACE) {
            return Err(ImageError::AddressOverflow { segment });
        }
        let mut order: Vec<usize> = (0..segments.len()).collect();
        order.sort_by_key(|&index| segments[index].address);
        // Furthest end reached so far by a non-empty segment, with that segment's input index.
        let mut reach: Option<(u128, usize)> = None;
        for &index in &order {
            let segment = &segments[index];
            if segment.bytes.is_empty() {
                continue;
            }
            if let Some((end, owner)) = reach
                && u128::from(segment.address) < end
            {
                return Err(ImageError::Overlap {
                    first: owner.min(index),
                    second: owner.max(index),
                });
            }
            let end = segment_end(segment);
            if reach.is_none_or(|(furthest, _)| end > furthest) {
                reach = Some((end, index));
            }
        }
        segments.sort_by_key(|segment| segment.address);
        Ok(Self { segments, entry })
    }

    /// Returns the segments, sorted by address.
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Returns the entry point, if any.
    pub fn entry(&self) -> Option<u64> {
        self.entry
    }

    /// Returns the byte at `address`, or `None` if no segment holds it.
    pub fn byte_at(&self, address: u64) -> Option<u8> {
        let after = self
            .segments
            .partition_point(|segment| segment.address <= address);
        // Non-empty segments do not overlap, so only the last one starting at or before
        // `address` can hold it.
        let segment = self.segments[..after]
            .iter()
            .rev()
            .find(|segment| !segment.bytes.is_empty())?;
        let offset = usize::try_from(address - segment.address).ok()?;
        segment.bytes.get(offset).copied()
    }
}

/// The source of one run of bytes: `len` bytes at `address` come from `line` of `file`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceMapEntry {
    /// Address of the first byte.
    pub address: u64,
    /// Number of bytes (0 for a line that produces none, such as a label alone).
    pub len: u32,
    /// The source file.
    pub file: FileId,
    /// The 1-based source line.
    pub line: u32,
    /// The file bytes of the statement that produced the bytes.
    pub range: TextRange,
}

/// Address ↔ source line mapping, searchable both ways in logarithmic time.
///
/// Entries that hold bytes are expected not to overlap (the assembler emits each byte from one
/// line). When they do, [`at_address`](SourceMap::at_address) only considers the last non-empty
/// entry starting at or before the address.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SourceMap {
    /// Entries sorted by address (stable).
    entries: Vec<SourceMapEntry>,
    /// Indexes into `entries`, sorted by file, line, then address.
    by_line: Vec<usize>,
}

impl SourceMap {
    /// Builds both indexes: entries by address, and (file, line) to addresses.
    pub fn new(mut entries: Vec<SourceMapEntry>) -> Self {
        entries.sort_by_key(|entry| entry.address);
        let mut by_line: Vec<usize> = (0..entries.len()).collect();
        by_line.sort_by_key(|&index| {
            let entry = &entries[index];
            (entry.file, entry.line, entry.address)
        });
        Self { entries, by_line }
    }

    /// Returns the entry whose bytes contain `address`, or `None`.
    pub fn at_address(&self, address: u64) -> Option<&SourceMapEntry> {
        let after = self
            .entries
            .partition_point(|entry| entry.address <= address);
        let entry = self.entries[..after]
            .iter()
            .rev()
            .find(|entry| entry.len > 0)?;
        (address - entry.address < u64::from(entry.len)).then_some(entry)
    }

    /// Returns the address of every entry of `line` in `file`, sorted (one per entry, empty
    /// entries included).
    pub fn addresses(&self, file: FileId, line: u32) -> impl Iterator<Item = u64> + '_ {
        let key = (file, line);
        let key_of = |index: &usize| {
            let entry = &self.entries[*index];
            (entry.file, entry.line)
        };
        let low = self.by_line.partition_point(|index| key_of(index) < key);
        let high = self.by_line.partition_point(|index| key_of(index) <= key);
        self.by_line[low..high]
            .iter()
            .map(|&index| self.entries[index].address)
    }

    /// Returns the entries, sorted by address.
    pub fn entries(&self) -> &[SourceMapEntry] {
        &self.entries
    }
}
