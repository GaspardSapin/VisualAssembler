# External imports

One entry per external dependency: what it does here, where it is used, and on which machines it is
known to work. This complements `DEPENDANCE.md`, the addition log with versions and licences.

Legend: **yes** = verified by a real run · **compiled only** = built, never executed · **limited** =
works under conditions (stated) · **no** = known not to work · **unverified** = neither run nor checked
· **n/a** = not relevant.

What "yes" rests on (2026-10-14; details in `docs/perf/M0-backend.md`):
- **Windows 11 x64**, the reference machine (Ryzen AI 7 350, integrated AMD Radeon 860M): build,
  `make test`, `make lint`, `make security`, and GUI bench runs with wgpu (DX12 and Vulkan) and with
  glow (OpenGL through WGL).
- **Linux x64**: Ubuntu 26.04 under WSL2, which is the only Linux tested. Build, `make test`,
  `make lint`, and GUI bench runs under WSLg with **software rendering only** (Mesa llvmpipe/lavapipe).
  Native Linux desktops were not tested.
- **ARM64**: `visual-assembler` and `va` cross-built for `aarch64-unknown-linux-gnu` from WSL (compiled
  only). Windows ARM64 and Linux ARM64 CI runners are configured but have not run yet.
- macOS and Apple Silicon are a non-goal: never built, never claimed. No NVIDIA or Intel GPU was
  tested. No crate here uses GPU compute (ROCm/CUDA: n/a).

---

### eframe
- **License**: MIT OR Apache-2.0
- **Used for**: the GUI window, the event loop, egui integration, renderer selection (wgpu or glow), AccessKit.
- **Used in**: `apps/visual-assembler/src/lib.rs`, `apps/visual-assembler/src/spike.rs`,
  `apps/visual-assembler/src/bench/recorder.rs`, `apps/visual-assembler/src/bench/platform.rs`;
  tests `apps/visual-assembler/tests/spike.rs` and `apps/visual-assembler/tests/renderer_backend.rs`
- **Hardware**: x86-64 yes · ARM64 compiled only (Linux), unverified (Windows) · Apple Silicon
  unverified · Intel Mac unverified · AMD GPU yes (Radeon 860M: DX12, Vulkan, OpenGL 3.3 on Windows) ·
  NVIDIA GPU unverified · software rendering yes (llvmpipe/lavapipe under WSLg)
- **OS**: Windows yes (11 x64) · Linux limited (WSLg X11 yes; WSLg Wayland: wgpu crashes the
  compositor, glow works; native desktops unverified) · macOS unverified
- **Notes**: default features off (no URL opening, no persistence). Wayland and X11 libraries are
  loaded at run time, so building on Linux needs no `-dev` package. On Windows the app pins wgpu to
  DX12 unless `WGPU_BACKEND` is set.

### clap
- **License**: MIT OR Apache-2.0
- **Used for**: command-line grammars, `--help`/`--version`, usage errors (exit 2).
- **Used in**: `apps/va/src/cli.rs`, `apps/va/src/main.rs`, `apps/visual-assembler/src/cli.rs`,
  `apps/visual-assembler/src/lib.rs`, `xtask/src/main.rs`, `xtask/src/bench/mod.rs`, `xtask/src/ci.rs`,
  `xtask/src/docs.rs`, `xtask/src/hooks.rs`
- **Hardware**: x86-64 yes · ARM64 compiled only (Linux, in `va` and `visual-assembler`) · Apple
  Silicon unverified · Intel Mac unverified · GPU n/a
- **OS**: Windows yes · Linux yes (WSL Ubuntu 26.04) · macOS unverified
- **Notes**: feature `derive`. clap 4.6.7's exit codes (0 for help/version, 2 for errors) are part of
  the CLI contract and are tested.

### serde
- **License**: MIT OR Apache-2.0
- **Used for**: `Serialize` derives of every JSON document (`va.info/1`, `va.bench.run/1`, `va.bench.report/1`).
- **Used in**: `apps/va/src/commands/info.rs`, `apps/visual-assembler/src/bench/report.rs`,
  `xtask/src/bench/mod.rs`, `xtask/src/bench/report.rs`, `xtask/src/bench/stats.rs`
- **Hardware**: x86-64 yes · ARM64 compiled only (Linux) · Apple Silicon unverified · Intel Mac
  unverified · GPU n/a
- **OS**: Windows yes · Linux yes · macOS unverified
- **Notes**: feature `derive`. TOML files (M4) will reuse it.

### serde_json
- **License**: MIT OR Apache-2.0
- **Used for**: writing the JSON lines and reports; parsing `cargo metadata`, cargo's build messages
  and the GUI's bench lines in xtask.
- **Used in**: `apps/va/src/commands/info.rs`, `apps/visual-assembler/src/bench/report.rs`,
  `xtask/src/bench/report.rs`, `xtask/src/bench/runner.rs`, `xtask/src/layering.rs`, `xtask/src/locate.rs`
- **Hardware**: x86-64 yes · ARM64 compiled only (Linux) · Apple Silicon unverified · Intel Mac
  unverified · GPU n/a
- **OS**: Windows yes · Linux yes · macOS unverified
- **Notes**: none.

### windows-sys
- **License**: MIT OR Apache-2.0
- **Used for**: process CPU time and peak memory of the GUI in bench mode (`GetProcessTimes`,
  `GetProcessMemoryInfo`). Unsafe FFI, allow-listed.
- **Used in**: `apps/visual-assembler/src/bench/process_windows.rs`
- **Hardware**: x86-64 yes · ARM64 unverified (Windows ARM64 CI not run yet) · GPU n/a
- **OS**: Windows yes · Linux n/a · macOS n/a
- **Notes**: features `Win32_Foundation`, `Win32_System_ProcessStatus`, `Win32_System_Threading`.
  The same 0.61.2 is already in eframe's graph, alongside older 0.52/0.59/0.60 copies pulled by other
  crates.

### libc
- **License**: MIT OR Apache-2.0
- **Used for**: process CPU time of the GUI in bench mode (`getrusage(RUSAGE_SELF)`). Unsafe FFI,
  allow-listed. Peak memory comes from `/proc/self/status` through `std::fs`.
- **Used in**: `apps/visual-assembler/src/bench/process_linux.rs`
- **Hardware**: x86-64 yes · ARM64 compiled only · GPU n/a
- **OS**: Linux yes (WSL Ubuntu 26.04) · Windows n/a · macOS n/a
- **Notes**: declared for `cfg(target_os = "linux")` only.

### raw-window-handle
- **License**: MIT OR Apache-2.0 OR Zlib
- **Used for**: reporting the window system that winit really used (`win32`, `wayland`, `xlib`,
  `xcb`) in the bench line.
- **Used in**: `apps/visual-assembler/src/bench/platform.rs`
- **Hardware**: x86-64 yes · ARM64 compiled only (Linux) · Apple Silicon unverified · Intel Mac
  unverified · GPU n/a
- **OS**: Windows yes (`win32`) · Linux yes (`wayland` and `xlib` seen under WSLg) · macOS unverified
- **Notes**: a GUI crate under rule R3: allowed in `visual-assembler` only.

### egui_kittest
- **License**: MIT OR Apache-2.0
- **Used for**: headless UI tests that query the AccessKit tree; no window, no GPU.
- **Used in**: `apps/visual-assembler/tests/spike.rs` (dev-dependency)
- **Hardware**: x86-64 yes · ARM64 unverified (CI runners not run yet) · Apple Silicon unverified ·
  Intel Mac unverified · GPU n/a (used without its `wgpu` and `snapshot` features)
- **OS**: Windows yes · Linux yes · macOS unverified
- **Notes**: image and snapshot tests are not used until a GPU-free renderer is decided (architecture §8).

### anyhow
- **License**: MIT OR Apache-2.0
- **Used for**: error context chains in the task runner (`error: {e:#}`).
- **Used in**: `xtask/src/main.rs`, `xtask/src/ci.rs`, `xtask/src/docs.rs`, `xtask/src/exec.rs`,
  `xtask/src/hooks.rs`, `xtask/src/layering.rs`, `xtask/src/locate.rs`, `xtask/src/bench/mod.rs`,
  `xtask/src/bench/report.rs`, `xtask/src/bench/runner.rs`
- **Hardware**: x86-64 yes · ARM64 unverified (xtask is not cross-built) · Apple Silicon unverified ·
  Intel Mac unverified · GPU n/a
- **OS**: Windows yes · Linux yes · macOS unverified
- **Notes**: developer tooling only; never in a shipped binary.

---

## Notable transitive imports
Not declared by any manifest; they arrive through eframe or egui_kittest. They are listed because
they decide GPU, window-system and accessibility support.

### wgpu 30.0.1 (with egui-wgpu 0.36.2)
- **License**: MIT OR Apache-2.0
- **Used for**: the `renderer-wgpu` renderer. It is the only default feature of the current build.
  Under the user's decision (m0-foundations.md §19, architecture §9.8) it becomes the automatic
  fallback after glow, and the first choice on Windows aarch64. That change is not implemented yet
  (fix-wave row 19-2).
- **Used in**: through eframe; backend pin in `apps/visual-assembler/src/lib.rs` (`pin_windows_wgpu_backend`).
- **Hardware**: AMD Radeon 860M yes on Windows (DX12 chosen; Vulkan works but painted idle frames) ·
  software Vulkan (lavapipe) yes under WSLg X11 · NVIDIA unverified · Intel GPU unverified ·
  ARM64 compiled only (Linux) · Windows ARM64 unverified (DX12 expected native)
- **OS**: Windows yes · Linux limited (WSLg X11 yes; **WSLg Wayland no**: the compositor crashed in
  3 of 5 launches, weston 9.0.0 / WSLg 1.0.73.2) · macOS unverified
- **Notes**: wgpu uses about 250 MiB more peak RSS than glow on the shared-memory Radeon 860M
  (allocator block sizes; `docs/perf/M0-backend.md` §3.4).

### glow 0.17.0 (with egui_glow 0.36.2, glutin 0.32.3, glutin-winit 0.5.0)
- **License**: glow MIT OR Apache-2.0 OR Zlib; glutin Apache-2.0; glutin-winit MIT
- **Used for**: the `renderer-glow` renderer, and the GL string query of the bench. In the current
  build it is opt-in (`--no-default-features --features renderer-glow`) and CI lints it. Under the
  user's decision (m0 §19, architecture §9.8) it becomes the first `auto` choice on Windows x64 and
  Linux, and the fallback on Windows aarch64 (not implemented yet).
- **Used in**: through eframe; `apps/visual-assembler/src/bench/platform.rs` (`gl_string`).
- **Hardware**: AMD Radeon 860M yes on Windows (OpenGL 3.3 core through WGL) · llvmpipe yes under
  WSLg (X11 and Wayland) · NVIDIA unverified · ARM64 compiled only (Linux) · Windows ARM64 unverified
  (no native OpenGL driver guaranteed)
- **OS**: Windows yes · Linux yes under WSLg · macOS unverified

### winit 0.30.13
- **License**: Apache-2.0
- **Used for**: windows and input. Wayland and X11 libraries are loaded at run time (`dlopen`).
- **Used in**: through eframe.
- **Hardware**: n/a beyond the OS
- **OS**: Windows yes · Linux limited (WSLg X11 yes; WSLg Wayland works with glow but wgpu crashes
  the compositor; native desktops unverified) · macOS unverified
- **Notes**: under WSLg, `env -u WAYLAND_DISPLAY` makes winit use X11. m0 §19.1 makes winit a direct,
  Linux-only dependency (`=0.30.13`, default features off, feature `x11`) so that the app can force
  X11 under WSL unless `VA_WAYLAND=1`. It is not added yet (fix-wave rows 19-1 and 19-2).

### accesskit 0.24.1 (accesskit_winit 0.32.2, accesskit_windows 0.32.1, accesskit_unix 0.21.1) and zbus 5.19.0
- **License**: MIT OR Apache-2.0 (accesskit_winit: Apache-2.0; zbus: MIT)
- **Used for**: the accessibility tree: UI Automation on Windows, AT-SPI over D-Bus (zbus) on Linux.
  egui_kittest queries the same tree in tests.
- **Used in**: through eframe.
- **OS**: Windows yes (headless tests and the running window) · Linux yes (tests) · screen readers
  were never tested (unverified)
- **Notes**: zbus connects over TCP if the environment's bus address is `tcp:` (accepted risk,
  `SECURITY.md`).

### x11rb 0.13.2
- **License**: MIT OR Apache-2.0
- **Used for**: the X11 protocol, for winit and for the clipboard (arboard).
- **OS**: Linux yes under WSLg X11
- **Notes**: connects over TCP when `DISPLAY=host:N` (accepted risk, `SECURITY.md`).

### epaint_default_fonts 0.36.2
- **License**: (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0
- **Used for**: the embedded UI fonts (eframe feature `default_fonts`).
- **OS**: every OS (data only)
- **Notes**: the only crate allowed OFL-1.1 and Ubuntu-font-1.0 by `.cargo/deny.toml`.

### kittest 0.4.0
- **License**: MIT OR Apache-2.0
- **Used for**: AccessKit tree queries behind egui_kittest (tests only).
- **OS**: Windows yes · Linux yes

---

## Summary
| Import | License | Windows x64 | Linux x64 | ARM64 | macOS | GPU |
|--------|---------|-------------|-----------|-------|-------|-----|
| eframe | MIT OR Apache-2.0 | yes | limited (WSLg X11; Wayland wgpu no) | compiled only (Linux) | unverified | AMD iGPU yes, software yes, NVIDIA unverified |
| clap | MIT OR Apache-2.0 | yes | yes | compiled only (Linux) | unverified | n/a |
| serde | MIT OR Apache-2.0 | yes | yes | compiled only (Linux) | unverified | n/a |
| serde_json | MIT OR Apache-2.0 | yes | yes | compiled only (Linux) | unverified | n/a |
| windows-sys | MIT OR Apache-2.0 | yes | n/a | unverified | n/a | n/a |
| libc | MIT OR Apache-2.0 | n/a | yes | compiled only | n/a | n/a |
| raw-window-handle | MIT OR Apache-2.0 OR Zlib | yes | yes | compiled only (Linux) | unverified | n/a |
| egui_kittest | MIT OR Apache-2.0 | yes | yes | unverified | unverified | n/a |
| anyhow | MIT OR Apache-2.0 | yes | yes | unverified | unverified | n/a |
| wgpu | MIT OR Apache-2.0 | yes (DX12) | limited (WSLg X11 only) | compiled only (Linux) | unverified | AMD iGPU yes, NVIDIA unverified |
| glow | MIT OR Apache-2.0 OR Zlib | yes | yes (WSLg) | compiled only (Linux) | unverified | AMD iGPU yes, llvmpipe yes |
| winit | Apache-2.0 | yes | limited | compiled only (Linux) | unverified | n/a |
