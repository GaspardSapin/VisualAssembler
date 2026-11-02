//! The document text and display columns (m4-editor-shell.md §3.1).
//!
//! [`Text`] wraps a `ropey::Rope` built with only the `cr_lines` feature: line breaks are LF,
//! CRLF and a lone CR, never U+0085 or U+2028, so its line count equals
//! `va_core::source::LineIndex::line_count`. Terminators are stored verbatim.

use std::fmt;
use std::io;
use std::ops::ControlFlow;

use ropey::{Rope, RopeSlice};

/// Largest document the editor holds, in bytes: the `va-core` source limit (16 MiB).
pub const MAX_DOCUMENT_BYTES: usize = va_core::source::MAX_SOURCE_BYTES;

/// A document text: a rope whose line breaks are LF, CRLF and lone CR (ropey `cr_lines`),
/// never U+0085 or U+2028. `Clone` is O(1) (shared nodes); the type is `Send + Sync`.
#[derive(Clone, Debug, Default)]
pub struct Text {
    /// The rope holding the text, terminators stored verbatim.
    rope: Rope,
}

impl PartialEq for Text {
    fn eq(&self, other: &Self) -> bool {
        self.rope == other.rope
    }
}

impl Eq for Text {}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.rope.chunks().try_for_each(|chunk| f.write_str(chunk))
    }
}

/// One line handed to [`Text::scan_lines`]: its index, start offset, content and terminator.
pub(crate) struct LineRef<'a> {
    /// 0-based line index.
    pub(crate) index: usize,
    /// Byte offset of the line start in the text.
    pub(crate) start: usize,
    /// The content, without the terminator.
    pub(crate) content: &'a str,
    /// The terminator: `"\n"`, `"\r\n"`, `"\r"` or `""` on the last line.
    pub(crate) terminator: &'a str,
}

/// Length in bytes of the line terminator that ends `line` (0 when it has none).
fn terminator_len(line: &str) -> usize {
    if line.ends_with("\r\n") {
        2
    } else if line.ends_with('\n') || line.ends_with('\r') {
        1
    } else {
        0
    }
}

/// Number of chars of the terminator that ends the rope line `line` (0, 1 or 2).
fn terminator_chars(line: RopeSlice<'_>) -> usize {
    let chars = line.len_chars();
    match chars.checked_sub(1).and_then(|last| line.get_char(last)) {
        Some('\n') if chars >= 2 && line.get_char(chars - 2) == Some('\r') => 2,
        Some('\n' | '\r') => 1,
        _ => 0,
    }
}

impl Text {
    /// Builds the text of `text` (O(n)).
    pub fn new(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
        }
    }

    /// Wraps a rope built by this crate (`replace_all`).
    pub(crate) fn from_rope(rope: Rope) -> Self {
        Self { rope }
    }

    /// The rope, for the other modules of this crate.
    pub(crate) fn rope(&self) -> &Rope {
        &self.rope
    }

    /// The rope, mutably, for [`crate::edit::ChangeSet`] application.
    pub(crate) fn rope_mut(&mut self) -> &mut Rope {
        &mut self.rope
    }

    /// Length of the text in bytes.
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Tells whether the text holds no byte (it still has one, empty, line).
    pub fn is_empty(&self) -> bool {
        self.rope.len_bytes() == 0
    }

    /// Number of lines, at least 1; equal to `va_core::source::LineIndex::line_count`.
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// `line` clamped to the last line.
    fn clamp_line(&self, line: usize) -> usize {
        line.min(self.line_count().saturating_sub(1))
    }

    /// Byte offset of the start of `line` (clamped to the last line).
    pub fn line_start(&self, line: usize) -> usize {
        self.rope.line_to_byte(self.clamp_line(line))
    }

    /// Byte offset of the end of the content of `line`, before its terminator (line clamped).
    pub fn line_end(&self, line: usize) -> usize {
        let line = self.clamp_line(line);
        if line + 1 >= self.line_count() {
            return self.len_bytes();
        }
        let start = self.rope.line_to_byte(line);
        let next = self.rope.line_to_byte(line + 1);
        let Some(last) = next.checked_sub(1) else {
            return start;
        };
        if self.rope.get_byte(last) == Some(b'\n')
            && last > start
            && self.rope.get_byte(last - 1) == Some(b'\r')
        {
            last - 1
        } else {
            last.max(start)
        }
    }

    /// The line holding `byte` (clamped to the length).
    pub fn line_of(&self, byte: usize) -> usize {
        self.rope.byte_to_line(byte.min(self.len_bytes()))
    }

    /// Clears `out`, then copies the content of `line` (no terminator) into it.
    pub fn line_text_into(&self, line: usize, out: &mut String) {
        out.clear();
        let start = self.line_start(line);
        let end = self.line_end(line);
        for chunk in self.rope.byte_slice(start..end).chunks() {
            out.push_str(chunk);
        }
    }

    /// The content of `line` (no terminator), borrowed from the rope when it is contiguous,
    /// else copied into `buf`.
    pub(crate) fn line_content<'a>(&'a self, line: usize, buf: &'a mut String) -> &'a str {
        let start = self.line_start(line);
        let end = self.line_end(line);
        let slice = self.rope.byte_slice(start..end);
        match slice.as_str() {
            Some(text) => text,
            None => {
                buf.clear();
                for chunk in slice.chunks() {
                    buf.push_str(chunk);
                }
                buf.as_str()
            }
        }
    }

    /// Copies the bytes `start..end` into a new string. Both ends are clamped to the length
    /// and moved down to a char boundary, so the call never panics.
    pub fn slice_to_string(&self, start: usize, end: usize) -> String {
        let end = self.floor_boundary(end);
        let start = self.floor_boundary(start.min(end));
        let mut out = String::with_capacity(end - start);
        for chunk in self.rope.byte_slice(start..end).chunks() {
            out.push_str(chunk);
        }
        out
    }

    /// The text as a sequence of string chunks, in order.
    pub fn chunks(&self) -> impl Iterator<Item = &str> + '_ {
        self.rope.chunks()
    }

    /// Writes the whole text to `out`, chunk by chunk.
    ///
    /// # Errors
    /// The first error of `out`.
    pub fn write_to(&self, out: &mut dyn io::Write) -> io::Result<()> {
        for chunk in self.rope.chunks() {
            out.write_all(chunk.as_bytes())?;
        }
        Ok(())
    }

    /// Length in chars of the longest line (O(n): used on load and on replace only).
    pub fn widest_line_chars(&self) -> usize {
        self.rope
            .lines()
            .map(|line| line.len_chars() - terminator_chars(line))
            .max()
            .unwrap_or(0)
    }

    /// Length in chars of the content of `line` (O(log n)).
    pub(crate) fn line_chars(&self, line: usize) -> usize {
        let slice = self.rope.line(self.clamp_line(line));
        slice.len_chars() - terminator_chars(slice)
    }

    /// Tells whether `byte` is at most the length and on a char boundary.
    pub fn is_char_boundary(&self, byte: usize) -> bool {
        byte <= self.len_bytes() && self.floor_boundary(byte) == byte
    }

    /// `byte` clamped to the length and moved down to a char boundary.
    pub(crate) fn floor_boundary(&self, byte: usize) -> usize {
        let byte = byte.min(self.len_bytes());
        self.rope.char_to_byte(self.rope.byte_to_char(byte))
    }

    /// `byte` clamped, on a char boundary and never between the CR and the LF of a CRLF pair
    /// (moved before the CR): a valid caret position.
    pub(crate) fn snap(&self, byte: usize) -> usize {
        let byte = self.floor_boundary(byte);
        if byte > 0
            && self.rope.get_byte(byte) == Some(b'\n')
            && self.rope.get_byte(byte - 1) == Some(b'\r')
        {
            byte - 1
        } else {
            byte
        }
    }

    /// The previous caret position before `byte`: one char back, a CRLF pair counting as one.
    pub(crate) fn prev_boundary(&self, byte: usize) -> usize {
        let index = self.rope.byte_to_char(byte.min(self.len_bytes()));
        let Some(before) = index.checked_sub(1) else {
            return 0;
        };
        let target = if self.rope.get_char(before) == Some('\n')
            && before > 0
            && self.rope.get_char(before - 1) == Some('\r')
        {
            before - 1
        } else {
            before
        };
        self.rope.char_to_byte(target)
    }

    /// The next caret position after `byte`: one char forward, a CRLF pair counting as one.
    pub(crate) fn next_boundary(&self, byte: usize) -> usize {
        let index = self.rope.byte_to_char(byte.min(self.len_bytes()));
        let target = match self.rope.get_char(index) {
            None => index,
            Some('\r') if self.rope.get_char(index + 1) == Some('\n') => index + 2,
            Some(_) => index + 1,
        };
        self.rope.char_to_byte(target)
    }

    /// Display column of `byte` in its line (tabs expanded to `tab_width` stops). O(column).
    pub(crate) fn col_of(&self, byte: usize, tab_width: u8) -> u32 {
        let byte = self.floor_boundary(byte);
        let line = self.line_of(byte);
        let start = self.rope.byte_to_char(self.line_start(line));
        let count = self.rope.byte_to_char(byte) - start;
        self.rope
            .chars_at(start)
            .take(count)
            .fold(0, |col, c| advance(col, c, tab_width))
    }

    /// The char boundary of `line` at or before display column `col` (as
    /// [`byte_at_display_col`]), as a text offset. O(column).
    pub(crate) fn byte_at_col(&self, line: usize, col: u32, tab_width: u8) -> usize {
        let start = self.line_start(line);
        let end = self.line_end(line);
        let mut byte = start;
        let mut current = 0;
        for c in self.rope.chars_at(self.rope.byte_to_char(start)) {
            if byte >= end {
                break;
            }
            let next = advance(current, c, tab_width);
            if next > col {
                break;
            }
            current = next;
            byte += c.len_utf8();
        }
        byte
    }

    /// Calls `f` with every line from `first` to the end, in order, until it breaks; returns
    /// the break value. A line is borrowed from the rope when it is contiguous, else copied
    /// into one reused buffer.
    pub(crate) fn scan_lines<B>(
        &self,
        first: usize,
        mut f: impl FnMut(LineRef<'_>) -> ControlFlow<B>,
    ) -> Option<B> {
        if first >= self.line_count() {
            return None;
        }
        let mut buf = String::new();
        let mut start = self.rope.line_to_byte(first);
        for (offset, slice) in self.rope.lines_at(first).enumerate() {
            let whole: &str = match slice.as_str() {
                Some(text) => text,
                None => {
                    buf.clear();
                    for chunk in slice.chunks() {
                        buf.push_str(chunk);
                    }
                    &buf
                }
            };
            let (content, terminator) = whole.split_at(whole.len() - terminator_len(whole));
            let line = LineRef {
                index: first + offset,
                start,
                content,
                terminator,
            };
            if let ControlFlow::Break(value) = f(line) {
                return Some(value);
            }
            start += whole.len();
        }
        None
    }
}

/// The display column after `c` when it starts at column `col`: a tab advances to the next
/// multiple of `tab_width` (taken as 1 when 0), any other char by one.
pub(crate) fn advance(col: u32, c: char, tab_width: u8) -> u32 {
    if c == '\t' {
        let width = u32::from(tab_width.max(1));
        (col / width).saturating_add(1).saturating_mul(width)
    } else {
        col.saturating_add(1)
    }
}

/// `byte` clamped to `line` and moved down to a char boundary.
fn floor_char_boundary(line: &str, byte: usize) -> usize {
    let mut byte = byte.min(line.len());
    while !line.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}

/// Display column of `byte_in_line` in `line`: a tab advances to the next multiple of
/// `tab_width`, any other char by one. An offset inside a char counts from that char's start.
pub fn display_col(line: &str, byte_in_line: usize, tab_width: u8) -> u32 {
    let end = floor_char_boundary(line, byte_in_line);
    line[..end]
        .chars()
        .fold(0, |col, c| advance(col, c, tab_width))
}

/// The char boundary of `line` at or before display column `col` (tabs as in
/// [`display_col`]); `line.len()` past the end.
pub fn byte_at_display_col(line: &str, col: u32, tab_width: u8) -> usize {
    let mut current = 0;
    for (index, c) in line.char_indices() {
        let next = advance(current, c, tab_width);
        if next > col {
            return index;
        }
        current = next;
    }
    line.len()
}
