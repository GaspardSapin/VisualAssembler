//! Covers `src/spike.rs` headless with `egui_kittest` over the AccessKit tree: the static UI
//! shows its texts and schedules no repaint (reactive mode). No GPU and no window
//! (docs/specs/m0-foundations.md §11).
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use eframe::egui::ViewportId;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use visual_assembler::spike::{self, SUBTITLE, WINDOW_TITLE};

#[test]
fn shows_title_and_subtitle() {
    let mut harness = Harness::new_ui(spike::draw);
    harness.run();
    harness.get_by_label(WINDOW_TITLE);
    harness.get_by_label(SUBTITLE);
}

#[test]
fn static_ui_schedules_no_repaint() {
    let mut harness = Harness::new_ui(spike::draw);
    harness.run();
    let repaint_delay = harness.output().viewport_output[&ViewportId::ROOT].repaint_delay;
    // `Duration::MAX` is egui's "no repaint scheduled" sentinel.
    assert_eq!(repaint_delay, Duration::MAX);
}
