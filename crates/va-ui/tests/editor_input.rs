//! Tests of the editor input, caret and blink (m4-editor-shell.md §5.6, §10.2, exit criterion
//! E5): keys, text, clipboard events, mouse, IME, the focus lock and the reactive blink rule.
#![allow(clippy::unwrap_used)]

mod support;

use std::time::Duration;

use egui::{Event, ImeEvent, Key, Modifiers, OutputCommand};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use support::{
    EDITOR_LABEL, EditorState, click_at, document, editor_harness, numbered_text, repaint_delay,
    set_time, settle,
};
use va_editor::edit::Motion;
use va_editor::selection::{Selection, Selections};
use va_ui::editor::{BLINK_HALF_PERIOD, BLINK_TIMEOUT};

/// A settled, focused editor over `text` with the caret at `caret`.
fn editor_at(text: &str, caret: usize) -> Harness<'static, EditorState> {
    let mut state = EditorState::new(text);
    state
        .doc
        .set_selections(Selections::single(Selection::caret(caret)));
    editor_harness(state)
}

/// Presses `key` with `modifiers` and runs the frames.
fn press(harness: &mut Harness<'_, EditorState>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.step();
}

/// Sends one event and runs its frame.
fn send(harness: &mut Harness<'_, EditorState>, event: Event) {
    harness.event(event);
    harness.step();
}

/// The (anchor, head) pairs of the selections.
fn selections(harness: &Harness<'_, EditorState>) -> Vec<(usize, usize)> {
    harness
        .state()
        .doc
        .selections()
        .iter()
        .map(|s| (s.anchor, s.head))
        .collect()
}

/// The texts the last frame put on the clipboard.
fn copied(harness: &Harness<'_, EditorState>) -> Vec<String> {
    harness
        .output()
        .platform_output
        .commands
        .iter()
        .filter_map(|command| match command {
            OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Where `motion` takes a caret at `from` in `text`, as `va-editor` computes it.
fn expected_after(text: &str, from: usize, motion: Motion, extend: bool) -> (usize, usize) {
    let mut doc = document(text);
    doc.set_selections(Selections::single(Selection::caret(from)));
    doc.move_carets(motion, extend, 8);
    let primary = doc.selections().primary();
    (primary.anchor, primary.head)
}

#[test]
fn text_events_insert_and_report_a_change() {
    let mut harness = editor_at("move", 4);
    send(&mut harness, Event::Text(".l".to_owned()));
    let state = harness.state();
    assert_eq!(state.text(), "move.l");
    let last = state.last.as_ref().unwrap();
    assert!(last.changed);
    assert!(last.caret_moved);
    assert!(state.doc.is_dirty());
    send(&mut harness, Event::Text("\n".to_owned()));
    assert_eq!(
        harness.state().text(),
        "move.l",
        "a line break arrives as a key"
    );
}

#[test]
fn enter_keeps_the_indentation_and_backspace_deletes() {
    let text = "\tmove.l\td0,d1";
    let mut harness = editor_at(text, text.len());
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().text(), "\tmove.l\td0,d1\n\t");
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    assert_eq!(harness.state().text(), "\tmove.l\td0,d1\n");
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    assert_eq!(harness.state().text(), text);
    press(&mut harness, Modifiers::CTRL, Key::Backspace);
    let mut expected = document(text);
    expected.set_selections(Selections::single(Selection::caret(text.len())));
    let _changed = expected.edit(
        va_editor::edit::EditCommand::DeleteWordLeft,
        &harness.state().options,
        Duration::ZERO,
    );
    assert_eq!(harness.state().text(), expected.text().to_string());
}

#[test]
fn arrows_shift_and_ctrl_move_the_carets() {
    let text = "move.l d0,d1\nrts";
    let mut harness = editor_at(text, 0);
    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(selections(&harness), [(1, 1)]);
    press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(selections(&harness), [(0, 0)]);
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(selections(&harness), [(13, 13)]);
    press(&mut harness, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(selections(&harness), [(0, 0)]);

    press(&mut harness, Modifiers::SHIFT, Key::ArrowRight);
    press(&mut harness, Modifiers::SHIFT, Key::ArrowRight);
    assert_eq!(selections(&harness), [(0, 2)]);
    assert_eq!(harness.state().doc.selected_texts(), ["mo"]);

    harness
        .state_mut()
        .doc
        .set_selections(Selections::single(Selection::caret(0)));
    press(&mut harness, Modifiers::CTRL, Key::ArrowRight);
    let (_, head) = expected_after(text, 0, Motion::WordRight, false);
    assert_eq!(selections(&harness), [(head, head)]);
    press(
        &mut harness,
        Modifiers::CTRL | Modifiers::SHIFT,
        Key::ArrowRight,
    );
    let (_, second) = expected_after(text, head, Motion::WordRight, false);
    assert_eq!(selections(&harness), [(head, second)]);
    press(&mut harness, Modifiers::CTRL, Key::ArrowLeft);
    let (_, back) = expected_after(text, second, Motion::WordLeft, false);
    assert_eq!(selections(&harness)[0].1, back);

    press(&mut harness, Modifiers::CTRL, Key::End);
    assert_eq!(selections(&harness), [(text.len(), text.len())]);
    press(&mut harness, Modifiers::NONE, Key::Home);
    assert_eq!(selections(&harness), [(13, 13)]);
    press(&mut harness, Modifiers::CTRL, Key::Home);
    assert_eq!(selections(&harness), [(0, 0)]);
    press(&mut harness, Modifiers::NONE, Key::End);
    assert_eq!(selections(&harness), [(12, 12)]);
}

#[test]
fn page_down_moves_by_the_visible_rows_minus_one() {
    let mut harness = editor_at(&numbered_text(1_000), 0);
    let metrics = harness.state().view.metrics();
    let rows = (metrics.viewport_size.y / metrics.row_height).floor() as usize - 1;
    press(&mut harness, Modifiers::NONE, Key::PageDown);
    let state = harness.state();
    let head = state.doc.selections().primary().head;
    assert_eq!(state.doc.text().line_of(head), rows);
    press(&mut harness, Modifiers::NONE, Key::PageUp);
    assert_eq!(selections(&harness), [(0, 0)]);
}

#[test]
fn tab_inserts_and_keeps_the_focus() {
    let mut state = EditorState::new("rts");
    state.other_widget = true;
    let mut harness = editor_harness(state);
    assert!(harness.get_by_label(EDITOR_LABEL).is_focused());
    press(&mut harness, Modifiers::NONE, Key::Tab);
    settle(&mut harness, 2);
    assert_eq!(harness.state().text(), "\trts");
    assert!(harness.get_by_label(EDITOR_LABEL).is_focused());
    press(&mut harness, Modifiers::SHIFT, Key::Tab);
    settle(&mut harness, 2);
    assert_eq!(harness.state().text(), "rts");
    assert!(harness.get_by_label(EDITOR_LABEL).is_focused());
    press(&mut harness, Modifiers::NONE, Key::Escape);
    settle(&mut harness, 2);
    assert!(harness.get_by_label(EDITOR_LABEL).is_focused());
    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    settle(&mut harness, 2);
    assert!(harness.get_by_label(EDITOR_LABEL).is_focused());
    assert_eq!(selections(&harness), [(1, 1)]);
}

#[test]
fn escape_collapses_to_the_primary_caret() {
    let mut harness = editor_at("aa\nbb\ncc", 0);
    harness.state_mut().doc.add_caret(3);
    harness.state_mut().doc.add_caret(6);
    settle(&mut harness, 1);
    assert_eq!(harness.state().doc.selections().len(), 3);
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert_eq!(selections(&harness), [(6, 6)]);

    harness
        .state_mut()
        .doc
        .set_selections(Selections::single(Selection::new(0, 2)));
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert_eq!(selections(&harness), [(2, 2)]);
}

#[test]
fn copy_cut_and_paste_with_two_carets() {
    let mut harness = editor_at("aa\nbb", 0);
    harness.state_mut().doc.set_selections(Selections::from_vec(
        vec![Selection::new(0, 2), Selection::new(3, 5)],
        1,
    ));
    send(&mut harness, Event::Copy);
    assert_eq!(copied(&harness), ["aa\nbb"]);
    assert_eq!(harness.state().text(), "aa\nbb");

    send(&mut harness, Event::Cut);
    assert_eq!(copied(&harness), ["aa\nbb"]);
    assert_eq!(harness.state().text(), "\n");
    assert_eq!(harness.state().doc.selections().len(), 2);

    send(&mut harness, Event::Paste("x\ny".to_owned()));
    assert_eq!(harness.state().text(), "x\ny");

    // Nothing selected: the clipboard is left alone.
    send(&mut harness, Event::Copy);
    assert!(copied(&harness).is_empty());
}

#[test]
fn read_only_refuses_edits_but_moves_and_copies() {
    let mut state = EditorState::new("rts");
    state.read_only = true;
    let mut harness = editor_harness(state);
    send(&mut harness, Event::Text("x".to_owned()));
    press(&mut harness, Modifiers::NONE, Key::Delete);
    send(&mut harness, Event::Paste("y".to_owned()));
    assert_eq!(harness.state().text(), "rts");
    assert!(!harness.state().last.as_ref().unwrap().changed);
    press(&mut harness, Modifiers::SHIFT, Key::End);
    assert_eq!(selections(&harness), [(0, 3)]);
    send(&mut harness, Event::Cut);
    assert_eq!(copied(&harness), ["rts"]);
    assert_eq!(harness.state().text(), "rts");
}

#[test]
fn clicks_place_extend_and_add_carets() {
    let mut harness = editor_at("move.l d0,d1\nadd.w d2,d3\nrts", 0);
    let pos = harness.state().char_pos(1, 4);
    click_at(&harness, pos, Modifiers::NONE);
    settle(&mut harness, 2);
    assert_eq!(selections(&harness), [(17, 17)]);

    let pos = harness.state().char_pos(2, 2);
    click_at(&harness, pos, Modifiers::SHIFT);
    settle(&mut harness, 2);
    assert_eq!(selections(&harness), [(17, 27)]);

    let pos = harness.state().char_pos(0, 3);
    click_at(&harness, pos, Modifiers::ALT);
    settle(&mut harness, 2);
    assert_eq!(selections(&harness), [(3, 3), (17, 27)]);
    assert_eq!(harness.state().doc.selections().primary_index(), 0);
}

#[test]
fn double_click_selects_a_word_and_triple_click_a_line() {
    let mut harness = editor_at("move.l d0,d1\nrts", 0);
    set_time(&mut harness, 100.0);
    let pos = harness.state().char_pos(0, 2);
    click_at(&harness, pos, Modifiers::NONE);
    click_at(&harness, pos, Modifiers::NONE);
    settle(&mut harness, 2);
    assert_eq!(harness.state().doc.selected_texts(), ["move.l"]);

    set_time(&mut harness, 200.0);
    click_at(&harness, pos, Modifiers::NONE);
    click_at(&harness, pos, Modifiers::NONE);
    click_at(&harness, pos, Modifiers::NONE);
    settle(&mut harness, 2);
    assert_eq!(harness.state().doc.selected_texts(), ["move.l d0,d1\n"]);
}

#[test]
fn dragging_selects() {
    let mut harness = editor_at("move.l d0,d1\nrts", 0);
    let from = harness.state().char_pos(0, 1);
    let to = harness.state().char_pos(1, 2);
    harness.event(Event::PointerMoved(from));
    harness.drag_at(from);
    settle(&mut harness, 1);
    for step in 1..=4 {
        let t = step as f32 / 4.0;
        harness.event(Event::PointerMoved(from + (to - from) * t));
        harness.step();
    }
    harness.drop_at(to);
    settle(&mut harness, 2);
    assert_eq!(selections(&harness), [(1, 15)]);
}

#[test]
fn ime_commit_inserts_and_preedit_does_not_edit() {
    let mut harness = editor_at("", 0);
    send(
        &mut harness,
        Event::Ime(ImeEvent::Preedit {
            text: "e".to_owned(),
            active_range_chars: None,
        }),
    );
    assert_eq!(harness.state().text(), "");
    assert!(harness.output().platform_output.ime.is_some());
    send(&mut harness, Event::Ime(ImeEvent::Commit("é".to_owned())));
    assert_eq!(harness.state().text(), "é");
    settle(&mut harness, 1);
    let ime = harness.output().platform_output.ime.unwrap();
    let rect = harness.state().rect();
    assert!(rect.contains(ime.cursor_rect.min));
}

#[test]
fn blink_off_requests_no_repaint() {
    let mut harness = editor_at(&numbered_text(100), 0);
    settle(&mut harness, 3);
    assert!(harness.get_by_label(EDITOR_LABEL).is_focused());
    assert_eq!(repaint_delay(&harness), Duration::MAX);
}

#[test]
fn blink_on_wakes_up_until_the_timeout() {
    let mut state = EditorState::new(&numbered_text(100));
    state.style.cursor_blink = true;
    let mut harness = editor_harness(state);
    set_time(&mut harness, 50.0);
    send(&mut harness, Event::Text("x".to_owned()));
    // egui itself repaints twice after an input frame.
    settle(&mut harness, 2);
    let delay = repaint_delay(&harness);
    assert!(
        delay > Duration::ZERO && delay <= BLINK_HALF_PERIOD,
        "{delay:?}"
    );

    // The wake-up lands on the next phase change; egui shortens it by the predicted frame time.
    set_time(&mut harness, 50.0 + 0.6);
    settle(&mut harness, 1);
    let predicted = f64::from(harness.input().predicted_dt);
    let expected = 2.0 * BLINK_HALF_PERIOD.as_secs_f64() - 0.6 - predicted;
    let delay = repaint_delay(&harness).as_secs_f64();
    assert!(
        (delay - expected).abs() < 1e-3,
        "{delay} s, expected {expected} s"
    );

    // The last shown phase reaches the timeout: no wake-up is needed any more.
    set_time(&mut harness, 50.0 + 9.9);
    settle(&mut harness, 1);
    assert_eq!(repaint_delay(&harness), Duration::MAX);

    set_time(&mut harness, 50.0 + BLINK_TIMEOUT.as_secs_f64() + 0.1);
    settle(&mut harness, 2);
    assert_eq!(repaint_delay(&harness), Duration::MAX);

    // New input restarts the blink.
    send(&mut harness, Event::Text("y".to_owned()));
    settle(&mut harness, 2);
    let delay = repaint_delay(&harness);
    assert!(
        delay > Duration::ZERO && delay <= BLINK_HALF_PERIOD,
        "{delay:?}"
    );
}

#[test]
fn blink_on_but_unfocused_requests_nothing() {
    let mut state = EditorState::new("rts");
    state.style.cursor_blink = true;
    state.focus_on_start = false;
    let mut harness = editor_harness(state);
    settle(&mut harness, 2);
    assert!(!harness.get_by_label(EDITOR_LABEL).is_focused());
    assert_eq!(repaint_delay(&harness), Duration::MAX);

    // Focused, but the window is not.
    let mut state = EditorState::new("rts");
    state.style.cursor_blink = true;
    let mut harness = editor_harness(state);
    harness.input_mut().focused = false;
    settle(&mut harness, 2);
    assert_eq!(repaint_delay(&harness), Duration::MAX);
}
