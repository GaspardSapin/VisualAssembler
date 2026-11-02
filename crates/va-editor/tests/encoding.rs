//! Tests of `va_editor::encoding`: detection, BOM, binary and size checks, the Windows-1252
//! table, `first_unencodable`, `encode_to` and line-ending detection (m4-editor-shell.md §3.1).
#![allow(clippy::unwrap_used)]

use std::io::{self, Write};

use va_editor::encoding::{
    DecodeError, Encoding, LineEnding, decode, detect_line_ending, encode_to, first_unencodable,
};
use va_editor::text::{MAX_DOCUMENT_BYTES, Text};

/// Encodes `text` into a byte vector.
fn encoded(text: &str, encoding: Encoding) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    encode_to(&Text::new(text), encoding, &mut out)?;
    Ok(out)
}

#[test]
fn ids_round_trip() {
    for encoding in Encoding::ALL {
        assert_eq!(Encoding::from_id(encoding.id()), Some(encoding));
    }
    for ending in LineEnding::ALL {
        assert_eq!(LineEnding::from_id(ending.id()), Some(ending));
    }
    assert_eq!(Encoding::Utf8Bom.id(), "utf-8-bom");
    assert_eq!(LineEnding::CrLf.as_str(), "\r\n");
    assert_eq!(Encoding::from_id("UTF-8"), None);
}

#[test]
fn utf8_is_detected_and_a_bom_is_stripped() {
    assert_eq!(
        decode("move.l é".as_bytes(), None),
        Ok((String::from("move.l é"), Encoding::Utf8))
    );
    assert_eq!(
        decode(b"\xEF\xBB\xBFrts\n", None),
        Ok((String::from("rts\n"), Encoding::Utf8Bom))
    );
    assert_eq!(
        decode(b"\xEF\xBB\xBF", None),
        Ok((String::new(), Encoding::Utf8Bom))
    );
    assert_eq!(decode(b"", None), Ok((String::new(), Encoding::Utf8)));
}

#[test]
fn a_nul_byte_in_the_first_8_kib_is_binary() {
    assert_eq!(
        decode(b"ab\0cd", None),
        Err(DecodeError::Binary { offset: 2 })
    );
    let mut late = vec![b'a'; 8 * 1024];
    late.push(0);
    assert_eq!(
        decode(&late, None),
        Ok((
            String::from_utf8(late[..8 * 1024].to_vec()).unwrap() + "\0",
            Encoding::Utf8
        ))
    );
    // A forced encoding skips the binary test.
    assert_eq!(
        decode(b"a\0", Some(Encoding::Windows1252)),
        Ok((String::from("a\0"), Encoding::Windows1252))
    );
}

#[test]
fn invalid_utf8_reports_the_first_bad_byte() {
    assert_eq!(
        decode(b"abc\xFFdef", None),
        Err(DecodeError::NotUtf8 { offset: 3 })
    );
    // The offset counts the BOM.
    assert_eq!(
        decode(b"\xEF\xBB\xBFab\xC3", None),
        Err(DecodeError::NotUtf8 { offset: 5 })
    );
    assert_eq!(
        decode(b"\xE9t\xE9", None),
        Err(DecodeError::NotUtf8 { offset: 0 })
    );
    assert_eq!(
        decode(b"\xEF\xBB\xBFok", Some(Encoding::Utf8)),
        Ok((String::from("ok"), Encoding::Utf8))
    );
    assert_eq!(
        decode(b"\xFF", Some(Encoding::Utf8Bom)),
        Err(DecodeError::NotUtf8 { offset: 0 })
    );
}

#[test]
fn more_than_16_mib_is_too_large_even_when_forced() {
    let exact = vec![b'a'; MAX_DOCUMENT_BYTES];
    assert!(decode(&exact, None).is_ok());
    let over = vec![b'a'; MAX_DOCUMENT_BYTES + 1];
    let error = Err(DecodeError::TooLarge {
        bytes: MAX_DOCUMENT_BYTES + 1,
    });
    assert_eq!(decode(&over, None), error);
    assert_eq!(decode(&over, Some(Encoding::Windows1252)), error);
}

#[test]
fn all_256_bytes_round_trip_through_windows_1252() {
    let bytes: Vec<u8> = (0..=255).collect();
    let (text, encoding) = decode(&bytes, Some(Encoding::Windows1252)).unwrap();
    assert_eq!(encoding, Encoding::Windows1252);
    assert_eq!(text.chars().count(), 256);
    assert_eq!(
        first_unencodable(&Text::new(&text), Encoding::Windows1252),
        None
    );
    assert_eq!(encoded(&text, Encoding::Windows1252).unwrap(), bytes);
}

#[test]
fn windows_1252_maps_the_c1_range_through_the_whatwg_table() {
    let (text, _) = decode(
        b"\x80\x81\x8A\x8D\x8F\x90\x93\x94\x9D\x9F\xA0\xE9\xFF",
        Some(Encoding::Windows1252),
    )
    .unwrap();
    assert_eq!(
        text,
        "\u{20AC}\u{0081}\u{0160}\u{008D}\u{008F}\u{0090}\u{201C}\u{201D}\u{009D}\u{0178}\u{00A0}é\u{00FF}"
    );
}

#[test]
fn first_unencodable_finds_chars_outside_the_code_page() {
    let text = Text::new("café € ok — fin 😀");
    assert_eq!(
        first_unencodable(&text, Encoding::Windows1252),
        Some((21, '😀'))
    );
    let text = Text::new("a\u{0080}b");
    assert_eq!(
        first_unencodable(&text, Encoding::Windows1252),
        Some((1, '\u{0080}'))
    );
    assert_eq!(first_unencodable(&text, Encoding::Utf8), None);
    assert_eq!(first_unencodable(&text, Encoding::Utf8Bom), None);
    assert_eq!(
        first_unencodable(&Text::new("ĉ"), Encoding::Windows1252),
        Some((0, 'ĉ'))
    );
}

#[test]
fn encode_to_writes_each_encoding() {
    assert_eq!(encoded("é\n", Encoding::Utf8).unwrap(), "é\n".as_bytes());
    assert_eq!(
        encoded("é\n", Encoding::Utf8Bom).unwrap(),
        b"\xEF\xBB\xBF\xC3\xA9\n"
    );
    assert_eq!(
        encoded("é€\r\n", Encoding::Windows1252).unwrap(),
        b"\xE9\x80\r\n"
    );
    let error = encoded("ok 😀", Encoding::Windows1252).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn encode_to_reports_sink_errors() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("disk full"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    for encoding in Encoding::ALL {
        let error = encode_to(&Text::new("x"), encoding, &mut Broken).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
    }
}

#[test]
fn decoded_bytes_encode_back_identically() {
    for (bytes, expected) in [
        (&b"rts\r\n"[..], Encoding::Utf8),
        (&b"\xEF\xBB\xBFrts\n"[..], Encoding::Utf8Bom),
    ] {
        let (text, encoding) = decode(bytes, None).unwrap();
        assert_eq!(encoding, expected);
        assert_eq!(encoded(&text, encoding).unwrap(), bytes);
    }
}

#[test]
fn the_first_terminator_gives_the_line_ending() {
    assert_eq!(detect_line_ending(&Text::new("no break")), None);
    assert_eq!(detect_line_ending(&Text::new("")), None);
    assert_eq!(
        detect_line_ending(&Text::new("a\nb\r\n")),
        Some(LineEnding::Lf)
    );
    assert_eq!(
        detect_line_ending(&Text::new("a\r\nb\n")),
        Some(LineEnding::CrLf)
    );
    assert_eq!(
        detect_line_ending(&Text::new("a\rb\r\n")),
        Some(LineEnding::Cr)
    );
    assert_eq!(detect_line_ending(&Text::new("\r")), Some(LineEnding::Cr));
    let long = format!("{}\r\n", "x".repeat(100_000));
    assert_eq!(
        detect_line_ending(&Text::new(&long)),
        Some(LineEnding::CrLf)
    );
}
