//! Tests of `va_editor::document`: revisions, O(1) snapshots, the widest-line hint, no-op edits,
//! dirty state, external changes, the size limit and the selection commands (Ctrl+D,
//! Ctrl+Alt+Up/Down, word and line selection) (m4-editor-shell.md §3.3, §10.1).
#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use va_editor::document::{Document, Snapshot};
use va_editor::edit::{Change, ChangeSet, EditCommand, EditOptions};
use va_editor::encoding::{Encoding, LineEnding};
use va_editor::history::EditKind;
use va_editor::selection::{Selection, Selections};
use va_editor::text::{MAX_DOCUMENT_BYTES, Text};

/// Tabs of width 4.
const OPTS: EditOptions = EditOptions {
    tab_width: 4,
    insert_spaces: false,
};

/// A clean LF document of `text`.
fn doc(text: &str) -> Document {
    Document::new(Text::new(text), Encoding::Utf8, LineEnding::Lf)
}

/// Sets one caret or selection.
fn select(doc: &mut Document, anchor: usize, head: usize) {
    doc.set_selections(Selections::single(Selection::new(anchor, head)));
}

/// The `(start, end)` ranges of the selections.
fn ranges(doc: &Document) -> Vec<(usize, usize)> {
    doc.selections()
        .iter()
        .map(|sel| (sel.start(), sel.end()))
        .collect()
}

/// Types `c` at time 0.
fn insert(doc: &mut Document, c: &str) -> bool {
    doc.edit(EditCommand::InsertText(c.to_owned()), &OPTS, Duration::ZERO)
}

#[test]
fn a_new_document_is_clean_with_a_caret_at_zero() {
    let doc = doc("a\r\nb");
    assert!(!doc.is_dirty());
    assert_eq!(doc.revision(), 0);
    assert_eq!(ranges(&doc), [(0, 0)]);
    assert_eq!(doc.line_ending(), LineEnding::CrLf);
    assert_eq!(doc.encoding(), Encoding::Utf8);
    let empty = Document::new(Text::new("x"), Encoding::Utf8Bom, LineEnding::Cr);
    assert_eq!(empty.line_ending(), LineEnding::Cr);
    assert_eq!(empty.encoding(), Encoding::Utf8Bom);
    assert!(!doc.can_undo() && !doc.can_redo());
}

#[test]
fn document_and_snapshot_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Document>();
    assert_send_sync::<Snapshot>();
}

#[test]
fn every_applied_change_increments_the_revision() {
    let mut doc = doc("ab");
    select(&mut doc, 2, 2);
    assert!(insert(&mut doc, "c"));
    assert_eq!(doc.revision(), 1);
    assert!(insert(&mut doc, "d"));
    assert_eq!(doc.revision(), 2);
    assert!(doc.undo());
    assert_eq!(doc.revision(), 3);
    assert!(doc.redo());
    assert_eq!(doc.revision(), 4);
    doc.replace_text(Text::new("zz"));
    assert_eq!(doc.revision(), 5);
    // Selection changes and refused edits do not count.
    select(&mut doc, 0, 0);
    assert!(!doc.edit(EditCommand::Backspace, &OPTS, Duration::ZERO));
    assert_eq!(doc.revision(), 5);
}

#[test]
fn no_op_edits_return_false() {
    let mut doc = doc("a\nb");
    assert!(!doc.edit(EditCommand::Backspace, &OPTS, Duration::ZERO));
    assert!(!doc.edit(EditCommand::DeleteSelection, &OPTS, Duration::ZERO));
    assert!(!doc.edit(EditCommand::MoveLinesUp, &OPTS, Duration::ZERO));
    assert!(!doc.edit(EditCommand::Outdent, &OPTS, Duration::ZERO));
    assert!(!insert(&mut doc, ""));
    assert!(!doc.is_dirty());
    assert!(!doc.can_undo());
}

#[test]
fn snapshots_share_storage() {
    let big = "x".repeat(63) + "\n";
    let big = big.repeat(MAX_DOCUMENT_BYTES / 64);
    let mut doc = doc(&big);
    let fastest = (0..5)
        .map(|_| {
            let started = Instant::now();
            let snapshot = doc.snapshot();
            let elapsed = started.elapsed();
            assert_eq!(snapshot.text.len_bytes(), MAX_DOCUMENT_BYTES);
            elapsed
        })
        .min()
        .unwrap();
    assert!(
        fastest < Duration::from_millis(1),
        "16 MiB snapshot took {fastest:?}"
    );
    let snapshot = doc.snapshot();
    assert_eq!(snapshot.revision, 0);
    assert_eq!(snapshot.line_ending, LineEnding::Lf);
    // Later edits leave the snapshot as it was (a same-size replacement: the text is at the
    // size limit).
    select(&mut doc, 0, 1);
    assert!(insert(&mut doc, "!"));
    assert_eq!(snapshot.text.slice_to_string(0, 2), "xx");
    assert_eq!(doc.text().slice_to_string(0, 2), "!x");
    assert_eq!(doc.snapshot().revision, 1);
}

#[test]
fn the_widest_line_hint_grows_and_is_exact_after_replace() {
    let mut doc = doc("ab\nabcd\nc");
    assert_eq!(doc.widest_line_hint(), 4);
    select(&mut doc, 10, 10);
    for _ in 0..6 {
        assert!(insert(&mut doc, "é"));
    }
    assert_eq!(doc.widest_line_hint(), 7);
    // Deleting never shrinks it.
    let len = doc.text().len_bytes();
    select(&mut doc, 8, len);
    assert!(doc.edit(EditCommand::DeleteSelection, &OPTS, Duration::ZERO));
    assert_eq!(doc.widest_line_hint(), 7);
    doc.replace_text(Text::new("a\nbb"));
    assert_eq!(doc.widest_line_hint(), 2);
    assert!(doc.undo());
    assert_eq!(doc.widest_line_hint(), 7);
    // A paste of several lines counts every one of them.
    let mut doc = self::doc("");
    assert!(doc.edit(
        EditCommand::Paste(String::from("a\nabcdefgh\nab")),
        &OPTS,
        Duration::ZERO
    ));
    assert_eq!(doc.widest_line_hint(), 8);
}

#[test]
fn encoding_and_line_ending_changes_count_for_the_dirty_state() {
    let mut doc = doc("a\nb");
    doc.set_encoding(Encoding::Windows1252);
    assert!(doc.is_dirty());
    assert_eq!(doc.encoding(), Encoding::Windows1252);
    doc.set_encoding(Encoding::Utf8);
    assert!(!doc.is_dirty());
    doc.set_encoding(Encoding::Utf8Bom);
    doc.mark_saved();
    assert!(!doc.is_dirty());
    // A replacement that changes the line ending is dirty even with the same content id path.
    doc.replace_text(Text::new("a\r\nb"));
    assert_eq!(doc.line_ending(), LineEnding::CrLf);
    assert!(doc.is_dirty());
    assert!(doc.undo());
    assert!(!doc.is_dirty());
}

#[test]
fn mark_saved_at_handles_edits_made_during_a_save() {
    let mut doc = doc("");
    assert!(insert(&mut doc, "a"));
    let snapshot = doc.snapshot();
    // Two seconds later: a group of its own.
    let later = Duration::from_secs(2);
    assert!(doc.edit(EditCommand::InsertText(String::from("b")), &OPTS, later));
    assert!(doc.mark_saved_at(&snapshot));
    assert!(
        doc.is_dirty(),
        "the edit made during the save keeps it dirty"
    );
    assert!(doc.undo());
    assert_eq!(doc.text().to_string(), "a");
    assert!(!doc.is_dirty(), "back at the saved content");
    // The current revision behaves as `mark_saved`.
    let snapshot = doc.snapshot();
    assert!(insert(&mut doc, "c"));
    assert!(doc.undo());
    let current = doc.snapshot();
    assert!(doc.mark_saved_at(&current));
    assert!(!doc.is_dirty());
    // A snapshot encoding that differs from the document's keeps it dirty.
    let mut other = snapshot.clone();
    other.revision = doc.revision();
    other.encoding = Encoding::Windows1252;
    assert!(doc.mark_saved_at(&other));
    assert!(doc.is_dirty());
    // A revision that is too old is refused.
    for _ in 0..300 {
        assert!(insert(&mut doc, "x"));
    }
    assert!(!doc.mark_saved_at(&snapshot));
}

#[test]
fn apply_maps_selections_and_is_undoable() {
    let mut doc = doc("move d0,d1");
    doc.set_selections(Selections::from_vec(
        vec![Selection::caret(4), Selection::new(5, 7)],
        1,
    ));
    let set = ChangeSet::new(
        vec![
            Change {
                start: 4,
                end: 4,
                insert: String::from(".l"),
            },
            Change {
                start: 5,
                end: 5,
                insert: String::from("#"),
            },
        ],
        doc.text(),
    )
    .unwrap();
    doc.apply(set, EditKind::Other, Duration::ZERO);
    assert_eq!(doc.text().to_string(), "move.l #d0,d1");
    // The caret moved after the insertion at it; the selection did not grow over the `#`.
    assert_eq!(ranges(&doc), [(6, 6), (8, 10)]);
    assert_eq!(doc.selections().primary_index(), 1);
    assert_eq!(doc.revision(), 1);
    assert!(doc.undo());
    assert_eq!(doc.text().to_string(), "move d0,d1");
    // A set built for another text that no longer fits is ignored.
    let stale = ChangeSet::new(
        vec![Change {
            start: 12,
            end: 13,
            insert: String::new(),
        }],
        &Text::new("a longer text here"),
    )
    .unwrap();
    doc.apply(stale, EditKind::Other, Duration::ZERO);
    assert_eq!(doc.text().to_string(), "move d0,d1");
}

#[test]
fn edits_never_grow_a_document_past_the_size_limit() {
    let mut doc = doc(&"x".repeat(MAX_DOCUMENT_BYTES - 1));
    select(&mut doc, 0, 0);
    assert!(insert(&mut doc, "a"));
    assert!(!insert(&mut doc, "b"));
    assert_eq!(doc.text().len_bytes(), MAX_DOCUMENT_BYTES);
    // Edits that do not grow it still work.
    assert!(doc.edit(EditCommand::Delete, &OPTS, Duration::ZERO));
    select(&mut doc, 0, 1);
    assert!(insert(&mut doc, "c"));
}

#[test]
fn set_selections_makes_positions_valid() {
    let mut doc = doc("é\r\nb");
    doc.set_selections(Selections::from_vec(
        vec![Selection::new(1, 99), Selection::caret(3)],
        0,
    ));
    assert_eq!(ranges(&doc), [(0, 5)]);
    doc.set_selections(Selections::single(Selection::caret(3)));
    assert_eq!(ranges(&doc), [(2, 2)]);
}

#[test]
fn select_next_occurrence_selects_the_word_then_the_next_matches() {
    let mut doc = doc("foo bar foo foo");
    select(&mut doc, 9, 9);
    assert!(doc.select_next_occurrence());
    assert_eq!(ranges(&doc), [(8, 11)]);
    assert!(doc.select_next_occurrence());
    assert_eq!(ranges(&doc), [(8, 11), (12, 15)]);
    assert_eq!(doc.selections().primary_index(), 1);
    // It wraps around to the first match.
    assert!(doc.select_next_occurrence());
    assert_eq!(ranges(&doc), [(0, 3), (8, 11), (12, 15)]);
    assert_eq!(doc.selections().primary_index(), 0);
    assert!(!doc.select_next_occurrence());
    assert_eq!(doc.selected_texts(), ["foo", "foo", "foo"]);
}

#[test]
fn select_next_occurrence_matches_whole_words_only_after_its_word_expansion() {
    let mut doc = doc("loop loop2 loop .loop");
    select(&mut doc, 1, 1);
    assert!(doc.select_next_occurrence());
    assert!(doc.select_next_occurrence());
    assert_eq!(ranges(&doc), [(0, 4), (11, 15)]);
    assert!(!doc.select_next_occurrence());
    // A selection made by the user matches anywhere, case-sensitive.
    let mut doc = self::doc("loop loop2 LOOP");
    select(&mut doc, 0, 4);
    assert!(doc.select_next_occurrence());
    assert_eq!(ranges(&doc), [(0, 4), (5, 9)]);
    assert!(!doc.select_next_occurrence());
    // No word under the caret: nothing.
    let mut doc = self::doc("a  , b");
    select(&mut doc, 2, 2);
    assert!(!doc.select_next_occurrence());
}

#[test]
fn add_caret_is_alt_click() {
    let mut doc = doc("ab\r\ncd");
    select(&mut doc, 1, 1);
    doc.add_caret(5);
    assert_eq!(ranges(&doc), [(1, 1), (5, 5)]);
    assert_eq!(doc.selections().primary_index(), 1);
    // Inside a CRLF pair: moved before the CR. On an existing caret: merged, primary.
    doc.add_caret(3);
    assert_eq!(ranges(&doc), [(1, 1), (2, 2), (5, 5)]);
    doc.add_caret(1);
    assert_eq!(ranges(&doc), [(1, 1), (2, 2), (5, 5)]);
    assert_eq!(doc.selections().primary_index(), 0);
    assert!(insert(&mut doc, "-"));
    assert_eq!(doc.text().to_string(), "a-b-\r\nc-d");
}

#[test]
fn select_next_occurrence_finds_multi_line_selections() {
    let mut doc = doc("a\nb\na\nb\n");
    select(&mut doc, 0, 3);
    assert!(doc.select_next_occurrence());
    assert_eq!(ranges(&doc), [(0, 3), (4, 7)]);
}

#[test]
fn add_caret_vertical_adds_carets_at_the_goal_column() {
    let mut doc = doc("abcd\nx\nabcdef");
    select(&mut doc, 3, 3);
    doc.add_caret_vertical(false, 4);
    assert_eq!(ranges(&doc), [(3, 3), (6, 6)]);
    assert_eq!(doc.selections().primary().goal, Some(3));
    doc.add_caret_vertical(false, 4);
    assert_eq!(ranges(&doc), [(3, 3), (6, 6), (10, 10)]);
    assert_eq!(doc.selections().primary_index(), 2);
    // Nothing below the last line.
    doc.add_caret_vertical(false, 4);
    assert_eq!(doc.selections().len(), 3);
    let mut doc = self::doc("ab\ncd");
    select(&mut doc, 4, 4);
    doc.add_caret_vertical(true, 4);
    assert_eq!(ranges(&doc), [(1, 1), (4, 4)]);
    doc.add_caret_vertical(true, 4);
    assert_eq!(doc.selections().len(), 2);
    // Typing goes to every caret.
    assert!(insert(&mut doc, "!"));
    assert_eq!(doc.text().to_string(), "a!b\nc!d");
}

#[test]
fn select_word_and_line() {
    let mut doc = doc("move.l d0,d1\n\nrts");
    doc.select_word_at(2);
    assert_eq!(ranges(&doc), [(0, 6)]);
    doc.select_word_at(6);
    assert_eq!(ranges(&doc), [(0, 6)]);
    doc.select_word_at(9);
    assert_eq!(ranges(&doc), [(7, 9)]);
    doc.select_word_at(10);
    assert_eq!(ranges(&doc), [(10, 12)]);
    doc.select_word_at(13);
    assert_eq!(ranges(&doc), [(13, 13)]);
    let mut doc = self::doc("a , b");
    doc.select_word_at(2);
    assert_eq!(ranges(&doc), [(2, 3)]);
    doc.select_word_at(4);
    assert_eq!(ranges(&doc), [(4, 5)]);
    let mut doc = self::doc("a\r\nbc\r\nd");
    doc.select_line(1);
    assert_eq!(ranges(&doc), [(3, 7)]);
    assert_eq!(doc.selected_texts(), ["bc\r\n"]);
    doc.select_line(99);
    assert_eq!(ranges(&doc), [(7, 8)]);
}

#[test]
fn selected_texts_follow_document_order() {
    let mut doc = doc("one two three");
    doc.set_selections(Selections::from_vec(
        vec![
            Selection::new(8, 13),
            Selection::caret(3),
            Selection::new(4, 7),
        ],
        0,
    ));
    assert_eq!(doc.selected_texts(), ["", "two", "three"]);
}
