//! Command line and renderer selection of the GUI binary (docs/specs/m0-foundations.md §4.3,
//! amended by §17.10 and §19).
//!
//! ```text
//! visual-assembler [--renderer <auto|glow|wgpu>] [--bench <startup|idle>] [--duration-secs <N>]
//! visual-assembler --help | --version
//! ```
//!
//! Parsing is done by clap ([`GuiArgs`]); the rules that depend on the build or on several flags
//! together live in [`resolve`], which turns the arguments into a [`LaunchPlan`].
//!
//! Renderer selection (§19.1, architecture §6 and §9.8) is a set of pure functions, so every rule
//! is testable without a window:
//! - [`resolve_renderer`]: precedence `--renderer` > `VA_RENDERER` > `settings.toml` (M4) >
//!   built-in `auto`, and the per-target order of `auto` ([`auto_order`]);
//! - [`parse_injected_failures`]: the `VA_TEST_FAIL_RENDERER` test hook (§19.3);
//! - [`window_system_policy`]: X11 under WSL unless `VA_WAYLAND=1`;
//! - [`is_fallback_eligible`] and [`classify_failure`]: what a failed renderer attempt leads to,
//!   with the exact messages of §19.4.

use std::ffi::OsStr;
use std::fmt;
use std::time::Duration;

use clap::{Parser, ValueEnum};

use crate::bench::DEFAULT_IDLE_SECS;

/// Environment variable that chooses the renderer when `--renderer` is absent (§19.1).
pub const ENV_RENDERER: &str = "VA_RENDERER";

/// Environment variable of the renderer failure injection hook (§19.3).
pub const ENV_FAIL_RENDERER: &str = "VA_TEST_FAIL_RENDERER";

/// Environment variable that keeps winit's Wayland backend under WSL when it is exactly `1`.
pub const ENV_WAYLAND: &str = "VA_WAYLAND";

/// Raw command-line arguments of `visual-assembler`, as parsed by clap.
#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(
    name = "visual-assembler",
    bin_name = "visual-assembler",
    version,
    about = "VisualAssembler desktop IDE for learning assembly language.",
    long_about = None
)]
pub struct GuiArgs {
    /// Renderer to use [default: $VA_RENDERER, else auto]
    #[arg(long, value_enum)]
    pub renderer: Option<RendererChoice>,

    /// Run a measurement scenario, print one JSON line on stdout, then exit
    #[arg(long, value_enum)]
    pub bench: Option<BenchScenario>,

    /// Length of the idle measurement window in seconds, 1 to 3600 [default: 10] (only with --bench idle)
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u64).range(1..=3600))]
    pub duration_secs: Option<u64>,
}

/// A renderer choice: `auto` or a forced renderer (§19.1). Present whatever the enabled features.
///
/// The variant docs are user-facing: clap prints them in `--help` (see `auto_order` for the rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
pub enum RendererChoice {
    /// glow, then wgpu if glow fails to start (wgpu first on Windows ARM64)
    Auto,
    /// OpenGL through glow only, no fallback
    Glow,
    /// wgpu only (DX12 on Windows unless WGPU_BACKEND is set), no fallback
    Wgpu,
}

impl RendererChoice {
    /// Lowercase name used on the command line, in `VA_RENDERER` and in bench JSON lines.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Glow => "glow",
            Self::Wgpu => "wgpu",
        }
    }

    /// Parses an exact lowercase name (`auto`, `glow`, `wgpu`), as clap does for `--renderer`.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "auto" => Some(Self::Auto),
            "glow" => Some(Self::Glow),
            "wgpu" => Some(Self::Wgpu),
            _ => None,
        }
    }

    /// The forced renderer, or `None` for [`RendererChoice::Auto`].
    pub fn forced(self) -> Option<Renderer> {
        match self {
            Self::Auto => None,
            Self::Glow => Some(Renderer::Glow),
            Self::Wgpu => Some(Renderer::Wgpu),
        }
    }
}

impl fmt::Display for RendererChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A concrete renderer that the binary may be built with. Present whatever the enabled features.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Renderer {
    /// glow (OpenGL through glutin), feature `renderer-glow`.
    Glow,
    /// wgpu (DX12, Vulkan, Metal or GL backend), feature `renderer-wgpu`.
    Wgpu,
}

impl Renderer {
    /// Lowercase name used in feature names, messages and bench JSON lines.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Glow => "glow",
            Self::Wgpu => "wgpu",
        }
    }
}

impl fmt::Display for Renderer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A bench scenario (docs/specs/m0-foundations.md §4.5). Present whatever the enabled features.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
pub enum BenchScenario {
    /// Time from `main()` to the first presented frame, then exit.
    Startup,
    /// Frames and CPU time during an idle window after the first frame, then exit.
    Idle,
}

impl BenchScenario {
    /// Lowercase name used on the command line and in bench JSON lines.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Idle => "idle",
        }
    }
}

impl fmt::Display for BenchScenario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Validated command-line parameters, produced by [`resolve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaunchPlan {
    /// The `--renderer` value, if given; always compiled in when forced. `None` defers to the
    /// next precedence level ([`resolve_renderer`]).
    pub renderer: Option<RendererChoice>,
    /// Bench scenario to run, or `None` for a normal interactive session.
    pub bench: Option<BenchPlan>,
}

/// A bench run requested on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchPlan {
    /// Scenario to measure.
    pub scenario: BenchScenario,
    /// Length of the idle measurement window. Only used by [`BenchScenario::Idle`]; it holds
    /// the default length for the startup scenario.
    pub idle_duration: Duration,
}

/// A combination that clap accepts but the binary cannot honour (exit code 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageError {
    /// The requested renderer (from `--renderer` or `VA_RENDERER`, or the first renderer of the
    /// `auto` order when none is compiled) is not compiled into this binary.
    RendererNotCompiled(Renderer),
    /// `--duration-secs` was given without `--bench idle`.
    DurationWithoutIdle,
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RendererNotCompiled(renderer) => write!(
                f,
                "renderer '{renderer}' is not compiled into this binary (build with --features renderer-{renderer})"
            ),
            Self::DurationWithoutIdle => f.write_str("--duration-secs requires --bench idle"),
        }
    }
}

impl std::error::Error for UsageError {}

/// Renderers compiled into this binary. Membership only: the order of `auto` is [`auto_order`].
const COMPILED_RENDERERS: &[Renderer] = &[
    #[cfg(feature = "renderer-glow")]
    Renderer::Glow,
    #[cfg(feature = "renderer-wgpu")]
    Renderer::Wgpu,
];

/// Returns the renderers compiled into this binary (membership only; see [`auto_order`]).
pub fn compiled_renderers() -> &'static [Renderer] {
    COMPILED_RENDERERS
}

/// Validates parsed arguments against the renderers in `compiled` and builds the launch plan.
///
/// Rules (docs/specs/m0-foundations.md §4.3, §19.1):
/// - `--duration-secs` is only valid with `--bench idle`;
/// - a forced `--renderer` that is not in `compiled` is an error;
/// - an empty `compiled` is an error (a real build has at least one renderer: `compile_error!`
///   in `lib.rs`); it reports glow, the first renderer of `auto` on most targets.
///
/// An absent `--renderer` stays `None`: [`resolve_renderer`] applies the next levels.
///
/// # Errors
/// Returns the [`UsageError`] that the binary reports with exit code 2.
pub fn resolve(args: &GuiArgs, compiled: &[Renderer]) -> Result<LaunchPlan, UsageError> {
    if args.duration_secs.is_some() && args.bench != Some(BenchScenario::Idle) {
        return Err(UsageError::DurationWithoutIdle);
    }

    match args.renderer.and_then(RendererChoice::forced) {
        Some(forced) if !compiled.contains(&forced) => {
            return Err(UsageError::RendererNotCompiled(forced));
        }
        None if compiled.is_empty() => return Err(UsageError::RendererNotCompiled(Renderer::Glow)),
        _ => {}
    }

    let bench = args.bench.map(|scenario| BenchPlan {
        scenario,
        idle_duration: Duration::from_secs(args.duration_secs.unwrap_or(DEFAULT_IDLE_SECS)),
    });

    Ok(LaunchPlan {
        renderer: args.renderer,
        bench,
    })
}

/// Where the resolved renderer choice comes from (§19.1). M4 adds the `settings.toml` level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChoiceSource {
    /// The `--renderer` flag.
    Cli,
    /// The `VA_RENDERER` environment variable.
    Env,
    /// Neither: the built-in `auto`.
    Default,
}

impl ChoiceSource {
    /// How messages name the source: `--renderer`, `VA_RENDERER` or `default`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "--renderer",
            Self::Env => ENV_RENDERER,
            Self::Default => "default",
        }
    }
}

impl fmt::Display for ChoiceSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The resolved renderer selection: the choice, its source, and the renderers to try in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RendererPlan {
    /// The resolved choice, reported as `renderer_requested` in bench lines.
    pub requested: RendererChoice,
    /// Where [`RendererPlan::requested`] comes from.
    pub source: ChoiceSource,
    /// Renderers to try, in order: one entry when forced, the compiled part of [`auto_order`]
    /// for `auto`. Never empty.
    pub order: Vec<Renderer>,
}

/// Output of [`resolve_renderer`]: the plan, plus the warnings to print on stderr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RendererResolution {
    /// The selection to launch.
    pub plan: RendererPlan,
    /// Warning texts without the `warning: ` prefix (an ignored invalid `VA_RENDERER`).
    pub warnings: Vec<String>,
}

/// Returns the order of `auto` for a target, as named by `std::env::consts::{OS, ARCH}`.
///
/// Windows aarch64: wgpu, then glow (DX12 is native there; OpenGL needs an optional pack).
/// Every other target: glow, then wgpu (§19.1, architecture §9.8).
pub fn auto_order(os: &str, arch: &str) -> [Renderer; 2] {
    if os == "windows" && arch == "aarch64" {
        [Renderer::Wgpu, Renderer::Glow]
    } else {
        [Renderer::Glow, Renderer::Wgpu]
    }
}

/// Resolves the renderer selection (docs/specs/m0-foundations.md §19.1). Pure.
///
/// - `cli`: the `--renderer` value; `env`: the raw `VA_RENDERER` value (`None` when unset);
/// - `compiled`: the renderers of the build; `os`/`arch`: the target (`std::env::consts`).
///
/// Precedence: `--renderer` > `VA_RENDERER` > built-in `auto` (the `settings.toml` level comes
/// in M4, between the variable and `auto`). An invalid `VA_RENDERER` (anything but exactly
/// `auto`, `glow` or `wgpu`, including an empty or non-Unicode value) adds the warning
/// `ignoring VA_RENDERER='<v>' (expected auto, glow or wgpu)` and is ignored, even when
/// `--renderer` overrides it. `<v>` is the lossy text of the value through `str::escape_debug`
/// (§19.9 S19-2), so a CR, newline or escape sequence shows as `\r`, `\n` or `\u{1b}` instead of
/// breaking the line or driving the terminal. In a single-renderer build `auto` is reduced to
/// that renderer.
///
/// # Errors
/// [`UsageError::RendererNotCompiled`] when the forced renderer is not in `compiled`, or when
/// `compiled` is empty (the first renderer of the `auto` order is reported).
pub fn resolve_renderer(
    cli: Option<RendererChoice>,
    env: Option<&OsStr>,
    compiled: &[Renderer],
    os: &str,
    arch: &str,
) -> Result<RendererResolution, UsageError> {
    let mut warnings = Vec::new();
    let env_choice = env.and_then(|raw| {
        let parsed = raw.to_str().and_then(RendererChoice::from_name);
        if parsed.is_none() {
            warnings.push(format!(
                "ignoring {ENV_RENDERER}='{}' (expected auto, glow or wgpu)",
                raw.to_string_lossy().escape_debug()
            ));
        }
        parsed
    });

    let (requested, source) = match (cli, env_choice) {
        (Some(choice), _) => (choice, ChoiceSource::Cli),
        (None, Some(choice)) => (choice, ChoiceSource::Env),
        (None, None) => (RendererChoice::Auto, ChoiceSource::Default),
    };

    let order = match requested.forced() {
        Some(forced) if compiled.contains(&forced) => vec![forced],
        Some(forced) => return Err(UsageError::RendererNotCompiled(forced)),
        None => {
            let preferred = auto_order(os, arch);
            let order: Vec<Renderer> = preferred
                .into_iter()
                .filter(|renderer| compiled.contains(renderer))
                .collect();
            if order.is_empty() {
                return Err(UsageError::RendererNotCompiled(preferred[0]));
            }
            order
        }
    };

    Ok(RendererResolution {
        plan: RendererPlan {
            requested,
            source,
            order,
        },
        warnings,
    })
}

/// Parsed `VA_TEST_FAIL_RENDERER` (docs/specs/m0-foundations.md §19.3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InjectedFailures {
    /// Renderers whose app creator must fail, without duplicates, in [`Renderer`] order.
    pub renderers: Vec<Renderer>,
    /// The raw value when the variable is set (lossy if not Unicode), reported as
    /// `injected_failure` in bench lines. Never escaped: serde_json escapes it in the bench line,
    /// while every stderr warning echoes it through `str::escape_debug` (§19.9 S19-2).
    pub raw: Option<String>,
    /// Warning texts without the `warning: ` prefix (ignored values).
    pub warnings: Vec<String>,
}

impl InjectedFailures {
    /// True when `renderer`'s app creator must return the injected failure.
    pub fn contains(&self, renderer: Renderer) -> bool {
        self.renderers.contains(&renderer)
    }

    /// The loud startup warning (without the `warning: ` prefix), when a failure is simulated.
    /// It quotes [`InjectedFailures::raw`] through `str::escape_debug` (§19.9 S19-2).
    pub fn startup_warning(&self) -> Option<String> {
        if self.renderers.is_empty() {
            return None;
        }
        let raw = self.raw.as_deref().unwrap_or_default().escape_debug();
        Some(format!(
            "{ENV_FAIL_RENDERER}={raw} is set: simulating a renderer initialization failure (test hook)"
        ))
    }
}

/// Parses the raw `VA_TEST_FAIL_RENDERER` value (`None` when unset). Pure.
///
/// The value is a comma-separated list of `glow` and `wgpu`. Items are trimmed; empty items are
/// skipped silently; any other item adds the warning
/// `ignoring unknown VA_TEST_FAIL_RENDERER value '<v>' (expected glow or wgpu)`. A non-Unicode
/// value is ignored as a whole with one warning. Every echoed value (the trimmed item, or the
/// lossy text of a non-Unicode value) goes through `str::escape_debug` (§19.9 S19-2).
pub fn parse_injected_failures(raw: Option<&OsStr>) -> InjectedFailures {
    let Some(raw) = raw else {
        return InjectedFailures::default();
    };
    let mut failures = InjectedFailures {
        raw: Some(raw.to_string_lossy().into_owned()),
        ..InjectedFailures::default()
    };
    let Some(text) = raw.to_str() else {
        failures.warnings.push(format!(
            "ignoring {ENV_FAIL_RENDERER}='{}' (not valid Unicode; expected glow, wgpu or glow,wgpu)",
            raw.to_string_lossy().escape_debug()
        ));
        return failures;
    };
    for item in text
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        match RendererChoice::from_name(item).and_then(RendererChoice::forced) {
            Some(renderer) => {
                if !failures.renderers.contains(&renderer) {
                    failures.renderers.push(renderer);
                }
            }
            None => failures.warnings.push(format!(
                "ignoring unknown {ENV_FAIL_RENDERER} value '{}' (expected glow or wgpu)",
                item.escape_debug()
            )),
        }
    }
    failures.renderers.sort_unstable();
    failures
}

/// Window-system decision for a launch (§19.1, "WSL window system").
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowSystemPolicy {
    /// True when winit must use X11 (`EventLoopBuilderExtX11::with_x11`) for every attempt.
    pub force_x11: bool,
    /// Warning text without the `warning: ` prefix, for `VA_WAYLAND=1` with wgpu under WSL.
    pub warning: Option<String>,
}

/// Decides the window system. Pure.
///
/// Under WSL (`is_wsl`), every renderer uses X11 unless `VA_WAYLAND` is exactly `1`; with
/// `VA_WAYLAND=1` and a plan that includes wgpu, the warning
/// `VA_WAYLAND=1 with the wgpu renderer can crash the WSLg compositor (weston 9)` is returned.
/// Outside WSL nothing is forced and `VA_WAYLAND` has no effect (native Linux keeps winit's
/// default, Wayland first).
pub fn window_system_policy(
    is_wsl: bool,
    va_wayland: Option<&OsStr>,
    order: &[Renderer],
) -> WindowSystemPolicy {
    if !is_wsl {
        return WindowSystemPolicy::default();
    }
    if va_wayland != Some(OsStr::new("1")) {
        return WindowSystemPolicy {
            force_x11: true,
            warning: None,
        };
    }
    let warning = order.contains(&Renderer::Wgpu).then(|| {
        format!("{ENV_WAYLAND}=1 with the wgpu renderer can crash the WSLg compositor (weston 9)")
    });
    WindowSystemPolicy {
        force_x11: false,
        warning,
    }
}

/// A renderer initialization failure raised by our own app creator (§19.2, §19.3).
///
/// eframe reports it as `eframe::Error::AppCreation`, which [`is_fallback_eligible`] accepts only
/// when it carries this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererInitError {
    /// `VA_TEST_FAIL_RENDERER` lists the renderer being created (the §19.3 test hook).
    Injected,
}

impl fmt::Display for RendererInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Injected => write!(f, "injected renderer failure ({ENV_FAIL_RENDERER})"),
        }
    }
}

impl std::error::Error for RendererInitError {}

/// True when `err` is a renderer initialization error (§19.2, condition 3), whenever it happened.
///
/// glow: `Glutin`, `NoGlutinConfigs`, `OpenGL`; wgpu: `Wgpu`; both: `AppCreation` carrying a
/// [`RendererInitError`]. Never `Winit`/`WinitEventLoop` (window system or display failures,
/// which the other renderer would hit too) nor any other `AppCreation` error.
pub fn is_renderer_init_error(err: &eframe::Error) -> bool {
    match err {
        eframe::Error::AppCreation(inner) => inner.downcast_ref::<RendererInitError>().is_some(),
        eframe::Error::Winit(_) | eframe::Error::WinitEventLoop(_) => false,
        #[cfg(feature = "renderer-glow")]
        eframe::Error::Glutin(_)
        | eframe::Error::NoGlutinConfigs(..)
        | eframe::Error::OpenGL(_) => true,
        #[cfg(feature = "renderer-wgpu")]
        eframe::Error::Wgpu(_) => true,
    }
}

/// True when a failed attempt may fall back to the next renderer, the plan and the next renderer
/// aside (§19.2, conditions 2 and 3): `err` is a renderer initialization error
/// ([`is_renderer_init_error`]) and the first frame never started (`ui_started` is false: the
/// app's `ui` never ran).
pub fn is_fallback_eligible(err: &eframe::Error, ui_started: bool) -> bool {
    !ui_started && is_renderer_init_error(err)
}

/// Describes a renderer failure for messages: the inner text of an `AppCreation` error that
/// carries a [`RendererInitError`] (e.g. `injected renderer failure (VA_TEST_FAIL_RENDERER)`),
/// eframe's own text otherwise.
pub fn describe_failure(err: &eframe::Error) -> String {
    if let eframe::Error::AppCreation(inner) = err
        && let Some(init) = inner.downcast_ref::<RendererInitError>()
    {
        return init.to_string();
    }
    err.to_string()
}

/// What `launch` does after attempt `attempt` of a plan failed ([`classify_failure`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptFailure {
    /// Try `next`. Print `warning` on stderr (prefixed `warning: `); the bench line reports
    /// `reason` as `fallback_reason`.
    FallBack {
        /// The renderer of the next attempt.
        next: Renderer,
        /// The §19.4 fallback warning, without the prefix.
        warning: String,
        /// The first line of the failure description.
        reason: String,
    },
    /// Stop with exit code 3. Print `error` (prefixed `error: `), then `note` if any (prefixed
    /// `note: `).
    Fatal {
        /// The §19.4 error text, without the prefix.
        error: String,
        /// The §19.4 note, without the prefix.
        note: Option<String>,
    },
}

/// Classifies the failure of attempt number `attempt` (0-based index into `plan.order`), with the
/// exact texts of docs/specs/m0-foundations.md §19.4. Pure.
///
/// - `auto`, a next renderer left, [`is_fallback_eligible`]: fall back;
/// - `auto`, last renderer, renderer initialization error before the first frame:
///   `no renderer could be initialized (tried glow, then wgpu): <error>` plus the README note;
/// - forced, same kind of error: `the <r> renderer failed to initialize: <error> (forced by
///   <--renderer|VA_RENDERER>; try --renderer auto)`;
/// - anything else (platform failure, error after the first frame):
///   `cannot create the window or the renderer: <error>` (unchanged since §4.3).
pub fn classify_failure(
    plan: &RendererPlan,
    attempt: usize,
    err: &eframe::Error,
    ui_started: bool,
) -> AttemptFailure {
    let description = describe_failure(err);
    let failed = plan.order.get(attempt).copied();
    let next = plan.order.get(attempt + 1).copied();
    let auto = plan.requested == RendererChoice::Auto;
    let eligible = is_fallback_eligible(err, ui_started);

    match (failed, next) {
        (Some(failed), Some(next)) if auto && eligible => AttemptFailure::FallBack {
            next,
            warning: format!(
                "the {failed} renderer failed to initialize: {description}; falling back to {next}"
            ),
            reason: description.lines().next().unwrap_or_default().to_owned(),
        },
        (Some(_), None) if auto && eligible => {
            let tried: Vec<&str> = plan
                .order
                .iter()
                .map(|renderer| renderer.as_str())
                .collect();
            AttemptFailure::Fatal {
                error: format!(
                    "no renderer could be initialized (tried {}): {description}",
                    tried.join(", then ")
                ),
                note: Some("see the README section \"Renderer selection\"".to_owned()),
            }
        }
        (Some(failed), _) if !auto && eligible => AttemptFailure::Fatal {
            error: format!(
                "the {failed} renderer failed to initialize: {description} (forced by {}; try --renderer auto)",
                plan.source
            ),
            note: None,
        },
        _ => AttemptFailure::Fatal {
            error: format!("cannot create the window or the renderer: {err}"),
            note: None,
        },
    }
}
