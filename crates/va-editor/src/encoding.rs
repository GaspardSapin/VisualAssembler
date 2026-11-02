//! File encodings, decoding and line endings (m4-editor-shell.md §3.1).
//!
//! UTF-8 (with or without a byte order mark) is detected; Windows-1252 is only used when the user
//! forces it, through a hand-written table of its 32 bytes 0x80–0x9F (the WHATWG index, with the
//! five undefined bytes mapped to the C1 control of the same value, so every byte round-trips).

use std::io;

use crate::text::{MAX_DOCUMENT_BYTES, Text};

/// The UTF-8 byte order mark.
const BOM: &[u8] = b"\xEF\xBB\xBF";

/// Number of leading bytes searched for a NUL byte, the binary-file test.
const BINARY_SNIFF_BYTES: usize = 8 * 1024;

/// Windows-1252 bytes 0x80–0x9F (WHATWG `index-windows-1252`). The undefined bytes 0x81, 0x8D,
/// 0x8F, 0x90 and 0x9D map to U+0081, U+008D, U+008F, U+0090 and U+009D.
const CP1252_HIGH: [char; 32] = [
    '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}',
    '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
];

/// The encoding of a file on disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// UTF-8 without a byte order mark.
    Utf8,
    /// UTF-8 with a byte order mark (stripped on load, written back on save).
    Utf8Bom,
    /// Windows-1252, only when forced by the user (lossless for every byte).
    Windows1252,
}

impl Encoding {
    /// Every encoding, in declaration order.
    pub const ALL: [Self; 3] = [Self::Utf8, Self::Utf8Bom, Self::Windows1252];

    /// Returns the identifier of the encoding: `"utf-8"`, `"utf-8-bom"` or `"windows-1252"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Utf8Bom => "utf-8-bom",
            Self::Windows1252 => "windows-1252",
        }
    }

    /// The encoding whose [`id`](Self::id) is exactly `id`.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|encoding| encoding.id() == id)
    }
}

/// A line terminator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineEnding {
    /// `\n`.
    Lf,
    /// `\r\n`.
    CrLf,
    /// A lone `\r`.
    Cr,
}

impl LineEnding {
    /// Every line ending, in declaration order.
    pub const ALL: [Self; 3] = [Self::Lf, Self::CrLf, Self::Cr];

    /// Returns the identifier of the line ending: `"lf"`, `"crlf"` or `"cr"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Lf => "lf",
            Self::CrLf => "crlf",
            Self::Cr => "cr",
        }
    }

    /// Returns the terminator itself: `"\n"`, `"\r\n"` or `"\r"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
        }
    }

    /// The line ending whose [`id`](Self::id) is exactly `id`.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|ending| ending.id() == id)
    }
}

/// Why [`decode`] rejected a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DecodeError {
    /// The file is larger than `MAX_DOCUMENT_BYTES`.
    TooLarge {
        /// Its size in bytes.
        bytes: usize,
    },
    /// The file is not valid UTF-8 (and no encoding was forced).
    NotUtf8 {
        /// Offset of the first invalid byte.
        offset: usize,
    },
    /// A NUL byte in the first 8 KiB: the file is taken as binary.
    Binary {
        /// Offset of that byte.
        offset: usize,
    },
}

/// Decodes the bytes of a file: detects the encoding when `forced` is `None`, else decodes
/// with the forced one. Returns the text (BOM stripped) and its encoding.
///
/// Detection, in order: over [`MAX_DOCUMENT_BYTES`] → `TooLarge`; a NUL byte in the first
/// 8 KiB → `Binary`; a UTF-8 BOM → [`Encoding::Utf8Bom`] (stripped); valid UTF-8 →
/// [`Encoding::Utf8`]; else `NotUtf8` with the offset of the first invalid byte of the input
/// (the BOM included). A forced encoding skips the binary test: Windows-1252 decodes every byte;
/// forced UTF-8 strips a leading BOM and still needs valid UTF-8.
///
/// # Errors
/// A [`DecodeError`] for a too large, binary or non-UTF-8 file.
pub fn decode(bytes: &[u8], forced: Option<Encoding>) -> Result<(String, Encoding), DecodeError> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(DecodeError::TooLarge { bytes: bytes.len() });
    }
    match forced {
        Some(Encoding::Windows1252) => Ok((decode_1252(bytes), Encoding::Windows1252)),
        Some(encoding) => {
            let body = bytes.strip_prefix(BOM).unwrap_or(bytes);
            utf8(body, bytes.len() - body.len()).map(|text| (text, encoding))
        }
        None => {
            let sniff = &bytes[..bytes.len().min(BINARY_SNIFF_BYTES)];
            if let Some(offset) = sniff.iter().position(|&byte| byte == 0) {
                return Err(DecodeError::Binary { offset });
            }
            match bytes.strip_prefix(BOM) {
                Some(body) => utf8(body, BOM.len()).map(|text| (text, Encoding::Utf8Bom)),
                None => utf8(bytes, 0).map(|text| (text, Encoding::Utf8)),
            }
        }
    }
}

/// Validates `body` as UTF-8; `skip` is the number of input bytes before it (for the offset).
fn utf8(body: &[u8], skip: usize) -> Result<String, DecodeError> {
    match std::str::from_utf8(body) {
        Ok(text) => Ok(text.to_owned()),
        Err(error) => Err(DecodeError::NotUtf8 {
            offset: skip + error.valid_up_to(),
        }),
    }
}

/// The char of the Windows-1252 byte `byte`.
fn decode_byte_1252(byte: u8) -> char {
    match byte {
        0x80..=0x9F => CP1252_HIGH[usize::from(byte - 0x80)],
        _ => char::from(byte),
    }
}

/// Decodes every byte of `bytes` as Windows-1252.
fn decode_1252(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len());
    text.extend(bytes.iter().map(|&byte| decode_byte_1252(byte)));
    text
}

/// The Windows-1252 byte of `c`, if the code page has one.
fn encode_char_1252(c: char) -> Option<u8> {
    let code = u32::from(c);
    if code < 0x80 || (0xA0..=0xFF).contains(&code) {
        return u8::try_from(code).ok();
    }
    let index = CP1252_HIGH.iter().position(|&high| high == c)?;
    u8::try_from(index).ok().map(|index| 0x80 + index)
}

/// The first char of `text` that `encoding` cannot represent, with its byte offset. UTF-8
/// represents every char, so only Windows-1252 can return `Some`.
pub fn first_unencodable(text: &Text, encoding: Encoding) -> Option<(usize, char)> {
    if encoding != Encoding::Windows1252 {
        return None;
    }
    let mut offset = 0;
    for chunk in text.chunks() {
        if !chunk.is_ascii()
            && let Some((index, c)) = chunk
                .char_indices()
                .find(|&(_, c)| encode_char_1252(c).is_none())
        {
            return Some((offset + index, c));
        }
        offset += chunk.len();
    }
    None
}

/// Writes `text` to `out` in `encoding` (with the BOM for [`Encoding::Utf8Bom`]).
///
/// # Errors
/// The first error of `out`, or `InvalidData` on the first char outside Windows-1252 (callers
/// check [`first_unencodable`] first; the bytes before that char may have been written).
pub fn encode_to(text: &Text, encoding: Encoding, out: &mut dyn io::Write) -> io::Result<()> {
    match encoding {
        Encoding::Utf8 => text.write_to(out),
        Encoding::Utf8Bom => {
            out.write_all(BOM)?;
            text.write_to(out)
        }
        Encoding::Windows1252 => {
            let mut buf = Vec::new();
            for chunk in text.chunks() {
                buf.clear();
                for c in chunk.chars() {
                    let byte = encode_char_1252(c)
                        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))?;
                    buf.push(byte);
                }
                out.write_all(&buf)?;
            }
            Ok(())
        }
    }
}

/// The first line terminator of `text`, if it has one (O(offset of that terminator)).
pub fn detect_line_ending(text: &Text) -> Option<LineEnding> {
    let rope = text.rope();
    let mut offset = 0;
    for chunk in rope.chunks() {
        let bytes = chunk.as_bytes();
        if let Some(index) = bytes.iter().position(|&b| b == b'\n' || b == b'\r') {
            return Some(if bytes[index] == b'\n' {
                LineEnding::Lf
            } else if rope.get_byte(offset + index + 1) == Some(b'\n') {
                LineEnding::CrLf
            } else {
                LineEnding::Cr
            });
        }
        offset += chunk.len();
    }
    None
}
