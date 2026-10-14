# Dependencies

Addition log of external dependencies: what was added, when, at which version, under which licence.
Where each one is used and on which machines it runs is in `IMPORT.md`.

Rules (architecture R6, m0-foundations.md §3.1 and §13): every crate is pinned **once, exactly (`=`)**
in the root `[workspace.dependencies]`, and members write `name = { workspace = true }`. Only crates
already used by a member are pinned (cargo-deny `unused = "deny"`). Versions were checked on crates.io
on 2026-10-13, skipping releases younger than about 14 days. A new dependency goes back to the
Architect, and gets a line here and an entry in `IMPORT.md` in the same change.

## Runtime
Linked into the shipped binaries `visual-assembler` and `va`.

| Package | Version | License | Purpose | Added on | Notes |
|---------|---------|---------|---------|----------|-------|
| eframe | =0.36.2 | MIT OR Apache-2.0 | GUI framework: window, egui, renderer integration, AccessKit | 2026-10-13 | `visual-assembler` only. `default-features = false` with `accesskit`, `default_fonts`, `wayland`, `x11`, so `links` (URL opening) and `persistence` are off. The app's features add `eframe/wgpu` and/or `eframe/glow`. |
| clap | =4.6.7 | MIT OR Apache-2.0 | Command-line parsing, help, usage errors and exit codes | 2026-10-13 | Feature `derive`. Used by `va`, `visual-assembler` and `xtask`. |
| serde | =1.0.229 | MIT OR Apache-2.0 | Derive for the JSON documents (`va.info/1`, bench line and report) | 2026-10-13 | Feature `derive`. Used by `va`, `visual-assembler` and `xtask`. |
| serde_json | =1.0.151 | MIT OR Apache-2.0 | `--json` output, bench JSON lines, `cargo metadata` parsing | 2026-10-13 | 1.0.152–1.0.154 were too recent on 2026-10-13 |
| windows-sys | =0.61.2 | MIT OR Apache-2.0 | `GetProcessTimes` and `GetProcessMemoryInfo` for the bench | 2026-10-13 | Windows only. Features `Win32_Foundation`, `Win32_System_ProcessStatus`, `Win32_System_Threading`. Same version as eframe's own graph. |
| libc | =0.2.189 | MIT OR Apache-2.0 | `getrusage` for the bench | 2026-10-13 | Linux only. Already in the graph through winit/wgpu. |
| raw-window-handle | =0.6.2 | MIT OR Apache-2.0 OR Zlib | Reads the window system actually used (Win32, Wayland, Xlib, Xcb) for the bench line | 2026-10-13 | Already in the graph |

## Development
Used by tests or by the task runner only; never shipped.

| Package | Version | License | Purpose | Added on | Notes |
|---------|---------|---------|---------|----------|-------|
| egui_kittest | =0.36.2 | MIT OR Apache-2.0 | Headless UI tests over the AccessKit tree | 2026-10-13 | Dev-dependency of `visual-assembler`, without its `wgpu` and `snapshot` features (no GPU needed) |
| anyhow | =1.0.104 | MIT OR Apache-2.0 | Error context in `xtask` | 2026-10-13 | `xtask` only. `xtask` also uses clap, serde and serde_json (above). |

## Tools (not linked)
| Tool | Version | License | Purpose | Added on | Notes |
|------|---------|---------|---------|----------|-------|
| Rust toolchain | 1.99.0 | MIT OR Apache-2.0 | Compiler, cargo, rustfmt, clippy | 2026-10-13 | `rust-toolchain.toml` |
| cargo-deny | 0.20.2 | MIT OR Apache-2.0 | Licence, ban, advisory and source policy (`.cargo/deny.toml`) | 2026-10-13 | `make install`; the CI `deny` job builds it from crates.io with `--locked` |
| actions/checkout | v7.0.1 (SHA-pinned) | MIT | CI checkout | 2026-10-13 | CI only |
| dtolnay/rust-toolchain | master of 2026-10-13 (SHA-pinned) | MIT | CI toolchain install | 2026-10-13 | CI only |
| Swatinem/rust-cache | v2.9.2 (SHA-pinned) | LGPL-3.0 | CI build cache (`check` job only) | 2026-10-13 | CI only, never linked or distributed (see the alerts below) |

## Notable transitive dependencies
Pulled by the crates above, recorded from `Cargo.lock` (m0-foundations.md §13.2). They are not pinned
in the manifests. An eframe bump moves them all.

| Package | Version | License | Pulled by | Role |
|---------|---------|---------|-----------|------|
| egui, egui-winit, egui-wgpu, egui_glow | 0.36.2 | MIT OR Apache-2.0 | eframe | UI library and its winit, wgpu and glow integrations |
| wgpu (wgpu-core, wgpu-hal, naga) | 30.0.1 | MIT OR Apache-2.0 | egui-wgpu | wgpu renderer: DX12, Vulkan and GL backends (Metal unused). Only default of the current build; the fallback after glow under m0 §19 (first choice on Windows aarch64), not implemented yet |
| glow | 0.17.0 | MIT OR Apache-2.0 OR Zlib | egui_glow | OpenGL renderer (`renderer-glow`). Opt-in in the current build; the first `auto` choice on Windows x64 and Linux under m0 §19, not implemented yet |
| winit | 0.30.13 | Apache-2.0 | eframe | Windowing; Wayland and X11 libraries are loaded at run time |
| glutin / glutin-winit | 0.32.3 / 0.5.0 | Apache-2.0 / MIT | eframe (glow) | GL context creation |
| accesskit / accesskit_winit / accesskit_windows / accesskit_unix | 0.24.1 / 0.32.2 / 0.32.1 / 0.21.1 | MIT OR Apache-2.0 (accesskit_winit: Apache-2.0) | eframe | Accessibility tree (UI Automation on Windows, AT-SPI over zbus on Linux) |
| zbus | 5.19.0 | MIT | accesskit_unix | D-Bus client for AT-SPI (Linux) |
| x11rb | 0.13.2 | MIT OR Apache-2.0 | winit, arboard (Linux) | X11 protocol client |
| epaint_default_fonts | 0.36.2 | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 | eframe (`default_fonts`) | Embedded UI fonts |
| kittest | 0.4.0 | MIT OR Apache-2.0 | egui_kittest (dev) | AccessKit test queries |

## License alerts
- **Font licences, data only.** `epaint_default_fonts` embeds fonts under OFL-1.1 and Ubuntu-font-1.0.
  `.cargo/deny.toml` allows these two licences for this crate only. Shipped binaries must carry the
  font licence notices (M8 packaging).
- **Attribution licences in the graph.** cargo-deny allows them: Apache-2.0-only crates (winit, glutin,
  accesskit_winit), BSL-1.0 (clipboard-win 5.4.1, error-code 3.4.0), ISC (libloading 0.8.9),
  Unicode-3.0 (unicode-ident 1.0.27) and Zlib (foldhash 0.2.0, zlib-rs 0.6.8). All are permissive. Binary
  distribution (M8) must ship the licence texts.
- **LGPL-3.0 CI action.** `Swatinem/rust-cache` runs only on GitHub's runners as a build tool. It is
  never linked into, nor distributed with, VisualAssembler, so no obligation reaches the product.
- **Planned, not added yet**: `winit` =0.30.13 (Apache-2.0, already in the graph) as a direct,
  Linux-only dependency with default features off and feature `x11`, for the X11-under-WSL rule
  (m0 §19.1, fix-wave row 19-1). No new licence and no new network surface: X11 over TCP is already
  an accepted risk in `SECURITY.md`.
- **Planned for later milestones** (§9 and §13.3 of the M0 spec): `option-ext` 0.2.0 (MPL-2.0, through
  `directories`, M4) and `notify` 8.2.0 (CC0-1.0, M4) will each need a per-crate cargo-deny
  exception. The SingleStepTests 680x0 vectors publish **no licence**. They will be fetched at test
  time into the gitignored `xtask/cache/` (M3), and never committed or shipped.
- **No GPL, LGPL, AGPL, SSPL, BUSL or non-commercial licence** in the Rust dependency graph.
  `make security` reported `licenses ok` on 2026-10-14.
