//! Text buffer (ropey), selections, grouped undo/redo, search/replace (m4-editor-shell.md §3).
//! No UI, no threads, no I/O beyond `std::io::Write` sinks.
//!
//! Allowed internal dependencies: `va-core`.
//!
//! All offsets are **bytes** into the document text (after a stripped BOM), on char boundaries;
//! lines are **0-based** `usize`. Time is passed in as a `Duration` since an arbitrary origin, so
//! tests control it.
//!
//! WP0 scaffold: each module below holds the frozen signatures of m4 §3 with `todo!()` bodies.
//! WP-A implements them and may move each inline module into its own file (`text.rs`, …); the
//! module paths (`va_editor::text::Text`, `va_editor::encoding::DecodeError`, …) stay as they are.

/// The document text and display columns (m4 §3.1).
pub mod text {
    #![expect(
        unused_variables,
        clippy::todo,
        clippy::ptr_arg,
        reason = "WP0 stub: WP-A implements m4 §3.1"
    )]

    /// Largest document the editor holds, in bytes: the `va-core` source limit (16 MiB).
    pub const MAX_DOCUMENT_BYTES: usize = va_core::source::MAX_SOURCE_BYTES;

    /// A document text: a rope whose line breaks are LF, CRLF and lone CR (ropey `cr_lines`),
    /// never U+0085 or U+2028. `Clone` is O(1) (shared nodes); the type is `Send + Sync`.
    #[derive(Clone, Debug)]
    pub struct Text {
        /// The rope holding the text, terminators stored verbatim.
        rope: ropey::Rope,
    }

    impl Text {
        /// Builds the text of `text` (O(n)).
        pub fn new(text: &str) -> Self {
            todo!()
        }

        /// Length of the text in bytes.
        pub fn len_bytes(&self) -> usize {
            todo!()
        }

        /// Number of lines, at least 1; equal to `va_core::source::LineIndex::line_count`.
        pub fn line_count(&self) -> usize {
            todo!()
        }

        /// Byte offset of the start of `line` (clamped to the last line).
        pub fn line_start(&self, line: usize) -> usize {
            todo!()
        }

        /// Byte offset of the end of the content of `line`, before its terminator (line clamped).
        pub fn line_end(&self, line: usize) -> usize {
            todo!()
        }

        /// The line holding `byte` (clamped to the length).
        pub fn line_of(&self, byte: usize) -> usize {
            todo!()
        }

        /// Clears `out`, then copies the content of `line` (no terminator) into it.
        pub fn line_text_into(&self, line: usize, out: &mut String) {
            todo!()
        }

        /// Copies the bytes `start..end` into a new string.
        pub fn slice_to_string(&self, start: usize, end: usize) -> String {
            todo!()
        }

        /// The text as a sequence of string chunks, in order.
        pub fn chunks(&self) -> impl Iterator<Item = &str> + '_ {
            self.rope.chunks()
        }

        /// Writes the whole text to `out`, chunk by chunk.
        ///
        /// # Errors
        /// The first error of `out`.
        pub fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
            todo!()
        }

        /// Length in chars of the longest line (O(n): used on load and on replace only).
        pub fn widest_line_chars(&self) -> usize {
            todo!()
        }
    }

    /// Display column of `byte_in_line` in `line`: a tab advances to the next multiple of
    /// `tab_width`, any other char by one.
    pub fn display_col(line: &str, byte_in_line: usize, tab_width: u8) -> u32 {
        todo!()
    }

    /// The char boundary of `line` at or before display column `col` (tabs as in
    /// [`display_col`]).
    pub fn byte_at_display_col(line: &str, col: u32, tab_width: u8) -> usize {
        todo!()
    }
}

/// File encodings, decoding and line endings (m4 §3.1).
pub mod encoding {
    #![expect(
        unused_variables,
        clippy::todo,
        reason = "WP0 stub: WP-A implements m4 §3.1"
    )]

    use crate::text::Text;

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
        /// Returns the identifier of the encoding: `"utf-8"`, `"utf-8-bom"` or `"windows-1252"`.
        pub const fn id(self) -> &'static str {
            match self {
                Self::Utf8 => "utf-8",
                Self::Utf8Bom => "utf-8-bom",
                Self::Windows1252 => "windows-1252",
            }
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
    /// # Errors
    /// A [`DecodeError`] for a too large, binary or non-UTF-8 file.
    pub fn decode(
        bytes: &[u8],
        forced: Option<Encoding>,
    ) -> Result<(String, Encoding), DecodeError> {
        todo!()
    }

    /// The first char of `text` that `encoding` cannot represent, with its byte offset.
    pub fn first_unencodable(text: &Text, encoding: Encoding) -> Option<(usize, char)> {
        todo!()
    }

    /// Writes `text` to `out` in `encoding` (with the BOM for [`Encoding::Utf8Bom`]).
    ///
    /// # Errors
    /// The first error of `out`, or `InvalidData` on the first char outside Windows-1252.
    pub fn encode_to(
        text: &Text,
        encoding: Encoding,
        out: &mut dyn std::io::Write,
    ) -> std::io::Result<()> {
        todo!()
    }

    /// The first line terminator of `text`, if it has one.
    pub fn detect_line_ending(text: &Text) -> Option<LineEnding> {
        todo!()
    }
}

/// Selections and carets (m4 §3.2).
pub mod selection {
    #![expect(
        unused_variables,
        clippy::todo,
        reason = "WP0 stub: WP-A implements m4 §3.2"
    )]

    /// Most carets a document holds; extra selections are dropped.
    pub const MAX_CARETS: usize = 10_000;

    /// One selection: from `anchor` to `head` (the caret), in bytes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Selection {
        /// The fixed end.
        pub anchor: usize,
        /// The moving end: the caret.
        pub head: usize,
        /// Display column kept by vertical moves.
        pub goal: Option<u32>,
    }

    impl Selection {
        /// An empty selection (a caret) at `at`.
        pub fn caret(at: usize) -> Self {
            todo!()
        }

        /// The lower end of the selection.
        pub fn start(&self) -> usize {
            todo!()
        }

        /// The upper end of the selection.
        pub fn end(&self) -> usize {
            todo!()
        }

        /// Tells whether the selection holds no text.
        pub fn is_empty(&self) -> bool {
            todo!()
        }
    }

    /// The selections of a document: non-empty, sorted, non-overlapping, one of them primary.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Selections {
        /// The selections, sorted and non-overlapping, never empty.
        list: Vec<Selection>,
        /// Index of the primary selection in `list`.
        primary: usize,
    }

    impl Selections {
        /// A set holding only `s`.
        pub fn single(s: Selection) -> Self {
            todo!()
        }

        /// Sorts, merges and caps `v` (at [`MAX_CARETS`]); `primary` indexes `v`.
        pub fn from_vec(v: Vec<Selection>, primary: usize) -> Self {
            todo!()
        }

        /// The selections, in document order.
        pub fn iter(&self) -> impl Iterator<Item = &Selection> {
            self.list.iter()
        }

        /// Number of selections, at least 1.
        pub fn len(&self) -> usize {
            todo!()
        }

        /// Always false: a set holds at least one selection.
        pub fn is_empty(&self) -> bool {
            false
        }

        /// The primary selection.
        pub fn primary(&self) -> &Selection {
            todo!()
        }

        /// Index of the primary selection in document order.
        pub fn primary_index(&self) -> usize {
            todo!()
        }

        /// Adds `s`, merged with any selection it overlaps; it becomes the primary one.
        pub fn push(&mut self, s: Selection) {
            todo!()
        }

        /// Keeps only the primary selection.
        pub fn collapse_to_primary(&mut self) {
            todo!()
        }
    }
}

/// Changes, edit commands and motions (m4 §3.2).
pub mod edit {
    #![expect(
        unused_variables,
        clippy::todo,
        reason = "WP0 stub: WP-A implements m4 §3.2"
    )]

    use crate::text::Text;

    /// Replacement of the old-text bytes `start..end` by `insert`.
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Change {
        /// Start of the replaced range in the old text.
        pub start: usize,
        /// End of the replaced range in the old text.
        pub end: usize,
        /// The inserted text.
        pub insert: String,
    }

    /// A set of changes applied together: sorted and non-overlapping.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ChangeSet {
        /// The changes, sorted and non-overlapping.
        changes: Vec<Change>,
    }

    /// Why [`ChangeSet::new`] rejected its changes.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum ChangeError {
        /// Two changes overlap.
        Overlap,
        /// A range ends past the text.
        OutOfBounds,
        /// A range end is not on a char boundary.
        NotCharBoundary,
    }

    /// Which side an offset sticks to when a change inserts text exactly there.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum Assoc {
        /// Before the inserted text.
        Before,
        /// After the inserted text.
        After,
    }

    impl ChangeSet {
        /// Checks and sorts `changes` against `text`.
        ///
        /// # Errors
        /// A [`ChangeError`] for overlapping, out-of-bounds or misaligned ranges.
        pub fn new(changes: Vec<Change>, text: &Text) -> Result<Self, ChangeError> {
            todo!()
        }

        /// The changes, sorted.
        pub fn changes(&self) -> &[Change] {
            &self.changes
        }

        /// Maps an old-text offset to the new text.
        pub fn map_offset(&self, byte: usize, assoc: Assoc) -> usize {
            todo!()
        }
    }

    /// An editing command, applied at every selection, back to front, in one [`ChangeSet`]
    /// (m4 §3.2 table).
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub enum EditCommand {
        /// Replaces each selection with the text.
        InsertText(String),
        /// Pastes the text, one line per caret when the line and caret counts match.
        Paste(String),
        /// Inserts the document line ending and the current line's indentation.
        NewLine,
        /// Inserts a tab or spaces, or indents a selection spanning lines.
        Tab,
        /// Adds one indentation level at each touched line start.
        Indent,
        /// Removes one indentation level at each touched line start.
        Outdent,
        /// Deletes the selection, else the char before the caret.
        Backspace,
        /// Deletes the selection, else the char after the caret.
        Delete,
        /// Deletes to the previous word boundary.
        DeleteWordLeft,
        /// Deletes to the next word boundary.
        DeleteWordRight,
        /// Deletes the selected text.
        DeleteSelection,
        /// Deletes every touched line.
        DeleteLines,
        /// Duplicates every touched line.
        DuplicateLines,
        /// Moves every touched line up by one.
        MoveLinesUp,
        /// Moves every touched line down by one.
        MoveLinesDown,
        /// Comments or uncomments every touched line with `prefix`.
        ToggleLineComment {
            /// The line comment prefix of the architecture (e.g. `";"`).
            prefix: &'static str,
        },
    }

    /// A caret motion.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum Motion {
        /// One char left.
        Left,
        /// One char right.
        Right,
        /// To the previous word boundary.
        WordLeft,
        /// To the next word boundary.
        WordRight,
        /// One line up, keeping the goal column.
        Up,
        /// One line down, keeping the goal column.
        Down,
        /// Smart home: the first non-blank char, then column 0.
        LineStart,
        /// The end of the line content.
        LineEnd,
        /// That many lines up.
        PageUp(u32),
        /// That many lines down.
        PageDown(u32),
        /// The start of the document.
        DocStart,
        /// The end of the document.
        DocEnd,
        /// That byte offset.
        To(usize),
    }

    /// Indentation options of the edit commands.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct EditOptions {
        /// Tab stops, in columns.
        pub tab_width: u8,
        /// Whether `Tab` inserts spaces instead of a tab character.
        pub insert_spaces: bool,
    }

    /// Tells whether `c` belongs to a word: alphanumeric, `_`, `.` or `@` (assembly
    /// identifiers).
    pub fn is_word_char(c: char) -> bool {
        todo!()
    }
}

/// Undo grouping limits and edit kinds (m4 §3.3).
pub mod history {
    /// Longest pause between two edits of one undo group, in milliseconds.
    pub const UNDO_GROUP_MS: u64 = 1_000;

    /// Most undo groups kept; the oldest are dropped first.
    pub const MAX_UNDO_GROUPS: usize = 10_000;

    /// Most inserted plus deleted bytes kept by the undo history.
    pub const MAX_UNDO_BYTES: usize = 64 * 1024 * 1024;

    /// The kind of an edit, for undo grouping.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum EditKind {
        /// One char typed per caret.
        Typing,
        /// `Backspace` or `Delete`.
        Deleting,
        /// Anything else: always closes the open group.
        Other,
    }
}

/// The document: text, selections, history and dirty state (m4 §3.3).
pub mod document {
    #![expect(
        unused_variables,
        dead_code,
        clippy::todo,
        reason = "WP0 stub: WP-A implements m4 §3.3"
    )]

    use std::time::Duration;

    use crate::edit::{ChangeSet, EditCommand, EditOptions, Motion};
    use crate::encoding::{Encoding, LineEnding};
    use crate::history::EditKind;
    use crate::selection::Selections;
    use crate::text::Text;

    /// A frozen state of a document, for workers: `Send + Sync`, taken in O(1).
    #[derive(Clone, Debug)]
    pub struct Snapshot {
        /// The text.
        pub text: Text,
        /// The revision it was taken at.
        pub revision: u64,
        /// The encoding of the document.
        pub encoding: Encoding,
        /// The line ending of the document.
        pub line_ending: LineEnding,
    }

    /// An open document.
    #[derive(Debug)]
    pub struct Document {
        /// The current text.
        text: Text,
        /// The current selections.
        selections: Selections,
        /// Incremented by every applied change, undo and redo included.
        revision: u64,
        /// The encoding used on save.
        encoding: Encoding,
        /// The terminator of new line breaks.
        line_ending: LineEnding,
    }

    impl Document {
        /// A clean document of `text`; its line ending is the detected one, else `default_eol`.
        pub fn new(text: Text, encoding: Encoding, default_eol: LineEnding) -> Self {
            todo!()
        }

        /// The current text.
        pub fn text(&self) -> &Text {
            todo!()
        }

        /// The current selections.
        pub fn selections(&self) -> &Selections {
            todo!()
        }

        /// Replaces the selections (closes the open undo group).
        pub fn set_selections(&mut self, s: Selections) {
            todo!()
        }

        /// The revision: +1 per applied change, undo and redo included.
        pub fn revision(&self) -> u64 {
            todo!()
        }

        /// Tells whether the content differs from the saved state.
        pub fn is_dirty(&self) -> bool {
            todo!()
        }

        /// Marks the current content as saved.
        pub fn mark_saved(&mut self) {
            todo!()
        }

        /// The encoding used on save.
        pub fn encoding(&self) -> Encoding {
            todo!()
        }

        /// Changes the encoding used on save; makes the document dirty.
        pub fn set_encoding(&mut self, e: Encoding) {
            todo!()
        }

        /// The terminator of new line breaks.
        pub fn line_ending(&self) -> LineEnding {
            todo!()
        }

        /// An upper bound of the widest line in chars, grown by edits (exact after
        /// [`Document::replace_text`]).
        pub fn widest_line_hint(&self) -> usize {
            todo!()
        }

        /// Runs `cmd` at every selection at time `at`; returns false when nothing changed.
        pub fn edit(&mut self, cmd: EditCommand, opts: &EditOptions, at: Duration) -> bool {
            todo!()
        }

        /// Applies external changes (M5 quick fix) as an edit of `kind` at time `at`.
        pub fn apply(&mut self, changes: ChangeSet, kind: EditKind, at: Duration) {
            todo!()
        }

        /// Replaces the whole text in one undo group (replace all, reload).
        pub fn replace_text(&mut self, text: Text) {
            todo!()
        }

        /// Moves every caret by `motion`, extending the selections when `extend`.
        pub fn move_carets(&mut self, motion: Motion, extend: bool, tab_width: u8) {
            todo!()
        }

        /// Adds a caret on the line above (`up`) or below the primary one.
        pub fn add_caret_vertical(&mut self, up: bool, tab_width: u8) {
            todo!()
        }

        /// Selects the word under the caret, then the next match of the primary selection
        /// (Ctrl+D); returns false when nothing was added.
        pub fn select_next_occurrence(&mut self) -> bool {
            todo!()
        }

        /// Selects the word at `byte`.
        pub fn select_word_at(&mut self, byte: usize) {
            todo!()
        }

        /// Selects `line`, terminator included.
        pub fn select_line(&mut self, line: usize) {
            todo!()
        }

        /// The text of every selection, in document order.
        pub fn selected_texts(&self) -> Vec<String> {
            todo!()
        }

        /// Undoes the last group; returns false when there is none.
        pub fn undo(&mut self) -> bool {
            todo!()
        }

        /// Redoes the last undone group; returns false when there is none.
        pub fn redo(&mut self) -> bool {
            todo!()
        }

        /// A snapshot of the current state, in O(1).
        pub fn snapshot(&self) -> Snapshot {
            todo!()
        }
    }
}

/// Find and replace, line by line, with a linear-time regex engine (m4 §3.4).
pub mod search {
    #![expect(
        unused_variables,
        dead_code,
        clippy::todo,
        reason = "WP0 stub: WP-A implements m4 §3.4"
    )]

    use std::sync::atomic::AtomicBool;

    use crate::text::Text;

    /// A search request.
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Query {
        /// The pattern: a literal text, or a regex when `regex`.
        pub pattern: String,
        /// Whether `pattern` is a regular expression.
        pub regex: bool,
        /// Whether case matters.
        pub case_sensitive: bool,
        /// Whether a match must be a whole word.
        pub whole_word: bool,
    }

    /// Why [`Matcher::new`] rejected a query.
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub enum QueryError {
        /// The pattern is empty.
        Empty,
        /// The regex does not parse.
        Invalid {
            /// The regex crate's message, shown raw.
            detail: String,
        },
        /// The compiled regex exceeds a size or nesting limit.
        TooBig,
    }

    /// A compiled query: `Clone + Send + Sync`.
    #[derive(Clone, Debug)]
    pub struct Matcher {
        /// The compiled pattern.
        regex: regex::Regex,
    }

    impl Matcher {
        /// Compiles `query` within [`REGEX_SIZE_LIMIT`] and [`REGEX_NEST_LIMIT`].
        ///
        /// # Errors
        /// A [`QueryError`] for an empty, invalid or too big pattern.
        pub fn new(query: &Query) -> Result<Self, QueryError> {
            todo!()
        }
    }

    /// Most matches collected by one search.
    pub const MAX_MATCHES: usize = 100_000;

    /// Size limit of a compiled regex, in bytes (the DFA gets twice as much).
    pub const REGEX_SIZE_LIMIT: usize = 1 << 20;

    /// Nesting limit of a regex.
    pub const REGEX_NEST_LIMIT: u32 = 64;

    /// The matches of a search.
    #[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Matches {
        /// Byte ranges of the matches, in order.
        pub ranges: Vec<(usize, usize)>,
        /// Whether the search stopped at [`MAX_MATCHES`].
        pub truncated: bool,
    }

    /// Every match of `m` in `text`; `None` when `cancel` was set.
    pub fn find_all(text: &Text, m: &Matcher, cancel: &AtomicBool) -> Option<Matches> {
        todo!()
    }

    /// The next (or previous, when `backwards`) match from `from`, wrapping once.
    pub fn find_from(
        text: &Text,
        m: &Matcher,
        from: usize,
        backwards: bool,
    ) -> Option<(usize, usize)> {
        todo!()
    }

    /// The replacement of the match `found` of `line` by `template` (`$1`, `${name}` and `$$`
    /// in regex mode; the template as is in literal mode).
    pub fn expand_replacement(
        m: &Matcher,
        line: &str,
        found: (usize, usize),
        template: &str,
    ) -> String {
        todo!()
    }

    /// The text with every match replaced, and the number of replacements; `None` when
    /// `cancel` was set.
    pub fn replace_all(
        text: &Text,
        m: &Matcher,
        template: &str,
        cancel: &AtomicBool,
    ) -> Option<(Text, usize)> {
        todo!()
    }
}
