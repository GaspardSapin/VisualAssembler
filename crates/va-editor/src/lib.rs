//! Text buffer (ropey), selections, grouped undo/redo, search/replace (m4-editor-shell.md §3).
//! No UI, no threads, no I/O beyond `std::io::Write` sinks.
//!
//! Allowed internal dependencies: `va-core`.
//!
//! All offsets are **bytes** into the document text (after a stripped BOM), on char boundaries;
//! lines are **0-based** `usize`. Time is passed in as a `Duration` since an arbitrary origin, so
//! tests control it.

pub mod document;
pub mod edit;
pub mod encoding;
pub mod history;
pub mod search;
pub mod selection;
pub mod text;
