//! Command line of the GUI binary (docs/specs/m0-foundations.md §4.3).
//!
//! ```text
//! visual-assembler [--renderer <wgpu|glow>] [--bench <startup|idle>] [--duration-secs <N>]
//! visual-assembler --help | --version
//! ```
//!
//! Parsing is done by clap ([`GuiArgs`]); the rules that depend on the build or on several flags
//! together live in [`resolve`], which turns the arguments into a [`LaunchPlan`].

use std::fmt;
use std::time::Duration;

use clap::{Parser, ValueEnum};

use crate::bench::DEFAULT_IDLE_SECS;

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
    /// Renderer to use [default: wgpu if compiled in, else glow]
    #[arg(long, value_enum)]
    pub renderer: Option<RendererChoice>,

    /// Run a measurement scenario, print one JSON line on stdout, then exit
    #[arg(long, value_enum)]
    pub bench: Option<BenchScenario>,

    /// Length of the idle measurement window in seconds, 1 to 3600 [default: 10] (only with --bench idle)
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u64).range(1..=3600))]
    pub duration_secs: Option<u64>,
}

/// A renderer that the binary may be built with. Present whatever the enabled features.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
pub enum RendererChoice {
    /// wgpu (DX12, Vulkan, Metal or GL backend), feature `renderer-wgpu`.
    Wgpu,
    /// glow (OpenGL through glutin), feature `renderer-glow`.
    Glow,
}

impl RendererChoice {
    /// Lowercase name used on the command line, in feature names and in bench JSON lines.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wgpu => "wgpu",
            Self::Glow => "glow",
        }
    }
}

impl fmt::Display for RendererChoice {
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

/// Validated launch parameters, produced by [`resolve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaunchPlan {
    /// Renderer to start eframe with; always one that is compiled into the binary.
    pub renderer: RendererChoice,
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

/// A command-line combination that clap accepts but the binary cannot honour (exit code 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageError {
    /// The requested renderer (explicit, or the default) is not compiled into this binary.
    RendererNotCompiled(RendererChoice),
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

/// Renderers compiled into this binary, in order of preference (wgpu first).
const COMPILED_RENDERERS: &[RendererChoice] = &[
    #[cfg(feature = "renderer-wgpu")]
    RendererChoice::Wgpu,
    #[cfg(feature = "renderer-glow")]
    RendererChoice::Glow,
];

/// Returns the renderers compiled into this binary, in order of preference (wgpu first).
pub fn compiled_renderers() -> &'static [RendererChoice] {
    COMPILED_RENDERERS
}

/// Validates parsed arguments against the renderers in `compiled` and builds the launch plan.
///
/// Rules (docs/specs/m0-foundations.md §4.3):
/// - `--duration-secs` is only valid with `--bench idle`;
/// - the renderer defaults to wgpu when it is in `compiled`, else to glow;
/// - a renderer that is not in `compiled` (explicit or default) is an error.
///
/// # Errors
/// Returns the [`UsageError`] that the binary reports with exit code 2.
pub fn resolve(args: &GuiArgs, compiled: &[RendererChoice]) -> Result<LaunchPlan, UsageError> {
    if args.duration_secs.is_some() && args.bench != Some(BenchScenario::Idle) {
        return Err(UsageError::DurationWithoutIdle);
    }

    let renderer = match args.renderer {
        Some(requested) => requested,
        None if compiled.contains(&RendererChoice::Wgpu) => RendererChoice::Wgpu,
        None => RendererChoice::Glow,
    };
    if !compiled.contains(&renderer) {
        return Err(UsageError::RendererNotCompiled(renderer));
    }

    let bench = args.bench.map(|scenario| BenchPlan {
        scenario,
        idle_duration: Duration::from_secs(args.duration_secs.unwrap_or(DEFAULT_IDLE_SECS)),
    });

    Ok(LaunchPlan { renderer, bench })
}
