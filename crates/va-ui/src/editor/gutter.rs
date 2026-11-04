//! The gutter (m4-editor-shell.md §5.2): right-aligned 1-based line numbers, as wide as the
//! digit count of the line count (at least 3), then a one-char marker column reserved for M5
//! (empty in M4). It is painted at the viewport's left edge, so it never scrolls horizontally.
//!
//! Columns, left to right: one padding column, the digits, the marker column, one gap column
//! before the text. Without line numbers only the marker and gap columns remain.

use egui::{Align2, Color32, FontId, Painter, pos2};

/// Smallest number of digit columns.
const MIN_DIGITS: usize = 3;

/// Digit columns of the gutter for `line_count` lines: the digit count of `line_count`, at
/// least 3. O(1) in the document size.
pub fn gutter_digits(line_count: usize) -> usize {
    let mut digits = 1;
    let mut rest = line_count;
    while rest >= 10 {
        rest /= 10;
        digits += 1;
    }
    digits.max(MIN_DIGITS)
}

/// Width of the gutter in char columns: padding, [`gutter_digits`], marker and gap when
/// `line_numbers`, else the marker and gap columns only.
pub fn gutter_columns(line_count: usize, line_numbers: bool) -> usize {
    if line_numbers {
        gutter_digits(line_count) + 3
    } else {
        2
    }
}

/// Where and how the gutter of one frame paints its numbers.
pub(crate) struct Gutter<'a> {
    /// Screen x of the gutter's left edge (the viewport's left edge).
    pub left: f32,
    /// Char advance of the code font.
    pub char_width: f32,
    /// Digit columns ([`gutter_digits`]).
    pub digits: usize,
    /// The code font.
    pub font: &'a FontId,
}

impl Gutter<'_> {
    /// Paints the number of `line` (0-based, shown 1-based) right-aligned in the digit columns,
    /// on the row starting at `top`.
    pub fn paint_number(&self, painter: &Painter, top: f32, line: usize, color: Color32) {
        let right = self.left + (1 + self.digits) as f32 * self.char_width;
        painter.text(
            pos2(right, top),
            Align2::RIGHT_TOP,
            (line + 1).to_string(),
            self.font.clone(),
            color,
        );
    }
}
