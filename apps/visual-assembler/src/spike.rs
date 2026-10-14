//! The M0 window: a static UI drawn in egui's reactive mode (docs/specs/m0-foundations.md §4.2).
//!
//! The normal path never requests a repaint: egui repaints only on input or window events. The
//! only repaint requests of the binary belong to the bench ([`crate::bench::recorder`]).
//!
//! The English literals below are a documented M0 exception to the i18n rule; M4 replaces them
//! with Fluent message ids.

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
pub struct SpikeApp {
    /// Present only when the binary runs a bench scenario (`--bench`).
    bench: Option<BenchRecorder>,
}

impl SpikeApp {
    /// Creates the app. `bench` is `Some` only when a bench scenario was requested.
    pub fn new(bench: Option<BenchRecorder>) -> Self {
        Self { bench }
    }
}

impl eframe::App for SpikeApp {
    /// Forwards to [`BenchRecorder::on_logic`] in bench mode; does nothing otherwise.
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(bench) = &mut self.bench {
            bench.on_logic(ctx, frame);
        }
    }

    /// Draws the UI with [`draw`], then calls [`BenchRecorder::on_ui`] in bench mode.
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        draw(ui);
        if let Some(bench) = &mut self.bench {
            bench.on_ui(ui.ctx(), frame);
        }
    }
}
