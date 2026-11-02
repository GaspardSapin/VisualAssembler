//! The document: text, selections, history and dirty state (m4-editor-shell.md §3.3).
//!
//! Every content state gets an id; undo and redo restore the ids, so undoing back to the saved
//! state makes the document clean again. The document is dirty when its content id, encoding or
//! line ending differs from the saved ones. A `Document` is `Send + Sync`, so it can be built on
//! a worker thread (`Document::new` is O(n): it measures the widest line).

use std::collections::VecDeque;
use std::time::Duration;

use crate::edit::{self, ChangeSet, EditCommand, EditOptions, Motion};
use crate::encoding::{Encoding, LineEnding, detect_line_ending};
use crate::history::{Body, EditKind, Group, History, Sides, Step};
use crate::search::is_whole_word;
use crate::selection::{Selection, Selections};
use crate::text::{MAX_DOCUMENT_BYTES, Text};

/// Number of recent revisions whose content id [`Document::mark_saved_at`] still knows.
const REVISION_LOG: usize = 256;

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

/// What the file on disk holds, as of the last save.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Saved {
    /// Content-state id.
    state: u64,
    /// Encoding.
    encoding: Encoding,
    /// Line ending.
    line_ending: LineEnding,
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
    /// Upper bound of the widest line, in chars.
    widest: usize,
    /// Undo and redo stacks.
    history: History,
    /// Id of the current content state.
    state: u64,
    /// Last content-state id handed out.
    last_state: u64,
    /// The saved state.
    saved: Saved,
    /// `(revision, content id)` of the recent revisions, oldest first.
    revisions: VecDeque<(u64, u64)>,
    /// Whether `select_next_occurrence` matches whole words (the selection came from its own
    /// word expansion).
    whole_word_next: bool,
}

impl Document {
    /// A clean document of `text`; its line ending is the detected one, else `default_eol`.
    /// O(n): it measures the widest line.
    pub fn new(text: Text, encoding: Encoding, default_eol: LineEnding) -> Self {
        let line_ending = detect_line_ending(&text).unwrap_or(default_eol);
        let widest = text.widest_line_chars();
        Self {
            text,
            selections: Selections::single(Selection::caret(0)),
            revision: 0,
            encoding,
            line_ending,
            widest,
            history: History::default(),
            state: 0,
            last_state: 0,
            saved: Saved {
                state: 0,
                encoding,
                line_ending,
            },
            revisions: VecDeque::from([(0, 0)]),
            whole_word_next: false,
        }
    }

    /// The current text.
    pub fn text(&self) -> &Text {
        &self.text
    }

    /// The current selections.
    pub fn selections(&self) -> &Selections {
        &self.selections
    }

    /// Replaces the selections; positions are clamped to the text, moved down to char
    /// boundaries and out of CRLF pairs. A change closes the open undo group.
    pub fn set_selections(&mut self, s: Selections) {
        let s = self.normalized(s);
        self.change_selections(s);
    }

    /// `s` with every position made a valid caret position of the text.
    fn normalized(&self, s: Selections) -> Selections {
        let valid = |byte: usize| self.text.snap(byte) == byte;
        if s.iter().all(|sel| valid(sel.anchor) && valid(sel.head)) {
            return s;
        }
        let primary = s.primary_index();
        let list = s
            .iter()
            .map(|sel| Selection {
                anchor: self.text.snap(sel.anchor),
                head: self.text.snap(sel.head),
                goal: sel.goal,
            })
            .collect();
        Selections::from_vec(list, primary)
    }

    /// Sets the selections from a user action other than an edit.
    fn change_selections(&mut self, s: Selections) {
        if s != self.selections {
            self.history.close();
            self.whole_word_next = false;
            self.selections = s;
        }
    }

    /// The revision: +1 per applied change, undo and redo included.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Tells whether the content, the encoding or the line ending differs from the saved state.
    pub fn is_dirty(&self) -> bool {
        self.saved
            != Saved {
                state: self.state,
                encoding: self.encoding,
                line_ending: self.line_ending,
            }
    }

    /// Marks the current content as saved (and closes the open undo group, so an undo comes
    /// back to exactly this state).
    pub fn mark_saved(&mut self) {
        self.saved = Saved {
            state: self.state,
            encoding: self.encoding,
            line_ending: self.line_ending,
        };
        self.history.close();
    }

    /// Marks the content of `snapshot` as saved, for a save that ran on a worker while edits
    /// went on (m4 §5.7): edits made since keep the document dirty, and undoing back to the
    /// snapshot's content makes it clean. Returns false, changing nothing, when the snapshot's
    /// revision is older than the last 256 revisions.
    pub fn mark_saved_at(&mut self, snapshot: &Snapshot) -> bool {
        let Some(&(_, state)) = self
            .revisions
            .iter()
            .rev()
            .find(|&&(revision, _)| revision == snapshot.revision)
        else {
            return false;
        };
        self.saved = Saved {
            state,
            encoding: snapshot.encoding,
            line_ending: snapshot.line_ending,
        };
        if snapshot.revision == self.revision {
            self.history.close();
        }
        true
    }

    /// The encoding used on save.
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// Changes the encoding used on save; the document is dirty while it differs from the
    /// saved encoding.
    pub fn set_encoding(&mut self, e: Encoding) {
        self.encoding = e;
    }

    /// The terminator of new line breaks.
    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    /// An upper bound of the widest line in chars, grown by edits (exact after
    /// [`Document::replace_text`]).
    pub fn widest_line_hint(&self) -> usize {
        self.widest
    }

    /// Tells whether [`Document::undo`] has a group to undo.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Tells whether [`Document::redo`] has a group to redo.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Runs `cmd` at every selection at time `at`; returns false when nothing changed (no
    /// change to make, or a result over [`MAX_DOCUMENT_BYTES`] that grows the text).
    pub fn edit(&mut self, cmd: EditCommand, opts: &EditOptions, at: Duration) -> bool {
        let kind = edit::kind_of(&cmd);
        let mut plan = edit::plan(&self.text, &self.selections, &cmd, opts, self.line_ending);
        if plan.changes.iter().all(edit::Change::is_noop)
            || !edit::is_valid_plan(&plan.changes, &self.text)
        {
            return false;
        }
        let set = ChangeSet::from_sorted(std::mem::take(&mut plan.changes));
        let len = self.text.len_bytes();
        let new_len = set.len_after(len);
        if new_len > MAX_DOCUMENT_BYTES && new_len > len {
            return false;
        }
        let selections = plan.selections(&set);
        self.commit(set, selections, kind, at);
        true
    }

    /// Applies external changes (M5 quick fix) as an edit of `kind` at time `at`. A set that no
    /// longer fits the text (built against an older revision) is ignored. Carets move after
    /// text inserted at them; selections never grow over text inserted at their edges.
    pub fn apply(&mut self, changes: ChangeSet, kind: EditKind, at: Duration) {
        if changes.changes().iter().all(edit::Change::is_noop) || !changes.fits(&self.text) {
            return;
        }
        let primary = self.selections.primary_index();
        let list = self
            .selections
            .iter()
            .map(|sel| {
                if sel.is_empty() {
                    return Selection::caret(changes.map_offset(sel.head, edit::Assoc::After));
                }
                let start = changes.map_offset(sel.start(), edit::Assoc::After);
                let end = changes.map_offset(sel.end(), edit::Assoc::Before);
                sel.with_range(start.min(end), end)
            })
            .collect();
        self.commit(changes, Selections::from_vec(list, primary), kind, at);
    }

    /// Applies `set` to the text, sets `selections` (new-text offsets, made valid caret
    /// positions) and records the step for undo.
    fn commit(&mut self, set: ChangeSet, selections: Selections, kind: EditKind, at: Duration) {
        let inverse = set.inverse(&self.text);
        set.apply_to(&mut self.text);
        self.grow_widest(&set);
        let selections = self.normalized(selections);
        let sel_before = std::mem::replace(&mut self.selections, selections);
        let state_before = self.state;
        self.state = self.fresh_state();
        let sides = Sides {
            sel_before,
            sel_after: self.selections.clone(),
            state_before,
            state_after: self.state,
        };
        let step = Step {
            forward: set,
            inverse,
        };
        self.history.record(step, kind, at, sides);
        self.whole_word_next = false;
        self.bump_revision();
    }

    /// A new content-state id.
    fn fresh_state(&mut self) -> u64 {
        self.last_state += 1;
        self.last_state
    }

    /// Increments the revision and logs its content id.
    fn bump_revision(&mut self) {
        self.revision += 1;
        self.revisions.push_back((self.revision, self.state));
        if self.revisions.len() > REVISION_LOG {
            self.revisions.pop_front();
        }
    }

    /// Grows the widest-line hint with the lines `set` (already applied) touched.
    fn grow_widest(&mut self, set: &ChangeSet) {
        for (index, change) in set.changes().iter().enumerate() {
            let start = set.new_start(index);
            let first = self.text.line_of(start);
            let last = self.text.line_of(start + change.insert.len());
            for line in first..=last {
                self.widest = self.widest.max(self.text.line_chars(line));
            }
        }
    }

    /// Replaces the whole text in one undo group (replace all, reload). Carets keep their line
    /// and byte column (clamped); the line ending is detected again (kept when the new text has
    /// no terminator); the widest-line hint becomes exact. O(n).
    pub fn replace_text(&mut self, text: Text) {
        self.history.close();
        let before = std::mem::replace(&mut self.text, text);
        let eol_before = self.line_ending;
        if let Some(found) = detect_line_ending(&self.text) {
            self.line_ending = found;
        }
        let widest_before = self.widest;
        self.widest = self.text.widest_line_chars();
        let primary = self.selections.primary_index();
        let list = self
            .selections
            .iter()
            .map(|sel| {
                let line = before.line_of(sel.head);
                let column = sel.head - before.line_start(line);
                let line = line.min(self.text.line_count() - 1);
                let start = self.text.line_start(line);
                let room = self.text.line_end(line) - start;
                Selection::caret(self.text.floor_boundary(start + column.min(room)))
            })
            .collect();
        let selections = Selections::from_vec(list, primary);
        let sel_before = std::mem::replace(&mut self.selections, selections);
        let state_before = self.state;
        self.state = self.fresh_state();
        let body = Body::Replace {
            before,
            after: self.text.clone(),
            eol_before,
            eol_after: self.line_ending,
            widest_before,
            widest_after: self.widest,
        };
        let sides = Sides {
            sel_before,
            sel_after: self.selections.clone(),
            state_before,
            state_after: self.state,
        };
        self.history.record_replace(body, sides, Duration::ZERO);
        self.whole_word_next = false;
        self.bump_revision();
    }

    /// Moves every caret by `motion`, extending the selections when `extend`.
    pub fn move_carets(&mut self, motion: Motion, extend: bool, tab_width: u8) {
        let primary = self.selections.primary_index();
        let list = self
            .selections
            .iter()
            .map(|sel| edit::move_selection(&self.text, sel, motion, extend, tab_width))
            .collect();
        self.change_selections(Selections::from_vec(list, primary));
    }

    /// Adds a caret at `byte` (Alt+click): it becomes the primary one, merged with a selection
    /// it touches or lies in. The position is made a valid caret position first.
    pub fn add_caret(&mut self, byte: usize) {
        let mut selections = self.selections.clone();
        selections.push(Selection::caret(self.text.snap(byte)));
        self.change_selections(selections);
    }

    /// Adds a caret on the line above (`up`) or below the primary one, at its goal column (it
    /// becomes the primary one); nothing at the first or last line.
    pub fn add_caret_vertical(&mut self, up: bool, tab_width: u8) {
        let primary = *self.selections.primary();
        let col = primary
            .goal
            .unwrap_or_else(|| self.text.col_of(primary.head, tab_width));
        let line = self.text.line_of(primary.head);
        let target = if up {
            line.checked_sub(1)
        } else {
            Some(line + 1).filter(|&next| next < self.text.line_count())
        };
        let Some(target) = target else {
            return;
        };
        let at = self.text.byte_at_col(target, col, tab_width);
        let mut selections = self.selections.clone();
        selections.push(Selection {
            anchor: at,
            head: at,
            goal: Some(col),
        });
        self.change_selections(selections);
    }

    /// Selects the word under the caret, then the next match of the primary selection
    /// (Ctrl+D); returns false when nothing was added.
    ///
    /// The search starts after the primary selection and wraps once, case-sensitive; it
    /// matches whole words only while the selection comes from this command's word expansion.
    /// Matches already selected (or overlapping a selection) are skipped.
    pub fn select_next_occurrence(&mut self) -> bool {
        let primary = *self.selections.primary();
        if primary.is_empty() {
            let Some((start, end)) = edit::run_at(&self.text, primary.head, true) else {
                return false;
            };
            let index = self.selections.primary_index();
            let mut list = self.selections.as_slice().to_vec();
            list[index] = Selection::new(start, end);
            self.change_selections(Selections::from_vec(list, index));
            self.whole_word_next = true;
            return true;
        }
        let needle = self.text.slice_to_string(primary.start(), primary.end());
        let haystack = self.text.slice_to_string(0, self.text.len_bytes());
        let whole_word = self.whole_word_next;
        let accept = |start: usize| {
            let end = start + needle.len();
            (!whole_word || is_whole_word(&haystack, start, end))
                && !self.selections.overlaps_range(start, end)
        };
        let found = haystack[primary.end()..]
            .match_indices(needle.as_str())
            .map(|(index, _)| primary.end() + index)
            .find(|&start| accept(start))
            .or_else(|| {
                haystack[..primary.start()]
                    .match_indices(needle.as_str())
                    .map(|(index, _)| index)
                    .find(|&start| accept(start))
            });
        let Some(start) = found else {
            return false;
        };
        let mut selections = self.selections.clone();
        selections.push(Selection::new(start, start + needle.len()));
        self.change_selections(selections);
        self.whole_word_next = whole_word;
        true
    }

    /// Selects the word at `byte` (double-click): the word chars around it, else the run of
    /// blanks or other chars there; a caret on an empty line.
    pub fn select_word_at(&mut self, byte: usize) {
        let selection = match edit::run_at(&self.text, byte, false) {
            Some((start, end)) => Selection::new(start, end),
            None => Selection::caret(self.text.snap(byte)),
        };
        self.change_selections(Selections::single(selection));
    }

    /// Selects `line` (clamped), terminator included.
    pub fn select_line(&mut self, line: usize) {
        let line = line.min(self.text.line_count() - 1);
        let start = self.text.line_start(line);
        let end = if line + 1 < self.text.line_count() {
            self.text.line_start(line + 1)
        } else {
            self.text.len_bytes()
        };
        self.change_selections(Selections::single(Selection::new(start, end)));
    }

    /// The text of every selection, in document order (empty for a caret).
    pub fn selected_texts(&self) -> Vec<String> {
        self.selections
            .iter()
            .map(|sel| self.text.slice_to_string(sel.start(), sel.end()))
            .collect()
    }

    /// Undoes the last group (text and selections from before it); returns false when there
    /// is none.
    pub fn undo(&mut self) -> bool {
        let Some(group) = self.history.take_undo() else {
            return false;
        };
        match &group.body {
            Body::Steps(steps) => {
                for step in steps.iter().rev() {
                    step.inverse.apply_to(&mut self.text);
                    self.grow_widest(&step.inverse);
                }
            }
            Body::Replace {
                before,
                eol_before,
                widest_before,
                ..
            } => {
                self.text = before.clone();
                self.line_ending = *eol_before;
                self.widest = *widest_before;
            }
        }
        self.restore(&group, true);
        self.history.push_redo(group);
        true
    }

    /// Redoes the last undone group (text and selections from after it); returns false when
    /// there is none.
    pub fn redo(&mut self) -> bool {
        let Some(group) = self.history.take_redo() else {
            return false;
        };
        match &group.body {
            Body::Steps(steps) => {
                for step in steps {
                    step.forward.apply_to(&mut self.text);
                    self.grow_widest(&step.forward);
                }
            }
            Body::Replace {
                after,
                eol_after,
                widest_after,
                ..
            } => {
                self.text = after.clone();
                self.line_ending = *eol_after;
                self.widest = *widest_after;
            }
        }
        self.restore(&group, false);
        self.history.push_undo(group);
        true
    }

    /// Restores the selections and content id of one side of `group` after undo (`before`)
    /// or redo.
    fn restore(&mut self, group: &Group, before: bool) {
        let sides = &group.sides;
        let (selections, state) = if before {
            (&sides.sel_before, sides.state_before)
        } else {
            (&sides.sel_after, sides.state_after)
        };
        self.selections = selections.clone();
        self.state = state;
        self.whole_word_next = false;
        self.bump_revision();
    }

    /// A snapshot of the current state, in O(1).
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            revision: self.revision,
            encoding: self.encoding,
            line_ending: self.line_ending,
        }
    }
}
