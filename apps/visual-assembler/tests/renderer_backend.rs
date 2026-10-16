//! QA (round post-measure, fix-wave row 2.4): the facts the Windows DX12 pin of `src/lib.rs`
//! (`pin_windows_wgpu_backend`; docs/specs/m0-foundations.md §17.13, docs/perf/M0-backend.md
//! §6.3 item 1) relies on.
//!
//! The pin is private and only runs inside `launch`, which opens a window, so it cannot run on a
//! headless runner. These tests need no window and no GPU. They fail when a dependency bump would
//! turn the pin into a silent no-op or into a launch failure:
//! - eframe's default `NativeOptions` must carry `WgpuSetup::CreateNew`, the only variant the pin
//!   rewrites. Any other default would skip the pin without a word, and wgpu would go back to its
//!   own pick (Vulkan on the reference machine, which failed G3);
//! - the DX12 backend must be compiled into wgpu on Windows. Otherwise `Backends::DX12` leaves no
//!   adapter and every Windows launch exits 3;
//! - wgpu must still name that backend `dx12`. The bench line, the xtask report and the row 2.4
//!   acceptance check ("plain `cargo xtask bench` reports backend `dx12`") compare that string.
//!
//! It also checks `visual_assembler::native_options`, the public builder of each renderer
//! attempt's options (§19.1, §19.2): the window, the renderer, the DX12 pin for every wgpu
//! attempt, wgpu's GL backend left out after a failed glow attempt, and the X11 hook; and
//! `visual_assembler::wgpu_backend_env_is_effective`, which decides when `WGPU_BACKEND` lifts the
//! DX12 pin and the GL exclusion (§19.9 S19-4), also under every row 19-8 value of the variable
//! (in a child process, since a test cannot set it in its own process).
//!
//! Only meaningful with the wgpu renderer: the glow-only build (`xtask ci` step 3) compiles this
//! file to an empty test binary.
#![cfg(feature = "renderer-wgpu")]
#![allow(clippy::unwrap_used)]

use eframe::egui_wgpu::WgpuSetup;
use eframe::wgpu::Backend;
use visual_assembler::cli::Renderer;

/// The pin only rewrites `WgpuSetup::CreateNew`. eframe's default must still be that variant,
/// whatever `WGPU_BACKEND` holds (eframe only reads the variable to fill the backend set).
#[test]
fn default_native_options_create_a_new_wgpu_instance() {
    let options = eframe::NativeOptions::default();
    assert!(
        matches!(options.wgpu_options.wgpu_setup, WgpuSetup::CreateNew(_)),
        "eframe's default wgpu setup is no longer `WgpuSetup::CreateNew`: the DX12 pin of \
         src/lib.rs would be skipped silently"
    );
}

/// The DX12 pin needs the DX12 backend in the wgpu build of the Windows binary.
#[cfg(windows)]
#[test]
fn dx12_backend_is_compiled_into_the_windows_build() {
    let compiled = eframe::wgpu::Instance::enabled_backend_features();
    assert!(
        compiled.contains(eframe::wgpu::Backends::DX12),
        "wgpu is built without DX12 ({compiled:?}): `Backends::DX12` would leave no adapter"
    );
}

/// The bench line reports `adapter.get_info().backend.to_str()`. The decision record and the
/// row 2.4 re-measurement expect `dx12` for the pinned backend, and the other names that §4.4
/// documents must stay stable too.
#[test]
fn backend_names_used_by_the_bench_line_are_stable() {
    assert_eq!(Backend::Dx12.to_str(), "dx12");
    assert_eq!(Backend::Vulkan.to_str(), "vulkan");
    assert_eq!(Backend::Gl.to_str(), "gl");
    assert_eq!(Backend::Metal.to_str(), "metal");
    assert_eq!(Backend::Noop.to_str(), "noop");
}

// ---------------------------------------------------------------------------------------------
// `visual_assembler::native_options` (docs/specs/m0-foundations.md §19.1, §19.2): the options of
// one renderer attempt, checked without a window. Tests cannot set `WGPU_BACKEND` (setting a
// variable is `unsafe` in edition 2024), so the backend assertions follow the inherited value.
// ---------------------------------------------------------------------------------------------

/// The wgpu backends of a set of options (`CreateNew` is asserted above to be eframe's default).
/// Used by the Windows pin test and the dual-build fallback test.
#[cfg(any(windows, feature = "renderer-glow"))]
fn wgpu_backends(options: &eframe::NativeOptions) -> eframe::wgpu::Backends {
    match &options.wgpu_options.wgpu_setup {
        WgpuSetup::CreateNew(setup) => setup.instance_descriptor.backends,
        other => panic!("unexpected wgpu setup: {other:?}"),
    }
}

/// eframe's own backend choice: `WGPU_BACKEND` when set, else every primary backend plus GL.
#[cfg(any(windows, feature = "renderer-glow"))]
fn eframe_default_backends() -> eframe::wgpu::Backends {
    wgpu_backends(&eframe::NativeOptions::default())
}

/// Asserts the M0 window and exactly `expected` in the options of an attempt with `renderer`.
fn assert_window_and_renderer(renderer: Renderer, expected: eframe::Renderer) {
    let options = visual_assembler::native_options(renderer, false, false).unwrap();
    assert_eq!(options.renderer, expected);
    assert_eq!(options.viewport.title.as_deref(), Some("VisualAssembler"));
    assert_eq!(options.viewport.app_id.as_deref(), Some("visualassembler"));
    assert_eq!(
        options.viewport.inner_size,
        Some(eframe::egui::vec2(1280.0, 800.0))
    );
    assert!(options.centered);
}

/// Every attempt gets the M0 window and exactly the requested renderer.
#[test]
fn native_options_carry_the_window_and_the_renderer() {
    assert_window_and_renderer(Renderer::Wgpu, eframe::Renderer::Wgpu);
    #[cfg(feature = "renderer-glow")]
    assert_window_and_renderer(Renderer::Glow, eframe::Renderer::Glow);
}

/// The inherited `WGPU_BACKEND` value (`None` when unset) and whether it is effective
/// (§19.9 S19-4).
#[cfg(any(windows, feature = "renderer-glow"))]
fn inherited_wgpu_backend() -> (Option<std::ffi::OsString>, bool) {
    let raw = std::env::var_os("WGPU_BACKEND");
    let effective = visual_assembler::wgpu_backend_env_is_effective(raw.as_deref());
    (raw, effective)
}

/// §19.9 S19-4: `WGPU_BACKEND` is effective only when wgpu takes a non-empty backend set from it:
/// valid Unicode and `Backends::from_comma_list` not empty (case-insensitive, items trimmed,
/// unknown items skipped).
#[test]
fn wgpu_backend_env_is_effective_only_for_a_non_empty_backend_set() {
    use std::ffi::OsStr;
    use visual_assembler::wgpu_backend_env_is_effective as effective;

    assert!(!effective(None));
    for not_effective in ["", "  ", ",", "bogus"] {
        assert!(
            !effective(Some(OsStr::new(not_effective))),
            "{not_effective:?}"
        );
    }
    for is_effective in ["vulkan", "gl", "DX12", " vk , bogus"] {
        assert!(
            effective(Some(OsStr::new(is_effective))),
            "{is_effective:?}"
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;
        assert!(!effective(Some(OsStr::from_bytes(b"\xff"))));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt as _;
        let unpaired_surrogate = std::ffi::OsString::from_wide(&[0xD800]);
        assert!(!effective(Some(&unpaired_surrogate)));
    }
}

/// §19.1: the DX12 pin applies to every wgpu attempt, forced or fallback, unless `WGPU_BACKEND`
/// is effective (§19.9 S19-4): unset or set but not effective gives DX12, effective gives
/// eframe's choice.
#[cfg(windows)]
#[test]
fn dx12_pin_applies_to_every_wgpu_attempt() {
    let (raw, effective) = inherited_wgpu_backend();
    for after_glow_failure in [false, true] {
        let options =
            visual_assembler::native_options(Renderer::Wgpu, false, after_glow_failure).unwrap();
        let expected = if effective {
            eframe_default_backends()
        } else {
            eframe::wgpu::Backends::DX12
        };
        assert_eq!(
            wgpu_backends(&options),
            expected,
            "after_glow_failure={after_glow_failure} WGPU_BACKEND={raw:?}"
        );
    }
}

/// A wgpu attempt after a failed glow attempt leaves out wgpu's GL backend (the GL context of the
/// glow attempt may still be current on the thread), unless `WGPU_BACKEND` is effective
/// (§19.9 S19-4). A first attempt keeps eframe's choice (outside the Windows DX12 pin).
#[cfg(feature = "renderer-glow")]
#[test]
fn wgpu_gl_backend_is_left_out_after_a_glow_failure() {
    let (raw, effective) = inherited_wgpu_backend();
    let fallback = visual_assembler::native_options(Renderer::Wgpu, false, true).unwrap();
    let first = visual_assembler::native_options(Renderer::Wgpu, false, false).unwrap();
    if effective {
        // wgpu's own choice from the variable, on both attempts.
        assert_eq!(
            wgpu_backends(&fallback),
            eframe_default_backends(),
            "{raw:?}"
        );
        assert_eq!(wgpu_backends(&first), eframe_default_backends(), "{raw:?}");
        return;
    }

    // Unset, or set but not effective: no GL after a glow failure.
    assert!(
        !wgpu_backends(&fallback).contains(eframe::wgpu::Backends::GL),
        "{:?} WGPU_BACKEND={raw:?}",
        wgpu_backends(&fallback)
    );
    // An empty or unrecognized value leaves eframe's instance with no backend on Linux (wgpu's own
    // reading, accepted by S19-4), so the set is non-empty only when the variable is unset.
    if raw.is_none() {
        assert!(!wgpu_backends(&fallback).is_empty());
    }
    if cfg!(windows) {
        assert_eq!(
            wgpu_backends(&fallback),
            eframe::wgpu::Backends::DX12,
            "{raw:?}"
        );
    } else {
        assert_eq!(wgpu_backends(&first), eframe_default_backends(), "{raw:?}");
        if raw.is_none() {
            assert!(wgpu_backends(&first).contains(eframe::wgpu::Backends::GL));
        }
        assert_eq!(
            wgpu_backends(&fallback),
            eframe_default_backends() - eframe::wgpu::Backends::GL,
            "{raw:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// QA (§19.9 re-check, row 19-8 b): the two env-dependent tests above only see the inherited
// `WGPU_BACKEND`, which is unset on CI, so a return to `var_os(..).is_none()` in `src/lib.rs`
// would still pass there. This test re-runs them in a child copy of this test binary under every
// value of the row 19-8 list. Spawning a child with a variable is safe; setting one in this
// process is `unsafe` in edition 2024.
// ---------------------------------------------------------------------------------------------

/// The env-dependent tests of this file that this build compiles.
const ENV_DEPENDENT_TESTS: &[&str] = &[
    #[cfg(windows)]
    "dx12_pin_applies_to_every_wgpu_attempt",
    #[cfg(feature = "renderer-glow")]
    "wgpu_gl_backend_is_left_out_after_a_glow_failure",
];

/// A `WGPU_BACKEND` value that is not valid Unicode (wgpu ignores it, so it is never effective).
fn non_unicode_wgpu_backend() -> std::ffi::OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt as _;
        std::ffi::OsString::from_vec(vec![0xff])
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt as _;
        std::ffi::OsString::from_wide(&[0xD800])
    }
}

/// §19.9 S19-4: under every row 19-8 value of `WGPU_BACKEND` (unset; set but not effective: empty,
/// blank, a lone comma, unrecognized, non-Unicode; effective: `vulkan`, `gl`, `DX12`,
/// ` vk , bogus`), the attempt options follow the helper: the DX12 pin (Windows) and the GL
/// exclusion after a glow failure step aside only for an effective value. Each value runs the
/// env-dependent tests in a child process and expects all of them to run and pass.
///
/// A non-effective value is what tells the fix apart from `is_none()`: on Windows, eframe's own
/// choice for `bogus` or a non-Unicode value is not DX12; on Linux, eframe keeps GL for a
/// non-Unicode value (wgpu ignores it), which the old test let back into the fallback.
#[test]
fn attempt_options_follow_every_kind_of_wgpu_backend_value() {
    if ENV_DEPENDENT_TESTS.is_empty() {
        return; // wgpu-only Linux build: neither the pin nor the exclusion exists.
    }
    let values: [Option<std::ffi::OsString>; 10] = [
        None,
        Some("".into()),
        Some("  ".into()),
        Some(",".into()),
        Some("bogus".into()),
        Some(non_unicode_wgpu_backend()),
        Some("vulkan".into()),
        Some("gl".into()),
        Some("DX12".into()),
        Some(" vk , bogus".into()),
    ];
    let expected = format!(
        "test result: ok. {} passed; 0 failed;",
        ENV_DEPENDENT_TESTS.len()
    );
    for value in values {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child
            .args(["--exact", "--test-threads=1"])
            .args(ENV_DEPENDENT_TESTS);
        match &value {
            None => child.env_remove("WGPU_BACKEND"),
            Some(raw) => child.env("WGPU_BACKEND", raw),
        };
        let output = child.output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success() && stdout.contains(&expected),
            "WGPU_BACKEND={value:?}: expected `{expected}`\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
    }
}

/// §19.1: the X11 event-loop hook is installed exactly when the policy forces X11 (Linux), and
/// never elsewhere (winit's X11 backend is Linux-only).
#[test]
fn x11_hook_follows_the_window_system_policy() {
    assert_x11_hook(Renderer::Wgpu);
    #[cfg(feature = "renderer-glow")]
    assert_x11_hook(Renderer::Glow);
}

/// Asserts that only a forced X11 policy installs the event-loop hook, and only on Linux.
fn assert_x11_hook(renderer: Renderer) {
    let forced = visual_assembler::native_options(renderer, true, false).unwrap();
    assert_eq!(
        forced.event_loop_builder.is_some(),
        cfg!(target_os = "linux"),
        "{renderer}"
    );
    let default = visual_assembler::native_options(renderer, false, false).unwrap();
    assert!(default.event_loop_builder.is_none(), "{renderer}");
}

/// QA (§19 audit round 1): every (renderer, X11, after-glow-failure) combination keeps the M0
/// window and exactly the requested renderer; the X11 hook follows `force_x11` alone (Linux);
/// and `run_and_return` stays on. §19.2 depends on it: with `run_and_return = false`, eframe
/// exits the process when its event loop ends and never returns the initialization error, so no
/// in-process fallback could happen.
#[test]
fn every_attempt_option_combination_keeps_the_fallback_preconditions() {
    let renderers = [
        (Renderer::Wgpu, eframe::Renderer::Wgpu),
        #[cfg(feature = "renderer-glow")]
        (Renderer::Glow, eframe::Renderer::Glow),
    ];
    for (renderer, expected) in renderers {
        for force_x11 in [false, true] {
            for after_glow_failure in [false, true] {
                let case = format!("{renderer} x11={force_x11} after={after_glow_failure}");
                let options =
                    visual_assembler::native_options(renderer, force_x11, after_glow_failure)
                        .unwrap();
                assert_eq!(options.renderer, expected, "{case}");
                assert!(options.run_and_return, "{case}");
                assert_eq!(
                    options.viewport.title.as_deref(),
                    Some("VisualAssembler"),
                    "{case}"
                );
                assert_eq!(
                    options.event_loop_builder.is_some(),
                    force_x11 && cfg!(target_os = "linux"),
                    "{case}"
                );
            }
        }
    }
}
