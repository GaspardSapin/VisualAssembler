//! `visual_assembler`: library behind the VisualAssembler desktop IDE binary.
//!
//! M0 scope (docs/specs/m0-foundations.md §4): one eframe window drawing a static UI in reactive
//! mode (no continuous repaint), a command line with stable exit codes, and the bench
//! instrumentation that measures startup and idle cost (`--bench startup|idle`).
//!
//! Renderer policy (§19, architecture §9.8): both renderers are compiled in by default; `auto`
//! tries glow then wgpu (wgpu then glow on Windows aarch64) and falls back at most once, in the
//! same process, when the first renderer fails to initialize before the first frame. A forced
//! renderer (`--renderer`, `VA_RENDERER`) never falls back.
//!
//! Modules: [`cli`] parses and validates the command line and holds the pure renderer-selection
//! rules, [`spike`] draws the M0 window, and [`bench`](mod@bench) measures it. [`run`] wires them
//! together for `main.rs`.

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
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use clap::Parser as _;

use crate::bench::platform;
use crate::bench::recorder::{BenchRecorder, SelectionFacts};
use crate::cli::{
    AttemptFailure, GuiArgs, InjectedFailures, LaunchPlan, Renderer, RendererInitError,
    RendererPlan, UsageError, WindowSystemPolicy,
};
use crate::spike::SpikeApp;

/// Exit code of a runtime I/O or infrastructure failure (docs/specs/m0-foundations.md §17.10):
/// clap's help, version or error text cannot be written, a bench thread cannot be created, or
/// the bench JSON line cannot be written or flushed. `error: …` goes to stderr when possible.
pub const EXIT_RUNTIME: u8 = 1;

/// Exit code of a usage error (clap errors, [`cli::resolve`] and [`cli::resolve_renderer`]
/// errors). Message on stderr.
pub const EXIT_USAGE: u8 = 2;

/// Exit code when the window or the renderer cannot be created (§19.4): a platform failure, an
/// error after the first frame, a forced renderer that fails, or `auto` with every renderer
/// failing.
pub const EXIT_WINDOW: u8 = 3;

/// Exit code when the bench watchdog fires: no measurement within the hard limit.
pub const EXIT_BENCH_TIMEOUT: u8 = 4;

/// Name given to eframe: native window title and default app id.
const APP_NAME: &str = "VisualAssembler";

/// Wayland app id (`xdg_toplevel.set_app_id`) and X11 `WM_CLASS`.
const APP_ID: &str = "visualassembler";

/// Initial inner size of the window, in logical pixels (width, height).
const INITIAL_INNER_SIZE: [f32; 2] = [1280.0, 800.0];

/// Everything `launch` needs besides the command line: the renderer plan, the injection hook and
/// the window-system policy, all resolved from the environment before the first attempt.
struct Selection {
    /// Renderer choice, its source and the renderers to try.
    renderer: RendererPlan,
    /// Parsed `VA_TEST_FAIL_RENDERER`.
    injected: InjectedFailures,
    /// X11 under WSL unless `VA_WAYLAND=1`.
    window: WindowSystemPolicy,
}

/// Runs the GUI: parses the command line, opens the window, and returns the process exit code.
///
/// `started` is the instant taken by the first statement of `main`; the bench measures
/// `startup_ms` from it.
///
/// Before any window, it reads `VA_RENDERER`, `VA_TEST_FAIL_RENDERER` and `VA_WAYLAND` and prints
/// their warnings on stderr (docs/specs/m0-foundations.md §19.1, §19.3).
///
/// Exit codes (§17.10, §19.4): 0 on a normal exit or a completed bench (a fallback included),
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

    let selection = match select(&plan) {
        Ok(selection) => selection,
        Err(err) => {
            report_error(err);
            return ExitCode::from(EXIT_USAGE);
        }
    };
    launch(&plan, &selection, started)
}

/// Resolves the renderer selection from `plan` and the environment, and prints its warnings.
///
/// Reads `VA_RENDERER` ([`cli::resolve_renderer`]), `VA_TEST_FAIL_RENDERER`
/// ([`cli::parse_injected_failures`], with the loud startup warning of §19.3) and `VA_WAYLAND`
/// ([`cli::window_system_policy`]). Warnings go to stderr, prefixed `warning: `, in that order.
///
/// # Errors
/// The [`UsageError`] of [`cli::resolve_renderer`] (a forced renderer that is not compiled in).
fn select(plan: &LaunchPlan) -> Result<Selection, UsageError> {
    let env_renderer = std::env::var_os(cli::ENV_RENDERER);
    let resolution = cli::resolve_renderer(
        plan.renderer,
        env_renderer.as_deref(),
        cli::compiled_renderers(),
        std::env::consts::OS,
        std::env::consts::ARCH,
    )?;
    resolution.warnings.iter().for_each(report_warning);

    let env_fail = std::env::var_os(cli::ENV_FAIL_RENDERER);
    let injected = cli::parse_injected_failures(env_fail.as_deref());
    injected.warnings.iter().for_each(report_warning);
    if let Some(warning) = injected.startup_warning() {
        report_warning(warning);
    }

    let env_wayland = std::env::var_os(cli::ENV_WAYLAND);
    let window = cli::window_system_policy(
        platform::is_wsl(),
        env_wayland.as_deref(),
        &resolution.plan.order,
    );
    if let Some(warning) = &window.warning {
        report_warning(warning);
    }

    Ok(Selection {
        renderer: resolution.plan,
        injected,
        window,
    })
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

/// Opens the window and runs the event loop until it closes, trying the renderers of the plan in
/// order (docs/specs/m0-foundations.md §19.2).
///
/// Each attempt calls `eframe::run_native` in this thread with fresh `NativeOptions` and a fresh
/// app creator. eframe keeps its winit event loop in a thread-local and reuses it, so a renderer
/// that failed during initialization leaves the loop ready for the next attempt.
/// [`cli::classify_failure`] decides, after a failure, between one fallback and exit 3.
///
/// The bench recorder (and its watchdog) is created once, before the first attempt, so that a
/// renderer initialization that hangs is also caught by the watchdog and a failed attempt neither
/// drops it nor resets its limit. Each attempt only borrows it.
fn launch(plan: &LaunchPlan, selection: &Selection, started: Instant) -> ExitCode {
    let mut recorder = match plan.bench {
        Some(bench_plan) => {
            let facts = SelectionFacts {
                requested: selection.renderer.requested,
                fallback: None,
                injected_failure: selection.injected.raw.clone(),
            };
            match BenchRecorder::new(bench_plan, started, facts) {
                Ok(recorder) => Some(recorder),
                Err(err) => {
                    report_error(format_args!(
                        "cannot start the bench watchdog thread: {err}"
                    ));
                    return ExitCode::from(EXIT_RUNTIME);
                }
            }
        }
        None => None,
    };

    let mut glow_failed = false;
    for (attempt, &renderer) in selection.renderer.order.iter().enumerate() {
        let options = match native_options(renderer, selection.window.force_x11, glow_failed) {
            Ok(options) => options,
            Err(err) => {
                report_error(err);
                return ExitCode::from(EXIT_USAGE);
            }
        };
        let ui_started = Arc::new(AtomicBool::new(false));
        let creator = app_creator(
            selection.injected.contains(renderer),
            recorder.as_mut(),
            Arc::clone(&ui_started),
        );
        let result = eframe::run_native(APP_NAME, options, creator);
        glow_failed |= renderer == Renderer::Glow;
        let err = match result {
            Ok(()) => return ExitCode::SUCCESS,
            Err(err) => err,
        };

        match cli::classify_failure(
            &selection.renderer,
            attempt,
            &err,
            ui_started.load(Ordering::SeqCst),
        ) {
            AttemptFailure::FallBack {
                next: _,
                warning,
                reason,
            } => {
                report_warning(warning);
                if let Some(recorder) = recorder.as_mut() {
                    recorder.record_fallback(renderer, reason);
                }
            }
            AttemptFailure::Fatal { error, note } => {
                report_error(error);
                if let Some(note) = note {
                    report_note(note);
                }
                return ExitCode::from(EXIT_WINDOW);
            }
        }
    }

    // `classify_failure` never falls back from the last renderer of the plan, and
    // `cli::resolve_renderer` never returns an empty plan, so the loop always returns.
    report_error("no renderer could be initialized (the renderer plan is empty)");
    ExitCode::from(EXIT_WINDOW)
}

/// Builds the app creator of one attempt.
///
/// With `inject` (`VA_TEST_FAIL_RENDERER` lists this attempt's renderer, §19.3), it returns
/// [`RendererInitError::Injected`] instead of the app. eframe calls the creator after the real
/// OpenGL or wgpu initialization, so the injected failure takes the same event-loop path as a
/// genuine one. Otherwise it builds the [`SpikeApp`], which borrows `bench` for the attempt.
fn app_creator<'a>(
    inject: bool,
    bench: Option<&'a mut BenchRecorder>,
    ui_started: Arc<AtomicBool>,
) -> eframe::AppCreator<'a> {
    Box::new(move |_creation_context| {
        if inject {
            return Err(RendererInitError::Injected.into());
        }
        Ok(Box::new(SpikeApp::new(bench, ui_started)))
    })
}

/// Builds fresh eframe options for one renderer attempt (docs/specs/m0-foundations.md §19.2:
/// every attempt gets new options). Public so that tests can check them without a window.
///
/// - Window: title `VisualAssembler`, app id `visualassembler`, 1280×800 logical px, centered.
/// - The Windows DX12 pin applies to every wgpu attempt, forced or fallback (§19.1), unless
///   `WGPU_BACKEND` is effective (see `wgpu_backend_env_is_effective`).
/// - `force_x11` installs the X11 event-loop hook (Linux only; see [`apply_window_system`]).
/// - `after_glow_failure` (a glow attempt already failed in this process) removes wgpu's GL
///   backend unless `WGPU_BACKEND` is effective; see [`exclude_wgpu_gl_backend`].
///
/// # Errors
/// [`UsageError::RendererNotCompiled`] when `renderer` is not compiled into this binary (only
/// possible in single-renderer builds; `cli::resolve_renderer` never plans such a renderer).
pub fn native_options(
    renderer: Renderer,
    force_x11: bool,
    // A single-renderer build never falls back, so it has nothing to do with this flag.
    #[cfg_attr(
        not(all(feature = "renderer-glow", feature = "renderer-wgpu")),
        allow(unused_variables)
    )]
    after_glow_failure: bool,
) -> Result<eframe::NativeOptions, UsageError> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(spike::WINDOW_TITLE)
            .with_app_id(APP_ID)
            .with_inner_size(INITIAL_INNER_SIZE),
        centered: true,
        renderer: eframe_renderer(renderer)?,
        ..eframe::NativeOptions::default()
    };
    #[cfg(all(windows, feature = "renderer-wgpu"))]
    let options = pin_windows_wgpu_backend(options);
    #[cfg(all(feature = "renderer-glow", feature = "renderer-wgpu"))]
    let options = if after_glow_failure {
        exclude_wgpu_gl_backend(options)
    } else {
        options
    };
    Ok(apply_window_system(options, force_x11))
}

/// True when `raw`, the value of `WGPU_BACKEND` (`None` when unset), is **effective**: wgpu itself
/// takes a non-empty backend set from it (docs/specs/m0-foundations.md §19.9 S19-4).
///
/// That is, `raw` is valid Unicode (wgpu-types 30.0.1 reads the variable with
/// `std::env::var(..).ok()`, so a non-Unicode value is ignored there) and
/// `wgpu::Backends::from_comma_list` of it is not empty. An empty, blank, unrecognized or
/// non-Unicode value is not effective, so it neither drops the Windows DX12 pin nor brings wgpu's
/// GL backend back after a glow failure. Public so that tests can check it without a window.
#[cfg(feature = "renderer-wgpu")]
pub fn wgpu_backend_env_is_effective(raw: Option<&std::ffi::OsStr>) -> bool {
    raw.and_then(std::ffi::OsStr::to_str)
        .is_some_and(|value| !eframe::wgpu::Backends::from_comma_list(value).is_empty())
}

/// Removes wgpu's GL backend from the options, unless `WGPU_BACKEND` is effective
/// ([`wgpu_backend_env_is_effective`]): an empty, blank, unrecognized or non-Unicode value keeps
/// GL out.
///
/// Used for a wgpu attempt that follows a failed glow attempt. glutin destroys the GL context of
/// a failed glow attempt without unbinding it, so that context stays current on the main thread.
/// wgpu's GL backend then creates its own EGL context on that thread, and wgpu-hal 30.0.1 panics
/// on the `EGL_BAD_ACCESS` of `eglMakeCurrent` (observed under WSLg X11, where glow uses GLX,
/// with the §19.3 injected failure; with the GL backend left out the same fallback succeeds on
/// Vulkan). The same situation follows a genuine failure after context creation, such as an
/// OpenGL version that egui_glow rejects. The GL backend would rarely help there anyway: it uses
/// the OpenGL driver that has just failed. On Windows the DX12 pin has already left GL out.
#[cfg(all(feature = "renderer-glow", feature = "renderer-wgpu"))]
fn exclude_wgpu_gl_backend(mut options: eframe::NativeOptions) -> eframe::NativeOptions {
    if !wgpu_backend_env_is_effective(std::env::var_os("WGPU_BACKEND").as_deref())
        && let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup
    {
        setup
            .instance_descriptor
            .backends
            .remove(eframe::wgpu::Backends::GL);
    }
    options
}

/// Makes winit use X11 when `force_x11` is true (docs/specs/m0-foundations.md §19.1: under WSL,
/// unless `VA_WAYLAND=1`).
///
/// The hook is installed on every attempt: eframe calls it only when it creates its thread-local
/// event loop, which happens on the first attempt and is skipped by later ones, so the first
/// attempt fixes the window system of the whole process.
#[cfg(target_os = "linux")]
fn apply_window_system(
    mut options: eframe::NativeOptions,
    force_x11: bool,
) -> eframe::NativeOptions {
    use winit::platform::x11::EventLoopBuilderExtX11 as _;

    if force_x11 {
        options.event_loop_builder = Some(Box::new(|builder| {
            builder.with_x11();
        }));
    }
    options
}

/// winit's X11 backend exists only on Linux, and [`cli::window_system_policy`] forces X11 only
/// under WSL, so other operating systems keep the options unchanged.
#[cfg(not(target_os = "linux"))]
fn apply_window_system(options: eframe::NativeOptions, _force_x11: bool) -> eframe::NativeOptions {
    options
}

/// Restricts wgpu to the DX12 backend on Windows unless `WGPU_BACKEND` is effective
/// ([`wgpu_backend_env_is_effective`]).
///
/// docs/specs/m0-foundations.md §17.13 and docs/perf/M0-backend.md §6.1: wgpu's default pick on
/// the reference machine was Vulkan, which failed the idle gate (G3), while DX12 passed every
/// measured gate with the lower p95 spawn→frame and is the native API on Windows ARM64. When
/// `WGPU_BACKEND` is effective, wgpu's own choice from that variable is kept, so every backend
/// stays measurable (`WGPU_BACKEND=vulkan cargo xtask bench …`); an empty, blank, unrecognized or
/// non-Unicode value keeps the pin (§19.9 S19-4). The options only matter when the wgpu renderer
/// is the one launched; glow ignores them. §19.1 keeps the pin for every wgpu use, forced or
/// fallback.
#[cfg(all(windows, feature = "renderer-wgpu"))]
fn pin_windows_wgpu_backend(mut options: eframe::NativeOptions) -> eframe::NativeOptions {
    if !wgpu_backend_env_is_effective(std::env::var_os("WGPU_BACKEND").as_deref())
        && let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup
    {
        setup.instance_descriptor.backends = eframe::wgpu::Backends::DX12;
    }
    options
}

/// Maps a renderer to eframe's renderer.
///
/// [`cli::resolve_renderer`] only plans compiled renderers; the error arm only exists in
/// single-renderer builds and keeps that invariant explicit instead of silently switching.
fn eframe_renderer(renderer: Renderer) -> Result<eframe::Renderer, UsageError> {
    match renderer {
        #[cfg(feature = "renderer-wgpu")]
        Renderer::Wgpu => Ok(eframe::Renderer::Wgpu),
        #[cfg(feature = "renderer-glow")]
        Renderer::Glow => Ok(eframe::Renderer::Glow),
        #[cfg(not(all(feature = "renderer-wgpu", feature = "renderer-glow")))]
        other => Err(UsageError::RendererNotCompiled(other)),
    }
}

/// Writes `error: <message>` on stderr.
///
/// stderr is the last reporting channel of the process: if writing to it fails there is nowhere
/// left to report that failure, so the write result is deliberately not propagated.
fn report_error(message: impl Display) {
    write_stderr_line("error", message);
}

/// Writes `warning: <message>` on stderr (same failure policy as [`report_error`]).
fn report_warning(message: impl Display) {
    write_stderr_line("warning", message);
}

/// Writes `note: <message>` on stderr (same failure policy as [`report_error`]).
fn report_note(message: impl Display) {
    write_stderr_line("note", message);
}

/// Writes `<prefix>: <message>` on stderr; a write failure is deliberately ignored (see
/// [`report_error`]).
fn write_stderr_line(prefix: &str, message: impl Display) {
    let mut stderr = std::io::stderr().lock();
    let _ignored = writeln!(stderr, "{prefix}: {message}");
}
