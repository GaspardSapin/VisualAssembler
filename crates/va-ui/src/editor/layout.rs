//! Geometry of the editor view (m4-editor-shell.md §5.2): fixed-height rows, the visible line
//! range, the capped line fetch and the display↔byte map of tab-expanded lines.
//!
//! Every function here is O(1) or O(length of one drawn line); nothing walks the document.

use std::ops::Range;

use egui::{Pos2, Rect, Vec2};
use va_editor::text::Text;

/// At most this many chars of a line are laid out; a longer line shows a truncation mark and
/// positions past the cap are drawn at the cap.
pub const MAX_DRAWN_CHARS: usize = 4_096;

/// Most bytes fetched for one drawn line: [`MAX_DRAWN_CHARS`] chars of at most 4 bytes.
const MAX_FETCHED_BYTES: usize = MAX_DRAWN_CHARS * 4;

/// Geometry of the last frame of an [`EditorView`](super::EditorView), for tests, the bench and
/// the shell (an extra public helper of §5.2).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewMetrics {
    /// Height of one row (the code font's row height), in points.
    pub row_height: f32,
    /// Advance of one char of the code font (a space), in points.
    pub char_width: f32,
    /// Width of the gutter (numbers and the M5 marker column), in points.
    pub gutter_width: f32,
    /// Size of the scrolled content: `line_count × row_height` high.
    pub content_size: Vec2,
    /// Size of the visible part (the scroll area's viewport).
    pub viewport_size: Vec2,
}

/// The geometry of one frame, in screen coordinates.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Geometry {
    /// Screen position of the content's top-left corner (moves with the scroll offset).
    pub origin: Pos2,
    /// Screen rectangle of the viewport.
    pub visible: Rect,
    /// Row height.
    pub row_height: f32,
    /// Char advance.
    pub char_width: f32,
    /// Gutter width.
    pub gutter_width: f32,
    /// Lines of the document (at least 1).
    pub line_count: usize,
}

impl Geometry {
    /// Screen x of the start of every line's text (scrolls horizontally).
    pub fn text_left(&self) -> f32 {
        self.origin.x + self.gutter_width
    }

    /// Screen y of the top of `line`.
    pub fn line_top(&self, line: usize) -> f32 {
        self.origin.y + line as f32 * self.row_height
    }

    /// The line under screen y, clamped to the document.
    pub fn line_at_y(&self, y: f32) -> usize {
        let row = ((y - self.origin.y) / self.row_height).floor().max(0.0) as usize;
        row.min(self.line_count - 1)
    }

    /// The first line intersecting the viewport.
    pub fn top_line(&self) -> usize {
        self.line_at_y(self.visible.min.y)
    }

    /// The lines to fetch and paint: those intersecting the viewport, plus one above and one
    /// below.
    pub fn drawn_lines(&self) -> Range<usize> {
        let top = ((self.visible.min.y - self.origin.y) / self.row_height).floor();
        let bottom = ((self.visible.max.y - self.origin.y) / self.row_height).ceil();
        let first = (top.max(0.0) as usize).saturating_sub(1);
        let end = (bottom.max(0.0) as usize)
            .saturating_add(1)
            .min(self.line_count);
        first.min(end)..end
    }

    /// Whole rows that fit in the viewport, minus one (PageUp/PageDown), at least 1.
    pub fn page_rows(&self) -> u32 {
        let rows = (self.visible.height() / self.row_height).floor() as u32;
        rows.saturating_sub(1).max(1)
    }

    /// The content rectangle (all lines, full width) in screen coordinates.
    pub fn content_rect(&self, size: Vec2) -> Rect {
        Rect::from_min_size(self.origin, size)
    }
}

/// Copies the drawn part of `line` into `out` (cleared first): the whole content when it has at
/// most [`MAX_DRAWN_CHARS`] chars, else its first [`MAX_DRAWN_CHARS`] chars. Returns whether the
/// line was truncated. Copies at most [`MAX_FETCHED_BYTES`] bytes, whatever the line length.
pub(crate) fn fetch_line(text: &Text, line: usize, out: &mut String) -> bool {
    let start = text.line_start(line);
    let end = text.line_end(line);
    if end - start <= MAX_FETCHED_BYTES {
        text.line_text_into(line, out);
    } else {
        out.clear();
        out.push_str(&text.slice_to_string(start, start + MAX_FETCHED_BYTES));
    }
    if let Some((cut, _)) = out.char_indices().nth(MAX_DRAWN_CHARS) {
        out.truncate(cut);
    }
    out.len() < end - start
}

/// One tab of a drawn line, expanded to `width` spaces starting at display char `display`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TabSpan {
    /// Byte offset of the tab in the line.
    pub byte: usize,
    /// Display char index of its first space.
    pub display: usize,
    /// Number of spaces it shows as (1..=tab width).
    pub width: usize,
}

/// The largest char boundary of `line` at or before `byte` (clamped to the length).
fn floor_boundary(line: &str, byte: usize) -> usize {
    let mut byte = byte.min(line.len());
    while !line.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}

/// Display char index of `byte` in the drawn `line` whose tabs are `tabs` (sorted by byte); a
/// byte past the drawn text maps to its end.
pub(crate) fn display_index(line: &str, tabs: Option<&[TabSpan]>, byte: usize) -> usize {
    let byte = floor_boundary(line, byte);
    let tabs = tabs.unwrap_or(&[]);
    let before = tabs.partition_point(|tab| tab.byte < byte);
    match before.checked_sub(1).and_then(|index| tabs.get(index)) {
        Some(tab) => tab.display + tab.width + line[tab.byte + 1..byte].chars().count(),
        None => line[..byte].chars().count(),
    }
}

/// The byte of the drawn `line` at display char index `display`: inside a tab's spaces, the
/// nearer edge of the tab; past the end, the line length.
pub(crate) fn byte_at_display(line: &str, tabs: Option<&[TabSpan]>, display: usize) -> usize {
    let tabs = tabs.unwrap_or(&[]);
    let (mut byte, mut col) = (0, 0);
    let after = tabs.partition_point(|tab| tab.display <= display);
    if let Some(tab) = after.checked_sub(1).and_then(|index| tabs.get(index)) {
        let inside = display - tab.display;
        if inside < tab.width {
            return if inside * 2 <= tab.width {
                tab.byte
            } else {
                tab.byte + 1
            };
        }
        byte = tab.byte + 1;
        col = tab.display + tab.width;
    }
    for (offset, _) in line[byte..].char_indices() {
        if col == display {
            return byte + offset;
        }
        col += 1;
    }
    line.len()
}
