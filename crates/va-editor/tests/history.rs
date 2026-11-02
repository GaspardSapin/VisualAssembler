//! Tests of `va_editor::history` through `Document`: undo grouping with injected time, undo and
//! redo of text and selections, the dirty state, the limits, `replace_text`, and undo/redo round
//! trips over random edits (m4-editor-shell.md §3.3, §10.1).
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use va_editor::document::Document;
use va_editor::edit::{EditCommand, EditOptions, Motion};
use va_editor::encoding::{Encoding, LineEnding};
use va_editor::history::{MAX_UNDO_BYTES, MAX_UNDO_GROUPS, UNDO_GROUP_MS};
use va_editor::selection::{Selection, Selections};
use va_editor::text::Text;

/// Tabs of width 4.
const OPTS: EditOptions = EditOptions {
    tab_width: 4,
    insert_spaces: false,
};

/// `ms` milliseconds since the origin.
fn ms(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

/// A clean LF document of `text` with a caret at its end.
fn doc(text: &str) -> Document {
    let mut doc = Document::new(Text::new(text), Encoding::Utf8, LineEnding::Lf);
    doc.set_selections(Selections::single(Selection::caret(text.len())));
    doc
}

/// Types `chars` one by one, `step` ms apart from `start`; returns the time after the last.
fn type_chars(doc: &mut Document, chars: &str, start: u64, step: u64) -> u64 {
    let mut at = start;
    for c in chars.chars() {
        assert!(doc.edit(EditCommand::InsertText(c.to_string()), &OPTS, ms(at)));
        at += step;
    }
    at
}

/// The text of `doc`.
fn text(doc: &Document) -> String {
    doc.text().to_string()
}

/// Undoes until nothing is left; returns the number of undone groups.
fn undo_all(doc: &mut Document) -> usize {
    let mut count = 0;
    while doc.undo() {
        count += 1;
    }
    count
}

#[test]
fn typing_a_word_quickly_is_one_group() {
    let mut doc = doc("");
    type_chars(&mut doc, "move.l", 0, 100);
    assert_eq!(text(&doc), "move.l");
    assert!(doc.can_undo());
    assert!(doc.undo());
    assert_eq!(text(&doc), "");
    assert!(!doc.undo());
}

#[test]
fn a_pause_of_one_second_splits_the_group() {
    let mut doc = doc("");
    let at = type_chars(&mut doc, "move", 0, 100);
    // The previous char was typed at at - 100: exactly UNDO_GROUP_MS later splits.
    type_chars(&mut doc, ".l", at - 100 + UNDO_GROUP_MS, 100);
    assert_eq!(text(&doc), "move.l");
    assert!(doc.undo());
    assert_eq!(text(&doc), "move");
    assert!(doc.undo());
    assert_eq!(text(&doc), "");
    // Just under the limit merges.
    let mut doc = self::doc("");
    type_chars(&mut doc, "ab", 0, UNDO_GROUP_MS - 1);
    assert_eq!(undo_all(&mut doc), 1);
}

#[test]
fn a_line_break_splits_the_group() {
    let mut doc = doc("");
    type_chars(&mut doc, "rts", 0, 10);
    assert!(doc.edit(EditCommand::NewLine, &OPTS, ms(40)));
    type_chars(&mut doc, "nop", 50, 10);
    assert_eq!(text(&doc), "rts\nnop");
    assert!(doc.undo());
    assert_eq!(text(&doc), "rts\n");
    assert!(doc.undo());
    assert_eq!(text(&doc), "rts");
    assert_eq!(undo_all(&mut doc), 1);
    // A typed line break char splits too.
    let mut doc = self::doc("");
    type_chars(&mut doc, "a\nb", 0, 10);
    assert_eq!(undo_all(&mut doc), 3);
}

#[test]
fn a_caret_jump_splits_the_group() {
    let mut doc = doc("xy");
    type_chars(&mut doc, "ab", 0, 10);
    doc.move_carets(Motion::DocStart, false, 4);
    type_chars(&mut doc, "cd", 30, 10);
    assert_eq!(text(&doc), "cdxyab");
    assert!(doc.undo());
    assert_eq!(text(&doc), "xyab");
    assert!(doc.undo());
    assert_eq!(text(&doc), "xy");
    // Moving away and back still splits: any selection change by the user closes the group.
    let mut doc = self::doc("");
    type_chars(&mut doc, "a", 0, 10);
    doc.move_carets(Motion::Left, false, 4);
    doc.move_carets(Motion::Right, false, 4);
    type_chars(&mut doc, "b", 20, 10);
    assert_eq!(undo_all(&mut doc), 2);
}

#[test]
fn deleting_groups_and_kind_changes_split() {
    let mut doc = doc("abcdef");
    for at in [0, 10, 20] {
        assert!(doc.edit(EditCommand::Backspace, &OPTS, ms(at)));
    }
    assert_eq!(text(&doc), "abc");
    type_chars(&mut doc, "X", 30, 10);
    assert!(doc.undo());
    assert_eq!(text(&doc), "abc");
    assert!(doc.undo());
    assert_eq!(text(&doc), "abcdef");
    // Paste and other commands are groups of their own.
    let mut doc = self::doc("");
    assert!(doc.edit(EditCommand::Paste(String::from("a")), &OPTS, ms(0)));
    assert!(doc.edit(EditCommand::Paste(String::from("b")), &OPTS, ms(1)));
    assert_eq!(undo_all(&mut doc), 2);
}

#[test]
fn multi_caret_typing_groups_and_undo_restores_selections() {
    let mut doc = doc("a\nb\nc");
    let carets = Selections::from_vec(
        vec![
            Selection::caret(1),
            Selection::caret(3),
            Selection::caret(5),
        ],
        2,
    );
    doc.set_selections(carets.clone());
    type_chars(&mut doc, "xy", 0, 50);
    assert_eq!(text(&doc), "axy\nbxy\ncxy");
    assert!(doc.undo());
    assert_eq!(text(&doc), "a\nb\nc");
    assert_eq!(doc.selections(), &carets);
    assert!(doc.redo());
    assert_eq!(text(&doc), "axy\nbxy\ncxy");
    let heads: Vec<usize> = doc.selections().iter().map(|sel| sel.head).collect();
    assert_eq!(heads, [3, 7, 11]);
    assert_eq!(doc.selections().primary_index(), 2);
}

#[test]
fn undo_restores_a_replaced_selection() {
    let mut doc = doc("hello world");
    let selected = Selections::single(Selection::new(11, 6));
    doc.set_selections(selected.clone());
    type_chars(&mut doc, "you", 0, 10);
    assert_eq!(text(&doc), "hello you");
    assert!(doc.undo());
    assert_eq!(text(&doc), "hello world");
    assert_eq!(doc.selections(), &selected);
}

#[test]
fn new_edits_clear_the_redo_stack() {
    let mut doc = doc("");
    type_chars(&mut doc, "a", 0, 10);
    assert!(doc.edit(EditCommand::NewLine, &OPTS, ms(20)));
    assert!(doc.undo());
    assert!(doc.can_redo());
    type_chars(&mut doc, "b", 5_000, 10);
    assert!(!doc.can_redo());
    assert!(!doc.redo());
    assert_eq!(text(&doc), "ab");
}

#[test]
fn undo_to_the_save_point_is_clean() {
    let mut doc = doc("");
    assert!(!doc.is_dirty());
    type_chars(&mut doc, "ab", 0, 10);
    assert!(doc.is_dirty());
    doc.mark_saved();
    assert!(!doc.is_dirty());
    // Saving closed the group: the next char is undone alone.
    type_chars(&mut doc, "c", 30, 10);
    assert!(doc.is_dirty());
    assert!(doc.undo());
    assert_eq!(text(&doc), "ab");
    assert!(!doc.is_dirty());
    assert!(doc.undo());
    assert!(doc.is_dirty());
    assert!(doc.redo());
    assert!(!doc.is_dirty());
    assert!(doc.redo());
    assert!(doc.is_dirty());
    // Typing the same text again is another content state: still dirty.
    assert!(doc.undo());
    type_chars(&mut doc, "c", 10_000, 10);
    assert_eq!(text(&doc), "abc");
    assert!(doc.is_dirty());
}

#[test]
fn the_group_limit_drops_the_oldest_groups() {
    let mut doc = doc("");
    for index in 0..=MAX_UNDO_GROUPS {
        // Two chars per edit: an `Other` edit, one group each.
        let at = ms(u64::try_from(index).unwrap());
        assert!(doc.edit(EditCommand::InsertText(String::from("ab")), &OPTS, at));
    }
    assert_eq!(undo_all(&mut doc), MAX_UNDO_GROUPS);
    assert_eq!(text(&doc), "ab");
}

#[test]
fn the_byte_limit_drops_the_oldest_groups() {
    let mut doc = doc("");
    let size = MAX_UNDO_BYTES / 8;
    let filled = |index: u8| char::from(b'a' + index).to_string().repeat(size);
    // A replacement keeps both texts: the first costs `size` (from empty), the next ones
    // 2 × `size`. After four groups the history holds 7/8 of the limit; the fifth goes over it
    // and drops the first group, the sixth drops the second.
    for index in 0..6 {
        doc.replace_text(Text::new(&filled(index)));
    }
    assert_eq!(undo_all(&mut doc), 4);
    assert_eq!(text(&doc), filled(1));
}

#[test]
fn replace_text_is_one_group_holding_both_texts() {
    let mut doc = doc("one\r\ntwo\r\n");
    doc.set_selections(Selections::single(Selection::caret(7)));
    type_chars(&mut doc, "!", 0, 10);
    doc.replace_text(Text::new("uno\ndos\n"));
    assert_eq!(text(&doc), "uno\ndos\n");
    assert_eq!(doc.line_ending(), LineEnding::Lf);
    // The caret keeps its line and column.
    assert_eq!(doc.selections().primary().head, 7);
    // Typing right after is not merged into the replacement.
    type_chars(&mut doc, "?", 20, 10);
    assert!(doc.undo());
    assert_eq!(text(&doc), "uno\ndos\n");
    assert!(doc.undo());
    assert_eq!(text(&doc), "one\r\ntw!o\r\n");
    assert_eq!(doc.line_ending(), LineEnding::CrLf);
    assert_eq!(doc.selections().primary().head, 8);
    assert!(doc.redo());
    assert_eq!(text(&doc), "uno\ndos\n");
    assert_eq!(doc.line_ending(), LineEnding::Lf);
}

/// A small deterministic generator (xorshift64*).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap()).unwrap()
    }
}

/// A random command among the editing ones.
fn random_command(rng: &mut Rng) -> EditCommand {
    match rng.below(12) {
        0..=3 => EditCommand::InsertText(String::from(["a", "é", " ", "\t"][rng.below(4)])),
        4 => EditCommand::Paste(String::from("x\r\ny")),
        5 => EditCommand::NewLine,
        6 | 7 => EditCommand::Backspace,
        8 => EditCommand::Delete,
        9 => EditCommand::DuplicateLines,
        10 => EditCommand::MoveLinesUp,
        _ => EditCommand::ToggleLineComment { prefix: ";" },
    }
}

#[test]
fn undo_and_redo_round_trip_over_random_edits() {
    let mut rng = Rng(0xD0D0_CACA_1234_5678);
    for _ in 0..200 {
        let start = "move.l d0,d1\r\nrts\n";
        let mut doc = doc(start);
        let mut states = vec![text(&doc)];
        let mut at = 0;
        for _ in 0..40 {
            if rng.below(4) == 0 {
                let motion =
                    [Motion::Left, Motion::Up, Motion::DocEnd, Motion::WordLeft][rng.below(4)];
                doc.move_carets(motion, rng.below(3) == 0, 4);
            }
            if rng.below(6) == 0 {
                doc.add_caret_vertical(rng.below(2) == 0, 4);
            }
            at += [10, 300, 1_500][rng.below(3)];
            if doc.edit(random_command(&mut rng), &OPTS, ms(at)) {
                states.push(text(&doc));
            }
        }
        let end = text(&doc);
        let end_selections = doc.selections().clone();
        // Every undo lands on a state seen before, in reverse order.
        let mut cursor = states.len() - 1;
        while doc.undo() {
            let now = text(&doc);
            let found = states[..cursor].iter().rposition(|state| *state == now);
            cursor = found.unwrap_or_else(|| panic!("unseen state {now:?}"));
        }
        assert_eq!(text(&doc), start);
        assert!(!doc.is_dirty());
        while doc.redo() {}
        assert_eq!(text(&doc), end);
        assert_eq!(doc.selections(), &end_selections);
        assert_eq!(doc.is_dirty(), end != start || doc.can_undo());
    }
}
