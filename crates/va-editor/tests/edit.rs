//! Tests of `va_editor::edit` through `Document::edit` and `Document::move_carets`: every
//! command of m4-editor-shell.md §3.2 with one and three carets, change sets, motions, and
//! properties (multi-caret edits never overlap, CRLF/LF terminators are kept).
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use va_core::source::LineIndex;
use va_editor::document::Document;
use va_editor::edit::{
    Assoc, Change, ChangeError, ChangeSet, EditCommand, EditOptions, Motion, is_word_char,
};
use va_editor::encoding::{Encoding, LineEnding};
use va_editor::selection::{Selection, Selections};
use va_editor::text::Text;

/// Tabs of width 4, tab characters.
const TABS: EditOptions = EditOptions {
    tab_width: 4,
    insert_spaces: false,
};

/// Tabs of width 4, spaces.
const SPACES: EditOptions = EditOptions {
    tab_width: 4,
    insert_spaces: true,
};

/// Parses `markup`: `|` is a caret, `[`…`]` a selection from anchor to head, `]`…`[` a reversed
/// one. Returns the text and the selections (the last one primary).
fn parse(markup: &str) -> (String, Vec<Selection>) {
    let mut text = String::new();
    let mut sels = Vec::new();
    let mut open: Option<(usize, char)> = None;
    for c in markup.chars() {
        match c {
            '|' => sels.push(Selection::caret(text.len())),
            '[' | ']' => match open.take() {
                None => open = Some((text.len(), c)),
                Some((at, '[')) => sels.push(Selection::new(at, text.len())),
                Some((at, _)) => sels.push(Selection::new(text.len(), at)),
            },
            _ => text.push(c),
        }
    }
    assert!(open.is_none(), "unclosed selection in {markup:?}");
    (text, sels)
}

/// A document of `markup` with line ending `eol` (when the text has none).
fn doc_eol(markup: &str, eol: LineEnding) -> Document {
    let (text, sels) = parse(markup);
    let mut doc = Document::new(Text::new(&text), Encoding::Utf8, eol);
    let primary = sels.len().saturating_sub(1);
    doc.set_selections(Selections::from_vec(sels, primary));
    doc
}

/// A document of `markup`, LF by default.
fn doc(markup: &str) -> Document {
    doc_eol(markup, LineEnding::Lf)
}

/// The markup of `doc`'s text and selections.
fn render(doc: &Document) -> String {
    let text = doc.text().to_string();
    let mut out = String::new();
    let mut cursor = 0;
    for sel in doc.selections().iter() {
        if sel.is_empty() {
            out.push_str(&text[cursor..sel.head]);
            out.push('|');
        } else {
            let (open, close) = if sel.is_reversed() {
                (']', '[')
            } else {
                ('[', ']')
            };
            out.push_str(&text[cursor..sel.start()]);
            out.push(open);
            out.push_str(&text[sel.start()..sel.end()]);
            out.push(close);
        }
        cursor = sel.end();
    }
    out.push_str(&text[cursor..]);
    out
}

/// Runs `cmd` on `before` and returns the markup after it (or `"unchanged"`).
fn run_with(before: &str, cmd: EditCommand, opts: &EditOptions, eol: LineEnding) -> String {
    let mut doc = doc_eol(before, eol);
    if doc.edit(cmd, opts, Duration::ZERO) {
        render(&doc)
    } else {
        assert_eq!(render(&doc), before, "a refused edit changed the document");
        String::from("unchanged")
    }
}

/// Runs `cmd` on `before` (LF, tabs) and returns the markup after it.
fn run(before: &str, cmd: EditCommand) -> String {
    run_with(before, cmd, &TABS, LineEnding::Lf)
}

/// Moves the carets of `before` and returns the markup after it.
fn moved(before: &str, motion: Motion, extend: bool) -> String {
    let mut doc = doc(before);
    doc.move_carets(motion, extend, 4);
    render(&doc)
}

#[test]
fn markup_round_trips() {
    for markup in ["|", "a|b", "a[bc]d", "a]bc[d", "|a\n|b", "[ab][cd]"] {
        assert_eq!(render(&doc(markup)), markup);
    }
}

#[test]
fn word_chars_are_assembly_identifiers() {
    for c in ['a', 'Z', '0', '_', '.', '@', 'é'] {
        assert!(is_word_char(c), "{c:?}");
    }
    for c in [' ', ',', '(', '#', '$', ';', '\t', '-'] {
        assert!(!is_word_char(c), "{c:?}");
    }
}

#[test]
fn change_sets_are_checked_and_sorted() {
    let text = Text::new("abcdé");
    let set = ChangeSet::new(
        vec![
            Change {
                start: 3,
                end: 4,
                insert: String::from("X"),
            },
            Change {
                start: 0,
                end: 1,
                insert: String::new(),
            },
        ],
        &text,
    )
    .unwrap();
    assert_eq!(set.changes()[0].start, 0);
    assert_eq!(set.changes()[1].insert, "X");
    let change = |start, end| Change {
        start,
        end,
        insert: String::new(),
    };
    assert_eq!(
        ChangeSet::new(vec![change(0, 3), change(2, 4)], &text),
        Err(ChangeError::Overlap)
    );
    assert_eq!(
        ChangeSet::new(vec![change(4, 99)], &text),
        Err(ChangeError::OutOfBounds)
    );
    assert_eq!(
        ChangeSet::new(vec![change(3, 2)], &text),
        Err(ChangeError::OutOfBounds)
    );
    assert_eq!(
        ChangeSet::new(vec![change(5, 5)], &text),
        Err(ChangeError::NotCharBoundary)
    );
    // Touching ranges and two insertions at one offset are fine.
    assert!(
        ChangeSet::new(
            vec![change(0, 2), change(2, 4), change(4, 4), change(4, 4)],
            &text
        )
        .is_ok()
    );
}

#[test]
fn map_offset_follows_the_assoc_rules() {
    let text = Text::new("0123456789");
    let set = ChangeSet::new(
        vec![
            Change {
                start: 2,
                end: 2,
                insert: String::from("ab"),
            },
            Change {
                start: 5,
                end: 7,
                insert: String::from("X"),
            },
        ],
        &text,
    )
    .unwrap();
    assert_eq!(set.map_offset(1, Assoc::Before), 1);
    assert_eq!(set.map_offset(2, Assoc::Before), 2);
    assert_eq!(set.map_offset(2, Assoc::After), 4);
    assert_eq!(set.map_offset(3, Assoc::Before), 5);
    // The start of a replaced range maps to the start of the insertion, its end past it.
    assert_eq!(set.map_offset(5, Assoc::After), 7);
    assert_eq!(set.map_offset(7, Assoc::Before), 8);
    // Strictly inside: start or end of the insertion.
    assert_eq!(set.map_offset(6, Assoc::Before), 7);
    assert_eq!(set.map_offset(6, Assoc::After), 8);
    assert_eq!(set.map_offset(10, Assoc::After), 11);
    assert!(
        Change {
            start: 1,
            end: 1,
            insert: String::new()
        }
        .is_noop()
    );
}

#[test]
fn insert_text_replaces_every_selection() {
    let insert = || EditCommand::InsertText(String::from("x"));
    assert_eq!(run("ab|c", insert()), "abx|c");
    assert_eq!(run("a[bc]d", insert()), "ax|d");
    assert_eq!(run("|a|b|c", insert()), "x|ax|bx|c");
    assert_eq!(run("[a]b]c[d", insert()), "x|bx|d");
    // Line breaks in the text become the document's.
    let crlf = run_with(
        "a|",
        EditCommand::InsertText(String::from("1\n2\r3")),
        &TABS,
        LineEnding::CrLf,
    );
    assert_eq!(crlf, "a1\r\n2\r\n3|");
    assert_eq!(
        run("a|b", EditCommand::InsertText(String::new())),
        "unchanged"
    );
}

#[test]
fn paste_distributes_lines_over_carets() {
    assert_eq!(
        run("a|b", EditCommand::Paste(String::from("x\ny"))),
        "ax\ny|b"
    );
    assert_eq!(
        run("|a\n|b\n|c", EditCommand::Paste(String::from("1\n2\n3"))),
        "1|a\n2|b\n3|c"
    );
    assert_eq!(
        run(
            "|a\n|b\n|c",
            EditCommand::Paste(String::from("1\r\n2\r\n3"))
        ),
        "1|a\n2|b\n3|c"
    );
    assert_eq!(
        run("|a\n|b\n|c", EditCommand::Paste(String::from("1\n2"))),
        "1\n2|a\n1\n2|b\n1\n2|c"
    );
    let crlf = run_with(
        "|",
        EditCommand::Paste(String::from("x\ny")),
        &TABS,
        LineEnding::CrLf,
    );
    assert_eq!(crlf, "x\r\ny|");
}

#[test]
fn new_line_keeps_the_indentation_up_to_the_caret() {
    assert_eq!(run("    move|", EditCommand::NewLine), "    move\n    |");
    assert_eq!(run("\t a|b", EditCommand::NewLine), "\t a\n\t |b");
    assert_eq!(run("|  ab", EditCommand::NewLine), "\n|  ab");
    assert_eq!(run("  |  ab", EditCommand::NewLine), "  \n  |  ab");
    assert_eq!(run("a[bc]d", EditCommand::NewLine), "a\n|d");
    assert_eq!(run("a|\nb|\nc|", EditCommand::NewLine), "a\n|\nb\n|\nc\n|");
    // The document line ending, whatever the line's own terminator.
    let mixed = run_with("a|\nb\r\n", EditCommand::NewLine, &TABS, LineEnding::Lf);
    assert_eq!(mixed, "a\n|\nb\r\n");
    let crlf = run_with("a|\r\nb\n", EditCommand::NewLine, &TABS, LineEnding::Lf);
    assert_eq!(crlf, "a\r\n|\r\nb\n");
}

#[test]
fn tab_inserts_a_tab_or_spaces_to_the_next_stop() {
    assert_eq!(run("a|b", EditCommand::Tab), "a\t|b");
    assert_eq!(
        run_with("a|b", EditCommand::Tab, &SPACES, LineEnding::Lf),
        "a   |b"
    );
    assert_eq!(
        run_with("abcd|", EditCommand::Tab, &SPACES, LineEnding::Lf),
        "abcd    |"
    );
    assert_eq!(
        run_with("\t|x", EditCommand::Tab, &SPACES, LineEnding::Lf),
        "\t    |x"
    );
    assert_eq!(run("|a\n|b\n|c", EditCommand::Tab), "\t|a\n\t|b\n\t|c");
    assert_eq!(run("a[b]c", EditCommand::Tab), "a\t|c");
    // A selection spanning lines indents them.
    assert_eq!(run("[a\nb]", EditCommand::Tab), "[\ta\n\tb]");
    assert_eq!(
        run_with("[a\nb]", EditCommand::Tab, &SPACES, LineEnding::Lf),
        "[    a\n    b]"
    );
}

#[test]
fn indent_adds_one_level_per_touched_line() {
    assert_eq!(run("a|b", EditCommand::Indent), "\ta|b");
    assert_eq!(
        run_with("a|b", EditCommand::Indent, &SPACES, LineEnding::Lf),
        "    a|b"
    );
    assert_eq!(run("|ab", EditCommand::Indent), "\t|ab");
    // Empty lines of a multi-line selection are skipped; one change per line.
    assert_eq!(run("[a\n\nb]", EditCommand::Indent), "[\ta\n\n\tb]");
    assert_eq!(run("|a|b|c", EditCommand::Indent), "\t|a|b|c");
    assert_eq!(run("|a\n|b\n|c", EditCommand::Indent), "\t|a\n\t|b\n\t|c");
    // A selection ending at column 0 does not touch that line.
    assert_eq!(run("[a\n]b", EditCommand::Indent), "[\ta\n]b");
}

#[test]
fn outdent_removes_one_level_per_touched_line() {
    assert_eq!(run("\ta|b", EditCommand::Outdent), "a|b");
    assert_eq!(run("      a|b", EditCommand::Outdent), "  a|b");
    assert_eq!(run("  a|b", EditCommand::Outdent), "a|b");
    assert_eq!(run("  \tx|", EditCommand::Outdent), "\tx|");
    assert_eq!(run("a|b", EditCommand::Outdent), "unchanged");
    assert_eq!(run("  |  a", EditCommand::Outdent), "|a");
    assert_eq!(run("\t|a\n\t|b\n|c", EditCommand::Outdent), "|a\n|b\n|c");
    assert_eq!(
        run_with(
            "[    a\n  b]",
            EditCommand::Outdent,
            &SPACES,
            LineEnding::Lf
        ),
        "[a\nb]"
    );
}

#[test]
fn backspace_and_delete_remove_one_char_or_the_selection() {
    assert_eq!(run("ab|c", EditCommand::Backspace), "a|c");
    assert_eq!(run("|abc", EditCommand::Backspace), "unchanged");
    assert_eq!(run("a\r\n|b", EditCommand::Backspace), "a|b");
    assert_eq!(run("a\r|b", EditCommand::Backspace), "a|b");
    assert_eq!(run("aé|", EditCommand::Backspace), "a|");
    assert_eq!(run("a[bc]d", EditCommand::Backspace), "a|d");
    assert_eq!(run("a|b|c|", EditCommand::Backspace), "|");
    assert_eq!(run("ab|\ncd|\nef|", EditCommand::Backspace), "a|\nc|\ne|");
    assert_eq!(run("a|bc", EditCommand::Delete), "a|c");
    assert_eq!(run("abc|", EditCommand::Delete), "unchanged");
    assert_eq!(run("a|\r\nb", EditCommand::Delete), "a|b");
    assert_eq!(run("|ab\n|cd\n|ef", EditCommand::Delete), "|b\n|d\n|f");
    assert_eq!(run("a]bc[d", EditCommand::Delete), "a|d");
}

#[test]
fn word_deletions_use_word_then_blank_runs() {
    assert_eq!(run("move.l d0|", EditCommand::DeleteWordLeft), "move.l |");
    assert_eq!(run("move.l |", EditCommand::DeleteWordLeft), "|");
    assert_eq!(run("a, b|", EditCommand::DeleteWordLeft), "a, |");
    assert_eq!(run("a\n|b", EditCommand::DeleteWordLeft), "a|b");
    assert_eq!(run("a\r\n|b", EditCommand::DeleteWordLeft), "a|b");
    assert_eq!(run("|move.l d0", EditCommand::DeleteWordRight), "|d0");
    assert_eq!(run("|  x", EditCommand::DeleteWordRight), "|x");
    assert_eq!(run("a|\nb", EditCommand::DeleteWordRight), "a|b");
    assert_eq!(run("|", EditCommand::DeleteWordRight), "unchanged");
    // Carets in one word share its deletion.
    assert_eq!(run("x foo|bar|baz|", EditCommand::DeleteWordLeft), "x |");
    assert_eq!(
        run("|ab cd\n|ef gh\n|ij", EditCommand::DeleteWordRight),
        "|cd\n|gh\n|"
    );
    assert_eq!(run("ab [cd] e[f]g|", EditCommand::DeleteWordLeft), "ab | |");
}

#[test]
fn delete_selection_ignores_carets() {
    assert_eq!(run("a[bc]d", EditCommand::DeleteSelection), "a|d");
    assert_eq!(run("a|bc", EditCommand::DeleteSelection), "unchanged");
    assert_eq!(run("[a]|b[c]", EditCommand::DeleteSelection), "|b|");
}

#[test]
fn delete_lines_keeps_the_caret_column() {
    assert_eq!(run("a\nb|b\nc", EditCommand::DeleteLines), "a\nc|");
    assert_eq!(run("a\nbb|b\ncccc", EditCommand::DeleteLines), "a\ncc|cc");
    assert_eq!(run("aaa\nb|b", EditCommand::DeleteLines), "a|aa");
    assert_eq!(run("a|", EditCommand::DeleteLines), "|");
    assert_eq!(run("|", EditCommand::DeleteLines), "unchanged");
    assert_eq!(run("|a\nb", EditCommand::DeleteLines), "|b");
    assert_eq!(run("a|\nb|\nc|\nd", EditCommand::DeleteLines), "d|");
    assert_eq!(run("|a\nb\n|c\nd\n|e", EditCommand::DeleteLines), "|b\n|d");
    assert_eq!(run("a\r\n|b\r\nc", EditCommand::DeleteLines), "a\r\n|c");
    assert_eq!(run("[a\nb]\nc", EditCommand::DeleteLines), "c|");
}

#[test]
fn duplicate_lines_puts_the_selection_on_the_lower_copy() {
    assert_eq!(run("a|b\nc", EditCommand::DuplicateLines), "ab\na|b\nc");
    assert_eq!(run("a\nb|", EditCommand::DuplicateLines), "a\nb\nb|");
    assert_eq!(
        run("[a\nb]\nc", EditCommand::DuplicateLines),
        "a\nb\n[a\nb]\nc"
    );
    assert_eq!(
        run("|a\n|b\n|c", EditCommand::DuplicateLines),
        "a\n|a\nb\n|b\nc\n|c"
    );
    assert_eq!(run("a\n|", EditCommand::DuplicateLines), "a\n\n|");
    assert_eq!(run("[a\n]b", EditCommand::DuplicateLines), "a\n[a\n]b");
    let crlf = run_with(
        "a|\r\nb",
        EditCommand::DuplicateLines,
        &TABS,
        LineEnding::CrLf,
    );
    assert_eq!(crlf, "a\r\na|\r\nb");
}

#[test]
fn move_lines_swaps_blocks_with_their_neighbour() {
    assert_eq!(run("a\nb|", EditCommand::MoveLinesUp), "b|\na");
    assert_eq!(run("|a\nb", EditCommand::MoveLinesUp), "unchanged");
    assert_eq!(
        run("a\n|b\n|c\n|d", EditCommand::MoveLinesUp),
        "|b\n|c\n|d\na"
    );
    assert_eq!(
        run("a\n[b\nc]\nd", EditCommand::MoveLinesUp),
        "[b\nc]\na\nd"
    );
    assert_eq!(run("a\n[b\n]c", EditCommand::MoveLinesUp), "[b\n]a\nc");
    assert_eq!(run("a|\nb", EditCommand::MoveLinesDown), "b\na|");
    assert_eq!(run("a\nb|", EditCommand::MoveLinesDown), "unchanged");
    assert_eq!(run("[a\n]b\nc", EditCommand::MoveLinesDown), "b\n[a\n]c");
    assert_eq!(run("[a\n]b", EditCommand::MoveLinesDown), "b\n[a]");
    assert_eq!(
        run("|a\n|b\n|c\nd", EditCommand::MoveLinesDown),
        "d\n|a\n|b\n|c"
    );
    // A block at the edge stays while the others move.
    assert_eq!(run("|a\nb\n|c", EditCommand::MoveLinesUp), "|a\n|c\nb");
}

#[test]
fn move_lines_keeps_every_terminator() {
    assert_eq!(run("a\r\nb|\nc", EditCommand::MoveLinesUp), "b|\r\na\nc");
    assert_eq!(run("a|\r\nb\rc", EditCommand::MoveLinesDown), "b\r\na|\rc");
}

#[test]
fn toggle_comment_both_ways() {
    let toggle = || EditCommand::ToggleLineComment { prefix: ";" };
    assert_eq!(run("mo|ve", toggle()), "; mo|ve");
    assert_eq!(run("; mo|ve", toggle()), "mo|ve");
    assert_eq!(run(";mo|ve", toggle()), "mo|ve");
    assert_eq!(run("  |", toggle()), "  ; |");
    assert_eq!(run("|", toggle()), "; |");
    assert_eq!(run("[  a\n\n    b]", toggle()), "[  ; a\n\n  ;   b]");
    assert_eq!(run("[  ; a\n\n  ;   b]", toggle()), "[  a\n\n    b]");
    // One uncommented line makes the whole block commented again.
    assert_eq!(run("[; a\nb]", toggle()), "[; ; a\n; b]");
    assert_eq!(run("|a\n|b\n|c", toggle()), "; |a\n; |b\n; |c");
    // The smallest indentation column falls inside the tab: the prefix goes before it.
    assert_eq!(run("\t|x\n  y|", toggle()), "; \t|x\n  ; y|");
    assert_eq!(
        run("a|", EditCommand::ToggleLineComment { prefix: "" }),
        "unchanged"
    );
    assert_eq!(
        run("a|", EditCommand::ToggleLineComment { prefix: "//" }),
        "// a|"
    );
}

#[test]
fn horizontal_motions() {
    assert_eq!(moved("ab|c", Motion::Left, false), "a|bc");
    assert_eq!(moved("a[bc]d", Motion::Left, false), "a|bcd");
    assert_eq!(moved("a[bc]d", Motion::Right, false), "abc|d");
    assert_eq!(moved("a[bc]d", Motion::Right, true), "a[bcd]");
    assert_eq!(moved("a\r\n|b", Motion::Left, false), "a|\r\nb");
    assert_eq!(moved("a|\r\nb", Motion::Right, false), "a\r\n|b");
    assert_eq!(moved("é|", Motion::Left, false), "|é");
    assert_eq!(moved("|", Motion::Left, false), "|");
    assert_eq!(moved("ab|c", Motion::Left, true), "a]b[c");
    assert_eq!(moved("|a\n|b", Motion::Right, false), "a|\nb|");
}

#[test]
fn word_motions() {
    assert_eq!(
        moved("|move.l d0,d1", Motion::WordRight, false),
        "move.l |d0,d1"
    );
    assert_eq!(
        moved("move.l |d0,d1", Motion::WordRight, false),
        "move.l d0|,d1"
    );
    assert_eq!(
        moved("move.l d0|,d1", Motion::WordRight, false),
        "move.l d0,|d1"
    );
    assert_eq!(
        moved("move.l d0,d1|", Motion::WordLeft, false),
        "move.l d0,|d1"
    );
    assert_eq!(moved("move.l |d0", Motion::WordLeft, false), "|move.l d0");
    assert_eq!(moved("a|\nb", Motion::WordRight, false), "a\n|b");
    assert_eq!(moved("a\n|b", Motion::WordLeft, false), "a|\nb");
    assert_eq!(moved("|ab cd", Motion::WordRight, true), "[ab ]cd");
}

#[test]
fn vertical_motions_keep_the_goal_column() {
    let mut doc = doc("abc|d\nx\nabcdef");
    doc.move_carets(Motion::Down, false, 4);
    assert_eq!(render(&doc), "abcd\nx|\nabcdef");
    assert_eq!(doc.selections().primary().goal, Some(3));
    doc.move_carets(Motion::Down, false, 4);
    assert_eq!(render(&doc), "abcd\nx\nabc|def");
    doc.move_carets(Motion::Up, false, 4);
    doc.move_carets(Motion::Up, false, 4);
    assert_eq!(render(&doc), "abc|d\nx\nabcdef");
    // Up on the first line goes to the start, Down on the last to the end.
    assert_eq!(moved("ab|c\nd", Motion::Up, false), "|abc\nd");
    assert_eq!(moved("a\nb|c", Motion::Down, false), "a\nbc|");
    // Tabs count to their stop.
    assert_eq!(
        moved("\tx|\nabcdefgh", Motion::Down, false),
        "\tx\nabcde|fgh"
    );
    assert_eq!(moved("ab|\ncd", Motion::Down, true), "ab[\ncd]");
    // A collapsed move starts from the selection's end that faces the motion.
    assert_eq!(
        moved("a]b[c\nxyz\nxyz", Motion::Down, false),
        "abc\nxy|z\nxyz"
    );
    assert_eq!(moved("a\n[1]23", Motion::PageUp(5), false), "|a\n123");
    assert_eq!(
        moved("|0\n1\n2\n3\n4", Motion::PageDown(3), false),
        "0\n1\n2\n|3\n4"
    );
    assert_eq!(moved("0|\n1\n2", Motion::PageDown(30), false), "0\n1\n2|");
}

#[test]
fn line_and_document_motions() {
    assert_eq!(moved("    mo|ve", Motion::LineStart, false), "    |move");
    assert_eq!(moved("    |move", Motion::LineStart, false), "|    move");
    assert_eq!(moved("|    move", Motion::LineStart, false), "    |move");
    assert_eq!(moved("   |  ", Motion::LineStart, false), "|     ");
    assert_eq!(moved("a|b\r\nc", Motion::LineEnd, false), "ab|\r\nc");
    assert_eq!(moved("a\nb|c", Motion::DocStart, false), "|a\nbc");
    assert_eq!(moved("a|\nbc", Motion::DocEnd, true), "a[\nbc]");
    assert_eq!(moved("|a\r\nb", Motion::To(2), false), "a|\r\nb");
    assert_eq!(moved("|é", Motion::To(1), false), "|é");
    assert_eq!(moved("|ab", Motion::To(99), false), "ab|");
    assert_eq!(moved("|a\n|b", Motion::DocEnd, false), "a\nb|");
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

/// Random valid caret positions of `doc`'s text, as selections (some empty, some not).
fn random_selections(rng: &mut Rng, doc: &Document) -> Selections {
    let text = doc.text().to_string();
    let positions: Vec<usize> = (0..=text.len())
        .filter(|&b| {
            text.is_char_boundary(b)
                && !(b > 0 && text.as_bytes()[b - 1] == b'\r' && text[b..].starts_with('\n'))
        })
        .collect();
    let count = 1 + rng.below(5);
    let list = (0..count)
        .map(|_| {
            let anchor = positions[rng.below(positions.len())];
            let head = if rng.below(2) == 0 {
                anchor
            } else {
                positions[rng.below(positions.len())]
            };
            Selection::new(anchor, head)
        })
        .collect();
    Selections::from_vec(list, rng.below(count))
}

/// Checks the invariants of `doc`: sorted, non-overlapping selections on valid caret positions
/// and a line index equal to `LineIndex`.
fn check_invariants(doc: &Document) {
    let text = doc.text().to_string();
    let all: Vec<&Selection> = doc.selections().iter().collect();
    for sel in &all {
        for at in [sel.anchor, sel.head] {
            assert!(text.is_char_boundary(at), "{at} in {text:?}");
            assert!(!(at > 0 && text.as_bytes()[at - 1] == b'\r' && text[at..].starts_with('\n')));
        }
    }
    for pair in all.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a.is_empty() || b.is_empty() {
            assert!(b.start() > a.end(), "{a:?} {b:?}");
        } else {
            assert!(b.start() >= a.end(), "{a:?} {b:?}");
        }
    }
    assert_eq!(
        doc.text().line_count(),
        LineIndex::new(&text).line_count() as usize
    );
    assert!(doc.widest_line_hint() >= doc.text().widest_line_chars());
}

/// Every command, for the random properties.
fn random_command(rng: &mut Rng) -> EditCommand {
    match rng.below(16) {
        0 => EditCommand::InsertText(String::from(["x", "é", " ", "ab", "\n"][rng.below(5)])),
        1 => EditCommand::Paste(String::from(["p\nq", "1\r\n2\n3", "z"][rng.below(3)])),
        2 => EditCommand::NewLine,
        3 => EditCommand::Tab,
        4 => EditCommand::Indent,
        5 => EditCommand::Outdent,
        6 => EditCommand::Backspace,
        7 => EditCommand::Delete,
        8 => EditCommand::DeleteWordLeft,
        9 => EditCommand::DeleteWordRight,
        10 => EditCommand::DeleteSelection,
        11 => EditCommand::DeleteLines,
        12 => EditCommand::DuplicateLines,
        13 => EditCommand::MoveLinesUp,
        14 => EditCommand::MoveLinesDown,
        _ => EditCommand::ToggleLineComment { prefix: ";" },
    }
}

#[test]
fn random_multi_caret_edits_never_overlap() {
    let mut rng = Rng(0xBADC_0FFE_E0DD_F00D);
    for round in 0..300 {
        let mut doc = doc_eol(
            "\tmove.l d0,d1\r\nloop: dbra d0,loop\n; é€\rrts",
            LineEnding::CrLf,
        );
        for _ in 0..12 {
            let selections = random_selections(&mut rng, &doc);
            doc.set_selections(selections);
            let opts = if rng.below(2) == 0 { TABS } else { SPACES };
            doc.edit(random_command(&mut rng), &opts, Duration::ZERO);
            check_invariants(&doc);
            let motion = [
                Motion::Left,
                Motion::WordRight,
                Motion::Down,
                Motion::LineStart,
            ][rng.below(4)];
            doc.move_carets(motion, rng.below(2) == 0, 4);
            check_invariants(&doc);
        }
        assert!(doc.text().len_bytes() < 100_000, "round {round}");
    }
}

#[test]
fn multi_caret_insert_matches_a_string_model() {
    let mut rng = Rng(0x0123_4567_89AB_CDEF);
    for _ in 0..500 {
        let mut doc = doc("ab\ncd é\r\nfg\rh");
        let selections = random_selections(&mut rng, &doc);
        doc.set_selections(selections);
        let mut expected = doc.text().to_string();
        for sel in doc.selections().as_slice().iter().rev() {
            expected.replace_range(sel.start()..sel.end(), "#");
        }
        assert!(doc.edit(
            EditCommand::InsertText(String::from("#")),
            &TABS,
            Duration::ZERO
        ));
        assert_eq!(doc.text().to_string(), expected);
        check_invariants(&doc);
    }
}

#[test]
fn edits_away_from_terminators_keep_them_all() {
    let mut rng = Rng(0xFEED_FACE_0000_0042);
    let source = "a1\r\nb2\nc3\rd4\r\ne5\nf6";
    let terminators = |text: &str| {
        let crlf = text.matches("\r\n").count();
        (
            crlf,
            text.matches('\n').count() - crlf,
            text.matches('\r').count() - crlf,
        )
    };
    for _ in 0..400 {
        let mut doc = doc(source);
        for _ in 0..6 {
            let selections = random_selections(&mut rng, &doc);
            doc.set_selections(selections);
            let cmd = match rng.below(4) {
                0 => EditCommand::MoveLinesUp,
                1 => EditCommand::MoveLinesDown,
                2 => EditCommand::Indent,
                _ => EditCommand::ToggleLineComment { prefix: ";" },
            };
            doc.edit(cmd, &TABS, Duration::ZERO);
        }
        let text = doc.text().to_string();
        assert_eq!(terminators(&text), terminators(source), "{text:?}");
        let mut lines: Vec<String> = va_core::source::lines(&text)
            .map(|(_, line)| line.trim_start_matches(['\t', ';', ' ']).to_owned())
            .collect();
        lines.sort();
        assert_eq!(lines, ["a1", "b2", "c3", "d4", "e5", "f6"]);
    }
}
