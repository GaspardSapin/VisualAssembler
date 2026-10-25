# Dependencies

Addition log of external dependencies: what was added, when, at which version, under which licence.
Where each one is used and on which machines it runs is in `IMPORT.md`.

Rules (architecture R6, m0-foundations.md §3.1 and §13): every crate is pinned **once, exactly (`=`)**
in the root `[workspace.dependencies]`, and members write `name = { workspace = true }`. Every
`[workspace.dependencies]` pin must be declared by at least one member with `workspace = true`;
`scripts/check_project_docs.py` check 3b enforces it. cargo-deny's `unused` rule only proves the
pinned version is in the resolved graph. Versions were checked on crates.io on 2026-10-13 (M0),
2026-10-17 (M1) and 2026-10-24 (M4, m4-editor-shell.md §8.3), skipping releases younger than about
14 days. A new dependency goes back to the Architect, and gets a line
here and an entry in `IMPORT.md` in the same change. Embedded third-party data (fonts) is recorded
here too, because cargo-deny never sees it.

## Runtime
Linked into the shipped binaries `visual-assembler` and `va`.

| Package | Version | License | Purpose | Added on | Notes |
|---------|---------|---------|---------|----------|-------|
| eframe | =0.36.2 | MIT OR Apache-2.0 | GUI framework: window, egui, renderer integration, AccessKit | 2026-10-13 | `visual-assembler` only. `default-features = false` with `accesskit`, `default_fonts`, `wayland`, `x11`, so `links` (URL opening) and `persistence` are off. The app's default features add both `eframe/glow` and `eframe/wgpu` (m0 §19.1). |
| clap | =4.6.7 | MIT OR Apache-2.0 | Command-line parsing, help, usage errors and exit codes | 2026-10-13 | Feature `derive`. Used by `va`, `visual-assembler` and `xtask`. |
| serde | =1.0.229 | MIT OR Apache-2.0 | Derive for the JSON documents (`va.info/1`, `va.lex/1`, `va.isa/1`, `va.isa.list/1`, `va.error/1`, bench line and report) | 2026-10-13 | Feature `derive`. Used by `va`, `visual-assembler` and `xtask`; since M4 WP0 (2026-10-24) also declared by `va-workspace`, for the TOML files with `toml` (m4 §8.2), used from wave 2. |
| serde_json | =1.0.151 | MIT OR Apache-2.0 | `--json` output, bench JSON lines, `cargo metadata` parsing | 2026-10-13 | 1.0.152–1.0.154 were too recent on 2026-10-13 |
| windows-sys | =0.61.2 | MIT OR Apache-2.0 | `GetProcessTimes` and `GetProcessMemoryInfo` for the bench | 2026-10-13 | Windows only. Features `Win32_Foundation`, `Win32_System_ProcessStatus`, `Win32_System_Threading`. Same version as eframe's own graph. |
| libc | =0.2.189 | MIT OR Apache-2.0 | `getrusage` for the bench | 2026-10-13 | Linux only. Already in the graph through winit/wgpu. |
| raw-window-handle | =0.6.2 | MIT OR Apache-2.0 OR Zlib | Reads the window system actually used (Win32, Wayland, Xlib, Xcb) for the bench line | 2026-10-13 | Already in the graph |
| fluent-bundle | =0.16.0 | Apache-2.0 OR MIT | Fluent message formatting for diagnostics and instruction docs: plurals, selectors, typed arguments (architecture §5, m1 §6.2) | 2026-10-17 | `va-workspace` (the localizer, linked into `va`); also a dev-dependency of `va-arch-m68k` (locale tests). Latest release on 2026-10-17, MSRV 1.67. Uses `fluent_bundle::concurrent::FluentBundle`. m1 §8. |
| unic-langid | =0.9.6 | MIT OR Apache-2.0 | `LanguageIdentifier`, the type `FluentBundle::new` requires | 2026-10-17 | `va-workspace` only, default features. Latest release on 2026-10-17. m1 §8. |
| winit | =0.30.13 | Apache-2.0 | X11 event loop under WSL: `EventLoopBuilderExtX11::with_x11`, so every renderer uses X11 unless `VA_WAYLAND=1` | 2026-10-15 | Linux only (`[target.'cfg(target_os = "linux")'.dependencies]` of `visual-assembler`). `default-features = false`, feature `x11`. The version eframe already pulls, so the hook's builder type is eframe's; no new package and no new licence. m0 §19.1, fix-wave rows 19-1 and 19-2. |
| ropey | =1.6.1 | MIT | Rope text buffer of the editor: O(log n) edits and line index on 100,000-line files, O(1) snapshots for the worker threads | 2026-10-24 | `va-editor` only. `default-features = false`, features `simd` and `cr_lines` (LF, CRLF and lone CR are breaks; U+0085 and U+2028 are not, matching `va_core::source::LineIndex`). Latest stable (2.0 is beta). Approved by the user on 2026-10-24, m4 §8.3. |
| regex | =1.13.1 | MIT OR Apache-2.0 | Find and replace in the editor with a guaranteed linear-time engine and size limits | 2026-10-24 | `va-editor` only. `default-features = false`, features `std`, `perf`, `unicode-case`, `unicode-perl` (the other Unicode tables are left out). Approved by the user on 2026-10-24, m4 §8.3. |
| toml | =1.1.8 | MIT OR Apache-2.0 | Parsing, with spans, of `settings.toml`, `keybindings.toml`, `.visualasm/project.toml`, `session.toml` and the backup index; writing the session | 2026-10-24 | `va-workspace`, a normal dependency, so **`va` ships it too** (it reads no TOML), with toml_parser, toml_datetime, toml_writer, serde_spanned and winnow: `cargo tree -p va -e normal` shows them on 2026-10-25 (m4 §15.1 M6). `default-features = false`, features `std`, `parse`, `display`, `serde`. Already in the graph through `egui_kittest`; now a normal dependency. Approved by the user on 2026-10-24, m4 §8.3. |
| sys-locale | =0.3.2 | MIT OR Apache-2.0 | The Windows UI language for `auto` (`system_ui_language`), without an `unsafe` allow-list entry in our code | 2026-10-24 | `va-workspace`, Windows only (`[target.'cfg(windows)'.dependencies]`), so **`va.exe` ships it too** (`cargo tree -p va -e normal` on Windows, 2026-10-25; absent from the Linux tree; m4 §15.1 M6). No dependency of its own. Declared in WP0, used from WP-D. Approved by the user on 2026-10-24, m4 §8.3. |
| rfd | =0.17.2 | MIT | Native Windows open and save dialogs, and the startup error box of the GUI | 2026-10-24 | `visual-assembler`, Windows only. `default-features = false` (no XDG portal, no Wayland code). Adds only itself (windows-sys 0.61.2, raw-window-handle and log were present). Declared in WP0, used from WP-G. Approved by the user on 2026-10-24, m4 §8.3. |
| egui | =0.36.2 | MIT OR Apache-2.0 | Widgets of `va-ui`, built on egui alone, never eframe (rule R3) | 2026-10-24 | Direct dependency of `va-ui` (default features). Same version eframe pulls, so no new package. Approved by the user on 2026-10-24, m4 §8.3. |

## Embedded assets
Third-party data compiled into the binaries. Not crates: cargo-deny does not see them.

| Asset | Version | License | Purpose | Added on | Notes |
|-------|---------|---------|---------|----------|-------|
| JetBrains Mono Regular (`assets/fonts/JetBrainsMono-Regular.ttf`, with `OFL.txt`) | 2.304 | OFL-1.1 | Embedded code font of the editor (architecture §9.6, m4 §5.5) | 2026-10-24 | Regular TTF only (273,900 bytes), copied unmodified from the release archive, which was downloaded with the user's approval of 2026-10-24 (m4 §8.4). Embedded by `crates/va-ui/src/fonts.rs` with its licence text. Source URL and SHA-256 hashes in `assets/fonts/README.md`. |

## Development
Used by tests or by the task runner only; never shipped.

| Package | Version | License | Purpose | Added on | Notes |
|---------|---------|---------|---------|----------|-------|
| egui_kittest | =0.36.2 | MIT OR Apache-2.0 | Headless UI tests over the AccessKit tree | 2026-10-13 | Dev-dependency of `visual-assembler` and, since M4 WP0 (2026-10-24), of `va-ui`, without its `wgpu` and `snapshot` features (no GPU needed) |
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
| egui, egui-winit, egui-wgpu, egui_glow | 0.36.2 | MIT OR Apache-2.0 | eframe (egui is also direct in `va-ui` since M4) | UI library and its winit, wgpu and glow integrations |
| wgpu (wgpu-core, wgpu-hal, naga) | 30.0.1 | MIT OR Apache-2.0 | egui-wgpu | wgpu renderer (`renderer-wgpu`, default): DX12, Vulkan and GL backends (Metal unused). The `auto` fallback after glow on Windows x64 and Linux, without its GL backend; the first `auto` choice on Windows aarch64 |
| glow | 0.17.0 | MIT OR Apache-2.0 OR Zlib | egui_glow | OpenGL renderer (`renderer-glow`, default). The first `auto` choice on Windows x64 and Linux; the fallback on Windows aarch64 |
| winit | 0.30.13 | Apache-2.0 | eframe (every OS); also direct on Linux (Runtime above) | Windowing; Wayland and X11 libraries are loaded at run time |
| glutin / glutin-winit | 0.32.3 / 0.5.0 | Apache-2.0 / MIT | eframe (glow) | GL context creation |
| accesskit / accesskit_winit / accesskit_windows / accesskit_unix | 0.24.1 / 0.32.2 / 0.32.1 / 0.21.1 | MIT OR Apache-2.0 (accesskit_winit: Apache-2.0) | eframe | Accessibility tree (UI Automation on Windows, AT-SPI over zbus on Linux) |
| zbus | 5.19.0 | MIT | accesskit_unix | D-Bus client for AT-SPI (Linux) |
| x11rb | 0.13.2 | MIT OR Apache-2.0 | winit, arboard (Linux) | X11 protocol client |
| epaint_default_fonts | 0.36.2 | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 | eframe (`default_fonts`) | Embedded UI fonts |
| kittest | 0.4.0 | MIT OR Apache-2.0 | egui_kittest (dev) | AccessKit test queries |
| fluent-syntax, fluent-langneg, intl-memoizer, intl_pluralrules, type-map | 0.12.0, 0.13.1, 0.5.3, 7.0.2, 0.5.1 | Apache-2.0 OR MIT | fluent-bundle (M1) | `.ftl` parser, language negotiation, memoized plural rules, typed storage |
| unic-langid-impl, tinystr | 0.9.6, 0.8.4 | MIT OR Apache-2.0; tinystr Unicode-3.0 | unic-langid (M1) | Language identifier parsing, small ASCII strings |
| self_cell | 1.3.0 | Apache-2.0 OR GPL-2.0-only (Apache-2.0 chosen) | fluent-bundle (M1) | Self-referential resource storage |
| displaydoc, thiserror, rustc-hash, smallvec, memchr | 0.2.7, 2.0.21, 2.1.3, 1.16.3, 2.8.3 | MIT OR Apache-2.0; memchr Unlicense OR MIT | fluent-bundle, tinystr (M1) | Error derives, hashing, small vectors, byte search (some were already in the graph through serde_json or the GUI stack) |
| zerovec, zerofrom | 0.11.8, 0.1.8 | Unicode-3.0 | tinystr's default features (M1) | **In `Cargo.lock` only, never compiled** (`cargo tree -p va-workspace` does not show them); ratified in m1 §8 and §12.2 |
| aho-corasick | 1.1.5 | Unlicense OR MIT | regex (M4) | Multi-literal search behind the regex engine. New in `Cargo.lock` on 2026-10-24 |
| regex-automata, regex-syntax | 0.4.18, 0.8.11 | MIT OR Apache-2.0 | regex (M4) | The regex engines and the pattern parser. New in `Cargo.lock` on 2026-10-24 |
| str_indices | 0.4.4 | MIT OR Apache-2.0 | ropey (M4) | Char and line index counting inside rope chunks (with `simd`). New in `Cargo.lock` on 2026-10-24; ropey's other dependency, smallvec, was present |
| toml_writer | 1.1.3 | MIT OR Apache-2.0 | toml (M4, feature `display`) | TOML output (the session file). New in `Cargo.lock` on 2026-10-24 |
| toml_parser, toml_datetime, serde_spanned, winnow | 1.1.5, 1.1.2, 1.1.2, 1.0.4 | MIT OR Apache-2.0; winnow MIT | toml (M4) | TOML parser, dates, spanned values, parser combinators. Already in the lock through `egui_kittest` (dev); since M4 also compiled into `va` and `visual-assembler` through `va-workspace` |
| log | 0.4.34 | MIT OR Apache-2.0 | rfd (M4, Windows), and others before | Logging facade; already in the graph |

## License alerts
- **Font licences, data only.** `epaint_default_fonts` embeds fonts under OFL-1.1 and Ubuntu-font-1.0.
  `.cargo/deny.toml` allows these two licences for this crate only. Shipped binaries must carry the
  font licence notices (M8 packaging).
- **Attribution licences in the graph.** cargo-deny allows them: Apache-2.0-only crates (winit, glutin,
  accesskit_winit), BSL-1.0 (clipboard-win 5.4.3, error-code 3.4.0), ISC (libloading 0.8.9),
  Unicode-3.0 (unicode-ident 1.0.27) and Zlib (foldhash 0.2.0, zlib-rs 0.6.8). All are permissive. Binary
  distribution (M8) must ship the licence texts.
- **LGPL-3.0 CI action.** `Swatinem/rust-cache` runs only on GitHub's runners as a build tool. It is
  never linked into, nor distributed with, VisualAssembler, so no obligation reaches the product.
- **winit, direct since §19** (Apache-2.0, already in the graph through eframe). No new licence and
  no new network surface: X11 over TCP is already an accepted risk in `SECURITY.md`.
- **Planned, then rejected for M4.** The M0 spec (§9, §13.3) planned `directories` (with
  `option-ext` 0.2.0, MPL-2.0) and `notify` 8.2.0 (CC0-1.0) for M4, each needing a per-crate
  cargo-deny exception. m4-editor-shell.md §8.3 rejected both (std resolves the folders; focus-gain
  checks replace file watching), so no exception was added. The SingleStepTests 680x0 vectors
  publish **no licence**. They will be fetched at test time into the gitignored `xtask/cache/` (M3),
  and never committed or shipped.
- **M4 additions** (ropey, regex, toml, sys-locale, rfd, egui direct, and their new transitive
  packages; m4 §8.3, approved by the user on 2026-10-24). All are MIT, MIT OR Apache-2.0, or
  Unlicense OR MIT (aho-corasick): no new licence and no policy change. `cargo deny check licenses
  bans` passed on the M4 lock on 2026-10-25 (`bans ok, licenses ok`).
- **Embedded font, OFL-1.1.** JetBrains Mono 2.304 Regular is data, not a crate, so cargo-deny's
  licence allow-list does not apply to it. The OFL allows embedding and redistribution with the
  software, commercial included, provided the licence travels with the font and the font is not sold
  by itself: the binaries embed `OFL.txt` (`EMBEDDED_FONT_LICENSE`), and M8 packaging copies it next
  to the executable (not implemented yet).
- **M1 additions** (fluent-bundle, unic-langid and their transitive packages, m1 §8). `self_cell` is
  dual-licensed Apache-2.0 OR GPL-2.0-only: VisualAssembler uses it under **Apache-2.0**, so no GPL
  obligation applies. `tinystr` (compiled) and `zerovec`/`zerofrom` (lock file only) are Unicode-3.0,
  already allowed. `memchr` is Unlicense OR MIT. `cargo deny check licenses bans` passed
  on that closure on 2026-10-17 with no policy change. The three advisory ignores of
  `.cargo/deny.toml` (RUSTSEC-2026-0370/0371/0372, zbus family) concern the M0 Linux GUI graph, not
  these crates (`SECURITY.md`).
- **No GPL, LGPL, AGPL, SSPL, BUSL or non-commercial licence** in the Rust dependency graph.
  `make security` reported `licenses ok` on 2026-10-14 and again on 2026-10-16, with the §19 work;
  the licence and ban checks passed again on the M4 lock on 2026-10-25.
