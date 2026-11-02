//! Undo grouping limits, edit kinds and the undo history (m4-editor-shell.md §3.3).
//!
//! A group holds the steps of one undoable action: each step is a [`ChangeSet`] and its
//! inverse, so undo and redo cost O(log n + size of the change). An edit merges into the open
//! group only when it has the same kind (`Typing` or `Deleting`), starts where the previous edit
//! of the group left the carets, and comes less than [`UNDO_GROUP_MS`] after it. A
//! `replace_text` group holds the two texts (two O(1) clones).

use std::collections::VecDeque;
use std::time::Duration;

use crate::edit::ChangeSet;
use crate::encoding::LineEnding;
use crate::selection::Selections;
use crate::text::Text;

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

/// One applied change set and the set that undoes it.
#[derive(Debug)]
pub(crate) struct Step {
    /// The applied set, in the offsets of the text before it.
    pub(crate) forward: ChangeSet,
    /// The undoing set, in the offsets of the text after it.
    pub(crate) inverse: ChangeSet,
}

impl Step {
    /// Inserted plus deleted bytes kept by the step.
    fn bytes(&self) -> usize {
        self.forward.inserted_bytes() + self.inverse.inserted_bytes()
    }
}

/// The content of a group.
#[derive(Debug)]
pub(crate) enum Body {
    /// Change steps, in the order they were applied.
    Steps(Vec<Step>),
    /// A whole-text replacement.
    Replace {
        /// The text before.
        before: Text,
        /// The text after.
        after: Text,
        /// The line ending before.
        eol_before: LineEnding,
        /// The line ending after.
        eol_after: LineEnding,
        /// The widest-line hint before.
        widest_before: usize,
        /// The widest-line hint after.
        widest_after: usize,
    },
}

/// The document state on both sides of a group or step.
#[derive(Clone, Debug)]
pub(crate) struct Sides {
    /// Selections before.
    pub(crate) sel_before: Selections,
    /// Selections after.
    pub(crate) sel_after: Selections,
    /// Content-state id before.
    pub(crate) state_before: u64,
    /// Content-state id after.
    pub(crate) state_after: u64,
}

/// One undoable action.
#[derive(Debug)]
pub(crate) struct Group {
    /// The kind of its edits.
    kind: EditKind,
    /// Its changes.
    pub(crate) body: Body,
    /// The state around it.
    pub(crate) sides: Sides,
    /// Inserted plus deleted bytes it keeps.
    bytes: usize,
    /// Time of its last edit.
    last_at: Duration,
}

/// The undo and redo stacks.
#[derive(Debug, Default)]
pub(crate) struct History {
    /// Undo groups, oldest first.
    undo: VecDeque<Group>,
    /// Redo groups, the next one to redo last.
    redo: Vec<Group>,
    /// Bytes kept by both stacks.
    bytes: usize,
    /// Whether the last undo group may still take edits.
    open: bool,
}

impl History {
    /// Closes the open group: the next edit starts a new one.
    pub(crate) fn close(&mut self) {
        self.open = false;
    }

    /// Records an applied step of `kind` at `at`, merged into the open group when the rules
    /// allow it; clears the redo stack and drops the oldest groups past the limits.
    pub(crate) fn record(&mut self, step: Step, kind: EditKind, at: Duration, sides: Sides) {
        self.clear_redo();
        let bytes = step.bytes();
        self.bytes += bytes;
        let merge = self.open
            && kind != EditKind::Other
            && self.undo.back().is_some_and(|group| {
                group.kind == kind
                    && matches!(group.body, Body::Steps(_))
                    && at.saturating_sub(group.last_at) < Duration::from_millis(UNDO_GROUP_MS)
                    && group.sides.sel_after.same_positions(&sides.sel_before)
            });
        match self.undo.back_mut() {
            Some(group) if merge => {
                if let Body::Steps(steps) = &mut group.body {
                    steps.push(step);
                }
                group.sides.sel_after = sides.sel_after;
                group.sides.state_after = sides.state_after;
                group.bytes += bytes;
                group.last_at = at;
            }
            _ => self.undo.push_back(Group {
                kind,
                body: Body::Steps(vec![step]),
                sides,
                bytes,
                last_at: at,
            }),
        }
        self.open = kind != EditKind::Other;
        self.trim();
    }

    /// Records a whole-text replacement as its own closed group.
    pub(crate) fn record_replace(&mut self, body: Body, sides: Sides, at: Duration) {
        self.clear_redo();
        let bytes = match &body {
            Body::Replace { before, after, .. } => before.len_bytes() + after.len_bytes(),
            Body::Steps(steps) => steps.iter().map(Step::bytes).sum(),
        };
        self.bytes += bytes;
        self.undo.push_back(Group {
            kind: EditKind::Other,
            body,
            sides,
            bytes,
            last_at: at,
        });
        self.open = false;
        self.trim();
    }

    /// Takes the group to undo, if any (closes the open group).
    pub(crate) fn take_undo(&mut self) -> Option<Group> {
        self.open = false;
        self.undo.pop_back()
    }

    /// Stores an undone group for redo.
    pub(crate) fn push_redo(&mut self, group: Group) {
        self.redo.push(group);
    }

    /// Takes the group to redo, if any (closes the open group).
    pub(crate) fn take_redo(&mut self) -> Option<Group> {
        self.open = false;
        self.redo.pop()
    }

    /// Stores a redone group for undo.
    pub(crate) fn push_undo(&mut self, group: Group) {
        self.undo.push_back(group);
        self.trim();
    }

    /// Tells whether there is a group to undo.
    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Tells whether there is a group to redo.
    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Drops the redo stack.
    fn clear_redo(&mut self) {
        for group in self.redo.drain(..) {
            self.bytes -= group.bytes;
        }
    }

    /// Drops the oldest undo groups past [`MAX_UNDO_GROUPS`] or [`MAX_UNDO_BYTES`] (the newest
    /// group is always kept).
    fn trim(&mut self) {
        while self.undo.len() > MAX_UNDO_GROUPS
            || (self.bytes > MAX_UNDO_BYTES && self.undo.len() > 1)
        {
            match self.undo.pop_front() {
                Some(group) => self.bytes -= group.bytes,
                None => break,
            }
        }
    }
}
