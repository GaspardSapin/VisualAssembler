//! Caret blink and caret painting (m4-editor-shell.md §5.6, reactive-mode rule).
//!
//! With blink on, the editor focused and the window focused, the caret alternates every
//! [`BLINK_HALF_PERIOD`] and the view asks for exactly one wake-up at the next phase change.
//! After [`BLINK_TIMEOUT`] without input the caret stays solid and no wake-up is asked for, so an
//! idle IDE paints no frame. Time is egui's input time, which tests control.

use std::time::Duration;

use egui::{Color32, Painter, Pos2, Rect, Stroke, pos2, vec2};

/// Time the caret stays shown, then hidden.
pub const BLINK_HALF_PERIOD: Duration = Duration::from_millis(530);

/// Time without input after which the caret stops blinking and stays solid.
pub const BLINK_TIMEOUT: Duration = Duration::from_secs(10);

/// Width of a caret, in points.
pub(crate) const CARET_WIDTH: f32 = 2.0;

/// The blink clock of one view: the time of the last input.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Blink {
    /// egui input time of the last input, in seconds.
    last_input: f64,
}

impl Blink {
    /// Restarts the blink cycle at `now` (an input happened): the caret is shown.
    pub fn restart(&mut self, now: f64) {
        self.last_input = now;
    }

    /// At `now`: whether the caret is shown, and the delay to the next change of that, if any.
    pub fn phase(&self, now: f64) -> (bool, Option<Duration>) {
        blink_phase(now - self.last_input)
    }
}

/// The blink state `elapsed` seconds after the last input: whether the caret is shown, and the
/// delay until that changes (`None`: it never changes again, the caret is solid).
pub(crate) fn blink_phase(elapsed: f64) -> (bool, Option<Duration>) {
    let elapsed = elapsed.max(0.0);
    let half = BLINK_HALF_PERIOD.as_secs_f64();
    let timeout = BLINK_TIMEOUT.as_secs_f64();
    if elapsed >= timeout {
        return (true, None);
    }
    let phase = (elapsed / half).floor();
    let shown = (phase as u64).is_multiple_of(2);
    let next = ((phase + 1.0) * half).min(timeout);
    if shown && next >= timeout {
        return (true, None);
    }
    let delay = Duration::try_from_secs_f64(next - elapsed).unwrap_or(Duration::ZERO);
    (shown, Some(delay))
}

/// The rectangle of a caret at `x` on the row starting at `top`.
pub(crate) fn caret_rect(x: f32, top: f32, row_height: f32) -> Rect {
    Rect::from_min_size(pos2(x, top), vec2(CARET_WIDTH, row_height))
}

/// Paints the IME pre-edit `galley` at `at` (the primary caret), over the text, underlined.
pub(crate) fn paint_preedit(
    painter: &Painter,
    at: Pos2,
    galley: std::sync::Arc<egui::Galley>,
    row_height: f32,
    background: Color32,
    color: Color32,
) {
    let width = galley.size().x;
    let area = Rect::from_min_size(at, vec2(width, row_height));
    painter.rect_filled(area, 0.0, background);
    painter.galley(at, galley, color);
    let underline_y = area.max.y - 1.0;
    painter.hline(area.x_range(), underline_y, Stroke::new(1.0, color));
}
