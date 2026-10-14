//! `visual-assembler`: the VisualAssembler desktop IDE (eframe window).
//!
//! This binary is a thin entry point: all logic lives in the `visual_assembler` library so that
//! the command line, the UI and the bench are testable headless.
//!
//! In Windows release builds the executable uses the GUI subsystem, so stdout and stderr exist
//! only when they are redirected (the bench pipes them). Use a debug build or the `va` CLI for
//! terminal work.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::process::ExitCode;
use std::time::Instant;

/// Records the process start instant first (it anchors `startup_ms`), then runs the app.
fn main() -> ExitCode {
    let started = Instant::now();
    visual_assembler::run(started)
}
