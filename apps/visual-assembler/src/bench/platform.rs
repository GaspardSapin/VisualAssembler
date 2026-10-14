//! Renderer, GPU adapter and window-system detection, plus WSL detection, for the bench line.
//!
//! [`RendererInfo::from_frame`] reads what eframe actually created (docs/specs/m0-foundations.md
//! §4.4): the wgpu adapter info or the GL strings, and the raw window handle kind (so a winit
//! fallback from Wayland to X11 is reported as it really happened).
//!
//! This file is on the unsafe allow-list (`UNSAFE_ALLOWLIST` in `xtask/src/layering.rs`) for a
//! single block: the `glGetString` query of the glow renderer.

use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};

/// Description of the renderer that eframe actually runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RendererInfo {
    /// `"wgpu"` or `"glow"`; `"unknown"` if the frame exposes neither (headless frames).
    pub renderer: &'static str,
    /// wgpu backend (`dx12`, `vulkan`, `gl`, `metal`, `noop`), `"gl"` for glow, else `"unknown"`.
    pub backend: &'static str,
    /// Adapter name (wgpu `AdapterInfo::name`, GL `GL_RENDERER`); `None` if empty or unavailable.
    pub adapter: Option<String>,
    /// Driver (wgpu `driver driver_info`, GL `GL_VERSION`); `None` if empty or unavailable.
    pub driver: Option<String>,
    /// `win32`, `wayland`, `xlib`, `xcb`, or `unknown` for any other handle kind or an error.
    pub window_system: &'static str,
}

impl RendererInfo {
    /// Reads the renderer, adapter, driver and window system from an eframe frame.
    pub fn from_frame(frame: &eframe::Frame) -> Self {
        let window_system = window_system(frame);

        #[cfg(feature = "renderer-wgpu")]
        if let Some(render_state) = frame.wgpu_render_state() {
            let info = render_state.adapter.get_info();
            return Self {
                renderer: "wgpu",
                backend: info.backend.to_str(),
                adapter: non_empty(info.name),
                driver: non_empty(format!("{} {}", info.driver, info.driver_info)),
                window_system,
            };
        }

        #[cfg(feature = "renderer-glow")]
        if let Some(gl) = frame.gl() {
            return Self {
                renderer: "glow",
                backend: "gl",
                adapter: gl_string(gl, eframe::glow::RENDERER),
                driver: gl_string(gl, eframe::glow::VERSION),
                window_system,
            };
        }

        Self {
            renderer: "unknown",
            backend: "unknown",
            adapter: None,
            driver: None,
            window_system,
        }
    }
}

/// Returns true when running under Windows Subsystem for Linux.
///
/// On Linux: `/proc/sys/kernel/osrelease` contains `microsoft` or `WSL` (case-insensitive). An
/// unreadable file means "not WSL". On every other OS: `false`.
pub fn is_wsl() -> bool {
    #[cfg(target_os = "linux")]
    {
        match std::fs::read_to_string("/proc/sys/kernel/osrelease") {
            Ok(release) => {
                let release = release.to_lowercase();
                release.contains("microsoft") || release.contains("wsl")
            }
            Err(_) => false,
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Maps the kind of the frame's raw window handle to its bench name.
fn window_system(frame: &eframe::Frame) -> &'static str {
    match frame.window_handle().map(|handle| handle.as_raw()) {
        Ok(RawWindowHandle::Win32(_)) => "win32",
        Ok(RawWindowHandle::Wayland(_)) => "wayland",
        Ok(RawWindowHandle::Xlib(_)) => "xlib",
        Ok(RawWindowHandle::Xcb(_)) => "xcb",
        Ok(_) | Err(_) => "unknown",
    }
}

/// Trims `text` and returns it, or `None` when nothing is left.
fn non_empty(text: String) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// Reads a `glGetString` value (`GL_RENDERER`, `GL_VERSION`) from the glow context.
///
/// glow panics when the driver returns a null pointer or a non-UTF-8 string; that panic is
/// caught (its message still reaches stderr through the panic hook) and the value becomes
/// `None`, reported as `null`, instead of aborting the whole bench run.
#[cfg(feature = "renderer-glow")]
#[allow(unsafe_code)]
fn gl_string(gl: &eframe::glow::Context, parameter: u32) -> Option<String> {
    use eframe::glow::HasContext as _;

    let queried = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: eframe calls `App::logic`/`App::ui` with the glow context of the root viewport
        // current on this thread (it is the only GL context of the process), and `parameter` is
        // one of the `glGetString` enums `GL_RENDERER` / `GL_VERSION`.
        unsafe { gl.get_parameter_string(parameter) }
    }));
    match queried {
        Ok(text) => non_empty(text),
        Err(_) => None,
    }
}
