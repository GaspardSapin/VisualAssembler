//! S-record and binary output (`src/asm/srec.rs`; m2 §4.12, §10.1): the exact §12.1 probe texts,
//! CRLF, the empty image, `parse_srec` errors and round trip, binary base and gap fill (WP-D).
//!
//! The images are built by hand (rule G1 of §15.3: `assemble` is another package). The probe
//! sources of §12.1 are not part of the repository, which keeps what they showed: each probe test
//! builds an image with that property and pins its whole text. The checksums were computed
//! independently of `srec.rs` (ones' complement of the byte sum, §15.2 W3).
#![allow(clippy::unwrap_used)]

use va_arch_m68k::asm::srec::{SrecError, SrecReason, parse_srec, to_binary, to_srec};
use va_core::image::{ProgramImage, Segment, SegmentKind};
use va_core::source::MAX_SOURCE_BYTES;

/// The opening record of every a68k S-record file (no module name).
const S0: &str = "S0030000FC";

/// The termination record without an entry address.
const S8_NONE: &str = "S804000000FB";

/// An image of `Code` segments.
fn image(segments: &[(u64, &[u8])], entry: Option<u64>) -> ProgramImage {
    segments_image(
        segments
            .iter()
            .map(|(address, bytes)| (*address, SegmentKind::Code, bytes.to_vec()))
            .collect(),
        entry,
    )
}

/// An image of the given segments, kinds included.
fn segments_image(segments: Vec<(u64, SegmentKind, Vec<u8>)>, entry: Option<u64>) -> ProgramImage {
    let segments = segments
        .into_iter()
        .map(|(address, kind, bytes)| Segment {
            address,
            kind,
            bytes,
        })
        .collect();
    ProgramImage::new(segments, entry).unwrap()
}

/// The records joined with CRLF after each, as a68k writes them.
fn crlf(records: &[&str]) -> String {
    records
        .iter()
        .map(|record| format!("{record}\r\n"))
        .collect()
}

/// The records joined with LF after each.
fn lf(records: &[&str]) -> String {
    records.iter().map(|record| format!("{record}\n")).collect()
}

/// The line and reason id with which `parse_srec` refuses `text`.
fn refused(text: &str) -> (u32, &'static str) {
    let SrecError { line, reason } = parse_srec(text).unwrap_err();
    (line, reason.id())
}

/// The program of probe t1's shape at `$1000`: 18 bytes, so one full record and one of 2.
const T1_CODE: [u8; 18] = [
    0x20, 0x3C, 0x12, 0x34, 0x56, 0x78, 0x72, 0x05, 0xD0, 0x81, 0x52, 0x42, 0x56, 0x43, 0x51, 0x84,
    0x4E, 0x75,
];

#[test]
fn reason_ids_are_the_nine_ids_of_w3() {
    let reasons = [
        (SrecReason::NotARecord, "not-a-record"),
        (SrecReason::UnsupportedType, "unsupported-type"),
        (SrecReason::BadHex, "bad-hex"),
        (SrecReason::BadLength, "bad-length"),
        (SrecReason::BadChecksum, "bad-checksum"),
        (SrecReason::BadCount, "bad-count"),
        (SrecReason::AfterEnd, "after-end"),
        (SrecReason::TooLarge, "too-large"),
        (SrecReason::Overlap, "overlap"),
    ];
    for (reason, id) in reasons {
        assert_eq!(reason.id(), id);
    }
}

#[test]
fn probe_t1_code_with_an_entry() {
    // §12.1: S2 records of 16 data bytes, then S8 with the END address (t1: S804001000EB).
    let text = to_srec(&image(&[(0x1000, &T1_CODE)], Some(0x1000)));
    assert_eq!(
        text,
        crlf(&[
            S0,
            "S214001000203C123456787205D081524256435184A1",
            "S2060010104E7516",
            "S804001000EB",
        ])
    );
}

#[test]
fn probe_t2_a_new_record_at_each_org_discontinuity() {
    // Two ORG blocks and no END operand: each segment starts a record, S8 holds 000000.
    let text = to_srec(&image(
        &[(0x1000, &[0x4E, 0x71, 0x4E, 0x71]), (0x2000, &[0x4E, 0x75])],
        None,
    ));
    assert_eq!(
        text,
        crlf(&[S0, "S2080010004E714E7169", "S2060020004E7516", S8_NONE])
    );
}

#[test]
fn probe_y5_records_start_at_the_segment_not_at_a_multiple_of_16() {
    let bytes: Vec<u8> = (0..17).collect();
    let text = to_srec(&image(&[(0x1003, &bytes)], Some(0x1004)));
    assert_eq!(
        text,
        crlf(&[
            S0,
            "S214001003000102030405060708090A0B0C0D0E0F60",
            "S20500101310C7",
            "S804001004E7",
        ])
    );
}

#[test]
fn probe_y2_addresses_past_24_bits_wrap() {
    // a68k wraps addresses past $FFFFFF (§12.1, y2); the assembler refuses them
    // (`asm-address-overflow`), so only a hand-built image has them.
    let text = to_srec(&image(&[(0x0100_0400, &[0x4E, 0x75])], Some(0x0100_0400)));
    assert_eq!(text, crlf(&[S0, "S2060004004E7532", "S804000400F7"]));
    // A record never crosses the end of the 24-bit space.
    let text = to_srec(&image(&[(0xFF_FFFC, &[1, 2, 3, 4, 5, 6])], None));
    assert_eq!(
        text,
        crlf(&[S0, "S208FFFFFC01020304F3", "S2060000000506EE", S8_NONE])
    );
}

#[test]
fn the_empty_image_is_the_two_framing_records() {
    assert_eq!(to_srec(&image(&[], None)), crlf(&[S0, S8_NONE]));
    // Empty segments hold no byte and give no record.
    assert_eq!(
        to_srec(&image(&[(0x1000, &[])], Some(0x1000))),
        crlf(&[S0, "S804001000EB"])
    );
}

#[test]
fn every_record_ends_with_crlf_on_every_os() {
    let text = to_srec(&image(&[(0x1000, &T1_CODE)], None));
    assert!(text.ends_with("\r\n"));
    assert_eq!(text.matches('\n').count(), 4);
    assert_eq!(text.matches("\r\n").count(), 4, "no bare LF");
    assert_eq!(text.matches('\r').count(), 4, "no bare CR");
}

#[test]
fn adjacent_segments_each_start_a_record() {
    let text = to_srec(&segments_image(
        vec![
            (0x1000, SegmentKind::Code, vec![0x4E, 0x71]),
            (0x1002, SegmentKind::Data, vec![0x12, 0x34]),
        ],
        None,
    ));
    assert_eq!(
        text,
        crlf(&[S0, "S2060010004E712A", "S2060010021234A1", S8_NONE])
    );
}

#[test]
fn the_round_trip_gives_the_image_back() {
    let long: Vec<u8> = (0..=255).cycle().take(1000).collect();
    let images = [
        image(&[], None),
        image(&[(0x1000, &T1_CODE)], Some(0x1000)),
        image(&[(0x1000, &[0x4E, 0x71]), (0x2000, &[0x4E, 0x75])], None),
        image(
            &[(0, &[0xFF]), (0x1003, &long), (0xFF_FFF0, &[7; 16])],
            Some(0xFF_FFFE),
        ),
    ];
    for image in images {
        assert_eq!(parse_srec(&to_srec(&image)).unwrap(), image);
    }
    // `to_srec` writes 000000 for "no entry", so an entry of 0 comes back as `None`.
    let parsed = parse_srec(&to_srec(&image(&[(0, &[0x4E, 0x75])], Some(0)))).unwrap();
    assert_eq!(parsed, image(&[(0, &[0x4E, 0x75])], None));
}

#[test]
fn segments_are_merged_whatever_the_record_order_and_lengths() {
    let expected = image(&[(0x1000, &[1, 2, 3, 4, 0xAA, 0xBB, 5, 6])], Some(0x1000));
    let in_order = [
        "S107100001020304DE",
        "S1051004AABB81",
        "S10510060506D9",
        "S9031000EC",
    ];
    let reversed = [
        "S10510060506D9",
        "S1051004AABB81",
        "S107100001020304DE",
        "S9031000EC",
    ];
    let one_record = ["S10B100001020304AABB05066A", "S804001000EB"];
    for records in [&in_order[..], &reversed[..], &one_record[..]] {
        assert_eq!(parse_srec(&crlf(records)).unwrap(), expected, "{records:?}");
    }
    // Every segment is `Code`, also when the writer started a record per segment.
    let text = crlf(&[S0, "S2060010004E712A", "S2060010021234A1", S8_NONE]);
    assert_eq!(
        parse_srec(&text).unwrap(),
        image(&[(0x1000, &[0x4E, 0x71, 0x12, 0x34])], None)
    );
}

#[test]
fn every_record_type_and_line_end_is_read() {
    // S1/S9, S2/S8 and S3/S7 addresses; S0 content and S5 are only checked.
    let text = lf(&[
        "S008000048454C4C4F83",
        "S107100001020304DE",
        "S3070001234501028C",
        "S5030002FA",
        "S70512345678E6",
    ]);
    assert_eq!(
        parse_srec(&text).unwrap(),
        image(
            &[(0x1000, &[1, 2, 3, 4]), (0x0001_2345, &[1, 2])],
            Some(0x1234_5678)
        )
    );
    assert_eq!(
        parse_srec("S9031234B6").unwrap().entry(),
        Some(0x1234),
        "a final line end is optional"
    );
    assert_eq!(parse_srec("S70500000000FA\r\n").unwrap().entry(), None);
    // Lowercase hex digits, empty lines (also a lone CR) and LF or CRLF in one text.
    let text = "\r\nS0030000fc\n\nS107100001020304de\r\n\r\nS804001000eb\n";
    assert_eq!(
        parse_srec(text).unwrap(),
        image(&[(0x1000, &[1, 2, 3, 4])], Some(0x1000))
    );
    // No termination record: no entry. A data record without data places nothing.
    assert_eq!(
        parse_srec(&crlf(&["S1032000DC", "S5030001FB"])).unwrap(),
        image(&[], None)
    );
    assert_eq!(parse_srec("").unwrap(), image(&[], None));
}

#[test]
fn each_record_check_has_its_reason() {
    let cases = [
        ("X0030000FC", "not-a-record"),
        ("s0030000FC", "not-a-record"),
        ("S", "not-a-record"),
        ("SX030000FC", "not-a-record"),
        (" S0030000FC", "not-a-record"),
        ("S4030000FC", "unsupported-type"),
        ("S6040000000B", "unsupported-type"),
        ("S1030000F", "bad-hex"),
        ("S10300G0FC", "bad-hex"),
        ("S0030000FC ", "bad-hex"),
        ("S1030000\u{e9}", "bad-hex"),
        ("S1", "bad-length"),
        ("S1040000FB", "bad-length"),
        ("S10200FD", "bad-length"),
        ("S1020000", "bad-length"),
        ("S2030000FC", "bad-length"),
        ("S30400000000", "bad-length"),
        ("S0030000FD", "bad-checksum"),
        ("S107100001020304DF", "bad-checksum"),
    ];
    for (record, id) in cases {
        assert_eq!(refused(record), (1, id), "{record:?}");
    }
}

#[test]
fn the_first_failing_check_of_a_record_wins() {
    // Unsupported type before bad hex, bad hex before bad length, length before checksum.
    assert_eq!(refused("S4030000F"), (1, "unsupported-type"));
    assert_eq!(refused("S10400G0FC"), (1, "bad-hex"));
    assert_eq!(refused("S1040000FF"), (1, "bad-length"));
    // A record's own checks run before the checks across records.
    let text = crlf(&["S9030000FC", "S1030000F"]);
    assert_eq!(refused(&text), (2, "bad-hex"));
}

#[test]
fn the_checks_across_records_have_their_reasons_and_lines() {
    // The S5 count is the number of S1–S3 records before it, empty ones included.
    let data = ["S0030000FC", "S107100001020304DE", "S1032000DC"];
    assert!(parse_srec(&crlf(&[&data[..], &["S5030002FA"]].concat())).is_ok());
    assert_eq!(
        refused(&crlf(&[&data[..], &["S5030001FB"]].concat())),
        (4, "bad-count")
    );
    assert_eq!(refused(&crlf(&["S5030001FB"])), (1, "bad-count"));
    // Any record after the first termination record, another one included.
    assert_eq!(
        refused(&crlf(&["S9030000FC", "S107100001020304DE"])),
        (2, "after-end")
    );
    assert_eq!(
        refused(&crlf(&["S9030000FC", "S9030000FC"])),
        (2, "after-end")
    );
    assert_eq!(refused(&crlf(&["S804001000EB", S0])), (2, "after-end"));
    // A wrong S5 after the end is a wrong count first.
    assert_eq!(
        refused(&crlf(&["S9030000FC", "S5030001FB"])),
        (2, "bad-count")
    );
    // Lines are counted in the text, empty ones included.
    assert_eq!(refused("\n\r\nS9030000FC\nS1032000DC"), (4, "after-end"));
}

#[test]
fn a_byte_placed_twice_is_an_overlap() {
    // $1000–$1003, then $1002–$1003 again.
    let text = crlf(&["S107100001020304DE", "S1051002AABB83"]);
    assert_eq!(refused(&text), (2, "overlap"));
    // Into a run merged from two records, and from below.
    let text = crlf(&["S107100001020304DE", "S10510040506DB", "S1050FFF0102E9"]);
    assert_eq!(refused(&text), (3, "overlap"));
    // Touching records do not overlap, in either order.
    let text = crlf(&["S10510040506DB", "S107100001020304DE"]);
    assert_eq!(
        parse_srec(&text).unwrap(),
        image(&[(0x1000, &[1, 2, 3, 4, 5, 6])], None)
    );
}

/// An S3 record of `len` zero bytes at `address`.
fn zero_record(address: u32, len: usize) -> String {
    let count = len + 5;
    let sum = address
        .to_be_bytes()
        .iter()
        .fold(count as u8, |sum, byte| sum.wrapping_add(*byte));
    format!(
        "S3{count:02X}{address:08X}{}{:02X}\r\n",
        "00".repeat(len),
        !sum
    )
}

#[test]
fn more_than_16_mib_of_data_is_too_large() {
    let full = 250;
    let records = MAX_SOURCE_BYTES / full;
    let mut text = String::new();
    let mut address = 0u32;
    for _ in 0..records {
        text.push_str(&zero_record(address, full));
        address += full as u32;
    }
    let last = MAX_SOURCE_BYTES - records * full;
    text.push_str(&zero_record(address, last));
    address += last as u32;
    // Exactly 16 MiB is accepted, as one segment.
    let parsed = parse_srec(&text).unwrap();
    assert_eq!(parsed.segments().len(), 1);
    assert_eq!(parsed.segments()[0].bytes.len(), MAX_SOURCE_BYTES);
    // One more byte passes the limit at that record.
    text.push_str(&zero_record(address, 1));
    let line = u32::try_from(records + 2).unwrap();
    assert_eq!(refused(&text), (line, "too-large"));
}

#[test]
fn the_binary_starts_at_the_lowest_address_and_fills_gaps_with_zeros() {
    let program = image(
        &[(0x1000, &[1, 2]), (0x1004, &[3]), (0x1006, &[4, 5])],
        None,
    );
    assert_eq!(to_binary(&program), (0x1000, vec![1, 2, 0, 0, 3, 0, 4, 5]));
    assert_eq!(
        to_binary(&image(&[(0x1000, &T1_CODE)], Some(0x1000))),
        (0x1000, T1_CODE.to_vec())
    );
    // Empty segments hold no byte: they neither set the base nor extend the end.
    let program = image(&[(0x0800, &[]), (0x1000, &[9]), (0x2000, &[])], None);
    assert_eq!(to_binary(&program), (0x1000, vec![9]));
}

#[test]
fn the_binary_of_an_empty_image_is_empty_at_0() {
    assert_eq!(to_binary(&image(&[], Some(0x1000))), (0, Vec::new()));
    assert_eq!(to_binary(&image(&[(0x1000, &[])], None)), (0, Vec::new()));
}
