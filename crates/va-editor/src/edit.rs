//! Changes, edit commands and motions (m4-editor-shell.md §3.2).
//!
//! A command is planned against the current text and selections: the plan is one sorted list of
//! non-overlapping [`Change`]s (old-text offsets) plus, for every selection, where it lands in the
//! new text. [`crate::document::Document`] applies the plan as one [`ChangeSet`] and records it
//! for undo. Motions move carets without changing the text.

use std::borrow::Cow;

use crate::encoding::LineEnding;
use crate::history::EditKind;
use crate::selection::{Selection, Selections};
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

impl Change {
    /// Tells whether the change does nothing (an empty range and an empty insertion).
    pub fn is_noop(&self) -> bool {
        self.start == self.end && self.insert.is_empty()
    }
}

/// A set of changes applied together: sorted and non-overlapping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeSet {
    /// The changes, sorted and non-overlapping.
    changes: Vec<Change>,
    /// For each change, then for the end: bytes removed and inserted by the changes before it.
    shifts: Vec<(usize, usize)>,
}

/// Why [`ChangeSet::new`] rejected its changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChangeError {
    /// Two changes overlap.
    Overlap,
    /// A range ends past the text (or starts after its end).
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
    /// Checks and sorts `changes` against `text`: each range must satisfy `start <= end <=
    /// len` with both ends on char boundaries, and no two ranges may overlap (touching ranges
    /// and several insertions at one offset are allowed; insertions at one offset keep their
    /// order).
    ///
    /// # Errors
    /// A [`ChangeError`] for overlapping, out-of-bounds or misaligned ranges.
    pub fn new(mut changes: Vec<Change>, text: &Text) -> Result<Self, ChangeError> {
        check_ranges(&changes, text)?;
        changes.sort_by_key(|change| (change.start, change.end));
        if changes.windows(2).any(|pair| pair[1].start < pair[0].end) {
            return Err(ChangeError::Overlap);
        }
        Ok(Self::from_sorted(changes))
    }

    /// Builds a set from changes already sorted and non-overlapping (checked by the caller).
    pub(crate) fn from_sorted(changes: Vec<Change>) -> Self {
        let mut shifts = Vec::with_capacity(changes.len() + 1);
        let (mut removed, mut inserted) = (0, 0);
        shifts.push((0, 0));
        for change in &changes {
            removed += change.end - change.start;
            inserted += change.insert.len();
            shifts.push((removed, inserted));
        }
        Self { changes, shifts }
    }

    /// The changes, sorted.
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    /// Maps an old-text offset to the new text.
    ///
    /// An offset before a change keeps its distance to it; an offset after a change moves by
    /// its length difference. An offset at the start of a replaced range maps to the start of
    /// the insertion, at its end to the end of the insertion, and strictly inside it to the start
    /// (`Before`) or the end (`After`). At a pure insertion, `assoc` chooses the side.
    pub fn map_offset(&self, byte: usize, assoc: Assoc) -> usize {
        let mut index = self.changes.partition_point(|change| change.end < byte);
        while let Some(change) = self.changes.get(index) {
            let (removed, inserted) = self.shifts[index];
            let new_start = change.start - removed + inserted;
            if byte < change.start {
                return byte - removed + inserted;
            }
            if change.start == change.end {
                if assoc == Assoc::Before {
                    return new_start;
                }
            } else if byte < change.end {
                return if byte == change.start || assoc == Assoc::Before {
                    new_start
                } else {
                    new_start + change.insert.len()
                };
            }
            index += 1;
        }
        let (removed, inserted) = self.shifts[self.changes.len()];
        byte.saturating_sub(removed) + inserted
    }

    /// Offset in the new text where change `index` starts (0 for an unknown index).
    pub(crate) fn new_start(&self, index: usize) -> usize {
        match (self.changes.get(index), self.shifts.get(index)) {
            (Some(change), Some(&(removed, inserted))) => change.start - removed + inserted,
            _ => 0,
        }
    }

    /// Length of the text after the set, for a text of `len_before` bytes.
    pub(crate) fn len_after(&self, len_before: usize) -> usize {
        let (removed, inserted) = self.shifts[self.changes.len()];
        len_before.saturating_sub(removed) + inserted
    }

    /// Bytes inserted by the set.
    pub(crate) fn inserted_bytes(&self) -> usize {
        self.shifts[self.changes.len()].1
    }

    /// The set that undoes this one, in new-text offsets; `old` is the text before this set.
    pub(crate) fn inverse(&self, old: &Text) -> Self {
        let changes = self
            .changes
            .iter()
            .enumerate()
            .map(|(index, change)| {
                let start = self.new_start(index);
                Change {
                    start,
                    end: start + change.insert.len(),
                    insert: old.slice_to_string(change.start, change.end),
                }
            })
            .collect();
        Self::from_sorted(changes)
    }

    /// Applies the set to `text`, back to front, in O(changes × log n + changed bytes).
    pub(crate) fn apply_to(&self, text: &mut Text) {
        let rope = text.rope_mut();
        for change in self.changes.iter().rev() {
            let start = rope.byte_to_char(change.start);
            let end = rope.byte_to_char(change.end);
            if end > start {
                rope.remove(start..end);
            }
            if !change.insert.is_empty() {
                rope.insert(start, &change.insert);
            }
        }
    }

    /// Tells whether every range still fits `text` (bounds and char boundaries).
    pub(crate) fn fits(&self, text: &Text) -> bool {
        check_ranges(&self.changes, text).is_ok()
    }
}

/// Checks the bounds and char boundaries of every range of `changes` against `text`.
fn check_ranges(changes: &[Change], text: &Text) -> Result<(), ChangeError> {
    let len = text.len_bytes();
    for change in changes {
        if change.start > change.end || change.end > len {
            return Err(ChangeError::OutOfBounds);
        }
        if !text.is_char_boundary(change.start) || !text.is_char_boundary(change.end) {
            return Err(ChangeError::NotCharBoundary);
        }
    }
    Ok(())
}

/// Tells whether `changes` are sorted, non-overlapping and fit `text`: what a plan must be.
pub(crate) fn is_valid_plan(changes: &[Change], text: &Text) -> bool {
    check_ranges(changes, text).is_ok()
        && changes.windows(2).all(|pair| pair[0].end <= pair[1].start)
}

/// An editing command, applied at every selection, back to front, in one [`ChangeSet`]
/// (m4 §3.2 table).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EditCommand {
    /// Replaces each selection with the text (its line breaks become the document's).
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
    c.is_alphanumeric() || matches!(c, '_' | '.' | '@')
}

/// Char classes of the word motions and of word selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Class {
    /// [`is_word_char`].
    Word,
    /// Whitespace other than a line break.
    Space,
    /// Any other char (punctuation, operators).
    Other,
    /// `\n` or `\r`.
    Break,
}

/// The class of `c`.
pub(crate) fn class(c: char) -> Class {
    if c == '\n' || c == '\r' {
        Class::Break
    } else if is_word_char(c) {
        Class::Word
    } else if c.is_whitespace() {
        Class::Space
    } else {
        Class::Other
    }
}

/// The undo kind of `cmd`: `Typing` for one non-break char, `Deleting` for `Backspace` and
/// `Delete`, else `Other`.
pub(crate) fn kind_of(cmd: &EditCommand) -> EditKind {
    match cmd {
        EditCommand::InsertText(text) => {
            let mut chars = text.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c != '\n' && c != '\r' => EditKind::Typing,
                _ => EditKind::Other,
            }
        }
        EditCommand::Backspace | EditCommand::Delete => EditKind::Deleting,
        _ => EditKind::Other,
    }
}

/// Where a planned selection end lands in the new text.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Pos {
    /// `offset` bytes after the new start of change `change` of the plan.
    Rel {
        /// Index of the change in the plan.
        change: usize,
        /// Bytes after the start of its insertion.
        offset: usize,
    },
    /// The old offset `at`, mapped through the whole set.
    Map {
        /// The old-text offset.
        at: usize,
        /// The side at an insertion.
        assoc: Assoc,
    },
}

/// One selection of a plan.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PlannedSel {
    /// Where the anchor lands.
    anchor: Pos,
    /// Where the head lands.
    head: Pos,
    /// Whether it is the primary selection.
    primary: bool,
}

impl PlannedSel {
    /// A caret at `pos`.
    fn caret(pos: Pos, primary: bool) -> Self {
        Self {
            anchor: pos,
            head: pos,
            primary,
        }
    }

    /// `sel` mapped through the set: an empty one with `empty`, else its start with `start`
    /// and its end with `end`, direction kept.
    fn mapped(sel: &Selection, start: Assoc, end: Assoc, empty: Assoc, primary: bool) -> Self {
        if sel.is_empty() {
            return Self::caret(
                Pos::Map {
                    at: sel.head,
                    assoc: empty,
                },
                primary,
            );
        }
        let low = Pos::Map {
            at: sel.start(),
            assoc: start,
        };
        let high = Pos::Map {
            at: sel.end(),
            assoc: end,
        };
        let (anchor, head) = if sel.is_reversed() {
            (high, low)
        } else {
            (low, high)
        };
        Self {
            anchor,
            head,
            primary,
        }
    }

    /// `sel` kept as it is (only shifted by changes before it, never grown).
    fn kept(sel: &Selection, primary: bool) -> Self {
        Self::mapped(sel, Assoc::After, Assoc::Before, Assoc::After, primary)
    }
}

/// The plan of one command: sorted, non-overlapping changes and the resulting selections.
#[derive(Debug, Default)]
pub(crate) struct Plan {
    /// The changes, in old-text offsets.
    pub(crate) changes: Vec<Change>,
    /// One entry per resulting selection.
    sels: Vec<PlannedSel>,
}

impl Plan {
    /// The resulting selections once `set` (built from [`Plan::changes`]) is applied.
    pub(crate) fn selections(&self, set: &ChangeSet) -> Selections {
        let resolve = |pos: Pos| match pos {
            Pos::Rel { change, offset } => set.new_start(change) + offset,
            Pos::Map { at, assoc } => set.map_offset(at, assoc),
        };
        let mut primary = 0;
        let list = self
            .sels
            .iter()
            .enumerate()
            .map(|(index, planned)| {
                if planned.primary {
                    primary = index;
                }
                Selection::new(resolve(planned.anchor), resolve(planned.head))
            })
            .collect();
        Selections::from_vec(list, primary)
    }

    /// Adds a change and returns its index.
    fn push_change(&mut self, start: usize, end: usize, insert: String) -> usize {
        self.changes.push(Change { start, end, insert });
        self.changes.len() - 1
    }

    /// Replaces `start..end` with `insert`; the selection becomes a caret `caret` bytes into
    /// the insertion.
    fn replace(&mut self, start: usize, end: usize, insert: String, caret: usize, primary: bool) {
        if start == end && insert.is_empty() {
            self.keep_caret(start, primary);
            return;
        }
        let change = self.push_change(start, end, insert);
        self.sels.push(PlannedSel::caret(
            Pos::Rel {
                change,
                offset: caret,
            },
            primary,
        ));
    }

    /// Deletes `start..end`, merged with the previous deletions it overlaps; the selection
    /// becomes a caret at the start of the merged deletion.
    fn delete(&mut self, start: usize, end: usize, primary: bool) {
        if start >= end {
            self.keep_caret(start, primary);
            return;
        }
        let (mut start, mut end) = (start, end);
        while let Some(last) = self.changes.last()
            && last.insert.is_empty()
            && start < last.end
        {
            start = start.min(last.start);
            end = end.max(last.end);
            self.changes.pop();
        }
        let change = self.changes.len();
        let merged = Pos::Rel { change, offset: 0 };
        for planned in self.sels.iter_mut().rev() {
            match planned.head {
                Pos::Rel { change: index, .. } if index >= change => {
                    planned.anchor = merged;
                    planned.head = merged;
                }
                Pos::Rel { .. } => break,
                Pos::Map { .. } => {}
            }
        }
        self.push_change(start, end, String::new());
        self.sels.push(PlannedSel::caret(merged, primary));
    }

    /// A caret at the old offset `at`, after any insertion there.
    fn keep_caret(&mut self, at: usize, primary: bool) {
        self.sels.push(PlannedSel::caret(
            Pos::Map {
                at,
                assoc: Assoc::After,
            },
            primary,
        ));
    }
}

/// The selections with their primary flags, in document order.
fn flagged(selections: &Selections) -> impl Iterator<Item = (&Selection, bool)> {
    let primary = selections.primary_index();
    selections
        .iter()
        .enumerate()
        .map(move |(index, sel)| (sel, index == primary))
}

/// `text` with every line break (LF, CRLF, lone CR) replaced by `eol`.
fn normalize_breaks(text: &str, eol: LineEnding) -> Cow<'_, str> {
    if !text.contains(['\r', '\n']) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    for (index, (_, line)) in va_core::source::lines(text).enumerate() {
        if index > 0 {
            out.push_str(eol.as_str());
        }
        out.push_str(line);
    }
    Cow::Owned(out)
}

/// First and last line touched by `sel`: a non-empty selection ending at column 0 of a later
/// line does not touch that line.
fn touched_lines(text: &Text, sel: &Selection) -> (usize, usize) {
    let first = text.line_of(sel.start());
    let mut last = text.line_of(sel.end());
    if last > first && sel.end() == text.line_start(last) {
        last -= 1;
    }
    (first, last)
}

/// Lines touched by consecutive selections, merged when they share a line (or, with
/// `join_adjacent`, when they follow each other).
struct Block {
    /// First line.
    first: usize,
    /// Last line.
    last: usize,
    /// The selections of the block, with their primary flags.
    sels: Vec<(Selection, bool)>,
}

/// The line blocks of `selections`, in order.
fn line_blocks(text: &Text, selections: &Selections, join_adjacent: bool) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    for (sel, primary) in flagged(selections) {
        let (first, last) = touched_lines(text, sel);
        if let Some(block) = blocks.last_mut() {
            let joins = first <= block.last || (join_adjacent && first == block.last + 1);
            if joins {
                block.last = block.last.max(last);
                block.sels.push((*sel, primary));
                continue;
            }
        }
        blocks.push(Block {
            first,
            last,
            sels: vec![(*sel, primary)],
        });
    }
    blocks
}

/// Every line touched by a selection, each once, in order.
fn touched_line_list(text: &Text, selections: &Selections) -> Vec<(usize, bool)> {
    let mut lines: Vec<(usize, bool)> = Vec::new();
    for sel in selections.iter() {
        let (first, last) = touched_lines(text, sel);
        let multi = last > first;
        for line in first..=last {
            if lines.last().is_some_and(|&(done, _)| line <= done) {
                continue;
            }
            lines.push((line, multi));
        }
    }
    lines
}

/// End of the leading spaces and tabs of `line`.
fn indent_end(text: &Text, line: usize) -> usize {
    let start = text.line_start(line);
    let end = text.line_end(line);
    let blanks = text
        .rope()
        .bytes_at(start)
        .take(end - start)
        .take_while(|&byte| byte == b' ' || byte == b'\t')
        .count();
    start + blanks
}

/// Tells whether the text at `at` starts with `prefix`.
fn starts_with_at(text: &Text, at: usize, prefix: &str) -> bool {
    at + prefix.len() <= text.len_bytes()
        && text
            .rope()
            .bytes_at(at)
            .take(prefix.len())
            .eq(prefix.bytes())
}

/// Plans `cmd` at every selection; `eol` is the document line ending.
pub(crate) fn plan(
    text: &Text,
    selections: &Selections,
    cmd: &EditCommand,
    opts: &EditOptions,
    eol: LineEnding,
) -> Plan {
    match cmd {
        EditCommand::InsertText(insert) => insert_text(selections, &normalize_breaks(insert, eol)),
        EditCommand::Paste(insert) => paste(selections, insert, eol),
        EditCommand::NewLine => new_line(text, selections, eol),
        EditCommand::Tab => {
            let spans = selections
                .iter()
                .any(|sel| text.line_of(sel.start()) != text.line_of(sel.end()));
            if spans {
                indent(text, selections, opts)
            } else {
                tab(text, selections, opts)
            }
        }
        EditCommand::Indent => indent(text, selections, opts),
        EditCommand::Outdent => outdent(text, selections, opts),
        EditCommand::Backspace => {
            deletion(selections, false, |head| (text.prev_boundary(head), head))
        }
        EditCommand::Delete => deletion(selections, false, |head| (head, text.next_boundary(head))),
        EditCommand::DeleteWordLeft => {
            deletion(selections, false, |head| (word_left(text, head), head))
        }
        EditCommand::DeleteWordRight => {
            deletion(selections, false, |head| (head, word_right(text, head)))
        }
        EditCommand::DeleteSelection => deletion(selections, true, |head| (head, head)),
        EditCommand::DeleteLines => delete_lines(text, selections, opts),
        EditCommand::DuplicateLines => duplicate_lines(text, selections, eol),
        EditCommand::MoveLinesUp => move_lines_up(text, selections),
        EditCommand::MoveLinesDown => move_lines_down(text, selections),
        EditCommand::ToggleLineComment { prefix } => toggle_comment(text, selections, prefix, opts),
    }
}

/// `InsertText`: each selection replaced by `insert`, the caret after it.
fn insert_text(selections: &Selections, insert: &str) -> Plan {
    let mut plan = Plan::default();
    for (sel, primary) in flagged(selections) {
        plan.replace(
            sel.start(),
            sel.end(),
            insert.to_owned(),
            insert.len(),
            primary,
        );
    }
    plan
}

/// `Paste`: line i to caret i when the text has as many lines as there are (several) carets.
fn paste(selections: &Selections, insert: &str, eol: LineEnding) -> Plan {
    let pieces: Vec<&str> = va_core::source::lines(insert)
        .map(|(_, line)| line)
        .collect();
    if selections.len() > 1 && pieces.len() == selections.len() {
        let mut plan = Plan::default();
        for ((sel, primary), piece) in flagged(selections).zip(pieces) {
            plan.replace(
                sel.start(),
                sel.end(),
                piece.to_owned(),
                piece.len(),
                primary,
            );
        }
        return plan;
    }
    insert_text(selections, &normalize_breaks(insert, eol))
}

/// `NewLine`: the line ending plus the indentation of the line, up to the caret.
fn new_line(text: &Text, selections: &Selections, eol: LineEnding) -> Plan {
    let mut plan = Plan::default();
    for (sel, primary) in flagged(selections) {
        let start = sel.start();
        let line = text.line_of(start);
        let indent = indent_end(text, line).min(start);
        let mut insert = String::from(eol.as_str());
        insert.push_str(&text.slice_to_string(text.line_start(line), indent));
        let caret = insert.len();
        plan.replace(start, sel.end(), insert, caret, primary);
    }
    plan
}

/// `Tab` without a selection spanning lines: a tab, or spaces to the next tab stop.
fn tab(text: &Text, selections: &Selections, opts: &EditOptions) -> Plan {
    let width = u32::from(opts.tab_width.max(1));
    let mut plan = Plan::default();
    for (sel, primary) in flagged(selections) {
        let insert = if opts.insert_spaces {
            let col = text.col_of(sel.start(), opts.tab_width);
            " ".repeat(usize::try_from(width - col % width).unwrap_or(1))
        } else {
            String::from("\t")
        };
        let caret = insert.len();
        plan.replace(sel.start(), sel.end(), insert, caret, primary);
    }
    plan
}

/// The selections of a line-start command, mapped through its changes.
fn line_start_sels(plan: &mut Plan, selections: &Selections) {
    for (sel, primary) in flagged(selections) {
        plan.sels.push(PlannedSel::mapped(
            sel,
            Assoc::Before,
            Assoc::After,
            Assoc::After,
            primary,
        ));
    }
}

/// `Indent`: one level at each touched line start (empty lines of a multi-line selection
/// skipped).
fn indent(text: &Text, selections: &Selections, opts: &EditOptions) -> Plan {
    let unit = if opts.insert_spaces {
        " ".repeat(usize::from(opts.tab_width.max(1)))
    } else {
        String::from("\t")
    };
    let mut plan = Plan::default();
    for (line, multi) in touched_line_list(text, selections) {
        let start = text.line_start(line);
        if multi && start == text.line_end(line) {
            continue;
        }
        plan.push_change(start, start, unit.clone());
    }
    line_start_sels(&mut plan, selections);
    plan
}

/// `Outdent`: a leading tab, or up to `tab_width` leading spaces, removed from each touched
/// line.
fn outdent(text: &Text, selections: &Selections, opts: &EditOptions) -> Plan {
    let mut plan = Plan::default();
    for (line, _) in touched_line_list(text, selections) {
        let start = text.line_start(line);
        let count = if text.rope().get_byte(start) == Some(b'\t') {
            1
        } else {
            text.rope()
                .bytes_at(start)
                .take(usize::from(opts.tab_width.max(1)))
                .take_while(|&byte| byte == b' ')
                .count()
        };
        if count > 0 {
            plan.push_change(start, start + count, String::new());
        }
    }
    line_start_sels(&mut plan, selections);
    plan
}

/// `ToggleLineComment`: removes `prefix` (and one space) when every non-blank touched line
/// has it after its indentation, else inserts `prefix` and a space at the smallest indentation
/// of the non-blank lines (at the end of each line's blanks when all are blank).
fn toggle_comment(text: &Text, selections: &Selections, prefix: &str, opts: &EditOptions) -> Plan {
    let mut plan = Plan::default();
    if prefix.is_empty() {
        return plan;
    }
    let lines: Vec<(usize, usize, bool)> = touched_line_list(text, selections)
        .into_iter()
        .map(|(line, _)| {
            let blanks_end = indent_end(text, line);
            (line, blanks_end, blanks_end == text.line_end(line))
        })
        .collect();
    let filled: Vec<(usize, usize)> = lines
        .iter()
        .filter(|&&(_, _, blank)| !blank)
        .map(|&(line, blanks_end, _)| (line, blanks_end))
        .collect();
    let commented = !filled.is_empty()
        && filled
            .iter()
            .all(|&(_, blanks_end)| starts_with_at(text, blanks_end, prefix));
    let mut marker = String::from(prefix);
    marker.push(' ');
    if commented {
        for &(line, blanks_end) in &filled {
            let mut end = blanks_end + prefix.len();
            if end < text.line_end(line) && text.rope().get_byte(end) == Some(b' ') {
                end += 1;
            }
            plan.push_change(blanks_end, end, String::new());
        }
    } else if let Some(col) = filled
        .iter()
        .map(|&(_, blanks_end)| text.col_of(blanks_end, opts.tab_width))
        .min()
    {
        for &(line, _) in &filled {
            let at = text.byte_at_col(line, col, opts.tab_width);
            plan.push_change(at, at, marker.clone());
        }
    } else {
        for &(_, blanks_end, _) in &lines {
            plan.push_change(blanks_end, blanks_end, marker.clone());
        }
    }
    line_start_sels(&mut plan, selections);
    plan
}

/// The deletion commands: a non-empty selection is deleted; an empty one deletes the range
/// `caret_range(head)` unless `selections_only`.
fn deletion(
    selections: &Selections,
    selections_only: bool,
    caret_range: impl Fn(usize) -> (usize, usize),
) -> Plan {
    let mut plan = Plan::default();
    for (sel, primary) in flagged(selections) {
        if !sel.is_empty() {
            plan.delete(sel.start(), sel.end(), primary);
        } else if selections_only {
            plan.keep_caret(sel.head, primary);
        } else {
            let (start, end) = caret_range(sel.head);
            plan.delete(start, end, primary);
        }
    }
    plan
}

/// `DeleteLines`: every touched line with its terminator (the previous terminator for the last
/// line); carets keep their display column on the line that takes the place.
fn delete_lines(text: &Text, selections: &Selections, opts: &EditOptions) -> Plan {
    let last_line = text.line_count() - 1;
    let len = text.len_bytes();
    let mut plan = Plan::default();
    for block in line_blocks(text, selections, true) {
        let (start, end) = if block.last < last_line {
            (
                text.line_start(block.first),
                text.line_start(block.last + 1),
            )
        } else if block.first > 0 {
            (text.line_end(block.first - 1), len)
        } else {
            (0, len)
        };
        if start == end {
            for (sel, primary) in &block.sels {
                plan.sels.push(PlannedSel::kept(sel, *primary));
            }
            continue;
        }
        let change = plan.push_change(start, end, String::new());
        for (sel, primary) in &block.sels {
            let col = text.col_of(sel.head, opts.tab_width);
            let pos = if block.last < last_line {
                let next = block.last + 1;
                let at = text.byte_at_col(next, col, opts.tab_width);
                Pos::Rel {
                    change,
                    offset: at - text.line_start(next),
                }
            } else if block.first > 0 {
                Pos::Map {
                    at: text.byte_at_col(block.first - 1, col, opts.tab_width),
                    assoc: Assoc::Before,
                }
            } else {
                Pos::Rel { change, offset: 0 }
            };
            plan.sels.push(PlannedSel::caret(pos, *primary));
        }
    }
    plan
}

/// `DuplicateLines`: a copy of every touched block inserted above it, so the selections end on
/// the lower copy.
fn duplicate_lines(text: &Text, selections: &Selections, eol: LineEnding) -> Plan {
    let mut plan = Plan::default();
    for block in line_blocks(text, selections, false) {
        let start = text.line_start(block.first);
        let mut insert = text.slice_to_string(start, text.line_end(block.last));
        insert.push_str(eol.as_str());
        plan.push_change(start, start, insert);
        for (sel, primary) in &block.sels {
            plan.sels.push(PlannedSel::mapped(
                sel,
                Assoc::After,
                Assoc::Before,
                Assoc::After,
                *primary,
            ));
        }
    }
    plan
}

/// `MoveLinesUp`: every block swapped with the line above it (blocks at the top stay).
fn move_lines_up(text: &Text, selections: &Selections) -> Plan {
    let mut plan = Plan::default();
    for block in line_blocks(text, selections, true) {
        if block.first == 0 {
            for (sel, primary) in &block.sels {
                plan.sels.push(PlannedSel::kept(sel, *primary));
            }
            continue;
        }
        let above = block.first - 1;
        let block_start = text.line_start(block.first);
        let block_end = text.line_end(block.last);
        let moved = text.slice_to_string(block_start, block_end);
        let between = text.slice_to_string(text.line_end(above), block_start);
        let mut insert = moved.clone();
        insert.push_str(&between);
        insert.push_str(&text.slice_to_string(text.line_start(above), text.line_end(above)));
        let change = plan.push_change(text.line_start(above), block_end, insert);
        let place = |at: usize| Pos::Rel {
            change,
            offset: if at <= block_end {
                at - block_start
            } else {
                moved.len() + between.len()
            },
        };
        for (sel, primary) in &block.sels {
            plan.sels.push(PlannedSel {
                anchor: place(sel.anchor),
                head: place(sel.head),
                primary: *primary,
            });
        }
    }
    plan
}

/// `MoveLinesDown`: every block swapped with the line below it (blocks at the bottom stay).
fn move_lines_down(text: &Text, selections: &Selections) -> Plan {
    let line_count = text.line_count();
    let mut plan = Plan::default();
    for block in line_blocks(text, selections, true) {
        if block.last + 1 >= line_count {
            for (sel, primary) in &block.sels {
                plan.sels.push(PlannedSel::kept(sel, *primary));
            }
            continue;
        }
        let below = block.last + 1;
        let block_start = text.line_start(block.first);
        let block_end = text.line_end(block.last);
        let between = text.slice_to_string(block_end, text.line_start(below));
        let next = text.slice_to_string(text.line_start(below), text.line_end(below));
        let shift = next.len() + between.len();
        let mut insert = next;
        insert.push_str(&between);
        insert.push_str(&text.slice_to_string(block_start, block_end));
        let change = plan.push_change(block_start, text.line_end(below), insert);
        let after = if below + 1 < line_count {
            text.line_start(below + 1)
        } else {
            text.len_bytes()
        };
        let place = |at: usize| {
            if at <= block_end {
                Pos::Rel {
                    change,
                    offset: shift + (at - block_start),
                }
            } else {
                Pos::Map {
                    at: after,
                    assoc: Assoc::Before,
                }
            }
        };
        for (sel, primary) in &block.sels {
            plan.sels.push(PlannedSel {
                anchor: place(sel.anchor),
                head: place(sel.head),
                primary: *primary,
            });
        }
    }
    plan
}

/// The next word boundary after `byte`: a line break alone, else the run of word (or other)
/// chars, then the blanks after it.
pub(crate) fn word_right(text: &Text, byte: usize) -> usize {
    let rope = text.rope();
    let mut index = rope.byte_to_char(byte.min(text.len_bytes()));
    let mut chars = rope.chars_at(index);
    let Some(first) = chars.next() else {
        return text.len_bytes();
    };
    index += 1;
    let first_class = class(first);
    if first_class == Class::Break {
        if first == '\r' && chars.next() == Some('\n') {
            index += 1;
        }
        return rope.char_to_byte(index);
    }
    let mut next = chars.next();
    if first_class != Class::Space {
        while let Some(c) = next
            && class(c) == first_class
        {
            index += 1;
            next = chars.next();
        }
    }
    while let Some(c) = next
        && class(c) == Class::Space
    {
        index += 1;
        next = chars.next();
    }
    rope.char_to_byte(index)
}

/// The previous word boundary before `byte`: a line break alone, else the blanks before it,
/// then the run of word (or other) chars before them.
pub(crate) fn word_left(text: &Text, byte: usize) -> usize {
    let rope = text.rope();
    let mut index = rope.byte_to_char(byte.min(text.len_bytes()));
    let mut chars = rope.chars_at(index);
    let Some(first) = chars.prev() else {
        return 0;
    };
    index -= 1;
    let mut run = class(first);
    if run == Class::Break {
        if first == '\n' && chars.prev() == Some('\r') {
            index -= 1;
        }
        return rope.char_to_byte(index);
    }
    let mut prev = chars.prev();
    if run == Class::Space {
        while let Some(c) = prev
            && class(c) == Class::Space
        {
            index -= 1;
            prev = chars.prev();
        }
        match prev.map(class) {
            Some(found @ (Class::Word | Class::Other)) => run = found,
            _ => return rope.char_to_byte(index),
        }
        index -= 1;
        prev = chars.prev();
    }
    while let Some(c) = prev
        && class(c) == run
    {
        index -= 1;
        prev = chars.prev();
    }
    rope.char_to_byte(index)
}

/// The run of same-class chars around `byte` within its line, preferring a word: the word
/// after `byte`, else the word before it, else (unless `words_only`) the blank or other run
/// after or before it. `None` on an empty line or when no word is there.
pub(crate) fn run_at(text: &Text, byte: usize, words_only: bool) -> Option<(usize, usize)> {
    let rope = text.rope();
    let index = rope.byte_to_char(text.snap(byte));
    let not_break = |c: &char| class(*c) != Class::Break;
    let after = rope.get_char(index).filter(not_break).map(class);
    let before = index
        .checked_sub(1)
        .and_then(|before| rope.get_char(before))
        .filter(not_break)
        .map(class);
    let run = match (after, before) {
        (Some(Class::Word), _) | (_, Some(Class::Word)) => Class::Word,
        _ if words_only => return None,
        (Some(found), _) | (None, Some(found)) => found,
        (None, None) => return None,
    };
    let mut start = index;
    let mut chars = rope.chars_at(index);
    while let Some(c) = chars.prev()
        && class(c) == run
    {
        start -= 1;
    }
    let end = index
        + rope
            .chars_at(index)
            .take_while(|&c| class(c) == run)
            .count();
    Some((rope.char_to_byte(start), rope.char_to_byte(end)))
}

/// Smart home from `head`: the first non-blank char of the line, or column 0 when already
/// there (or when the line is blank).
fn smart_home(text: &Text, head: usize) -> usize {
    let line = text.line_of(head);
    let start = text.line_start(line);
    let first = indent_end(text, line);
    if first == text.line_end(line) || head == first {
        start
    } else {
        first
    }
}

/// The head and goal of a vertical move of `lines` lines from `sel`.
fn vertical(
    text: &Text,
    sel: &Selection,
    lines: usize,
    up: bool,
    extend: bool,
    tab_width: u8,
) -> (usize, Option<u32>) {
    let collapse = !extend && !sel.is_empty();
    let from = match (collapse, up) {
        (true, true) => sel.start(),
        (true, false) => sel.end(),
        (false, _) => sel.head,
    };
    let col = match sel.goal {
        Some(goal) if from == sel.head => goal,
        _ => text.col_of(from, tab_width),
    };
    let line = text.line_of(from);
    let last = text.line_count() - 1;
    let head = if up {
        if line == 0 {
            0
        } else {
            text.byte_at_col(line.saturating_sub(lines), col, tab_width)
        }
    } else if line == last {
        text.len_bytes()
    } else {
        text.byte_at_col(line.saturating_add(lines).min(last), col, tab_width)
    };
    (head, Some(col))
}

/// `sel` moved by `motion` (extended when `extend`).
pub(crate) fn move_selection(
    text: &Text,
    sel: &Selection,
    motion: Motion,
    extend: bool,
    tab_width: u8,
) -> Selection {
    let collapse = !extend && !sel.is_empty();
    let page = |count: u32| usize::try_from(count).unwrap_or(usize::MAX);
    let (head, goal) = match motion {
        Motion::Left if collapse => (sel.start(), None),
        Motion::Right if collapse => (sel.end(), None),
        Motion::Left => (text.prev_boundary(sel.head), None),
        Motion::Right => (text.next_boundary(sel.head), None),
        Motion::WordLeft => (word_left(text, sel.head), None),
        Motion::WordRight => (word_right(text, sel.head), None),
        Motion::Up => vertical(text, sel, 1, true, extend, tab_width),
        Motion::Down => vertical(text, sel, 1, false, extend, tab_width),
        Motion::PageUp(count) => vertical(text, sel, page(count), true, extend, tab_width),
        Motion::PageDown(count) => vertical(text, sel, page(count), false, extend, tab_width),
        Motion::LineStart => (smart_home(text, sel.head), None),
        Motion::LineEnd => (text.line_end(text.line_of(sel.head)), None),
        Motion::DocStart => (0, None),
        Motion::DocEnd => (text.len_bytes(), None),
        Motion::To(byte) => (text.snap(byte), None),
    };
    Selection {
        anchor: if extend { sel.anchor } else { head },
        head,
        goal,
    }
}
