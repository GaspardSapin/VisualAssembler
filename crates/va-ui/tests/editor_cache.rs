//! Tests of the colouring cache (m4-editor-shell.md §5.3, §10.2, exit criterion E5): one
//! keystroke re-lexes exactly one line, scrolling back hits, style changes clear the cache, and
//! its size stays within `LINE_CACHE_CAPACITY`.
#![allow(clippy::unwrap_used)]

mod support;

use egui::Event;
use support::{EditorState, editor_harness, numbered_text, settle};
use va_ui::editor::{LINE_CACHE_CAPACITY, RenderStats};
use va_ui::theme::Theme;

/// The stats of the last frame.
fn frame_stats(harness: &egui_kittest::Harness<'_, EditorState>) -> RenderStats {
    harness.state().view.stats()
}

#[test]
fn a_warm_frame_lexes_nothing() {
    let mut harness = editor_harness(EditorState::new(&numbered_text(100_000)));
    let calls = harness.state().syntax.calls();
    harness.step();
    let stats = frame_stats(&harness);
    assert!(stats.lines_drawn > 20);
    assert_eq!(stats.lexed_lines, 0, "{stats:?}");
    assert_eq!(stats.cache_misses, 0);
    assert!(stats.cache_hits >= stats.lines_drawn);
    assert_eq!(harness.state().syntax.calls(), calls);
}

#[test]
fn one_keystroke_relexes_exactly_one_line() {
    let mut harness = editor_harness(EditorState::new(&numbered_text(100_000)));
    harness.step();
    let calls = harness.state().syntax.calls();
    harness.event(Event::Text("x".to_owned()));
    harness.step();
    let state = harness.state();
    assert!(state.text().starts_with("xl0:"));
    let stats = state.view.stats();
    assert_eq!(stats.lexed_lines, 1, "{stats:?}");
    assert_eq!(stats.cache_misses, 1, "{stats:?}");
    assert!(stats.cache_hits >= stats.lines_drawn - 1);
    assert_eq!(state.syntax.calls(), calls + 1);

    // A keystroke in the middle of a 100,000-line document behaves the same.
    harness.state_mut().view.scroll_to_line(50_000);
    settle(&mut harness, 2);
    let start = harness.state().doc.text().line_start(50_010);
    harness
        .state_mut()
        .doc
        .set_selections(va_editor::selection::Selections::single(
            va_editor::selection::Selection::caret(start),
        ));
    settle(&mut harness, 2);
    harness.event(Event::Text("y".to_owned()));
    harness.step();
    let stats = frame_stats(&harness);
    assert_eq!(stats.lexed_lines, 1, "{stats:?}");
    assert_eq!(stats.cache_misses, 1, "{stats:?}");
}

#[test]
fn scrolling_back_gives_only_hits() {
    let mut harness = editor_harness(EditorState::new(&numbered_text(10_000)));
    harness.state_mut().view.scroll_to_line(500);
    harness.step();
    assert!(frame_stats(&harness).cache_misses > 20);
    harness.state_mut().view.scroll_to_line(0);
    harness.step();
    let stats = frame_stats(&harness);
    assert_eq!(stats.cache_misses, 0, "{stats:?}");
    assert_eq!(stats.lexed_lines, 0);
    assert!(stats.cache_hits >= stats.lines_drawn);
}

#[test]
fn identical_lines_share_one_entry() {
    let text = vec!["\tmove.l\td0,d1"; 1_000].join("\n");
    let harness = editor_harness(EditorState::new(&text));
    assert_eq!(harness.state().view.cached_lines(), 1);
}

#[test]
fn a_style_generation_change_clears_the_cache() {
    let mut harness = editor_harness(EditorState::new(&numbered_text(1_000)));
    harness.state_mut().style.style_generation += 1;
    harness.step();
    let stats = frame_stats(&harness);
    assert_eq!(stats.cache_misses, stats.lines_drawn, "{stats:?}");
    assert_eq!(stats.lexed_lines, stats.lines_drawn);
    assert_eq!(harness.state().view.cached_lines(), stats.lines_drawn);
    harness.step();
    assert_eq!(frame_stats(&harness).cache_misses, 0);
}

#[test]
fn a_theme_font_or_tab_change_clears_the_cache_too() {
    let mut harness = editor_harness(EditorState::new(&numbered_text(1_000)));
    harness.state_mut().theme = Theme::dark();
    harness.step();
    let after_theme = frame_stats(&harness);
    assert_eq!(after_theme.cache_misses, after_theme.lines_drawn);

    harness.state_mut().style.tab_width = 4;
    harness.step();
    let after_tab = frame_stats(&harness);
    assert_eq!(after_tab.cache_misses, after_tab.lines_drawn);

    harness.state_mut().style.font.size = 20.0;
    harness.step();
    let after_size = frame_stats(&harness);
    assert_eq!(after_size.cache_misses, after_size.lines_drawn);
    harness.step();
    assert_eq!(frame_stats(&harness).cache_misses, 0);
}

#[test]
fn the_cache_stays_within_its_capacity() {
    let mut harness = editor_harness(EditorState::new(&numbered_text(20_000)));
    let rows = harness
        .state()
        .view
        .stats()
        .lines_drawn
        .saturating_sub(3)
        .max(1);
    let mut line = 0;
    while line < LINE_CACHE_CAPACITY + 2_000 {
        line += rows;
        harness.state_mut().view.scroll_to_line(line);
        harness.step();
        assert!(harness.state().view.cached_lines() <= LINE_CACHE_CAPACITY);
    }
    assert!(harness.state().view.cached_lines() > LINE_CACHE_CAPACITY / 2);

    // The first lines were the least recently used: they were evicted.
    harness.state_mut().view.scroll_to_line(0);
    harness.step();
    let stats = frame_stats(&harness);
    assert_eq!(stats.cache_misses, stats.lines_drawn, "{stats:?}");

    // The lines just drawn before stay cached.
    harness.state_mut().view.scroll_to_line(line);
    harness.step();
    assert_eq!(frame_stats(&harness).cache_misses, 0);
}
