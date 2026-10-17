//! Source texts, byte ranges, line splitting and line/column conversion (m1-core-isa-lexer.md
//! §3.2).
//!
//! Every stored span is a byte range ([`TextRange`]); display columns ([`LineCol::column`])
//! count Unicode scalar values, a tab counting as one. Line terminators are LF, CRLF and a lone
//! CR, identically in [`lines`] and [`LineIndex`]: `""` has 1 line, `"a\n"` has 2 (the last one
//! empty) and `"a\r\nb\rc"` has 3. This is ropey's counting with only its `cr_lines` feature,
//! which the M4 editor must use.

use std::iter::FusedIterator;

/// Largest accepted source file, in bytes (16 MiB), checked on the input before the BOM is
/// stripped.
pub const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;

/// Longest line a lexer lexes normally, in bytes (64 KiB); a longer line is one invalid token
/// (m1 §5.2).
pub const MAX_LINE_BYTES: usize = 64 * 1024;

/// The byte length of the UTF-8 byte order mark U+FEFF.
const BOM_LEN: usize = '\u{feff}'.len_utf8();

/// Converts a byte count or offset to `u32`, saturating at `u32::MAX` (texts are capped at
/// [`MAX_SOURCE_BYTES`], so saturation only happens on API misuse).
fn offset_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// Identifies one source file within a workspace or an assembly. The numbering is chosen by the
/// owner of the files (the CLI, the M4 workspace, the M2 source provider).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

/// A half-open byte range `start..end` of a text, with `start <= end` when built through
/// [`TextRange::new`].
///
/// A range is line-relative or file-relative depending on where it is stored; [`shifted`]
/// converts the first into the second. Methods never panic, even on a range built directly with
/// `end` below `start`, which then behaves as empty.
///
/// [`shifted`]: TextRange::shifted
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct TextRange {
    /// Offset of the first byte of the range.
    pub start: u32,
    /// Offset one past the last byte of the range.
    pub end: u32,
}

impl TextRange {
    /// Builds `start..end`; an `end` below `start` is raised to `start` (an empty range), never
    /// a panic.
    pub const fn new(start: u32, end: u32) -> Self {
        let end = if end < start { start } else { end };
        Self { start, end }
    }

    /// Returns the number of bytes in the range (0 when `end` is not above `start`).
    pub const fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Tells whether the range holds no byte.
    pub const fn is_empty(self) -> bool {
        self.end <= self.start
    }

    /// Returns the range moved right by `by` bytes, saturating at `u32::MAX`; used to turn a
    /// line-relative range into a file-relative one.
    pub const fn shifted(self, by: u32) -> Self {
        Self {
            start: self.start.saturating_add(by),
            end: self.end.saturating_add(by),
        }
    }

    /// Returns the part of `text` this range covers, or `None` if the range is out of bounds,
    /// reversed, or does not start and end on char boundaries.
    pub fn slice(self, text: &str) -> Option<&str> {
        text.get(self.start as usize..self.end as usize)
    }
}

/// A file-relative byte range of one source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    /// The file the range belongs to.
    pub file: FileId,
    /// Byte range within that file (after a stripped BOM, see [`SourceText`]).
    pub range: TextRange,
}

/// A display position: 1-based line and 1-based column, the column counting Unicode scalar
/// values (a tab counts as one), as egui's char cursor and `va hover f.asm:L:C` do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LineCol {
    /// Line number, starting at 1.
    pub line: u32,
    /// Column number in chars, starting at 1.
    pub column: u32,
}

/// A validated source text: at most [`MAX_SOURCE_BYTES`] bytes of input, valid UTF-8, with one
/// leading byte order mark stripped.
///
/// Offsets into [`as_str`](SourceText::as_str) are relative to the text after the BOM; the M4
/// editor uses the same type and restores the BOM on save through
/// [`had_bom`](SourceText::had_bom).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceText {
    /// The text, without the leading BOM.
    text: String,
    /// Whether the input started with a BOM that was stripped.
    had_bom: bool,
}

/// Why a source input was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SourceError {
    /// The input is larger than [`MAX_SOURCE_BYTES`].
    TooLarge {
        /// Size of the refused input, in bytes.
        bytes: usize,
    },
    /// The input is not valid UTF-8.
    NotUtf8 {
        /// Offset of the first invalid byte of the input (a BOM, if any, included).
        offset: usize,
    },
}

/// Fails with [`SourceError::TooLarge`] when `bytes` exceeds [`MAX_SOURCE_BYTES`].
fn check_size(bytes: usize) -> Result<(), SourceError> {
    if bytes > MAX_SOURCE_BYTES {
        Err(SourceError::TooLarge { bytes })
    } else {
        Ok(())
    }
}

impl SourceText {
    /// Validates raw file bytes: size first, then UTF-8, then strips one leading BOM.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, SourceError> {
        check_size(bytes.len())?;
        match String::from_utf8(bytes) {
            Ok(text) => Ok(Self::without_bom(text)),
            Err(error) => Err(SourceError::NotUtf8 {
                offset: error.utf8_error().valid_up_to(),
            }),
        }
    }

    /// Validates an already decoded text: size first, then strips one leading BOM.
    pub fn from_string(text: String) -> Result<Self, SourceError> {
        check_size(text.len())?;
        Ok(Self::without_bom(text))
    }

    /// Strips one leading BOM from `text` and records whether there was one.
    fn without_bom(mut text: String) -> Self {
        let had_bom = text.starts_with('\u{feff}');
        if had_bom {
            text.replace_range(..BOM_LEN, "");
        }
        Self { text, had_bom }
    }

    /// Returns the text, without the stripped BOM.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Tells whether the input started with a byte order mark (stripped from the text).
    pub fn had_bom(&self) -> bool {
        self.had_bom
    }

    /// Returns the text, without the stripped BOM.
    pub fn into_string(self) -> String {
        self.text
    }
}

/// Splits `text` into lines: each item is the line's byte range and its content, both without
/// the terminator (LF, CRLF or a lone CR).
///
/// There is always at least one line: `""` gives one empty line, and a text ending with a
/// terminator ends with an empty line.
pub fn lines(text: &str) -> Lines<'_> {
    Lines {
        text,
        next_start: 0,
        finished: false,
    }
}

/// Iterator over the lines of a text, returned by [`lines`].
#[derive(Clone, Debug)]
pub struct Lines<'a> {
    /// The whole text being split.
    text: &'a str,
    /// Byte offset where the next line starts.
    next_start: usize,
    /// Whether the last line (the one without a terminator) was returned.
    finished: bool,
}

impl<'a> Lines<'a> {
    /// Builds the item of the line whose content is `start..end`.
    fn item(&self, start: usize, end: usize) -> (TextRange, &'a str) {
        let range = TextRange::new(offset_u32(start), offset_u32(end));
        (range, self.text.get(start..end).unwrap_or(""))
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = (TextRange, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        let start = self.next_start;
        let rest = self.text.as_bytes().get(start..).unwrap_or(&[]);
        match rest.iter().position(|&byte| byte == b'\n' || byte == b'\r') {
            Some(index) => {
                let end = start + index;
                let crlf = rest[index] == b'\r' && rest.get(index + 1) == Some(&b'\n');
                self.next_start = end + if crlf { 2 } else { 1 };
                Some(self.item(start, end))
            }
            None => {
                self.finished = true;
                Some(self.item(start, self.text.len()))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.finished {
            (0, Some(0))
        } else {
            let remaining = self.text.len().saturating_sub(self.next_start);
            (1, remaining.checked_add(1))
        }
    }
}

impl FusedIterator for Lines<'_> {}

/// Line table of one text, for byte offset ↔ [`LineCol`] conversion.
///
/// It is built once per text and holds no reference to it: the methods that need the text take
/// it again, and must be given the same text (another text gives meaningless results, never a
/// panic).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineIndex {
    /// Content range of every line, in order (never empty).
    lines: Vec<TextRange>,
}

impl LineIndex {
    /// Indexes the lines of `text`, with the terminators of [`lines`].
    pub fn new(text: &str) -> Self {
        Self {
            lines: lines(text).map(|(range, _)| range).collect(),
        }
    }

    /// Returns the number of lines, at least 1.
    pub fn line_count(&self) -> u32 {
        offset_u32(self.lines.len())
    }

    /// Returns the content range (without terminator) of the 1-based `line`, or `None` if there
    /// is no such line.
    pub fn line_range(&self, line: u32) -> Option<TextRange> {
        let index = line.checked_sub(1)?;
        self.lines.get(index as usize).copied()
    }

    /// Converts a byte offset to a 1-based line and char column.
    ///
    /// `offset` is clamped to `text.len()`. An offset inside a line terminator gives the column
    /// just past the line content, and an offset inside a multi-byte char gives that char's
    /// column.
    pub fn position(&self, text: &str, offset: u32) -> LineCol {
        let offset = offset.min(offset_u32(text.len()));
        let index = self
            .lines
            .partition_point(|range| range.start <= offset)
            .saturating_sub(1);
        let Some(range) = self.lines.get(index) else {
            return LineCol { line: 1, column: 1 };
        };
        let mut end = offset.min(range.end) as usize;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        let chars = text
            .get(range.start as usize..end)
            .map_or(0, |prefix| prefix.chars().count());
        LineCol {
            line: offset_u32(index + 1),
            column: offset_u32(chars + 1),
        }
    }

    /// Converts a 1-based line and char column to a byte offset.
    ///
    /// The column just past the last char of the line is its end offset; a line or column of 0,
    /// a line past the end, or a column further right gives `None`.
    pub fn offset(&self, text: &str, at: LineCol) -> Option<u32> {
        let range = self.line_range(at.line)?;
        let skip = at.column.checked_sub(1)? as usize;
        let content = range.slice(text)?;
        let within = content
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(content.len()))
            .nth(skip)?;
        Some(range.start.saturating_add(offset_u32(within)))
    }
}
