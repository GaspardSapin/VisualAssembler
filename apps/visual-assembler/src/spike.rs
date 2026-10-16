//! The M0 window: a static UI drawn in egui's reactive mode (docs/specs/m0-foundations.md §4.2).
//!
//! The normal path never requests a repaint: egui repaints only on input or window events. The
//! only repaint requests of the binary belong to the bench ([`crate::bench::recorder`]).
//!
//! The English literals below are a documented M0 exception to the i18n rule; M4 replaces them
//! with Fluent message ids.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui;

use crate::bench::recorder::BenchRecorder;

/// Window title, also shown as the heading of the central panel.
pub const WINDOW_TITLE: &str = "VisualAssembler";

/// Line shown under the heading.
pub const SUBTITLE: &str =
    "Foundations build (M0): the editor, assembler and debugger arrive in later milestones.";

/// Draws the static M0 UI into `ui`: a central panel with the title and the subtitle.
pub fn draw(ui: &mut egui::Ui) {
    egui::CentralPanel::default_margins().show(ui, |ui| {
        ui.heading(WINDOW_TITLE);
        ui.label(SUBTITLE);
    });
}

/// The eframe application of M0: the static UI, plus the bench recorder when one is requested.
///
/// The app borrows the recorder instead of owning it (docs/specs/m0-foundations.md §19.2): the
/// recorder and its watchdog outlive a renderer attempt that fails, so the fallback attempt keeps
/// measuring from `main()` under the same watchdog limit.
pub struct SpikeApp<'a> {
    /// Present only when the binary runs a bench scenario (`--bench`).
    bench: Option<&'a mut BenchRecorder>,
    /// Set at the top of every `ui` call: the first frame has started, so a later failure of this
    /// attempt is never a fallback case (§19.2, condition 2).
    ui_started: Arc<AtomicBool>,
}

impl<'a> SpikeApp<'a> {
    /// Creates the app. `bench` is `Some` only when a bench scenario was requested; `ui_started`
    /// is shared with `launch`, which reads it after `eframe::run_native` returns.
    pub fn new(bench: Option<&'a mut BenchRecorder>, ui_started: Arc<AtomicBool>) -> Self {
        Self { bench, ui_started }
    }
}

impl eframe::App for SpikeApp<'_> {
    /// Forwards to [`BenchRecorder::on_logic`] in bench mode; does nothing otherwise.
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(bench) = &mut self.bench {
            bench.on_logic(ctx, frame);
        }
    }

    /// Marks the first frame as started, draws the UI with [`draw`], then calls
    /// [`BenchRecorder::on_ui`] in bench mode.
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.ui_started.store(true, Ordering::SeqCst);
        draw(ui);
        if let Some(bench) = &mut self.bench {
            bench.on_ui(ui.ctx(), frame);
        }
    }
}
