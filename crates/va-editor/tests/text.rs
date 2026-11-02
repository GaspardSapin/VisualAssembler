//! Tests of `va_editor::text`: line indexing against `va_core::source::LineIndex`, slices, chunks,
//! display columns and the widest line (m4-editor-shell.md §3.1, §10.1).
#![allow(clippy::unwrap_used)]

use va_core::source::LineIndex;
use va_editor::text::{MAX_DOCUMENT_BYTES, Text, byte_at_display_col, display_col};

/// A small deterministic generator (xorshift64*), so the random texts are the same every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap()).unwrap()
    }
}

/// Checks every line query of `Text` against `LineIndex` on `source`.
fn check_against_line_index(source: &str) {
    let text = Text::new(source);
    let index = LineIndex::new(source);
    assert_eq!(text.line_count(), index.line_count() as usize, "{source:?}");
    assert_eq!(text.len_bytes(), source.len());
    let mut buf = String::from("stale");
    for line in 0..text.line_count() {
        let range = index.line_range(u32::try_from(line + 1).unwrap()).unwrap();
        assert_eq!(
            text.line_start(line),
            range.start as usize,
            "{source:?} line {line}"
        );
        assert_eq!(
            text.line_end(line),
            range.end as usize,
            "{source:?} line {line}"
        );
        text.line_text_into(line, &mut buf);
        assert_eq!(buf, &source[range.start as usize..range.end as usize]);
    }
    for byte in 0..=source.len() {
        if source.is_char_boundary(byte) {
            let expected = index.position(source, u32::try_from(byte).unwrap()).line - 1;
            assert_eq!(
                text.line_of(byte),
                expected as usize,
                "{source:?} byte {byte}"
            );
        }
    }
}

#[test]
fn line_counts_match_line_index_on_every_terminator_kind() {
    for source in [
        "",
        "a",
        "a\n",
        "\n",
        "\n\n",
        "a\nb",
        "a\r\nb\r\n",
        "a\rb\r",
        "\r",
        "\r\n",
        "\r\r\n\n",
        "a\r\nb\rc\nd",
        "é\r\n€\n😀\rx",
        "trailing\r\n\r\n",
    ] {
        check_against_line_index(source);
    }
}

#[test]
fn ten_thousand_random_texts_match_line_index() {
    const ALPHABET: [&str; 8] = ["a", "b", " ", "\t", "\n", "\r", "\r\n", "é"];
    let mut rng = Rng(0x5EED_CAFE_F00D_0001);
    for _ in 0..10_000 {
        let len = rng.below(24);
        let source: String = (0..len)
            .map(|_| ALPHABET[rng.below(ALPHABET.len())])
            .collect();
        check_against_line_index(&source);
    }
}

#[test]
fn line_queries_clamp_out_of_range_arguments() {
    let text = Text::new("ab\ncd");
    assert_eq!(text.line_start(99), 3);
    assert_eq!(text.line_end(99), 5);
    assert_eq!(text.line_of(99), 1);
    let mut buf = String::new();
    text.line_text_into(99, &mut buf);
    assert_eq!(buf, "cd");
}

#[test]
fn slices_chunks_and_write_to_give_the_text_back() {
    let source = "move.l d0,d1 ; é\r\nrts\n";
    let text = Text::new(source);
    assert_eq!(text.slice_to_string(0, source.len()), source);
    assert_eq!(text.slice_to_string(5, 7), "l ");
    // Clamped to the length and moved down to char boundaries (é is 2 bytes at 15..17).
    assert_eq!(text.slice_to_string(16, 999), &source[15..]);
    assert_eq!(text.slice_to_string(9, 3), "");
    assert_eq!(text.chunks().collect::<String>(), source);
    let mut out = Vec::new();
    text.write_to(&mut out).unwrap();
    assert_eq!(out, source.as_bytes());
    assert_eq!(text.to_string(), source);
    assert_eq!(text, Text::new(source));
    assert!(Text::new("").is_empty());
    assert!(text.is_char_boundary(15) && !text.is_char_boundary(16) && !text.is_char_boundary(999));
}

#[test]
fn clone_is_cheap_and_text_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Text>();
    let text = Text::new(&"x".repeat(1 << 20));
    let copy = text.clone();
    assert_eq!(copy, text);
    assert_eq!(MAX_DOCUMENT_BYTES, 16 * 1024 * 1024);
}

#[test]
fn display_columns_expand_tabs_and_count_chars() {
    assert_eq!(display_col("abc", 2, 4), 2);
    assert_eq!(display_col("\tx", 1, 4), 4);
    assert_eq!(display_col("a\tx", 2, 4), 4);
    assert_eq!(display_col("abcd\tx", 5, 4), 8);
    assert_eq!(display_col("\t\t", 2, 8), 16);
    // Multi-byte chars count once; an offset inside a char counts from its start.
    assert_eq!(display_col("é€x", 5, 4), 2);
    assert_eq!(display_col("é€x", 1, 4), 0);
    assert_eq!(display_col("abc", 99, 4), 3);
    // A tab width of 0 behaves as 1.
    assert_eq!(display_col("\t\t", 2, 0), 2);
}

#[test]
fn byte_at_display_col_returns_the_boundary_at_or_before() {
    assert_eq!(byte_at_display_col("abc", 0, 4), 0);
    assert_eq!(byte_at_display_col("abc", 2, 4), 2);
    assert_eq!(byte_at_display_col("abc", 9, 4), 3);
    // Inside a tab: the tab's start.
    assert_eq!(byte_at_display_col("\tx", 2, 4), 0);
    assert_eq!(byte_at_display_col("\tx", 4, 4), 1);
    assert_eq!(byte_at_display_col("é€x", 2, 4), 5);
    for line in ["a\tb\tc", "é\t€", "\t\tx"] {
        for byte in (0..=line.len()).filter(|&b| line.is_char_boundary(b)) {
            let col = display_col(line, byte, 4);
            assert_eq!(byte_at_display_col(line, col, 4), byte, "{line:?} {byte}");
        }
    }
}

#[test]
fn widest_line_counts_chars_without_terminators() {
    assert_eq!(Text::new("").widest_line_chars(), 0);
    assert_eq!(Text::new("ab\r\nabcd\rabc\n").widest_line_chars(), 4);
    assert_eq!(Text::new("éé\n€€€").widest_line_chars(), 3);
    assert_eq!(Text::new("\r\n\r\n").widest_line_chars(), 0);
}

#[test]
fn long_texts_index_lines_across_rope_chunks() {
    let mut source = String::new();
    for line in 0..5_000 {
        source.push_str(&format!("line {line} ; é€"));
        source.push_str(["\n", "\r\n", "\r"][line % 3]);
    }
    let text = Text::new(&source);
    let index = LineIndex::new(&source);
    assert_eq!(text.line_count(), index.line_count() as usize);
    let mut buf = String::new();
    for line in (0..text.line_count()).step_by(97) {
        let range = index.line_range(u32::try_from(line + 1).unwrap()).unwrap();
        assert_eq!(text.line_start(line), range.start as usize);
        assert_eq!(text.line_end(line), range.end as usize);
        text.line_text_into(line, &mut buf);
        assert_eq!(buf, &source[range.start as usize..range.end as usize]);
    }
}
