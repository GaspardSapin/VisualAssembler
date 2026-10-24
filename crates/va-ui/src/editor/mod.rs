//! The editor widget (m4-editor-shell.md §5.2–§5.6): a custom, virtualized view of a
//! `va_editor::document::Document` inside `egui::ScrollArea::both().show_viewport`, coloured by
//! the architecture's lexer, never an egui `TextEdit`.
//!
//! Every per-frame cost is O(visible lines): only the lines that intersect the viewport, plus one
//! above and below, are fetched, lexed (through a per-line cache) and painted.
//!
//! WP0 scaffold: the frozen signatures of §5.2 with `todo!()` bodies. WP-E implements them and
//! may move them into `view.rs`, `layout.rs`, `gutter.rs`, `plain.rs` and the other modules of
//! §2, re-exported here so the paths `va_ui::editor::*` stay valid.

#![expect(
    unused_variables,
    dead_code,
    clippy::todo,
    reason = "WP0 stub: WP-E implements m4 §5.2-§5.6"
)]

use va_core::syntax::{KeywordInfo, LineState, SyntaxProvider, Token};
use va_editor::document::Document;
use va_editor::edit::EditOptions;

use crate::theme::Theme;

/// How the editor draws: font, tabs, gutter and caret options.
#[derive(Clone, Debug, PartialEq)]
pub struct EditorStyle {
    /// The code font (family `va-code`) and its size.
    pub font: egui::FontId,
    /// Tab stops, in columns.
    pub tab_width: u8,
    /// Whether the gutter shows line numbers.
    pub line_numbers: bool,
    /// Whether the caret's line gets the current-line background.
    pub highlight_current_line: bool,
    /// Whether the caret blinks (§5.6).
    pub cursor_blink: bool,
    /// Bumped on every change of theme, font, size, zoom, `pixels_per_point` or tab width; a
    /// change clears the line cache.
    pub style_generation: u64,
}

/// What a decoration marks. M5 adds `Diagnostic(Severity)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DecorationKind {
    /// A search match.
    SearchMatch,
    /// The current search match.
    CurrentSearchMatch,
}

/// A decorated byte range of the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Decoration {
    /// Start byte.
    pub start: usize,
    /// End byte.
    pub end: usize,
    /// What it marks.
    pub kind: DecorationKind,
}

/// Everything [`EditorView::show`] needs besides the document.
#[derive(Clone, Copy)]
pub struct EditorContext<'a> {
    /// The lexer colouring the lines.
    pub syntax: &'a dyn SyntaxProvider,
    /// The colours.
    pub theme: &'a Theme,
    /// The drawing options.
    pub style: &'a EditorStyle,
    /// The indentation options of the edit commands.
    pub options: EditOptions,
    /// Decorations to paint, sorted by start.
    pub decorations: &'a [Decoration],
    /// Whether edits are refused.
    pub read_only: bool,
    /// The line comment prefix of the architecture, if any.
    pub line_comment: Option<&'static str>,
    /// Accessibility label of the widget (the file name).
    pub label: &'a str,
}

/// What one [`EditorView::show`] did.
#[derive(Clone, Debug)]
pub struct EditorResponse {
    /// The widget's egui response.
    pub response: egui::Response,
    /// Whether the text changed.
    pub changed: bool,
    /// Whether a caret moved.
    pub caret_moved: bool,
}

/// Counters of the last frame, for tests and the bench.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RenderStats {
    /// Lines laid out and painted.
    pub lines_drawn: usize,
    /// Lines passed to the lexer.
    pub lexed_lines: usize,
    /// Line cache hits.
    pub cache_hits: usize,
    /// Line cache misses.
    pub cache_misses: usize,
}

/// The editor widget state of one tab: id, line cache, blink and IME state, last frame's stats.
#[derive(Debug)]
pub struct EditorView {
    /// The widget id.
    id: egui::Id,
}

impl EditorView {
    /// A view with an empty cache.
    pub fn new(id: egui::Id) -> Self {
        todo!()
    }

    /// The counters of the last frame.
    pub fn stats(&self) -> RenderStats {
        todo!()
    }

    /// The first visible line (0-based).
    pub fn top_line(&self) -> usize {
        todo!()
    }

    /// Draws `doc` and applies this frame's input to it.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut Document,
        ctx: &EditorContext<'_>,
    ) -> EditorResponse {
        todo!()
    }

    /// Scrolls so that `line` (0-based) is visible on the next frame.
    pub fn scroll_to_line(&mut self, line: usize) {
        todo!()
    }

    /// Gives the widget keyboard focus.
    pub fn request_focus(&self, ui: &egui::Ui) {
        todo!()
    }
}

/// The lexer used when the build has no architecture: whitespace runs, and one `Symbol` token
/// per other run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PlainSyntax;

impl SyntaxProvider for PlainSyntax {
    fn lex_line(&self, line: &str, state: LineState, out: &mut Vec<Token>) -> LineState {
        todo!()
    }

    fn keywords(&self) -> &[KeywordInfo] {
        &[]
    }
}
