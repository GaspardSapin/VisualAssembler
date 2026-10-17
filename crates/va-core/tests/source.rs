//! Tests for `src/source.rs`: `TextRange`, `SourceText` validation (size, UTF-8, BOM), `lines`
//! and `LineIndex` on every line terminator, and position/offset conversion.
#![allow(clippy::unwrap_used)]

use va_core::source::{
    LineCol, LineIndex, MAX_LINE_BYTES, MAX_SOURCE_BYTES, SourceError, SourceText, TextRange, lines,
};

/// The UTF-8 byte order mark.
const BOM: &[u8] = b"\xEF\xBB\xBF";

/// Collects the items of `lines(text)` as (start, end, content).
fn split(text: &str) -> Vec<(u32, u32, &str)> {
    lines(text)
        .map(|(range, content)| (range.start, range.end, content))
        .collect()
}

/// Builds a `LineCol`.
fn at(line: u32, column: u32) -> LineCol {
    LineCol { line, column }
}

// ---------------------------------------------------------------------------------------------
// TextRange
// ---------------------------------------------------------------------------------------------

#[test]
fn limits_have_the_specified_values() {
    assert_eq!(MAX_SOURCE_BYTES, 16 * 1024 * 1024);
    assert_eq!(MAX_LINE_BYTES, 64 * 1024);
}

#[test]
fn text_range_new_raises_a_reversed_end() {
    assert_eq!(TextRange::new(2, 5), TextRange { start: 2, end: 5 });
    assert_eq!(TextRange::new(5, 2), TextRange { start: 5, end: 5 });
    assert_eq!(TextRange::default(), TextRange::new(0, 0));
}

#[test]
fn text_range_len_and_is_empty() {
    assert_eq!(TextRange::new(2, 5).len(), 3);
    assert!(!TextRange::new(2, 5).is_empty());
    assert_eq!(TextRange::new(4, 4).len(), 0);
    assert!(TextRange::new(4, 4).is_empty());
    // Built directly, a reversed range behaves as empty instead of panicking.
    let reversed = TextRange { start: 9, end: 3 };
    assert_eq!(reversed.len(), 0);
    assert!(reversed.is_empty());
}

#[test]
fn text_range_shifted_saturates() {
    assert_eq!(TextRange::new(1, 4).shifted(10), TextRange::new(11, 14));
    assert_eq!(
        TextRange::new(1, 4).shifted(u32::MAX - 2),
        TextRange::new(u32::MAX - 1, u32::MAX)
    );
}

#[test]
fn text_range_slice_checks_bounds_and_char_boundaries() {
    let text = "aé€b"; // a(1) é(2) €(3) b(1)
    assert_eq!(TextRange::new(0, 1).slice(text), Some("a"));
    assert_eq!(TextRange::new(1, 3).slice(text), Some("é"));
    assert_eq!(TextRange::new(3, 7).slice(text), Some("€b"));
    assert_eq!(TextRange::new(7, 7).slice(text), Some(""));
    assert_eq!(TextRange::new(0, 8).slice(text), None, "past the end");
    assert_eq!(TextRange::new(2, 3).slice(text), None, "starts inside é");
    assert_eq!(TextRange::new(0, 2).slice(text), None, "ends inside é");
    assert_eq!(TextRange { start: 3, end: 1 }.slice(text), None, "reversed");
}

// ---------------------------------------------------------------------------------------------
// SourceText
// ---------------------------------------------------------------------------------------------

#[test]
fn from_bytes_accepts_utf8_without_bom() {
    let source = SourceText::from_bytes("nop ; é\n".as_bytes().to_vec()).unwrap();
    assert_eq!(source.as_str(), "nop ; é\n");
    assert!(!source.had_bom());
    assert_eq!(source.into_string(), "nop ; é\n");
}

#[test]
fn from_bytes_strips_exactly_one_bom() {
    let mut bytes = BOM.to_vec();
    bytes.extend_from_slice(b"rts");
    let source = SourceText::from_bytes(bytes).unwrap();
    assert_eq!(source.as_str(), "rts");
    assert!(source.had_bom());

    let mut twice = BOM.repeat(2);
    twice.extend_from_slice(b"rts");
    let source = SourceText::from_bytes(twice).unwrap();
    assert_eq!(source.as_str(), "\u{feff}rts");
    assert!(source.had_bom());
}

#[test]
fn a_bom_after_the_start_is_kept() {
    let source = SourceText::from_string("a\u{feff}".to_owned()).unwrap();
    assert_eq!(source.as_str(), "a\u{feff}");
    assert!(!source.had_bom());
}

#[test]
fn from_string_strips_one_bom() {
    let source = SourceText::from_string("\u{feff}move.l d0,d1".to_owned()).unwrap();
    assert_eq!(source.as_str(), "move.l d0,d1");
    assert!(source.had_bom());
    let source = SourceText::from_string(String::new()).unwrap();
    assert_eq!(source.as_str(), "");
    assert!(!source.had_bom());
}

#[test]
fn size_limit_accepts_exactly_the_maximum() {
    let source = SourceText::from_bytes(vec![b'a'; MAX_SOURCE_BYTES]).unwrap();
    assert_eq!(source.as_str().len(), MAX_SOURCE_BYTES);
    let source = SourceText::from_string("a".repeat(MAX_SOURCE_BYTES)).unwrap();
    assert_eq!(source.as_str().len(), MAX_SOURCE_BYTES);
}

#[test]
fn size_limit_rejects_one_byte_over() {
    assert_eq!(
        SourceText::from_bytes(vec![b'a'; MAX_SOURCE_BYTES + 1]),
        Err(SourceError::TooLarge {
            bytes: MAX_SOURCE_BYTES + 1
        })
    );
    assert_eq!(
        SourceText::from_string("a".repeat(MAX_SOURCE_BYTES + 1)),
        Err(SourceError::TooLarge {
            bytes: MAX_SOURCE_BYTES + 1
        })
    );
}

#[test]
fn size_limit_counts_the_bom() {
    let mut bytes = BOM.to_vec();
    bytes.resize(MAX_SOURCE_BYTES + 1, b'a');
    assert_eq!(
        SourceText::from_bytes(bytes),
        Err(SourceError::TooLarge {
            bytes: MAX_SOURCE_BYTES + 1
        })
    );
}

#[test]
fn size_is_checked_before_utf8() {
    assert_eq!(
        SourceText::from_bytes(vec![0xFF; MAX_SOURCE_BYTES + 1]),
        Err(SourceError::TooLarge {
            bytes: MAX_SOURCE_BYTES + 1
        })
    );
}

#[test]
fn not_utf8_reports_the_first_invalid_input_byte() {
    assert_eq!(
        SourceText::from_bytes(b"ab\xFFcd".to_vec()),
        Err(SourceError::NotUtf8 { offset: 2 })
    );
    assert_eq!(
        SourceText::from_bytes(b"\xFF".to_vec()),
        Err(SourceError::NotUtf8 { offset: 0 })
    );
    // A truncated multi-byte sequence at the end: the offset is its first byte.
    assert_eq!(
        SourceText::from_bytes("é".as_bytes()[..1].to_vec()),
        Err(SourceError::NotUtf8 { offset: 0 })
    );
    // The offset counts the input bytes, BOM included.
    let mut bytes = BOM.to_vec();
    bytes.extend_from_slice(b"a\xC3");
    assert_eq!(
        SourceText::from_bytes(bytes),
        Err(SourceError::NotUtf8 { offset: 4 })
    );
}

#[test]
fn nul_bytes_are_valid_utf8() {
    let source = SourceText::from_bytes(b"a\0b".to_vec()).unwrap();
    assert_eq!(source.as_str(), "a\0b");
}

// ---------------------------------------------------------------------------------------------
// lines
// ---------------------------------------------------------------------------------------------

#[test]
fn empty_text_is_one_empty_line() {
    assert_eq!(split(""), vec![(0, 0, "")]);
}

#[test]
fn lf_lines() {
    assert_eq!(split("a\nbc"), vec![(0, 1, "a"), (2, 4, "bc")]);
}

#[test]
fn crlf_lines() {
    assert_eq!(split("a\r\nbc"), vec![(0, 1, "a"), (3, 5, "bc")]);
}

#[test]
fn lone_cr_lines() {
    assert_eq!(split("a\rbc"), vec![(0, 1, "a"), (2, 4, "bc")]);
}

#[test]
fn mixed_terminators_as_specified() {
    assert_eq!(
        split("a\r\nb\rc"),
        vec![(0, 1, "a"), (3, 4, "b"), (5, 6, "c")]
    );
    assert_eq!(
        split("1\n2\r\n3\r4\n\r5"),
        vec![
            (0, 1, "1"),
            (2, 3, "2"),
            (5, 6, "3"),
            (7, 8, "4"),
            (9, 9, ""),
            (10, 11, "5")
        ]
    );
}

#[test]
fn trailing_terminator_gives_a_final_empty_line() {
    assert_eq!(split("a\n"), vec![(0, 1, "a"), (2, 2, "")]);
    assert_eq!(split("a\r\n"), vec![(0, 1, "a"), (3, 3, "")]);
    assert_eq!(split("a\r"), vec![(0, 1, "a"), (2, 2, "")]);
    assert_eq!(split("\n"), vec![(0, 0, ""), (1, 1, "")]);
}

#[test]
fn cr_cr_lf_is_two_terminators() {
    assert_eq!(
        split("a\r\r\nb"),
        vec![(0, 1, "a"), (2, 2, ""), (4, 5, "b")]
    );
}

#[test]
fn lf_cr_is_two_terminators() {
    assert_eq!(split("a\n\rb"), vec![(0, 1, "a"), (2, 2, ""), (3, 4, "b")]);
}

#[test]
fn line_contents_keep_multibyte_chars() {
    assert_eq!(
        split("é\r\n😀\u{2028}x"),
        vec![(0, 2, "é"), (4, 12, "😀\u{2028}x")]
    );
}

#[test]
fn lines_is_fused_and_its_size_hint_holds() {
    let mut iter = lines("a\nb");
    assert_eq!(iter.size_hint().0, 1);
    assert!(iter.next().is_some());
    assert!(iter.next().is_some());
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None);
    assert_eq!(iter.size_hint(), (0, Some(0)));
}

// ---------------------------------------------------------------------------------------------
// LineIndex
// ---------------------------------------------------------------------------------------------

#[test]
fn line_index_counts_like_lines() {
    for text in [
        "",
        "a",
        "a\n",
        "a\r\nb\rc",
        "\n\n",
        "\r\r\n\n",
        "x\ry\r\n",
        "é\n😀",
    ] {
        let index = LineIndex::new(text);
        let expected: Vec<TextRange> = lines(text).map(|(range, _)| range).collect();
        assert_eq!(index.line_count() as usize, expected.len(), "{text:?}");
        for (number, range) in (1..).zip(&expected) {
            assert_eq!(index.line_range(number), Some(*range), "{text:?}");
        }
    }
}

#[test]
fn line_counts_of_the_specified_examples() {
    assert_eq!(LineIndex::new("").line_count(), 1);
    assert_eq!(LineIndex::new("a\n").line_count(), 2);
    assert_eq!(LineIndex::new("a\r\nb\rc").line_count(), 3);
}

#[test]
fn line_range_is_one_based() {
    let index = LineIndex::new("ab\ncd");
    assert_eq!(index.line_range(0), None);
    assert_eq!(index.line_range(1), Some(TextRange::new(0, 2)));
    assert_eq!(index.line_range(2), Some(TextRange::new(3, 5)));
    assert_eq!(index.line_range(3), None);
    assert_eq!(index.line_range(u32::MAX), None);
}

#[test]
fn position_counts_chars_and_tabs_as_one_column() {
    let text = "\tmove.l\td0,d1\n; é😀x";
    let index = LineIndex::new(text);
    assert_eq!(index.position(text, 0), at(1, 1));
    assert_eq!(index.position(text, 1), at(1, 2), "after the tab");
    assert_eq!(index.position(text, 8), at(1, 9), "after the second tab");
    let second = 14; // start of line 2
    assert_eq!(index.position(text, second), at(2, 1));
    assert_eq!(index.position(text, second + 2), at(2, 3), "é");
    assert_eq!(
        index.position(text, second + 4),
        at(2, 4),
        "😀 after é (2 bytes)"
    );
    assert_eq!(
        index.position(text, second + 8),
        at(2, 5),
        "x after 😀 (4 bytes)"
    );
    assert_eq!(index.position(text, second + 9), at(2, 6), "end of text");
}

#[test]
fn position_clamps_past_the_end() {
    let text = "ab\nc";
    let index = LineIndex::new(text);
    assert_eq!(index.position(text, 4), at(2, 2));
    assert_eq!(index.position(text, 1000), at(2, 2));
    assert_eq!(index.position(text, u32::MAX), at(2, 2));
    let empty = LineIndex::new("");
    assert_eq!(empty.position("", 7), at(1, 1));
}

#[test]
fn position_inside_a_terminator_is_the_line_end() {
    let text = "ab\r\ncd";
    let index = LineIndex::new(text);
    assert_eq!(index.position(text, 2), at(1, 3), "on the CR");
    assert_eq!(index.position(text, 3), at(1, 3), "on the LF of CRLF");
    assert_eq!(index.position(text, 4), at(2, 1));
}

#[test]
fn position_inside_a_multibyte_char_is_that_char() {
    let text = "a€b";
    let index = LineIndex::new(text);
    assert_eq!(index.position(text, 1), at(1, 2));
    assert_eq!(index.position(text, 2), at(1, 2));
    assert_eq!(index.position(text, 3), at(1, 2));
    assert_eq!(index.position(text, 4), at(1, 3));
}

#[test]
fn offset_converts_back_and_rejects_past_the_line_end() {
    let text = "a€b\r\n\tc";
    let index = LineIndex::new(text);
    assert_eq!(index.offset(text, at(1, 1)), Some(0));
    assert_eq!(index.offset(text, at(1, 2)), Some(1));
    assert_eq!(index.offset(text, at(1, 3)), Some(4));
    assert_eq!(index.offset(text, at(1, 4)), Some(5), "end of line 1");
    assert_eq!(index.offset(text, at(1, 5)), None, "past the end of line 1");
    assert_eq!(index.offset(text, at(2, 1)), Some(7));
    assert_eq!(index.offset(text, at(2, 2)), Some(8), "after the tab");
    assert_eq!(index.offset(text, at(2, 3)), Some(9));
    assert_eq!(index.offset(text, at(2, 4)), None);
    assert_eq!(index.offset(text, at(3, 1)), None, "no line 3");
    assert_eq!(index.offset(text, at(0, 1)), None, "line 0");
    assert_eq!(index.offset(text, at(1, 0)), None, "column 0");
}

#[test]
fn position_and_offset_round_trip_on_every_char_boundary() {
    let text = "\u{feff}x\té€😀 ; ü\r\n\r\n\tlea\t4(a0),a1\r😀😀\n";
    let index = LineIndex::new(text);
    let mut checked = 0;
    for (range, content) in lines(text) {
        for (within, _) in content
            .char_indices()
            .chain(std::iter::once((content.len(), ' ')))
        {
            let offset = range.start + u32::try_from(within).unwrap();
            let position = index.position(text, offset);
            assert_eq!(
                index.offset(text, position),
                Some(offset),
                "offset {offset} -> {position:?}"
            );
            checked += 1;
        }
    }
    assert!(checked > 20);
}

#[test]
fn a_mismatched_text_never_panics() {
    let index = LineIndex::new("first line\nsecond line\n");
    let other = "é";
    for offset in 0..30 {
        let _ = index.position(other, offset);
    }
    for line in 0..5 {
        for column in 0..15 {
            let _ = index.offset(other, at(line, column));
        }
    }
}
