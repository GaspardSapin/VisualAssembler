//! Covers `src/cli.rs`: parsing with `GuiArgs::try_parse_from`, validation with `resolve`
//! (docs/specs/m0-foundations.md §4.3, §11), and the pure renderer-selection rules of §19
//! (`resolve_renderer`, `auto_order`, `parse_injected_failures`, `window_system_policy`,
//! `is_fallback_eligible`, `classify_failure`). No window, no GPU.
#![allow(clippy::unwrap_used)]

use std::ffi::OsStr;
use std::time::Duration;

use clap::Parser as _;
use visual_assembler::cli::{
    self, AttemptFailure, BenchPlan, BenchScenario, ChoiceSource, GuiArgs, LaunchPlan, Renderer,
    RendererChoice, RendererInitError, RendererPlan, UsageError,
};

/// Both renderers compiled in.
const BOTH: &[Renderer] = &[Renderer::Glow, Renderer::Wgpu];

/// Parses a command line (without the program name).
fn parse(args: &[&str]) -> GuiArgs {
    GuiArgs::try_parse_from(std::iter::once("visual-assembler").chain(args.iter().copied()))
        .unwrap()
}

/// §19.1 replaced "default wgpu when compiled": an absent `--renderer` defers to the next
/// precedence level, which ends at `auto` (glow first outside Windows aarch64).
#[test]
fn default_renderer_defers_to_auto() {
    let plan = cli::resolve(&parse(&[]), BOTH).unwrap();
    assert_eq!(
        plan,
        LaunchPlan {
            renderer: None,
            bench: None
        }
    );
    let resolution = cli::resolve_renderer(plan.renderer, None, BOTH, "windows", "x86_64").unwrap();
    assert_eq!(resolution.plan.requested, RendererChoice::Auto);
    assert_eq!(resolution.plan.order, [Renderer::Glow, Renderer::Wgpu]);
}

/// §19.1: in a single-renderer build `auto` is reduced to the compiled renderer.
#[test]
fn default_renderer_is_glow_without_wgpu() {
    let plan = cli::resolve(&parse(&[]), &[Renderer::Glow]).unwrap();
    assert_eq!(plan.renderer, None);
    let resolution =
        cli::resolve_renderer(plan.renderer, None, &[Renderer::Glow], "linux", "x86_64").unwrap();
    assert_eq!(resolution.plan.order, [Renderer::Glow]);
}

#[test]
fn explicit_renderer_is_kept() {
    let plan = cli::resolve(&parse(&["--renderer", "glow"]), BOTH).unwrap();
    assert_eq!(plan.renderer, Some(RendererChoice::Glow));
}

#[test]
fn renderer_not_compiled_is_an_error() {
    let err = cli::resolve(&parse(&["--renderer", "glow"]), &[Renderer::Wgpu]).unwrap_err();
    assert_eq!(err, UsageError::RendererNotCompiled(Renderer::Glow));
    let message = err.to_string();
    assert!(message.contains("not compiled"), "{message}");
    assert_eq!(
        message,
        "renderer 'glow' is not compiled into this binary (build with --features renderer-glow)"
    );
}

#[test]
fn bench_idle_defaults_to_ten_seconds() {
    let plan = cli::resolve(&parse(&["--bench", "idle"]), BOTH).unwrap();
    assert_eq!(
        plan.bench,
        Some(BenchPlan {
            scenario: BenchScenario::Idle,
            idle_duration: Duration::from_secs(10)
        })
    );
}

#[test]
fn bench_idle_takes_duration() {
    let plan = cli::resolve(&parse(&["--bench", "idle", "--duration-secs", "3"]), BOTH).unwrap();
    assert_eq!(
        plan.bench,
        Some(BenchPlan {
            scenario: BenchScenario::Idle,
            idle_duration: Duration::from_secs(3)
        })
    );
}

#[test]
fn bench_startup_has_no_duration_flag() {
    let err = cli::resolve(
        &parse(&["--bench", "startup", "--duration-secs", "3"]),
        BOTH,
    )
    .unwrap_err();
    assert_eq!(err, UsageError::DurationWithoutIdle);
    assert_eq!(err.to_string(), "--duration-secs requires --bench idle");
}

#[test]
fn duration_without_bench_is_an_error() {
    let err = cli::resolve(&parse(&["--duration-secs", "3"]), BOTH).unwrap_err();
    assert_eq!(err, UsageError::DurationWithoutIdle);
}

#[test]
fn duration_zero_is_a_parse_error() {
    let parsed = GuiArgs::try_parse_from([
        "visual-assembler",
        "--bench",
        "idle",
        "--duration-secs",
        "0",
    ]);
    assert!(parsed.is_err());
}

#[test]
fn duration_above_one_hour_is_a_parse_error() {
    let parsed = GuiArgs::try_parse_from([
        "visual-assembler",
        "--bench",
        "idle",
        "--duration-secs",
        "3601",
    ]);
    assert!(parsed.is_err());
    let plan = cli::resolve(
        &parse(&["--bench", "idle", "--duration-secs", "3600"]),
        BOTH,
    )
    .unwrap();
    assert_eq!(plan.bench.unwrap().idle_duration, Duration::from_secs(3600));
}

#[test]
fn compiled_renderers_match_features() {
    let compiled = cli::compiled_renderers();
    assert_eq!(
        compiled.contains(&Renderer::Wgpu),
        cfg!(feature = "renderer-wgpu")
    );
    assert_eq!(
        compiled.contains(&Renderer::Glow),
        cfg!(feature = "renderer-glow")
    );
    assert!(!compiled.is_empty());
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (audit round 1).
// ---------------------------------------------------------------------------------------------

/// Parses a command line (without the program name), keeping clap's error.
fn try_parse(args: &[&str]) -> Result<GuiArgs, clap::Error> {
    GuiArgs::try_parse_from(std::iter::once("visual-assembler").chain(args.iter().copied()))
}

#[test]
fn wgpu_requested_in_a_glow_only_build_is_an_error() {
    let err = cli::resolve(&parse(&["--renderer", "wgpu"]), &[Renderer::Glow]).unwrap_err();
    assert_eq!(err, UsageError::RendererNotCompiled(Renderer::Wgpu));
    assert_eq!(
        err.to_string(),
        "renderer 'wgpu' is not compiled into this binary (build with --features renderer-wgpu)"
    );
}

#[test]
fn no_compiled_renderer_is_an_error_not_a_panic() {
    let err = cli::resolve(&parse(&[]), &[]).unwrap_err();
    assert!(matches!(err, UsageError::RendererNotCompiled(_)), "{err:?}");
    assert!(err.to_string().contains("not compiled"), "{err}");
}

#[test]
fn startup_bench_plan_has_the_startup_scenario() {
    let plan = cli::resolve(&parse(&["--bench", "startup"]), BOTH).unwrap();
    assert_eq!(plan.renderer, None);
    assert_eq!(plan.bench.unwrap().scenario, BenchScenario::Startup);
}

#[test]
fn duration_lower_bound_is_inclusive() {
    let plan = cli::resolve(&parse(&["--bench", "idle", "--duration-secs", "1"]), BOTH).unwrap();
    assert_eq!(plan.bench.unwrap().idle_duration, Duration::from_secs(1));
}

#[test]
fn equals_syntax_and_any_flag_order_are_accepted() {
    let plan = cli::resolve(
        &parse(&["--duration-secs=7", "--renderer=glow", "--bench=idle"]),
        BOTH,
    )
    .unwrap();
    assert_eq!(
        plan,
        LaunchPlan {
            renderer: Some(RendererChoice::Glow),
            bench: Some(BenchPlan {
                scenario: BenchScenario::Idle,
                idle_duration: Duration::from_secs(7)
            })
        }
    );
}

#[test]
fn malformed_values_are_usage_errors() {
    for bad in [
        &["--duration-secs", "-1"][..],
        &["--duration-secs", "abc"],
        &["--duration-secs", ""],
        &["--duration-secs", "1.5"],
        &["--duration-secs", " 5"],
        &["--duration-secs", "18446744073709551616"],
        &["--renderer", ""],
        &["--renderer", "opengl"],
        &["--bench", "startup "],
        &["--bench", "all"],
        &["extra"],
    ] {
        let err = try_parse(bad).unwrap_err();
        assert_eq!(err.exit_code(), 2, "{bad:?}");
    }
}

#[test]
fn repeated_flags_are_usage_errors() {
    for bad in [
        &["--bench", "startup", "--bench", "idle"][..],
        &["--renderer", "wgpu", "--renderer", "glow"],
        &[
            "--bench",
            "idle",
            "--duration-secs",
            "3",
            "--duration-secs",
            "4",
        ],
    ] {
        let err = try_parse(bad).unwrap_err();
        assert_eq!(err.exit_code(), 2, "{bad:?}");
    }
}

#[test]
fn help_and_version_are_display_outcomes_with_exit_code_zero() {
    let version = try_parse(&["--version"]).unwrap_err();
    assert_eq!(version.kind(), clap::error::ErrorKind::DisplayVersion);
    assert_eq!(version.exit_code(), 0);
    assert_eq!(version.to_string(), "visual-assembler 0.1.0\n");

    let help = try_parse(&["--help"]).unwrap_err();
    assert_eq!(help.kind(), clap::error::ErrorKind::DisplayHelp);
    assert_eq!(help.exit_code(), 0);
}

// ---------------------------------------------------------------------------------------------
// Renderer selection (docs/specs/m0-foundations.md §19.1–§19.4). The first five tests are the
// §19.3 CI table; the others pin the exact §19.4 texts and the WSL rule without a window.
// ---------------------------------------------------------------------------------------------

/// The text of an `injected` failure, as eframe reports the app creator's error.
fn injected_error() -> eframe::Error {
    eframe::Error::AppCreation(Box::new(RendererInitError::Injected))
}

/// An `auto` plan in the glow-first order (every target but Windows aarch64).
fn auto_plan() -> RendererPlan {
    RendererPlan {
        requested: RendererChoice::Auto,
        source: ChoiceSource::Default,
        order: vec![Renderer::Glow, Renderer::Wgpu],
    }
}

/// A plan forcing `renderer` from `source`.
fn forced_plan(renderer: Renderer, source: ChoiceSource) -> RendererPlan {
    let requested = match renderer {
        Renderer::Glow => RendererChoice::Glow,
        Renderer::Wgpu => RendererChoice::Wgpu,
    };
    RendererPlan {
        requested,
        source,
        order: vec![renderer],
    }
}

/// §19.3 `renderer_values_parse`: `auto`, `glow` and `wgpu` parse; `vulkan` is a usage error.
#[test]
fn renderer_values_parse() {
    for (value, expected) in [
        ("auto", RendererChoice::Auto),
        ("glow", RendererChoice::Glow),
        ("wgpu", RendererChoice::Wgpu),
    ] {
        assert_eq!(parse(&["--renderer", value]).renderer, Some(expected));
        assert_eq!(expected.as_str(), value);
        assert_eq!(expected.to_string(), value);
        assert_eq!(RendererChoice::from_name(value), Some(expected));
    }
    let err = try_parse(&["--renderer", "vulkan"]).unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
    assert_eq!(err.exit_code(), 2);
    for bad in ["vulkan", "", "Auto", "GLOW", " glow", "dx12"] {
        assert_eq!(RendererChoice::from_name(bad), None, "{bad:?}");
    }
    assert_eq!(RendererChoice::Auto.forced(), None);
    assert_eq!(RendererChoice::Glow.forced(), Some(Renderer::Glow));
    assert_eq!(RendererChoice::Wgpu.forced(), Some(Renderer::Wgpu));
}

/// §19.3 `renderer_precedence`: `--renderer` > `VA_RENDERER` > built-in `auto`; an invalid
/// variable is ignored with exactly one warning.
#[test]
fn renderer_precedence() {
    let resolve = |cli_choice, env: Option<&str>| {
        cli::resolve_renderer(cli_choice, env.map(OsStr::new), BOTH, "windows", "x86_64").unwrap()
    };

    let neither = resolve(None, None);
    assert_eq!(neither.plan.requested, RendererChoice::Auto);
    assert_eq!(neither.plan.source, ChoiceSource::Default);
    assert!(neither.warnings.is_empty());

    let env_wgpu = resolve(None, Some("wgpu"));
    assert_eq!(env_wgpu.plan.requested, RendererChoice::Wgpu);
    assert_eq!(env_wgpu.plan.source, ChoiceSource::Env);
    assert_eq!(env_wgpu.plan.order, [Renderer::Wgpu]);
    assert!(env_wgpu.warnings.is_empty());

    let cli_auto = resolve(Some(RendererChoice::Auto), Some("wgpu"));
    assert_eq!(cli_auto.plan.requested, RendererChoice::Auto);
    assert_eq!(cli_auto.plan.source, ChoiceSource::Cli);
    assert_eq!(cli_auto.plan.order, [Renderer::Glow, Renderer::Wgpu]);

    let cli_glow = resolve(Some(RendererChoice::Glow), Some("wgpu"));
    assert_eq!(cli_glow.plan.requested, RendererChoice::Glow);
    assert_eq!(cli_glow.plan.source, ChoiceSource::Cli);

    let env_vulkan = resolve(None, Some("vulkan"));
    assert_eq!(env_vulkan.plan.requested, RendererChoice::Auto);
    assert_eq!(env_vulkan.plan.source, ChoiceSource::Default);
    assert_eq!(
        env_vulkan.warnings,
        ["ignoring VA_RENDERER='vulkan' (expected auto, glow or wgpu)"]
    );

    // Every value that is not exactly `auto`, `glow` or `wgpu` is invalid, the empty one too,
    // and the warning is printed even when `--renderer` overrides the variable.
    for bad in ["", "Glow", "auto ", "glow,wgpu"] {
        let resolution = resolve(None, Some(bad));
        assert_eq!(resolution.plan.source, ChoiceSource::Default, "{bad:?}");
        assert_eq!(
            resolution.warnings,
            [format!(
                "ignoring VA_RENDERER='{bad}' (expected auto, glow or wgpu)"
            )]
        );
    }
    let overridden = resolve(Some(RendererChoice::Wgpu), Some("bogus"));
    assert_eq!(overridden.plan.source, ChoiceSource::Cli);
    assert_eq!(overridden.warnings.len(), 1);

    assert_eq!(ChoiceSource::Cli.to_string(), "--renderer");
    assert_eq!(ChoiceSource::Env.to_string(), "VA_RENDERER");
    assert_eq!(ChoiceSource::Default.to_string(), "default");
}

/// A non-Unicode `VA_RENDERER` is invalid like any other value (warning, ignored).
#[cfg(unix)]
#[test]
fn non_unicode_renderer_variable_is_ignored_with_a_warning() {
    use std::os::unix::ffi::OsStrExt as _;
    let raw = OsStr::from_bytes(b"w\xffpu");
    let resolution = cli::resolve_renderer(None, Some(raw), BOTH, "linux", "x86_64").unwrap();
    assert_eq!(resolution.plan.requested, RendererChoice::Auto);
    assert_eq!(resolution.warnings.len(), 1);
    assert!(
        resolution.warnings[0].starts_with("ignoring VA_RENDERER='w"),
        "{:?}",
        resolution.warnings
    );
}

/// A forced renderer (CLI or variable) that is not compiled in is a usage error, never a silent
/// switch to the other renderer.
#[test]
fn forced_renderer_not_compiled_is_a_usage_error() {
    let from_env = cli::resolve_renderer(
        None,
        Some(OsStr::new("glow")),
        &[Renderer::Wgpu],
        "linux",
        "x86_64",
    )
    .unwrap_err();
    assert_eq!(from_env, UsageError::RendererNotCompiled(Renderer::Glow));
    let from_cli = cli::resolve_renderer(
        Some(RendererChoice::Wgpu),
        None,
        &[Renderer::Glow],
        "linux",
        "x86_64",
    )
    .unwrap_err();
    assert_eq!(from_cli, UsageError::RendererNotCompiled(Renderer::Wgpu));
    let nothing = cli::resolve_renderer(None, None, &[], "windows", "aarch64").unwrap_err();
    assert_eq!(nothing, UsageError::RendererNotCompiled(Renderer::Wgpu));
}

/// §19.3 `auto_order_per_target`.
#[test]
fn auto_order_per_target() {
    let order = |cli_choice, compiled: &[Renderer], os, arch| {
        cli::resolve_renderer(cli_choice, None, compiled, os, arch)
            .unwrap()
            .plan
            .order
    };
    assert_eq!(
        order(None, BOTH, "windows", "x86_64"),
        [Renderer::Glow, Renderer::Wgpu]
    );
    assert_eq!(
        order(None, BOTH, "windows", "aarch64"),
        [Renderer::Wgpu, Renderer::Glow]
    );
    assert_eq!(
        order(None, BOTH, "linux", "x86_64"),
        [Renderer::Glow, Renderer::Wgpu]
    );
    assert_eq!(
        order(None, BOTH, "linux", "aarch64"),
        [Renderer::Glow, Renderer::Wgpu]
    );
    assert_eq!(
        order(Some(RendererChoice::Auto), BOTH, "windows", "aarch64"),
        [Renderer::Wgpu, Renderer::Glow]
    );
    for (os, arch) in [
        ("windows", "x86_64"),
        ("windows", "aarch64"),
        ("linux", "x86_64"),
    ] {
        assert_eq!(
            order(Some(RendererChoice::Glow), BOTH, os, arch),
            [Renderer::Glow]
        );
        assert_eq!(
            order(Some(RendererChoice::Wgpu), BOTH, os, arch),
            [Renderer::Wgpu]
        );
        assert_eq!(order(None, &[Renderer::Glow], os, arch), [Renderer::Glow]);
        assert_eq!(order(None, &[Renderer::Wgpu], os, arch), [Renderer::Wgpu]);
    }
    assert_eq!(
        cli::auto_order("windows", "aarch64"),
        [Renderer::Wgpu, Renderer::Glow]
    );
    assert_eq!(
        cli::auto_order("macos", "aarch64"),
        [Renderer::Glow, Renderer::Wgpu]
    );
}

/// §19.3 `injection_list_parses`.
#[test]
fn injection_list_parses() {
    let parse_list = |raw: &str| cli::parse_injected_failures(Some(OsStr::new(raw)));

    let glow = parse_list("glow");
    assert_eq!(glow.renderers, [Renderer::Glow]);
    assert!(glow.contains(Renderer::Glow) && !glow.contains(Renderer::Wgpu));
    assert_eq!(glow.raw.as_deref(), Some("glow"));
    assert!(glow.warnings.is_empty());
    assert_eq!(
        glow.startup_warning().as_deref(),
        Some(
            "VA_TEST_FAIL_RENDERER=glow is set: simulating a renderer initialization failure (test hook)"
        )
    );

    let both = parse_list("glow,wgpu");
    assert_eq!(both.renderers, [Renderer::Glow, Renderer::Wgpu]);
    assert!(both.warnings.is_empty());
    assert_eq!(parse_list("wgpu, glow ,wgpu,").renderers, both.renderers);

    let empty = parse_list("");
    assert!(empty.renderers.is_empty());
    assert!(empty.warnings.is_empty());
    assert_eq!(empty.startup_warning(), None);
    assert_eq!(empty.raw.as_deref(), Some(""));

    let vulkan = parse_list("vulkan");
    assert!(vulkan.renderers.is_empty());
    assert_eq!(
        vulkan.warnings,
        ["ignoring unknown VA_TEST_FAIL_RENDERER value 'vulkan' (expected glow or wgpu)"]
    );
    assert_eq!(vulkan.startup_warning(), None);
    assert_eq!(vulkan.raw.as_deref(), Some("vulkan"));

    let mixed = parse_list("auto,wgpu");
    assert_eq!(mixed.renderers, [Renderer::Wgpu]);
    assert_eq!(mixed.warnings.len(), 1);

    let unset = cli::parse_injected_failures(None);
    assert!(unset.renderers.is_empty() && unset.warnings.is_empty());
    assert_eq!(unset.raw, None);
    assert_eq!(unset.startup_warning(), None);
}

/// §19.3 `fallback_eligibility`: every `eframe::Error` variant this test can construct is
/// classified as in §19.2. `Winit` (no public constructor) and the glutin variants (glutin is not
/// a dependency of this package) cannot be built here.
#[test]
fn fallback_eligibility() {
    let injected = injected_error();
    assert!(cli::is_fallback_eligible(&injected, false));
    assert!(!cli::is_fallback_eligible(&injected, true));
    assert!(cli::is_renderer_init_error(&injected));

    let other = eframe::Error::AppCreation("some other app creation failure".into());
    assert!(!cli::is_fallback_eligible(&other, false));
    assert!(!cli::is_fallback_eligible(&other, true));

    #[cfg(feature = "renderer-glow")]
    {
        let opengl = eframe::Error::OpenGL(eframe::egui_glow::PainterError::from(
            "egui_glow requires opengl 2.0+.".to_owned(),
        ));
        assert!(cli::is_fallback_eligible(&opengl, false));
        assert!(!cli::is_fallback_eligible(&opengl, true));
    }

    #[cfg(feature = "renderer-wgpu")]
    for wgpu in [
        eframe::egui_wgpu::WgpuError::NoSurfaceFormatsAvailable,
        eframe::egui_wgpu::WgpuError::CustomNativeAdapterSelectionError("none".to_owned()),
        eframe::egui_wgpu::WgpuError::HandleError(raw_window_handle::HandleError::Unavailable),
    ] {
        let wgpu = eframe::Error::Wgpu(wgpu);
        assert!(cli::is_fallback_eligible(&wgpu, false), "{wgpu}");
        assert!(!cli::is_fallback_eligible(&wgpu, true), "{wgpu}");
    }

    #[cfg(target_os = "linux")]
    for event_loop in [
        winit::error::EventLoopError::RecreationAttempt,
        winit::error::EventLoopError::ExitFailure(1),
    ] {
        let event_loop = eframe::Error::WinitEventLoop(event_loop);
        assert!(
            !cli::is_fallback_eligible(&event_loop, false),
            "{event_loop}"
        );
        assert!(!cli::is_renderer_init_error(&event_loop), "{event_loop}");
    }
}

/// The injected failure's text is the one of the §19.5 example (`fallback_reason`).
#[test]
fn injected_failure_is_described_by_its_own_text() {
    assert_eq!(
        RendererInitError::Injected.to_string(),
        "injected renderer failure (VA_TEST_FAIL_RENDERER)"
    );
    assert_eq!(
        cli::describe_failure(&injected_error()),
        "injected renderer failure (VA_TEST_FAIL_RENDERER)"
    );
    let other = eframe::Error::AppCreation("boom".into());
    assert_eq!(cli::describe_failure(&other), "app creation error: boom");
}

/// §19.4 "Fallback succeeded": the warning names both renderers; the reason is the first line.
#[test]
fn auto_falls_back_once_with_the_spec_warning() {
    let failure = cli::classify_failure(&auto_plan(), 0, &injected_error(), false);
    assert_eq!(
        failure,
        AttemptFailure::FallBack {
            next: Renderer::Wgpu,
            warning: "the glow renderer failed to initialize: injected renderer failure \
                      (VA_TEST_FAIL_RENDERER); falling back to wgpu"
                .to_owned(),
            reason: "injected renderer failure (VA_TEST_FAIL_RENDERER)".to_owned(),
        }
    );

    let reversed = RendererPlan {
        order: vec![Renderer::Wgpu, Renderer::Glow],
        ..auto_plan()
    };
    let AttemptFailure::FallBack { next, warning, .. } =
        cli::classify_failure(&reversed, 0, &injected_error(), false)
    else {
        panic!("expected a fallback");
    };
    assert_eq!(next, Renderer::Glow);
    assert!(
        warning.starts_with("the wgpu renderer failed to initialize:"),
        "{warning}"
    );
    assert!(warning.ends_with("; falling back to glow"), "{warning}");
}

/// §19.4 "`auto`, both renderers failed": exit 3 with the error and the README note.
#[test]
fn auto_with_every_renderer_failing_is_fatal() {
    let failure = cli::classify_failure(&auto_plan(), 1, &injected_error(), false);
    assert_eq!(
        failure,
        AttemptFailure::Fatal {
            error: "no renderer could be initialized (tried glow, then wgpu): injected renderer \
                    failure (VA_TEST_FAIL_RENDERER)"
                .to_owned(),
            note: Some("see the README section \"Renderer selection\"".to_owned()),
        }
    );

    let single = RendererPlan {
        order: vec![Renderer::Glow],
        ..auto_plan()
    };
    let AttemptFailure::Fatal { error, note } =
        cli::classify_failure(&single, 0, &injected_error(), false)
    else {
        panic!("a single-renderer auto plan cannot fall back");
    };
    assert!(
        error.starts_with("no renderer could be initialized (tried glow): "),
        "{error}"
    );
    assert!(note.is_some());
}

/// §19.4 "Forced renderer failed": no fallback, the source is named.
#[test]
fn forced_renderer_failure_is_fatal_and_names_its_source() {
    for (source, name) in [
        (ChoiceSource::Cli, "--renderer"),
        (ChoiceSource::Env, "VA_RENDERER"),
    ] {
        let failure = cli::classify_failure(
            &forced_plan(Renderer::Glow, source),
            0,
            &injected_error(),
            false,
        );
        assert_eq!(
            failure,
            AttemptFailure::Fatal {
                error: format!(
                    "the glow renderer failed to initialize: injected renderer failure \
                     (VA_TEST_FAIL_RENDERER) (forced by {name}; try --renderer auto)"
                ),
                note: None,
            }
        );
    }
}

/// §19.4 "Error after the first frame" and non-renderer errors: the unchanged message, no
/// fallback, even with a renderer left in an `auto` plan.
#[test]
fn late_or_foreign_errors_keep_the_window_message() {
    let late = cli::classify_failure(&auto_plan(), 0, &injected_error(), true);
    assert_eq!(
        late,
        AttemptFailure::Fatal {
            error: "cannot create the window or the renderer: app creation error: injected \
                    renderer failure (VA_TEST_FAIL_RENDERER)"
                .to_owned(),
            note: None,
        }
    );

    let foreign = eframe::Error::AppCreation("boom".into());
    for plan in [auto_plan(), forced_plan(Renderer::Wgpu, ChoiceSource::Cli)] {
        assert_eq!(
            cli::classify_failure(&plan, 0, &foreign, false),
            AttemptFailure::Fatal {
                error: "cannot create the window or the renderer: app creation error: boom"
                    .to_owned(),
                note: None,
            }
        );
    }

    #[cfg(target_os = "linux")]
    {
        let platform =
            eframe::Error::WinitEventLoop(winit::error::EventLoopError::RecreationAttempt);
        let AttemptFailure::Fatal { error, note } =
            cli::classify_failure(&auto_plan(), 0, &platform, false)
        else {
            panic!("a platform failure never falls back");
        };
        assert!(
            error.starts_with("cannot create the window or the renderer: winit EventLoopError"),
            "{error}"
        );
        assert_eq!(note, None);
    }
}

/// §19.1 "WSL window system": X11 under WSL unless `VA_WAYLAND` is exactly `1`; the compositor
/// warning only with wgpu in the plan; nothing outside WSL.
#[test]
fn wsl_uses_x11_unless_va_wayland_is_1() {
    let both = [Renderer::Glow, Renderer::Wgpu];
    let glow_only = [Renderer::Glow];

    for unset_or_other in [None, Some("0"), Some(""), Some("yes"), Some("1 ")] {
        let policy = cli::window_system_policy(true, unset_or_other.map(OsStr::new), &both);
        assert!(policy.force_x11, "{unset_or_other:?}");
        assert_eq!(policy.warning, None, "{unset_or_other:?}");
    }

    let wayland_wgpu = cli::window_system_policy(true, Some(OsStr::new("1")), &both);
    assert!(!wayland_wgpu.force_x11);
    assert_eq!(
        wayland_wgpu.warning.as_deref(),
        Some("VA_WAYLAND=1 with the wgpu renderer can crash the WSLg compositor (weston 9)")
    );

    let wayland_glow = cli::window_system_policy(true, Some(OsStr::new("1")), &glow_only);
    assert!(!wayland_glow.force_x11);
    assert_eq!(wayland_glow.warning, None);

    for va_wayland in [None, Some("1")] {
        let native = cli::window_system_policy(false, va_wayland.map(OsStr::new), &both);
        assert!(!native.force_x11);
        assert_eq!(native.warning, None);
    }
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (§19 audit round 1): the precedence levels that the §19.3 table leaves implicit,
// non-Unicode variables on every OS, the exact injection warnings, and the failure classes of a
// second attempt or of a wgpu-first plan.
// ---------------------------------------------------------------------------------------------

/// Builds a value that is not valid Unicode on the current OS: an invalid UTF-8 byte on Unix,
/// an unpaired UTF-16 surrogate on Windows. Its lossy form is `w\u{FFFD}`.
fn non_unicode_value() -> std::ffi::OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt as _;
        std::ffi::OsString::from_vec(vec![b'w', 0xff])
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt as _;
        std::ffi::OsString::from_wide(&[u16::from(b'w'), 0xD800])
    }
}

/// §19.1: `VA_RENDERER=auto` is a valid value. It selects `auto` from the variable (so a later
/// settings file cannot override it), with the target's order and no warning.
#[test]
fn variable_auto_is_an_explicit_choice() {
    for (os, arch, expected) in [
        ("windows", "x86_64", [Renderer::Glow, Renderer::Wgpu]),
        ("windows", "aarch64", [Renderer::Wgpu, Renderer::Glow]),
        ("linux", "aarch64", [Renderer::Glow, Renderer::Wgpu]),
    ] {
        let resolution =
            cli::resolve_renderer(None, Some(OsStr::new("auto")), BOTH, os, arch).unwrap();
        assert_eq!(
            resolution.plan.requested,
            RendererChoice::Auto,
            "{os}/{arch}"
        );
        assert_eq!(resolution.plan.source, ChoiceSource::Env, "{os}/{arch}");
        assert_eq!(resolution.plan.order, expected, "{os}/{arch}");
        assert!(resolution.warnings.is_empty(), "{os}/{arch}");
    }
    // `--renderer glow` still wins over `VA_RENDERER=auto`.
    let forced = cli::resolve_renderer(
        Some(RendererChoice::Glow),
        Some(OsStr::new("auto")),
        BOTH,
        "linux",
        "x86_64",
    )
    .unwrap();
    assert_eq!(forced.plan.order, [Renderer::Glow]);
    assert_eq!(forced.plan.source, ChoiceSource::Cli);
}

/// A variable that names a renderer missing from the build is overridden by `--renderer`, so it
/// must not turn into a usage error: only the level that wins is checked against the build.
#[test]
fn an_overridden_variable_is_never_checked_against_the_build() {
    for cli_choice in [RendererChoice::Auto, RendererChoice::Glow] {
        let resolution = cli::resolve_renderer(
            Some(cli_choice),
            Some(OsStr::new("wgpu")),
            &[Renderer::Glow],
            "linux",
            "x86_64",
        )
        .unwrap();
        assert_eq!(resolution.plan.order, [Renderer::Glow], "{cli_choice}");
        assert_eq!(resolution.plan.source, ChoiceSource::Cli, "{cli_choice}");
        assert!(resolution.warnings.is_empty(), "{cli_choice}");
    }
    // An invalid variable in a single-renderer build falls back to `auto`, reduced to that build.
    let ignored = cli::resolve_renderer(
        None,
        Some(OsStr::new("vulkan")),
        &[Renderer::Wgpu],
        "windows",
        "x86_64",
    )
    .unwrap();
    assert_eq!(ignored.plan.requested, RendererChoice::Auto);
    assert_eq!(ignored.plan.order, [Renderer::Wgpu]);
    assert_eq!(ignored.warnings.len(), 1);
}

/// A non-Unicode `VA_RENDERER` is ignored with one warning on every OS (the Unix-only test above
/// does not run on the Windows runners).
#[test]
fn non_unicode_renderer_variable_warns_on_every_os() {
    let raw = non_unicode_value();
    let resolution = cli::resolve_renderer(None, Some(&raw), BOTH, "windows", "x86_64").unwrap();
    assert_eq!(resolution.plan.requested, RendererChoice::Auto);
    assert_eq!(resolution.plan.source, ChoiceSource::Default);
    assert_eq!(
        resolution.warnings,
        ["ignoring VA_RENDERER='w\u{FFFD}' (expected auto, glow or wgpu)"]
    );
}

/// A non-Unicode `VA_TEST_FAIL_RENDERER` simulates nothing, warns once, and is still reported
/// (lossily) as `injected_failure`.
#[test]
fn non_unicode_injection_variable_is_ignored_as_a_whole() {
    let raw = non_unicode_value();
    let failures = cli::parse_injected_failures(Some(&raw));
    assert!(failures.renderers.is_empty());
    assert_eq!(failures.raw.as_deref(), Some("w\u{FFFD}"));
    assert_eq!(
        failures.warnings,
        [
            "ignoring VA_TEST_FAIL_RENDERER='w\u{FFFD}' (not valid Unicode; expected glow, wgpu or glow,wgpu)"
        ]
    );
    assert_eq!(failures.startup_warning(), None);
}

/// §19.3 "unknown values produce a warning and are ignored": one exact warning per unknown item,
/// in input order; `auto` is not a renderer; duplicates and empty items are silent.
#[test]
fn injection_items_are_checked_one_by_one() {
    let parse_list = |raw: &str| cli::parse_injected_failures(Some(OsStr::new(raw)));

    let mixed = parse_list("metal,glow,auto,dx12");
    assert_eq!(mixed.renderers, [Renderer::Glow]);
    assert_eq!(
        mixed.warnings,
        [
            "ignoring unknown VA_TEST_FAIL_RENDERER value 'metal' (expected glow or wgpu)",
            "ignoring unknown VA_TEST_FAIL_RENDERER value 'auto' (expected glow or wgpu)",
            "ignoring unknown VA_TEST_FAIL_RENDERER value 'dx12' (expected glow or wgpu)",
        ]
    );

    let duplicates = parse_list("glow,glow, glow");
    assert_eq!(duplicates.renderers, [Renderer::Glow]);
    assert!(duplicates.warnings.is_empty());

    assert_eq!(
        parse_list("wgpu,glow").renderers,
        [Renderer::Glow, Renderer::Wgpu]
    );

    for blank in [",,,", " ", " , "] {
        let failures = parse_list(blank);
        assert!(failures.renderers.is_empty(), "{blank:?}");
        assert!(failures.warnings.is_empty(), "{blank:?}");
        assert_eq!(failures.startup_warning(), None, "{blank:?}");
        assert_eq!(failures.raw.as_deref(), Some(blank));
    }

    // Separators other than a comma make one unknown item; the case of a value matters.
    for odd in ["glow;wgpu", "glow wgpu", "GLOW", "Wgpu"] {
        let failures = parse_list(odd);
        assert!(failures.renderers.is_empty(), "{odd:?}");
        assert_eq!(failures.warnings.len(), 1, "{odd:?}");
    }

    // The loud startup warning quotes the raw value, untrimmed.
    assert_eq!(
        parse_list(" glow , ,").startup_warning().as_deref(),
        Some(
            "VA_TEST_FAIL_RENDERER= glow , , is set: simulating a renderer initialization failure (test hook)"
        )
    );
}

/// §19.4 "Error after the first frame" also holds for a forced renderer: the unchanged window
/// message, not the forced one.
#[test]
fn forced_renderer_error_after_the_first_frame_keeps_the_window_message() {
    for source in [ChoiceSource::Cli, ChoiceSource::Env] {
        assert_eq!(
            cli::classify_failure(
                &forced_plan(Renderer::Glow, source),
                0,
                &injected_error(),
                true
            ),
            AttemptFailure::Fatal {
                error: "cannot create the window or the renderer: app creation error: injected \
                        renderer failure (VA_TEST_FAIL_RENDERER)"
                    .to_owned(),
                note: None,
            }
        );
    }
}

/// The second attempt of an `auto` plan: a renderer error gives the "no renderer" message with
/// the README note; any other error keeps the window message (no note, no second fallback).
#[test]
fn second_attempt_failures_never_fall_back_again() {
    let foreign = eframe::Error::AppCreation("boom".into());
    assert_eq!(
        cli::classify_failure(&auto_plan(), 1, &foreign, false),
        AttemptFailure::Fatal {
            error: "cannot create the window or the renderer: app creation error: boom".to_owned(),
            note: None,
        }
    );
    assert_eq!(
        cli::classify_failure(&auto_plan(), 1, &injected_error(), true),
        AttemptFailure::Fatal {
            error: "cannot create the window or the renderer: app creation error: injected \
                    renderer failure (VA_TEST_FAIL_RENDERER)"
                .to_owned(),
            note: None,
        }
    );

    #[cfg(feature = "renderer-wgpu")]
    {
        let wgpu = eframe::Error::Wgpu(eframe::egui_wgpu::WgpuError::NoSurfaceFormatsAvailable);
        assert_eq!(
            cli::classify_failure(&auto_plan(), 1, &wgpu, false),
            AttemptFailure::Fatal {
                error: "no renderer could be initialized (tried glow, then wgpu): WGPU error: \
                        There was no valid format for the surface at all."
                    .to_owned(),
                note: Some("see the README section \"Renderer selection\"".to_owned()),
            }
        );
    }
}

/// An attempt index outside the plan is a fatal window error, never a panic or a fallback.
#[test]
fn an_attempt_outside_the_plan_is_fatal() {
    for attempt in [2, 7, usize::MAX - 1] {
        let failure = cli::classify_failure(&auto_plan(), attempt, &injected_error(), false);
        let AttemptFailure::Fatal { error, note } = failure else {
            panic!("attempt {attempt} must not fall back");
        };
        assert!(
            error.starts_with("cannot create the window or the renderer: "),
            "{error}"
        );
        assert_eq!(note, None);
    }
}

/// §19.2 on Windows aarch64 (wgpu first): a `Wgpu` initialization error before the first frame
/// falls back to glow with the §19.4 warning; the reason is eframe's text.
#[cfg(feature = "renderer-wgpu")]
#[test]
fn wgpu_error_falls_back_to_glow_on_the_wgpu_first_order() {
    let reversed = cli::resolve_renderer(None, None, BOTH, "windows", "aarch64")
        .unwrap()
        .plan;
    assert_eq!(reversed.order, [Renderer::Wgpu, Renderer::Glow]);
    let wgpu = eframe::Error::Wgpu(eframe::egui_wgpu::WgpuError::NoSurfaceFormatsAvailable);
    assert_eq!(
        cli::classify_failure(&reversed, 0, &wgpu, false),
        AttemptFailure::FallBack {
            next: Renderer::Glow,
            warning: "the wgpu renderer failed to initialize: WGPU error: There was no valid \
                      format for the surface at all.; falling back to glow"
                .to_owned(),
            reason: "WGPU error: There was no valid format for the surface at all.".to_owned(),
        }
    );
    // The same error after the first frame stays fatal.
    assert!(matches!(
        cli::classify_failure(&reversed, 0, &wgpu, true),
        AttemptFailure::Fatal { note: None, .. }
    ));
}

/// §19.5 `fallback_reason` is the first line of the error, while the stderr warning carries the
/// whole description.
#[cfg(feature = "renderer-glow")]
#[test]
fn fallback_reason_is_the_first_line_of_a_multi_line_error() {
    let opengl = eframe::Error::OpenGL(eframe::egui_glow::PainterError::from(
        "shader version 1.00 is not supported\nsecond line of the driver log".to_owned(),
    ));
    let AttemptFailure::FallBack {
        next,
        warning,
        reason,
    } = cli::classify_failure(&auto_plan(), 0, &opengl, false)
    else {
        panic!("an OpenGL error before the first frame falls back");
    };
    assert_eq!(next, Renderer::Wgpu);
    assert_eq!(
        reason,
        "egui_glow: OpenGL: shader version 1.00 is not supported"
    );
    assert!(!reason.contains('\n'), "{reason:?}");
    assert!(
        warning.starts_with("the glow renderer failed to initialize: egui_glow: OpenGL: "),
        "{warning}"
    );
    assert!(
        warning.ends_with("second line of the driver log; falling back to wgpu"),
        "{warning}"
    );
}

/// `VA_WAYLAND` must be exactly the one-byte value `1`: a non-Unicode value starting with `1`
/// keeps the X11 default.
#[test]
fn non_unicode_va_wayland_is_not_1() {
    let mut raw = std::ffi::OsString::from("1");
    raw.push(non_unicode_value());
    let policy = cli::window_system_policy(true, Some(&raw), BOTH);
    assert!(policy.force_x11);
    assert_eq!(policy.warning, None);
}

// ---------------------------------------------------------------------------------------------
// §19.9 S19-2 (fix-wave row 19-8 a): every environment value echoed in a warning goes through
// `str::escape_debug`, so a stray CR, newline or escape sequence cannot break the line or drive
// the terminal. The bench value (`InjectedFailures::raw`) stays unescaped.
// ---------------------------------------------------------------------------------------------

/// True when `text` holds a CR, a LF, an ESC or any other control character.
fn has_control_character(text: &str) -> bool {
    text.chars().any(char::is_control)
}

/// An invalid `VA_RENDERER` with CR, LF and an ANSI escape gives one single-line warning that
/// spells the value with backslash escapes.
#[test]
fn invalid_renderer_variable_is_echoed_escaped() {
    let raw = OsStr::new("glow\r\n\u{1b}[2J");
    let resolution = cli::resolve_renderer(None, Some(raw), BOTH, "windows", "x86_64").unwrap();
    assert_eq!(resolution.plan.requested, RendererChoice::Auto);
    assert_eq!(resolution.plan.source, ChoiceSource::Default);
    assert_eq!(resolution.warnings.len(), 1, "{:?}", resolution.warnings);
    let warning = &resolution.warnings[0];
    for forbidden in ['\r', '\n', '\u{1b}'] {
        assert!(!warning.contains(forbidden), "{warning:?}");
    }
    assert!(!has_control_character(warning), "{warning:?}");
    assert_eq!(
        warning,
        r"ignoring VA_RENDERER='glow\r\n\u{1b}[2J' (expected auto, glow or wgpu)"
    );
    assert!(warning.contains(r"glow\r\n\u{1b}[2J"), "{warning:?}");
}

/// A forged second line in `VA_TEST_FAIL_RENDERER` stays inside one warning (the unknown item is
/// escaped after trimming), the startup warning has no control character, and `raw` (the bench
/// `injected_failure`) is the input unchanged.
#[test]
fn injection_values_are_echoed_escaped() {
    let input = "glow,bad\nerror: forged";
    let failures = cli::parse_injected_failures(Some(OsStr::new(input)));
    assert_eq!(failures.renderers, [Renderer::Glow]);
    assert_eq!(failures.raw.as_deref(), Some(input));
    assert_eq!(
        failures.warnings,
        [
            r"ignoring unknown VA_TEST_FAIL_RENDERER value 'bad\nerror: forged' (expected glow or wgpu)"
        ]
    );
    assert!(
        !failures.warnings.iter().any(|w| has_control_character(w)),
        "{:?}",
        failures.warnings
    );

    let startup = failures.startup_warning().unwrap();
    assert!(!has_control_character(&startup), "{startup:?}");
    assert_eq!(
        startup,
        r"VA_TEST_FAIL_RENDERER=glow,bad\nerror: forged is set: simulating a renderer initialization failure (test hook)"
    );

    // The trimmed item is escaped, not the surrounding spaces.
    let padded = cli::parse_injected_failures(Some(OsStr::new(" \tx\u{1b}y ,wgpu")));
    assert_eq!(padded.renderers, [Renderer::Wgpu]);
    assert_eq!(
        padded.warnings,
        [r"ignoring unknown VA_TEST_FAIL_RENDERER value 'x\u{1b}y' (expected glow or wgpu)"]
    );
    assert_eq!(padded.raw.as_deref(), Some(" \tx\u{1b}y ,wgpu"));
    assert_eq!(
        padded.startup_warning().as_deref(),
        Some(
            r"VA_TEST_FAIL_RENDERER= \tx\u{1b}y ,wgpu is set: simulating a renderer initialization failure (test hook)"
        )
    );
}

/// A non-Unicode value is echoed as its lossy text, escaped too, in both variables' warnings;
/// `raw` keeps the unescaped lossy text.
#[test]
fn non_unicode_values_are_echoed_escaped() {
    let mut raw = std::ffi::OsString::from("a\n");
    raw.push(non_unicode_value());

    let resolution = cli::resolve_renderer(None, Some(&raw), BOTH, "linux", "x86_64").unwrap();
    // `\\n` is the two characters `\` and `n`; U+FFFD is printable, so escape_debug keeps it.
    assert_eq!(
        resolution.warnings,
        ["ignoring VA_RENDERER='a\\nw\u{FFFD}' (expected auto, glow or wgpu)"]
    );

    let failures = cli::parse_injected_failures(Some(&raw));
    assert!(failures.renderers.is_empty());
    assert_eq!(failures.raw.as_deref(), Some("a\nw\u{FFFD}"));
    assert_eq!(
        failures.warnings,
        [
            "ignoring VA_TEST_FAIL_RENDERER='a\\nw\u{FFFD}' (not valid Unicode; expected glow, wgpu or glow,wgpu)"
        ]
    );
    assert!(
        !failures.warnings.iter().any(|w| has_control_character(w)),
        "{:?}",
        failures.warnings
    );
}
