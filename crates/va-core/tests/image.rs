//! Tests for `src/image.rs`: `ProgramImage` sorting, overlap and overflow rejection, `byte_at`,
//! and `SourceMap` lookups at entry edges and `addresses(file, line)` order.
#![allow(clippy::unwrap_used)]

use va_core::image::{ImageError, ProgramImage, Segment, SegmentKind, SourceMap, SourceMapEntry};
use va_core::source::{FileId, TextRange};

/// A code segment at `address` holding `bytes`.
fn code(address: u64, bytes: &[u8]) -> Segment {
    Segment {
        address,
        kind: SegmentKind::Code,
        bytes: bytes.to_vec(),
    }
}

/// A Bss segment of `len` zeros at `address`.
fn bss(address: u64, len: usize) -> Segment {
    Segment {
        address,
        kind: SegmentKind::Bss,
        bytes: vec![0; len],
    }
}

// ---------------------------------------------------------------------------------------------
// ProgramImage
// ---------------------------------------------------------------------------------------------

#[test]
fn segments_are_sorted_by_address() {
    let image = ProgramImage::new(
        vec![code(0x2000, &[3]), code(0x1000, &[1, 2]), bss(0x3000, 4)],
        Some(0x1000),
    )
    .unwrap();
    let addresses: Vec<u64> = image.segments().iter().map(|s| s.address).collect();
    assert_eq!(addresses, vec![0x1000, 0x2000, 0x3000]);
    assert_eq!(image.entry(), Some(0x1000));
    assert_eq!(image.segments()[2].kind, SegmentKind::Bss);
}

#[test]
fn an_empty_image_is_valid() {
    let image = ProgramImage::new(Vec::new(), None).unwrap();
    assert!(image.segments().is_empty());
    assert_eq!(image.entry(), None);
    assert_eq!(image.byte_at(0), None);
}

#[test]
fn adjacent_segments_do_not_overlap() {
    let image = ProgramImage::new(vec![code(4, &[5, 6]), code(0, &[1, 2, 3, 4])], None).unwrap();
    for (address, byte) in (0..6).zip([1, 2, 3, 4, 5, 6]) {
        assert_eq!(image.byte_at(address), Some(byte));
    }
    assert_eq!(image.byte_at(6), None);
}

#[test]
fn overlapping_segments_are_rejected_with_input_indexes() {
    assert_eq!(
        ProgramImage::new(vec![code(0x10, &[0; 4]), code(0x12, &[0; 4])], None),
        Err(ImageError::Overlap {
            first: 0,
            second: 1
        })
    );
    // Same pair, input order reversed: indexes are still smaller first.
    assert_eq!(
        ProgramImage::new(vec![code(0x12, &[0; 4]), code(0x10, &[0; 4])], None),
        Err(ImageError::Overlap {
            first: 0,
            second: 1
        })
    );
    // Same start address.
    assert_eq!(
        ProgramImage::new(
            vec![code(0, &[1]), code(0x40, &[1]), code(0x40, &[2])],
            None
        ),
        Err(ImageError::Overlap {
            first: 1,
            second: 2
        })
    );
}

#[test]
fn a_long_segment_overlapping_a_later_one_is_found() {
    // [0, 100) contains [50, 60), with [10, 20) in between in address order.
    let segments = vec![code(50, &[0; 10]), code(0, &[0; 100]), code(10, &[0; 10])];
    assert_eq!(
        ProgramImage::new(segments, None),
        Err(ImageError::Overlap {
            first: 1,
            second: 2
        })
    );
}

#[test]
fn empty_segments_never_overlap() {
    let image = ProgramImage::new(
        vec![
            code(0, &[1, 2, 3, 4]),
            code(2, &[]),
            code(4, &[]),
            code(0, &[]),
        ],
        None,
    )
    .unwrap();
    assert_eq!(image.segments().len(), 4);
    assert_eq!(
        image.byte_at(2),
        Some(3),
        "the empty segment at 2 hides nothing"
    );
    assert_eq!(image.byte_at(3), Some(4));
    assert_eq!(image.byte_at(4), None);
}

#[test]
fn an_empty_segment_does_not_hide_a_later_overlap() {
    let segments = vec![code(0, &[0; 10]), code(5, &[]), code(8, &[0; 4])];
    assert_eq!(
        ProgramImage::new(segments, None),
        Err(ImageError::Overlap {
            first: 0,
            second: 2
        })
    );
}

#[test]
fn segments_may_end_at_the_top_of_the_address_space() {
    let image = ProgramImage::new(
        vec![
            code(u64::MAX - 1, &[0xAA, 0xBB]),
            code(u64::MAX - 2, &[0x11]),
        ],
        None,
    )
    .unwrap();
    assert_eq!(image.byte_at(u64::MAX - 2), Some(0x11));
    assert_eq!(image.byte_at(u64::MAX - 1), Some(0xAA));
    assert_eq!(image.byte_at(u64::MAX), Some(0xBB));
    assert!(ProgramImage::new(vec![code(u64::MAX, &[1])], None).is_ok());
    assert!(ProgramImage::new(vec![code(u64::MAX, &[])], None).is_ok());
}

#[test]
fn a_segment_past_the_address_space_overflows() {
    assert_eq!(
        ProgramImage::new(vec![code(0, &[1]), code(u64::MAX, &[1, 2])], None),
        Err(ImageError::AddressOverflow { segment: 1 })
    );
    // Overflow is reported before any overlap.
    assert_eq!(
        ProgramImage::new(
            vec![code(0, &[1, 2]), code(1, &[3]), code(u64::MAX - 1, &[0; 3])],
            None
        ),
        Err(ImageError::AddressOverflow { segment: 2 })
    );
}

#[test]
fn byte_at_reads_segment_edges_and_gaps() {
    let image = ProgramImage::new(
        vec![
            code(0x100, &[0x4E, 0x71]),
            bss(0x200, 2),
            code(0x104, &[0x4E, 0x75]),
        ],
        Some(0x100),
    )
    .unwrap();
    assert_eq!(image.byte_at(0xFF), None, "before the first segment");
    assert_eq!(image.byte_at(0x100), Some(0x4E), "first byte");
    assert_eq!(image.byte_at(0x101), Some(0x71), "last byte");
    assert_eq!(image.byte_at(0x102), None, "gap");
    assert_eq!(image.byte_at(0x104), Some(0x4E));
    assert_eq!(image.byte_at(0x105), Some(0x75));
    assert_eq!(image.byte_at(0x200), Some(0), "Bss reads as zero");
    assert_eq!(image.byte_at(0x201), Some(0));
    assert_eq!(image.byte_at(0x202), None, "after the last segment");
    assert_eq!(image.byte_at(u64::MAX), None);
}

// ---------------------------------------------------------------------------------------------
// SourceMap
// ---------------------------------------------------------------------------------------------

/// An entry of `len` bytes at `address`, from `line` of `file`.
fn entry(address: u64, len: u32, file: u32, line: u32) -> SourceMapEntry {
    SourceMapEntry {
        address,
        len,
        file: FileId(file),
        line,
        range: TextRange::new(line * 10, line * 10 + 5),
    }
}

/// A small map: two files, a label line without bytes, a macro line with two entries.
fn sample_map() -> SourceMap {
    SourceMap::new(vec![
        entry(0x108, 4, 0, 5),
        entry(0x100, 2, 0, 1),
        entry(0x102, 0, 0, 2), // label alone
        entry(0x102, 6, 0, 3),
        entry(0x200, 2, 1, 1),
        entry(0x10C, 2, 0, 5), // second entry of line 5 (macro expansion)
        entry(0x0F0, 2, 0, 5), // and one placed before (given out of order)
    ])
}

#[test]
fn entries_are_sorted_by_address() {
    let map = sample_map();
    let addresses: Vec<u64> = map.entries().iter().map(|e| e.address).collect();
    assert_eq!(
        addresses,
        vec![0x0F0, 0x100, 0x102, 0x102, 0x108, 0x10C, 0x200]
    );
}

#[test]
fn at_address_finds_the_containing_entry_at_its_edges() {
    let map = sample_map();
    assert_eq!(map.at_address(0x0EF), None, "before every entry");
    assert_eq!(map.at_address(0x100).unwrap().line, 1, "first byte");
    assert_eq!(map.at_address(0x101).unwrap().line, 1, "last byte");
    assert_eq!(
        map.at_address(0x102).unwrap().line,
        3,
        "the empty label entry holds no byte"
    );
    assert_eq!(
        map.at_address(0x107).unwrap().line,
        3,
        "last byte of line 3"
    );
    assert_eq!(map.at_address(0x108).unwrap().line, 5);
    assert_eq!(map.at_address(0x10D).unwrap().address, 0x10C);
    assert_eq!(map.at_address(0x10E), None, "gap");
    assert_eq!(map.at_address(0x201).unwrap().file, FileId(1));
    assert_eq!(map.at_address(0x202), None, "after every entry");
    assert_eq!(map.at_address(u64::MAX), None);
}

#[test]
fn addresses_of_a_line_are_sorted() {
    let map = sample_map();
    assert_eq!(
        map.addresses(FileId(0), 5).collect::<Vec<_>>(),
        vec![0x0F0, 0x108, 0x10C]
    );
    assert_eq!(map.addresses(FileId(0), 2).collect::<Vec<_>>(), vec![0x102]);
    assert_eq!(map.addresses(FileId(1), 1).collect::<Vec<_>>(), vec![0x200]);
    assert_eq!(map.addresses(FileId(0), 4).count(), 0, "no entry");
    assert_eq!(map.addresses(FileId(2), 1).count(), 0, "unknown file");
}

#[test]
fn an_empty_map_finds_nothing() {
    let map = SourceMap::new(Vec::new());
    assert_eq!(map, SourceMap::default());
    assert!(map.entries().is_empty());
    assert_eq!(map.at_address(0), None);
    assert_eq!(map.addresses(FileId(0), 1).count(), 0);
}

#[test]
fn an_entry_ending_at_the_top_of_the_address_space() {
    let map = SourceMap::new(vec![entry(u64::MAX - 1, 2, 0, 1)]);
    assert_eq!(map.at_address(u64::MAX).unwrap().line, 1);
    assert_eq!(map.at_address(u64::MAX - 2), None);
}
