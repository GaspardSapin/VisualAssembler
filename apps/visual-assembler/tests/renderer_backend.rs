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
//! Only meaningful with the wgpu renderer: the glow-only build (`xtask ci` step 3) compiles this
//! file to an empty test binary.
#![cfg(feature = "renderer-wgpu")]

use eframe::egui_wgpu::WgpuSetup;
use eframe::wgpu::Backend;

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
