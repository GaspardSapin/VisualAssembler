//! `visual_assembler`: library behind the VisualAssembler desktop IDE binary.
//!
//! M0 scope (docs/specs/m0-foundations.md §4): one eframe window drawing a static UI in reactive
//! mode (no continuous repaint), a command line with stable exit codes, and the bench
//! instrumentation that measures startup and idle cost (`--bench startup|idle`).
//!
//! Modules: [`cli`] parses and validates the command line, [`spike`] draws the M0 window, and
//! [`bench`](mod@bench) measures it. [`run`] wires them together for `main.rs`.

#[cfg(not(any(feature = "renderer-wgpu", feature = "renderer-glow")))]
compile_error!(
    "visual-assembler needs at least one renderer: enable the `renderer-wgpu` or `renderer-glow` feature"
);

pub mod bench;
pub mod cli;
pub mod spike;

use std::fmt::Display;
use std::io::Write as _;
use std::process::ExitCode;
use std::time::Instant;

use clap::Parser as _;

use crate::bench::recorder::BenchRecorder;
use crate::cli::{GuiArgs, LaunchPlan, RendererChoice, UsageError};
use crate::spike::SpikeApp;

/// Exit code of a runtime I/O or infrastructure failure (docs/specs/m0-foundations.md §17.10):
/// clap's help, version or error text cannot be written, a bench thread cannot be created, or
/// the bench JSON line cannot be written or flushed. `error: …` goes to stderr when possible.
pub const EXIT_RUNTIME: u8 = 1;

/// Exit code of a usage error (clap errors and [`cli::resolve`] errors). Message on stderr.
pub const EXIT_USAGE: u8 = 2;

/// Exit code when the window or the renderer cannot be created (`eframe::run_native` failed).
pub const EXIT_WINDOW: u8 = 3;

/// Exit code when the bench watchdog fires: no measurement within the hard limit.
pub const EXIT_BENCH_TIMEOUT: u8 = 4;

/// Name given to eframe: native window title and default app id.
const APP_NAME: &str = "VisualAssembler";

/// Wayland app id (`xdg_toplevel.set_app_id`) and X11 `WM_CLASS`.
const APP_ID: &str = "visualassembler";

/// Initial inner size of the window, in logical pixels (width, height).
const INITIAL_INNER_SIZE: [f32; 2] = [1280.0, 800.0];

/// Runs the GUI: parses the command line, opens the window, and returns the process exit code.
///
/// `started` is the instant taken by the first statement of `main`; the bench measures
/// `startup_ms` from it.
///
/// Exit codes (docs/specs/m0-foundations.md §17.10): 0 on a normal exit or a completed bench,
/// [`EXIT_RUNTIME`] when clap's message cannot be printed or the bench infrastructure fails
/// (see `bench::recorder`), [`EXIT_USAGE`], [`EXIT_WINDOW`], and [`EXIT_BENCH_TIMEOUT`] (raised
/// by the bench watchdog thread itself).
pub fn run(started: Instant) -> ExitCode {
    let args = match GuiArgs::try_parse() {
        Ok(args) => args,
        Err(err) => return print_command_line_message(&err),
    };

    let plan = match cli::resolve(&args, cli::compiled_renderers()) {
        Ok(plan) => plan,
        Err(err) => {
            report_error(err);
            return ExitCode::from(EXIT_USAGE);
        }
    };

    launch(plan, started)
}

/// Prints clap's help, version or error message and returns the matching exit code.
///
/// `--help` and `--version` go to stdout with clap's code 0; real errors go to stderr with
/// [`EXIT_USAGE`]. When the message cannot be written (for example stdout is a full disk or a
/// closed pipe), the command line was understood but the process failed to do its job, so the
/// code is [`EXIT_RUNTIME`] whatever clap's own code would have been (§17.10). stdout is flushed
/// explicitly so that a write error is seen here rather than lost in the exit-time flush.
fn print_command_line_message(err: &clap::Error) -> ExitCode {
    let printed = err.print().and_then(|()| std::io::stdout().lock().flush());
    match printed {
        Ok(()) => ExitCode::from(u8::try_from(err.exit_code()).unwrap_or(EXIT_USAGE)),
        Err(io_err) => {
            report_error(format_args!(
                "cannot print the command-line message: {io_err}"
            ));
            ExitCode::from(EXIT_RUNTIME)
        }
    }
}

/// Opens the window for a validated plan and runs the event loop until the window closes.
fn launch(plan: LaunchPlan, started: Instant) -> ExitCode {
    let renderer = match eframe_renderer(plan.renderer) {
        Ok(renderer) => renderer,
        Err(err) => {
            report_error(err);
            return ExitCode::from(EXIT_USAGE);
        }
    };

    // The recorder (and its watchdog) starts before the window so that a renderer
    // initialization that hangs is also caught by the watchdog.
    let recorder = match plan.bench {
        Some(bench_plan) => match BenchRecorder::new(bench_plan, started) {
            Ok(recorder) => Some(recorder),
            Err(err) => {
                report_error(format_args!(
                    "cannot start the bench watchdog thread: {err}"
                ));
                return ExitCode::from(EXIT_RUNTIME);
            }
        },
        None => None,
    };

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(spike::WINDOW_TITLE)
            .with_app_id(APP_ID)
            .with_inner_size(INITIAL_INNER_SIZE),
        centered: true,
        renderer,
        ..eframe::NativeOptions::default()
    };
    #[cfg(all(windows, feature = "renderer-wgpu"))]
    let options = pin_windows_wgpu_backend(options);

    let result = eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |_creation_context| Ok(Box::new(SpikeApp::new(recorder)))),
    );

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            report_error(format_args!(
                "cannot create the window or the renderer: {err}"
            ));
            ExitCode::from(EXIT_WINDOW)
        }
    }
}

/// Restricts wgpu to the DX12 backend on Windows unless `WGPU_BACKEND` is set.
///
/// docs/specs/m0-foundations.md §17.13 and docs/perf/M0-backend.md §6.1: wgpu's default pick on
/// the reference machine was Vulkan, which failed the idle gate (G3), while DX12 passed every
/// measured gate with the lower p95 spawn→frame and is the native API on Windows ARM64. When
/// `WGPU_BACKEND` is set, wgpu's own choice from that variable is kept, so every backend stays
/// measurable (`WGPU_BACKEND=vulkan cargo xtask bench …`). The options only matter when the wgpu
/// renderer is the one launched; glow ignores them.
#[cfg(all(windows, feature = "renderer-wgpu"))]
fn pin_windows_wgpu_backend(mut options: eframe::NativeOptions) -> eframe::NativeOptions {
    if std::env::var_os("WGPU_BACKEND").is_none()
        && let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup
    {
        setup.instance_descriptor.backends = eframe::wgpu::Backends::DX12;
    }
    options
}

/// Maps a renderer choice to eframe's renderer.
///
/// [`cli::resolve`] already rejects a renderer that is not compiled in; the error arm only exists
/// in single-renderer builds and keeps that invariant explicit instead of silently falling back.
fn eframe_renderer(choice: RendererChoice) -> Result<eframe::Renderer, UsageError> {
    match choice {
        #[cfg(feature = "renderer-wgpu")]
        RendererChoice::Wgpu => Ok(eframe::Renderer::Wgpu),
        #[cfg(feature = "renderer-glow")]
        RendererChoice::Glow => Ok(eframe::Renderer::Glow),
        #[cfg(not(all(feature = "renderer-wgpu", feature = "renderer-glow")))]
        other => Err(UsageError::RendererNotCompiled(other)),
    }
}

/// Writes `error: <message>` on stderr.
///
/// stderr is the last reporting channel of the process: if writing to it fails there is nowhere
/// left to report that failure, so the write result is deliberately not propagated.
fn report_error(message: impl Display) {
    let mut stderr = std::io::stderr().lock();
    let _ignored = writeln!(stderr, "error: {message}");
}
