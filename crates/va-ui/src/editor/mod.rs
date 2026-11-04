//! The editor widget (m4-editor-shell.md §5.2–§5.6): a custom, virtualized view of a
//! `va_editor::document::Document` inside `egui::ScrollArea::both().show_viewport`, coloured by
//! the architecture's lexer, never an egui `TextEdit`.
//!
//! Every per-frame cost is O(visible lines): only the lines that intersect the viewport, plus one
//! above and below, are fetched, lexed (through a per-line cache) and painted.
//!
//! The items live in private modules and are re-exported here by name, so the paths
//! `va_ui::editor::*` are the public ones (m4 §15.1 M2):
//! - `view`: [`EditorView`] and the frozen types of §5.2 ([`EditorStyle`], [`EditorContext`],
//!   [`EditorResponse`], [`RenderStats`], [`Decoration`], [`DecorationKind`]);
//! - `layout`: rows, the visible line range, the capped line fetch ([`MAX_DRAWN_CHARS`]) and the
//!   display↔byte map of tab-expanded lines; [`ViewMetrics`];
//! - `cache`: colouring and the per-line cache (§5.3, [`LINE_CACHE_CAPACITY`]);
//! - `input`: keys, clipboard, IME and mouse (§5.6);
//! - `caret`: blink ([`BLINK_HALF_PERIOD`], [`BLINK_TIMEOUT`]) and caret painting (§5.6);
//! - `gutter`: line numbers and the M5 marker column ([`gutter_digits`], [`gutter_columns`]);
//! - `plain`: [`PlainSyntax`], the lexer of a build without any CPU.

mod cache;
mod caret;
mod gutter;
mod input;
mod layout;
mod plain;
mod view;

pub use cache::LINE_CACHE_CAPACITY;
pub use caret::{BLINK_HALF_PERIOD, BLINK_TIMEOUT};
pub use gutter::{gutter_columns, gutter_digits};
pub use layout::{MAX_DRAWN_CHARS, ViewMetrics};
pub use plain::PlainSyntax;
pub use view::{
    Decoration, DecorationKind, EditorContext, EditorResponse, EditorStyle, EditorView, RenderStats,
};
