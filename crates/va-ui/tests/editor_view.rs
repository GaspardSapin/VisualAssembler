//! Tests of the virtualized editor view (m4-editor-shell.md §5.2, §10.2, exit criterion E5).
#![allow(clippy::unwrap_used)]

mod support;

use egui::{Event, Modifiers};
use egui_kittest::kittest::Queryable;
use support::{CODE_FONT_SIZE, EDITOR_LABEL, EditorState, editor_harness, numbered_text, settle};
use va_core::syntax::{SyntaxProvider, TokenKind, check_line_tokens};
use va_editor::edit::Motion;
use va_editor::selection::{Selection, Selections};
use va_ui::editor::{
    Decoration, DecorationKind, MAX_DRAWN_CHARS, PlainSyntax, gutter_columns, gutter_digits,
};
use va_ui::fonts;

/// Most lines a frame may draw: the rows that can intersect the viewport, plus one above and
/// one below.
fn drawn_bound(state: &EditorState) -> usize {
    let metrics = state.view.metrics();
    let rows = (metrics.viewport_size.y / metrics.row_height).ceil() as usize + 1;
    rows + 2
}

/// Rows fully inside the viewport.
fn full_rows(state: &EditorState) -> usize {
    let metrics = state.view.metrics();
    (metrics.viewport_size.y / metrics.row_height).floor() as usize
}

#[test]
fn only_visible_lines_are_drawn_on_100k_lines() {
    let mut harness = editor_harness(EditorState::new(&numbered_text(100_000)));
    let state = harness.state();
    assert!(state.view.metrics().viewport_size.y > 500.0);
    let stats = state.view.stats();
    assert!(stats.lines_drawn >= full_rows(state));
    assert!(stats.lines_drawn <= drawn_bound(state), "{stats:?}");
    assert_eq!(state.view.top_line(), 0);
    assert!(state.syntax.calls() <= 4 * drawn_bound(state));

    harness.state_mut().view.scroll_to_line(50_000);
    harness.step();
    let state = harness.state();
    assert_eq!(state.view.top_line(), 50_000);
    assert!(state.view.stats().lines_drawn <= drawn_bound(state));
    assert!(state.view.stats().lines_drawn >= full_rows(state));

    harness.state_mut().view.scroll_to_line(99_999);
    settle(&mut harness, 2);
    let state = harness.state();
    let top = state.view.top_line();
    assert!(top > 99_999 - drawn_bound(state), "top line {top}");
    assert!(top + full_rows(state) >= 99_999, "the last line is visible");
    assert!(state.view.stats().lines_drawn <= drawn_bound(state));
    assert!(state.syntax.calls() <= 16 * drawn_bound(state));
}

#[test]
fn content_height_is_lines_times_row_height() {
    let harness = editor_harness(EditorState::new(&numbered_text(100_000)));
    let row_height = harness
        .ctx
        .fonts_mut(|f| f.row_height(&fonts::code_font(CODE_FONT_SIZE)));
    let metrics = harness.state().view.metrics();
    assert!((metrics.row_height - row_height).abs() < 1e-4);
    let expected = 100_000.0 * row_height;
    assert!((metrics.content_size.y - expected).abs() <= expected * 1e-6);
}

#[test]
fn caret_moves_scroll_the_view() {
    let mut harness = editor_harness(EditorState::new(&numbered_text(100_000)));
    harness.key_press_modifiers(Modifiers::CTRL, egui::Key::End);
    settle(&mut harness, 3);
    let state = harness.state();
    assert_eq!(
        state.doc.selections().primary().head,
        state.doc.text().len_bytes()
    );
    assert!(state.view.top_line() + full_rows(state) >= 99_999);

    harness.key_press_modifiers(Modifiers::CTRL, egui::Key::Home);
    settle(&mut harness, 3);
    assert_eq!(harness.state().view.top_line(), 0);
}

#[test]
fn long_lines_are_truncated_but_the_caret_reaches_their_end() {
    let long = "x".repeat(10_000);
    let mut harness = editor_harness(EditorState::new(&format!("{long}\nshort")));
    let state = harness.state();
    let metrics = state.view.metrics();
    let cap_width = metrics.gutter_width + (MAX_DRAWN_CHARS + 5) as f32 * metrics.char_width;
    assert!(metrics.content_size.x <= cap_width, "{metrics:?}");
    assert!(metrics.content_size.x > MAX_DRAWN_CHARS as f32 * metrics.char_width);
    let value = harness.get_by_label(EDITOR_LABEL).value().unwrap();
    assert_eq!(value.chars().count(), MAX_DRAWN_CHARS);

    harness.key_press(egui::Key::End);
    settle(&mut harness, 2);
    assert_eq!(harness.state().doc.selections().primary().head, 10_000);
    assert!(harness.state().view.stats().lines_drawn <= 4);
}

#[test]
fn gutter_width_follows_the_digit_count() {
    assert_eq!(gutter_digits(1), 3);
    assert_eq!(gutter_digits(9), 3);
    assert_eq!(gutter_digits(999), 3);
    assert_eq!(gutter_digits(1_000), 4);
    assert_eq!(gutter_digits(99_999), 5);
    assert_eq!(gutter_digits(100_000), 6);
    assert_eq!(gutter_columns(9, true), 6);
    assert_eq!(gutter_columns(99_999, true), 8);
    assert_eq!(gutter_columns(99_999, false), 2);

    for (lines, columns) in [(9, 6), (99_999, 8)] {
        let harness = editor_harness(EditorState::new(&numbered_text(lines)));
        let metrics = harness.state().view.metrics();
        let expected = columns as f32 * metrics.char_width;
        assert!(
            (metrics.gutter_width - expected).abs() < 1e-3,
            "{lines} lines: {metrics:?}"
        );
    }

    let mut state = EditorState::new(&numbered_text(9));
    state.style.line_numbers = false;
    let harness = editor_harness(state);
    let metrics = harness.state().view.metrics();
    assert!((metrics.gutter_width - 2.0 * metrics.char_width).abs() < 1e-3);
}

#[test]
fn the_widget_is_a_text_edit_found_by_its_label() {
    let mut harness = editor_harness(EditorState::new("first line\nsecond line"));
    let node = harness.get_by_label(EDITOR_LABEL);
    assert_eq!(node.value().as_deref(), Some("first line"));
    assert!(node.is_focused());

    harness.key_press(egui::Key::ArrowDown);
    settle(&mut harness, 1);
    let node = harness.get_by_label(EDITOR_LABEL);
    assert_eq!(node.value().as_deref(), Some("second line"));
}

#[test]
fn zoom_with_ctrl_wheel_and_style_changes_work_headless() {
    let mut harness = editor_harness(EditorState::new(&numbered_text(1_000)));
    let center = harness.state().rect().center();
    harness.event(Event::PointerMoved(center));
    harness.event(Event::Zoom(1.5));
    settle(&mut harness, 3);
    let zoom = harness.ctx.zoom_factor();
    assert!((zoom - 1.5).abs() < 1e-3, "zoom {zoom}");
    let state = harness.state();
    assert!(state.view.stats().lines_drawn > 0);
    assert!(state.view.stats().lines_drawn <= drawn_bound(state));

    harness.event(Event::Zoom(100.0));
    settle(&mut harness, 3);
    assert!((harness.ctx.zoom_factor() - fonts::ZOOM_MAX).abs() < 1e-3);

    // The window is now smaller in points: hover the editor again.
    let center = harness.state().rect().center();
    harness.event(Event::PointerMoved(center));
    harness.event(Event::Zoom(0.0001));
    settle(&mut harness, 3);
    assert!((harness.ctx.zoom_factor() - fonts::ZOOM_MIN).abs() < 1e-3);
    let state = harness.state();
    assert!(state.view.stats().lines_drawn <= drawn_bound(state));

    // Away from the editor, the wheel zoom does nothing.
    harness.event(Event::PointerGone);
    harness.event(Event::Zoom(2.0));
    settle(&mut harness, 3);
    assert!((harness.ctx.zoom_factor() - fonts::ZOOM_MIN).abs() < 1e-3);
}

#[test]
fn a_tab_is_drawn_to_the_next_stop_and_hit_tests_back() {
    let mut harness = editor_harness(EditorState::new("a\tb"));
    // Column 8 is the `b` after the tab (tab width 8).
    let pos = harness.state().char_pos(0, 8);
    support::click_at(&harness, pos, Modifiers::NONE);
    settle(&mut harness, 2);
    assert_eq!(harness.state().doc.selections().primary().head, 2);

    // Column 2 is inside the tab's spaces, nearer its start.
    let pos = harness.state().char_pos(0, 2);
    support::click_at(&harness, pos, Modifiers::NONE);
    settle(&mut harness, 2);
    assert_eq!(harness.state().doc.selections().primary().head, 1);

    // Column 6 is nearer the tab's end.
    let pos = harness.state().char_pos(0, 6);
    support::click_at(&harness, pos, Modifiers::NONE);
    settle(&mut harness, 2);
    assert_eq!(harness.state().doc.selections().primary().head, 2);
}

#[test]
fn plain_syntax_splits_whitespace_and_symbol_runs() {
    let mut tokens = Vec::new();
    let line = "  move.l\td0, d1 ; é";
    let _state = PlainSyntax.lex_line(line, va_core::syntax::LineState::INITIAL, &mut tokens);
    check_line_tokens(line, &tokens).unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        [
            TokenKind::Whitespace,
            TokenKind::Symbol,
            TokenKind::Whitespace,
            TokenKind::Symbol,
            TokenKind::Whitespace,
            TokenKind::Symbol,
            TokenKind::Whitespace,
            TokenKind::Symbol,
            TokenKind::Whitespace,
            TokenKind::Symbol,
        ]
    );
    tokens.clear();
    let _state = PlainSyntax.lex_line("", va_core::syntax::LineState::INITIAL, &mut tokens);
    assert!(tokens.is_empty());
    assert!(PlainSyntax.keywords().is_empty());
}

#[test]
fn a_document_end_motion_from_the_outside_is_followed() {
    let mut harness = editor_harness(EditorState::new(&numbered_text(10_000)));
    harness
        .state_mut()
        .doc
        .move_carets(Motion::DocEnd, false, 8);
    settle(&mut harness, 3);
    let state = harness.state();
    assert!(state.view.top_line() + full_rows(state) >= 9_999);
}

/// What one painted shape is, for the painting-order test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Painted {
    /// A filled rectangle of this colour.
    Rect(egui::Color32),
    /// A text galley.
    Text,
}

/// The shapes of the last frame, in painting order.
fn painted(harness: &egui_kittest::Harness<'_, EditorState>) -> Vec<(Painted, egui::Rect)> {
    let mut out = Vec::new();
    for clipped in &harness.output().shapes {
        match &clipped.shape {
            egui::Shape::Rect(rect) => out.push((Painted::Rect(rect.fill), rect.rect)),
            egui::Shape::Text(text) => out.push((Painted::Text, text.visual_bounding_rect())),
            _ => {}
        }
    }
    out
}

#[test]
fn a_line_is_painted_background_selection_decoration_text_caret() {
    let mut state = EditorState::new("move.l d0,d1");
    state.decorations = vec![Decoration {
        start: 7,
        end: 9,
        kind: DecorationKind::SearchMatch,
    }];
    let selection = Selections::single(Selection::new(0, 4));
    state.doc.set_selections(selection);
    let mut harness = editor_harness(state);
    settle(&mut harness, 1);
    let theme = harness.state().theme;
    let shapes = painted(&harness);
    let index_of = |wanted: Painted| shapes.iter().position(|(kind, _)| *kind == wanted);
    let current = index_of(Painted::Rect(theme.current_line)).unwrap();
    let selection = index_of(Painted::Rect(theme.selection)).unwrap();
    let decoration = index_of(Painted::Rect(theme.search_match)).unwrap();
    let text = decoration
        + shapes[decoration..]
            .iter()
            .position(|(kind, _)| *kind == Painted::Text)
            .unwrap();
    let caret = text
        + shapes[text..]
            .iter()
            .position(|(kind, _)| *kind == Painted::Rect(theme.caret))
            .unwrap();
    assert!(current < selection && selection < decoration && decoration < text && text < caret);

    // The selection spans the first four columns, the match columns 7 and 8 (egui puts glyphs
    // on whole pixels, so widths are within a pixel of the char advance).
    let metrics = harness.state().view.metrics();
    let selection_width = shapes[selection].1.width();
    assert!((selection_width - 4.0 * metrics.char_width).abs() <= 1.0);
    let decoration_width = shapes[decoration].1.width();
    assert!((decoration_width - 2.0 * metrics.char_width).abs() <= 1.0);
    // The caret is 2 points wide, one row high, at the selection's head.
    let caret_rect = shapes[caret].1;
    assert!((caret_rect.width() - 2.0).abs() < 1e-3);
    assert!((caret_rect.height() - metrics.row_height).abs() < 1e-3);
    assert!((caret_rect.min.x - shapes[selection].1.max.x).abs() < 0.5);
}

#[test]
fn no_current_line_background_when_it_is_off() {
    let mut state = EditorState::new("rts");
    state.style.highlight_current_line = false;
    let mut harness = editor_harness(state);
    settle(&mut harness, 1);
    let theme = harness.state().theme;
    assert!(
        !painted(&harness)
            .iter()
            .any(|(kind, _)| *kind == Painted::Rect(theme.current_line))
    );
}

#[test]
fn dragging_past_the_bottom_edge_autoscrolls_while_held() {
    let mut harness = editor_harness(EditorState::new(&numbered_text(1_000)));
    let from = harness.state().char_pos(2, 1);
    let below = egui::pos2(from.x, harness.state().rect().max.y + 40.0);
    harness.event(Event::PointerMoved(from));
    harness.drag_at(from);
    settle(&mut harness, 1);
    harness.event(Event::PointerMoved(below));
    settle(&mut harness, 1);
    for _ in 0..20 {
        assert!(support::repaint_delay(&harness) <= std::time::Duration::from_millis(16));
        harness.step();
    }
    let state = harness.state();
    let scrolled_to = state.view.top_line();
    assert!(scrolled_to > 10, "top line {scrolled_to}");
    let primary = *state.doc.selections().primary();
    assert!(primary.head > primary.anchor);
    harness.drop_at(below);
    settle(&mut harness, 3);
    assert_eq!(support::repaint_delay(&harness), std::time::Duration::MAX);
    let top = harness.state().view.top_line();
    settle(&mut harness, 3);
    let after = harness.state().view.top_line();
    assert_eq!(after, top, "the scroll stops on release");
}
