# Security

This file lists the protections that are actually in place. The Security agent adds one entry per
protection when it is implemented. It is not a list of good intentions: anything not implemented is
left out, and planned work appears only in the tables at the end.

Last audit: M0 (foundations), security audit round 3, the first after the fix wave of
m0-foundations.md §18, 2026-10-14. Every entry below was checked against the code at that date. Where
a check was actually run, the entry says so. Bypasses are classified under the tripwire threat model
below (§18.1).

- Findings of this round are numbered R3-1 to R3-10. All are low and none is blocking.
- Round-2 findings:
  - resolved by §18: R2-1, R2-4, R2-5, R2-6, R2-8, R2-9, R2-10, R2-12 and R2-13;
  - accepted as Not blocking (§18.5): R2-2, R2-3 and R2-7;
  - accepted with the §18.2 wording: R2-11.
- Round-1 findings: F1 to F4 and F6 to F9 are resolved. F5 and F10 are accepted risks.

---

## Threat model (m0-foundations.md §18.1, binding; architecture §9.3 and §9.4)

The xtask source scan (rules S1 and S3 to S9) and the build-configuration check (step 0 of
`cargo xtask ci`) are **tripwires**. They catch network code, unsafe code or build-flag overrides
that someone adds *by accident or carelessly*. They are **not** a sandbox against a contributor who
sets out to get around them. That case is covered by three things:

- code review;
- cargo-deny, on dependencies;
- branch protection with required CI. This one is **not in place yet** (§17.17 item 3), so until then
  every CI check below is advisory.

Every scan or configuration bypass is classified under one of two classes:

| Class | Definition | Examples | Handling |
|---|---|---|---|
| **Blocking** | A *realistic accidental* pattern: something a contributor writes in good faith or by habit, or that a tool, editor or OS produces. It compiles with the workspace configuration, and nothing catches it: no tripwire, no compiler lint, no cargo-deny. | rustfmt-wrapped attributes; a file saved as `.RS` on Windows; an empty `RUSTFLAGS` exported by a shell profile; a `[build]` or `runner` line added to try something; a forgotten `[patch]`; a new crate folder missing from `members`; a direct dependency on an egui add-on crate; the locale git sets for hooks | Fixed in the next wave, with a regression test |
| **Not blocking** | Needs deliberate obfuscation | Renamed or aliased built-in macros; raw-identifier attributes (`#[r#path]`, `std::r#net`); out-of-tree target paths; `[workspace] exclude` combined with path tricks; crate renaming through `package =`; macros that ignore their input; cargo config `include` or escaped quoted keys (now caught anyway by §18.4) | A line in "Accepted limitations" only. Never an audit failure, never reopened. |

The guarantees these protections serve:

1. The product never initiates network I/O (architecture G4, §9.3).
2. Memory safety by default: `unsafe` exists only in three reviewed files (§9.4).
3. Build flags come only from the canonical `.cargo/config.toml` (rule R8).
4. No commit credits an AI tool (framework rule, exit criterion E6).

---

### Network, socket and telemetry crate ban in the dependency graph
- **Location**: `.cargo/deny.toml:L50-L106` (bans and the `mio` feature ban), `.cargo/deny.toml:L10-L17` (graph: 4 targets, all features), `.cargo/deny.toml:L23-L27` (advisories), `.cargo/deny.toml:L108-L116` (workspace dependencies and sources), `.github/workflows/ci.yml:L52-L67` (CI `deny` job), `Makefile:L55-L58` (`make security`)
- **Language**: TOML policy for cargo-deny 0.20.2. GitHub Actions YAML. GNU make 4.4.1.
- **Description**: The command `cargo deny --locked --manifest-path Cargo.toml --config .cargo/deny.toml check` fails when any of 46 named crates appears in the resolved graph. The graph covers 4 targets (x86_64 and aarch64, Windows MSVC and Linux GNU) with every workspace feature enabled. The names cover:
  - HTTP clients and servers, QUIC, WebSocket and TLS (20 of the 22 entries of spec §9; the other two, `sentry` and `opentelemetry`, are in the telemetry line below);
  - sockets and async network runtimes (`socket2`, `tokio`, `async-std`, `async-net`, `smol`);
  - DNS (`hickory-resolver`, `hickory-proto`, `trust-dns-resolver`, `trust-dns-proto`);
  - RPC and web servers (`tonic`, `axum`, `actix-web`, `warp`, `ws`), mail (`lettre`), git and ssh (`git2`, `libgit2-sys`, `ssh2`, `libssh2-sys`);
  - telemetry (`sentry`, `sentry-core`, `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, `cadence`, `statsd`).

  More rules:
  - `mio` is allowed only without its `net` feature.
  - Crates may come only from the crates.io index (`unknown-registry` and `unknown-git` are `deny`). Wildcard versions are denied.
  - `--locked` (§18.7) turns a stale `Cargo.lock` into an error instead of re-resolving it.
  - The explicit command form means a stray root `deny.toml` cannot shadow the policy.

  CI `deny` job (§18.7, resolves R2-10):
  - cargo-deny 0.20.2 is built from crates.io with `cargo install --locked --root "$RUNNER_TEMP/cargo-deny"`.
  - The build runs from `${{ runner.temp }}`, outside the checkout, so the repository's `.cargo/config.toml` does not apply to it.
  - The job has no rust-cache step, so the policy tool is rebuilt on every run and never restored from a cache.

  Verified on 2026-10-14:
  - `make -n security` prints the §18.7 command.
  - The command ran on Windows and gave `advisories ok, bans ok, licenses ok, sources ok`, plus duplicate-version warnings, which the policy allows.
  - The `deny` job in `ci.yml` is identical to the §18.7 block. It has not run on GitHub yet.
- **Rationale**: Architecture goal G4 and §9.3: an offline product with no telemetry, enforced by tooling rather than by promise. Resolves F3, F9 and R2-10.
- **Limitations**:
  - The list matches crate names, so a network-capable crate that is not named gets through. Examples: `nix` or `rustix` with their socket features, bindings to C networking libraries such as `zmq`, or `dns-lookup`.
  - `async-io` and `polling` cannot be banned, because zbus needs them. In the Linux graph, `rustix` 1.1.5 is built with its `net` feature. A workspace crate that adds `rustix` as a direct dependency gets safe socket APIs that neither cargo-deny nor the source scan sees (accepted risk below).
  - Transports that the environment selects are already in the Linux graph and are accepted: zbus 5.19 and x11rb 0.13.2 (accepted risk below).
  - `Cargo.lock` lists `tokio` 1.53.2, with feature `rt` only, for wgpu's `wasm32`-only `wasm-bindgen-futures`. cargo-deny prunes it, because `[graph] targets` has no wasm target. Adding a wasm target would make the ban fire.
  - Path sources are never checked by cargo-deny. The `[patch]`/`[replace]` and unlisted-package checks below close the realistic routes for such a source.
  - The check does not see what dependency build scripts and proc macros do at build time.
  - `cargo xtask ci` does not run cargo-deny. Only the CI `deny` job and `make security` do, and E1 runs `make security` on Windows only (§17.14).
  - R3-10: the usage comment at `.cargo/deny.toml:L5-L8` predates §18.7 and shows the command without `--locked`. Someone who copies it gets a run that re-resolves a stale `Cargo.lock` instead of failing. The Makefile and CI use the full command.
  - The CI job finds `cargo-deny` through `PATH`:
    - `$RUNNER_TEMP/cargo-deny/bin` is prepended through `GITHUB_PATH`;
    - `~/.cargo/bin` is already on `PATH`, so cargo keeps the `PATH` order;
    - calling the binary by its absolute path would remove that dependency on order.
  - On a runner that has no rustup, `dtolnay/rust-toolchain` downloads `rustup-init` over HTTPS without a pinned checksum (seen in its `action.yml` at the pinned SHA).
- **Failure mode**: Fail-closed for the named crates and features, for unknown sources and for a stale lock file. Fail-open for crates that are not named.

### eframe built without its default features (no URL opening)
- **Location**: `Cargo.toml:L34`, `apps/visual-assembler/Cargo.toml:L20-L23`
- **Language**: TOML (Cargo manifest), Rust 1.99.0
- **Description**: eframe is pinned with `default-features = false`. It keeps only `accesskit`, `default_fonts`, `wayland` and `x11`, plus one renderer feature per build. So the `links` feature, which opens URLs in a browser through the `webbrowser` crate, is not compiled in. `webbrowser` is not in `Cargo.lock` (verified 2026-10-14). eframe's `persistence` feature is off too, so eframe stores no app state on disk.
- **Rationale**: Offline product (G4). No UI path can open a web page.
- **Limitations**: Re-enabling eframe's default features would restore URL opening without any check failing. So would adding a crate that turns `links` back on through feature unification. No `[[bans.features]]` rule guards this.
- **Failure mode**: Fail-open on a manifest change. The protection holds only while the manifests stay as they are.

### Layering rules and GUI-crate families (rules R1 to R5 and R7)
- **Location**: `xtask/src/layering.rs:L68-L95` (GUI families, exact names, the `va-ui` allowance), `xtask/src/layering.rs:L136-L360` (crate map and `check_layering`). Run by `xtask/src/ci.rs:L398-L412`. Tests in `xtask/tests/layering.rs`.
- **Language**: Rust 1.99.0 (edition 2024)
- **Description**: The check runs on the output of `cargo metadata --no-deps`.
  - Every workspace package must have a rule in the §17.2 table. A package without one is a violation.
  - Every dependency kind is checked under the crate's real name, so a `package =` rename does not hide it.
  - Apps and xtask are leaves: no package may depend on them.
  - GUI crates are matched by family (§18.5, resolves R2-13). A crate counts as GUI when its normalized name (lower case, `_` and `-` equal) starts with `egui`, `eframe`, `epaint`, `emath`, `ecolor`, `wgpu`, `naga`, `winit`, `glutin`, `glow` or `accesskit`, or equals `raw-window-handle`. Only `visual-assembler` may use any of them. `va-ui` may use exactly `egui` and `egui_kittest`.
  - A malformed metadata document is a violation.

  The tests pass on Windows (2026-10-14).
- **Rationale**: Rules R3 and R4. Besides the GUI stack, this keeps two network-capable stacks out of the headless CLI's dependency closure: AccessKit's zbus and the X11 client stack.
- **Limitations**: Only direct dependencies are checked. Two kinds of crate still pass: a third-party egui add-on whose name lies outside the families (for example `<x>-egui`), and a different GUI toolkit. R7 and review cover them. §18.5 rejected a resolved-graph check.
- **Failure mode**: Fail-closed: a package without a rule, a forbidden edge or a malformed document fails the step.

### Source scan of every workspace member (rules S1 and S3 to S9)
- **Location**: `xtask/src/layering.rs:L31-L66` (allow-lists and patterns), `xtask/src/layering.rs:L362-L634` (code view S5, shebang rule), `xtask/src/layering.rs:L636-L741` (token helpers), `xtask/src/layering.rs:L757-L817` (network, S7), `xtask/src/layering.rs:L843-L935` (inclusion, S8), `xtask/src/layering.rs:L984-L1022` (per-file rules), `xtask/src/layering.rs:L1024-L1155` (walk: S1, S3, S4, S9), `xtask/src/layering.rs:L1157-L1204` (live workspace). Run by `xtask/src/ci.rs:L398-L412`: the "layering and source checks" step of `cargo xtask ci`, in the CI `check` job on 4 runners. Regression tests in `xtask/tests/layering.rs`.
- **Language**: Rust 1.99.0 (edition 2024)
- **Description**:
  - S1: the folder of every workspace member that `cargo metadata --no-deps` reports is walked. That includes `xtask`, and implicit members too, because cargo lists them.
  - S2 is removed (§18.5, resolves R2-12): no file is exempt. `xtask/src/layering.rs` and `xtask/tests/layering.rs` are scanned under every rule.
  - S3: only cargo's target folder and `<root>/target` are skipped. Every other folder named `target` is scanned.
  - S4: any symlink or junction under a member folder is a `[symlink]` violation and is never followed.
  - S5: patterns are matched on a code view. Comments (nested ones included) are blanked, and so is the content of string, raw-string, byte-string, C-string and char literals. Line numbers are kept.
  - S5, shebang (resolves R2-1): a first-line shebang is blanked exactly as `rustc_lexer::strip_shebang` does it. After an optional BOM, the text starts with `#!`, and the next token is not `[` (rustc's whitespace and non-doc comments are skipped while looking for it). So `#!"` can no longer desynchronize the view.
  - S6 is described in the unsafe entry below.
  - S7: outside `NET_ALLOWLIST`, which is empty, these are violations:
    - `std::net` and `core::net`, with whitespace removed;
    - `std::{…}` and `core::{…}` groups that import `net`;
    - the identifiers `TcpStream`, `TcpListener`, `UdpSocket`, `ToSocketAddrs`, `to_socket_addrs` and `lookup_host`.
  - S8: `include!` and `#[path = …]`, including `#[path]` inside `cfg_attr`, must take a plain string literal that meets every condition below. `INCLUDE_ALLOWLIST` is empty.
    - It ends in `.rs`, in any ASCII case.
    - It has no `..` segment and no backslash.
    - It is not absolute.
    - It is not a prefixed or raw literal.
  - S9 (resolves R2-4): a file is pattern-scanned when its name ends in `.rs` ignoring ASCII case, on every OS.
  - Unreadable folders and files, and non-UTF-8 sources, are errors. Nothing is skipped silently.

  Verified on 2026-10-14, on Windows:
  - `cargo test -p xtask --locked`: 82 layering tests pass and 3 are ignored.
  - `cargo test -p xtask --test layering -- --ignored live_workspace` reports 0 violations on the live workspace.
  - Keep the name filter in that command. Without it, `-- --ignored` and `--include-ignored` also run the two §18.1 record tests, which fail by design (R3-7).
  - 17 edge cases were run through the real `scan_text` in a scratch crate: escaped quotes, byte, char, raw and C strings, nested comments, raw lifetimes, BOM plus shebang, `#!` with an attribute on the next line, `include!(concat!(env!("OUT_DIR"), …))`, and `cfg_attr(…, path = …)`. Each one gave the expected result.
- **Rationale**: Architecture §9.3 and §9.4, F1 and R2-1, R2-4 and R2-12. This is a tripwire under §18.1, not a sandbox.
- **Limitations**:
  - The Not-blocking bypasses are listed under "Accepted limitations".
  - Doc comments and `#[doc = "…"]` strings are blanked, so code in doctests is never scanned. `cargo test` compiles and runs doctests, but they are never shipped. A doctest calling `std::net::TcpStream::connect` gives 0 violations (probe). The `[lints]` table does not reach doctests either: a doctest with an `unsafe` block passes under `[lints.rust] unsafe_code = "deny"` (verified with cargo 1.99.0).
  - The scan cannot see network I/O done through FFI in an unsafe-allow-listed file, through child processes (`std::process::Command` running `curl`), or inside dependencies.
  - It also misses the safe socket APIs of crates already in the graph (rustix `net`, zbus `tcp:`, x11rb `host:N`) once one of them becomes a direct dependency.
  - S3 canonicalizes the skipped folders. A `CARGO_TARGET_DIR` that resolves to a member folder therefore makes the scan skip that whole member.
  - Frontmatter is still unstable in rustc 1.99.0. When it stabilizes, the code view must skip it the way rustc does, or a desync like R2-1 comes back.
- **Failure mode**: Fail-closed for the listed patterns, for symlinks and for I/O errors. Fail-open for the accepted bypasses.

### Unsafe code policy: deny lint, source rule S6, per-file allow-list, documented blocks
- **Location**: `Cargo.toml:L42-L53` (`unsafe_code = "deny"`, `clippy::undocumented_unsafe_blocks`), `xtask/src/layering.rs:L31-L36` (`UNSAFE_ALLOWLIST`), `xtask/src/layering.rs:L819-L841` (S6), `xtask/src/ci.rs:L39-L67` (clippy with `-D warnings`, all features then glow only; tests)
- **Language**: Rust 1.99.0 (edition 2024), TOML
- **Description**: Every member inherits `unsafe_code = "deny"`. rustc 1.99.0 also reports `global_asm!` under this lint (verified in round 2). Outside the three allow-listed files (`apps/visual-assembler/src/bench/process_windows.rs`, `process_linux.rs` and `platform.rs`), S6 flags:
  - every whole-word `unsafe` token in the code view, including edition-2024 forms such as `unsafe extern` and `#[unsafe(no_mangle)]`;
  - every `unsafe_code` token whose innermost enclosing group, of any kind, is not introduced by `deny(` or `forbid(` (§18.5, CR1-02).

  More rules:
  - Wrapped, macro-passed, `expect(…)` and `cfg_attr(…, allow(unsafe_code))` forms therefore all fail.
  - Clippy with `-D warnings` stops `warn(unsafe_code)` and any unsafe block without a `// SAFETY:` comment.
  - Rule R8 is enforced by the `[lints]` check and by step 0 (entries below).
  - In M0 the workspace has 5 unsafe blocks, all in the allow-listed files.
  - `platform.rs` allows `unsafe_code` on `gl_string` only, so the compiler still rejects unsafe code anywhere else in that file. The two process files allow it for the whole file.
- **Rationale**: Architecture §9.4. Memory safety by default, with every exception visible and reviewed (F1, F2).
- **Limitations**:
  - The scan bypasses under "Accepted limitations" hide `#![allow(unsafe_code)]` and `unsafe` from S6. The compiler lint stays the primary guard.
  - The allow-list works per file. Unsafe code can be added anywhere in the two process files without the scan noticing, and inside `gl_string`.
  - Unsafe code emitted by a proc macro is not seen by the scan.
  - Unsafe code in dependencies (wgpu, winit, windows-sys, glow and others) is not audited. Only RustSec `unsound` advisories cover it.
- **Failure mode**: Fail-closed through the compiler lint and S6 for ordinary code.

### Workspace lint inheritance check (rule R8, lint part)
- **Location**: `scripts/check_project_docs.py:L78-L79`, `scripts/check_project_docs.py:L397-L444` (`[lints]`), `scripts/check_project_docs.py:L294-L337` (member list). Run by the CI `docs` job (`.github/workflows/ci.yml:L42-L50`) and by `cargo xtask docs` (`xtask/src/docs.rs:L43-L63`).
- **Language**: Python 3.11+ (tomllib)
- **Description**:
  - The `lints` table of each `[workspace].members` manifest must equal `{"workspace": True}`. The key set is compared and the value is tested with `is True`, so `workspace = 1` and any `[lints.*]` override fail.
  - The root manifest must keep `workspace.lints.rust.unsafe_code = "deny"` as a plain string.
  - On 2026-10-14 the script reported no `[lints]` and no `[deps]` problem on the M0 tree.
- **Rationale**: F2: a member could re-allow `unsafe_code` in its own manifest.
- **Limitations**:
  - Only listed members are checked. Unlisted packages are reported by the next entry; see R3-3 for what that check skips.
  - The other root lints (`[workspace.lints.clippy]`, `missing_docs`) are not checked.
  - Without tomllib (Python older than 3.11), the check is skipped with a warning only. The CI `docs` job does not pass `--strict`, so that warning would not fail it.
- **Failure mode**: Fail-closed for listed members. Fail-open when tomllib is missing.

### Unlisted-package and `[patch]`/`[replace]` checks (§18.6)
- **Location**: `scripts/check_project_docs.py:L91-L102` (constants), `scripts/check_project_docs.py:L447-L534` (`path_identity`, `iter_nested_manifests`, `check_implicit_members`, `check_patch_tables`), `scripts/check_project_docs.py:L620-L624` (calls). Run by the CI `docs` job and by `cargo xtask docs`.
- **Language**: Python 3.11+ (tomllib)
- **Description**:
  - Check 3d (resolves R2-8) walks every folder below the root. It skips the names in `IGNORED_DIRS` (`target` included), `*.egg-info` and untracked gitignored paths, and it does not follow symlinked folders.
  - A manifest named `cargo.toml` in any case is reported as `[deps] <path>: package inside the workspace is not listed in [workspace].members` when it has a `[package]` table (or the legacy `[project]`) and its folder is not a listed member. Members are compared by resolved path, case-folded on Windows.
  - Packages under `[workspace].exclude` are reported too, so a vendored copy of a crate cannot sit outside the checks. A manifest that cannot be read or parsed, and a folder that cannot be listed, are reported as well.
  - Check 3e (resolves R2-9): a root `[patch]` table (any registry) or `[replace]` table is reported as `[deps] Cargo.toml: [patch]/[replace] tables are not allowed`.
  - Verified on 2026-10-14 with scratch fixtures: an unlisted package, an excluded vendored package and a root `[patch.crates-io]` were each reported. The real repository has none.
- **Rationale**: Cargo silently makes an in-tree path dependency a member, which escapes the `[lints]` and `[deps]` checks. A `[patch]` to a path crate replaces a dependency with unreviewed code that has no checksum.
- **Limitations**:
  - R3-3: folders named like an `IGNORED_DIRS` entry that the repository's `.gitignore` does not ignore (`out`, `env`, `.tox`, `.gradle`, `site-packages`, `.terraform`, `.nuxt`, `.hg`, `.svn`) are skipped. A committed package in such a folder is not reported (verified with a fixture). What remains: layering still flags any package that is not named `va-arch-*`, and S6 still flags `unsafe`.
  - A `[patch]` in user-level cargo configuration is not visible. The repository configuration is canonical (step 0).
  - Without tomllib, both checks are skipped with a warning.
  - In CI the problem is found after the push, and it blocks a merge only once branch protection exists.
- **Failure mode**: Fail-closed for walked manifests, parse errors and unlistable folders. Fail-open for the skipped folder names.

### Build-flag integrity check (`cargo xtask ci` step 0, rule R8 build part)
- **Location**: `xtask/src/ci.rs:L69-L106` (variable names, `CANONICAL_CARGO_CONFIG`), `xtask/src/ci.rs:L182-L221` (environment), `xtask/src/ci.rs:L223-L341` (canonical comparison), `xtask/src/ci.rs:L343-L369` (files), `xtask/src/ci.rs:L371-L396` (step 0), `xtask/src/ci.rs:L139-L153` (step order). Tests in `xtask/tests/ci.rs`.
- **Language**: Rust 1.99.0
- **Description**: Step 0 runs before every other step, also with `--no-fmt`. `cargo xtask ci` fails with exit 1 when any of these holds:
  - `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `CARGO_BUILD_RUSTFLAGS` or any `CARGO_TARGET_*_RUSTFLAGS` is **set, even to an empty value** (§18.3, resolves R2-6). Names are compared ignoring ASCII case on every OS, and a non-Unicode name is compared in its lossy form.
  - `.cargo/config.toml` differs from `CANONICAL_CARGO_CONFIG` (8 lines) after normalization: blank lines and full-line `#` comments are dropped, and all whitespace is removed from each remaining line. Every extra, missing, changed or reordered line is reported with its line number (§18.4, resolves R2-5). That covers top-level keys, `include`, quoted keys, `runner`, alias changes and trailing comments.
  - `.cargo/config.toml` is missing or unreadable.
  - The extension-less `.cargo/config` exists, or cannot be inspected.

  Verified on 2026-10-14:
  - The 33 `ci.rs` tests pass on Windows. They include the live configuration and the binary test that sets each variable to an empty value.
  - `RUSTFLAGS= <built xtask> ci --no-fmt` exits 1 at step 0, before any other step, on Windows (Git Bash) and in WSL. Unset, the step prints `build configuration: ok`.
  - The `action.yml` files of the pinned actions export no `RUSTFLAGS`-type variable. dtolnay sets `CARGO_HOME`, `CARGO_INCREMENTAL`, `CARGO_TERM_COLOR`, `CARGO_REGISTRIES_CRATES_IO_PROTOCOL` and, on old toolchains, `CARGO_HTTP_MULTIPLEXING`.
- **Rationale**: F2 (QA-1-F4.1), R2-5, R2-6: an environment variable or a config line could silently drop `+crt-static` or add `-A unsafe_code`.
- **Limitations**:
  - R3-2: whitespace is also removed inside quoted strings. `rustflags = ["-C", "target-feature=+crt-static "]`, with a stray space, therefore passes step 0. rustc 1.99.0 only warns about an unknown feature `crt-static ` and links `VCRUNTIME140.dll` (verified with dumpbin on a probe). That warning is not a lint, so `-D warnings` does not stop it. Only the manual E4 check catches it.
  - Accepted (§17.3.3): user-level configuration cannot be inspected on stable cargo. That covers `$CARGO_HOME/config.toml` and any `.cargo/config.toml` in a parent folder of the checkout.
  - The same goes for a `.cargo/config.toml` inside a member folder. Cargo reads it only when invoked from that folder. `cargo xtask ci`, make and CI always run from the root.
  - `RUSTC_WRAPPER`, `RUSTC_WORKSPACE_WRAPPER`, `RUSTC`, `RUSTDOCFLAGS` and `CARGO_PROFILE_*` are not checked. CI sets none of them.
  - Step 0 runs only inside `cargo xtask ci`. `make test`, `make lint` and `make build` do not run it.
  - Step 0 runs inside xtask, so cargo has already built xtask with whatever flags the environment and configuration supplied. Step 0 then reports those flags and fails the run. The `xtask` alias itself is read before step 0: a changed alias decides what `cargo xtask ci` runs (R3-8, review only).
  - PowerShell 5.1 cannot hold an empty variable (assigning `''` deletes it). The empty-value case therefore comes only from POSIX shells and from programs.
- **Failure mode**: Fail-closed: every read or inspection failure is reported as a problem, never as a pass.

### Audited unsafe FFI in the bench (process statistics, GL strings)
- **Location**: `apps/visual-assembler/src/bench/process_windows.rs:L34-L76`, `apps/visual-assembler/src/bench/process_linux.rs:L26-L38`, `apps/visual-assembler/src/bench/platform.rs:L107-L127`. Tests in `apps/visual-assembler/tests/bench_process.rs`.
- **Language**: Rust 1.99.0. windows-sys 0.61.2, libc 0.2.189, glow 0.17.0 (through eframe 0.36.2).
- **Description**: Each unsafe call has the narrowest possible scope and a `// SAFETY:` comment.
  - `GetProcessTimes` and `GetProcessMemoryInfo` receive the always-valid `GetCurrentProcess()` pseudo-handle and pointers taken from exclusive borrows of initialized locals. The `cb` size passed is exactly `size_of::<PROCESS_MEMORY_COUNTERS>()`, checked with `u32::try_from`.
  - `getrusage(RUSAGE_SELF)` writes into a `MaybeUninit`, and `assume_init` runs only after a 0 return.
  - The GL query runs on the UI thread, from `App::ui` (and from an unreachable fallback in `App::logic`). It goes through glow 0.17, which checks for a null `glGetString` result and for non-UTF-8 text by panicking. That panic is caught with `catch_unwind` and turned into `null`.
  - Every failure path becomes `None`, serialized as `null`. Nothing reads memory that a failed call did not write.

  The code was re-read in round 3. `cargo test -p visual-assembler --test bench_process --all-features` ran on Windows x64 on 2026-10-14: all 7 tests pass. Round 1 also ran it in WSL (Linux x64).
- **Rationale**: Soundness of the only unsafe code in the product (architecture §9.4).
- **Limitations**:
  - The `catch_unwind` works only with `panic = "unwind"`. Nothing enforces the root `[profile]` values, so a future `panic = "abort"` profile would turn a driver that returns NULL into a crash.
  - The struct layouts are those of windows-sys and libc and were not checked independently.
  - The ARM64 Windows and ARM64 Linux paths run only on the CI runners, which have not run yet.
- **Failure mode**: Fail-safe: a failed OS query yields `null` in the bench line, never undefined behaviour.

### Static MSVC C runtime for Windows binaries
- **Location**: `.cargo/config.toml:L9-L13`, guarded by step 0 (`xtask/src/ci.rs:L69-L396`)
- **Language**: TOML (Cargo configuration), rustc 1.99.0 MSVC
- **Description**: `-C target-feature=+crt-static` is set for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`. The C runtime is linked into the executable, so no `VCRUNTIME140.dll` or `api-ms-win-crt-*` DLL is loaded. On 2026-10-14, `dumpbin /dependents` ran on three binaries: the local x64 release `va.exe` and `visual-assembler.exe` (built 2026-10-13 from the current configuration), and the debug `xtask.exe`. None imports a CRT DLL. Step 0 rejects every `RUSTFLAGS`-type variable and every non-canonical config line that would replace these flags.
- **Rationale**: Architecture §9.2. No VC++ redistributable to install, and the binary cannot pick up a CRT DLL from the search path or from a stale system copy.
- **Limitations**:
  - R3-2: a stray space inside the feature string drops the flag while step 0 passes.
  - E4 stays a manual QA step. The ARM64 Windows build was not inspected.
  - Static linking does not protect the other DLLs the binaries load (F10, accepted until M8).
- **Failure mode**: Fail-closed for the checked variables and config lines. Fail-open for R3-2, which only the manual E4 check detects.

### Supply-chain pinning: committed lock file, `--locked` CI steps, pinned toolchain
- **Location**: `Cargo.toml:L31-L40`, `Cargo.lock`, `rust-toolchain.toml:L3-L6`, `xtask/src/ci.rs:L39-L67` (`--locked` on clippy and test), `.github/workflows/ci.yml:L92` (`--locked` release build), `.github/workflows/ci.yml:L65` and `L67` (`cargo install --locked`, `cargo deny --locked`), `Makefile:L58`, `.cargo/config.toml:L5-L6` (the `xtask` alias, without `--locked`), `.cargo/deny.toml:L23-L27` (advisories), `.cargo/deny.toml:L108-L116` (sources), `scripts/check_project_docs.py:L340-L394` (rule R6, `workspace = true`), `scripts/check_project_docs.py:L525-L534` (`[patch]`/`[replace]`)
- **Language**: TOML, Rust 1.99.0, Python 3
- **Description**:
  - Every external dependency is declared once in `[workspace.dependencies]`, today with an exact `=` pin. The `[deps]` check makes every member dependency use `workspace = true`.
  - `Cargo.lock`, which carries the crates.io checksums, is committed. `--locked` is passed to:
    - clippy and test inside `cargo xtask ci`, and the CI release build;
    - the cargo-deny install and the cargo-deny run;
    - `make security`.
  - In CI, only the `deny` job fails on a lock file that does not match the manifests (R3-9). The `check` job first runs the `xtask` alias, `run --package xtask --`, which has no `--locked`. That run rewrites a stale `Cargo.lock` on the runner, so the later `--locked` steps of the same job see a matching lock and pass. Verified on 2026-10-14 with a probe workspace on cargo 1.99.0: `cargo metadata --locked` failed on the stale lock, the alias run succeeded and rewrote it, and `cargo build --locked` then passed.
  - The toolchain is pinned to 1.99.0.
  - cargo-deny denies yanked crates and reports every `unsound` advisory. `unmaintained = "all"` covers transitive crates. RustSec vulnerability advisories fail the check, and the `ignore` list is empty.
  - Root `[patch]` and `[replace]` tables are rejected.
- **Rationale**: Reproducible builds, and no silent upgrade to a compromised or vulnerable version.
- **Limitations**:
  - R3-9: the `check` job, and `cargo xtask ci` locally, build and test against a lock file that the unlocked alias run may just have rewritten. With a stale lock, that job builds dependency versions that nobody committed. Only the `deny` job reports the mismatch, and until branch protection exists (R3-8) that failure does not block a merge.
  - R3-6: no tool enforces the exact `=` pins. A caret requirement in `[workspace.dependencies]` passes every check, because cargo-deny denies only `*`. The committed lock file still fixes what CI builds, once the `deny` job is a required check.
  - Nobody reviews the transitive versions one by one: there is no `cargo vet` or `cargo crev`.
  - The local `make test` and `make build`, `cargo xtask bench` and the `xtask` alias omit `--locked`.
  - A malicious version already present when the lock file is written is not detected.
  - Advisories are only as fresh as the last database fetch.
- **Failure mode**: Fail-closed in CI through the `deny` job (lock mismatch, yanked crate, advisory) and the `docs` job (patch table). The `check` job is fail-open on a stale lock (R3-9). Fail-open locally without `--locked`.

### Licence policy
- **Location**: `.cargo/deny.toml:L29-L48`, `Cargo.toml:L25-L26`, `license.workspace = true` in the 10 member manifests, `LICENSE-MIT`, `LICENSE-APACHE`
- **Language**: TOML (cargo-deny 0.20.2)
- **Description**:
  - Allow-list of licences: Apache-2.0, MIT, BSL-1.0, ISC, Unicode-3.0 and Zlib. OFL-1.1 and Ubuntu-font-1.0 are allowed for `epaint_default_fonts` only. The confidence threshold is 0.93.
  - The project is licensed `MIT OR Apache-2.0`. `[licenses.private] ignore = false`, so cargo-deny checks the workspace crates too.
  - Verified on 2026-10-14:
    - the sha256 of `LICENSE-APACHE` matches §17.1 (`62c7a1e3…3636a`);
    - `LICENSE-MIT` equals the §17.1 block;
    - both files are LF only;
    - `cargo deny check` reports `licenses ok`.
- **Rationale**: Keeps the graph redistributable under the permissive project licence. No copyleft or non-commercial code.
- **Limitations**:
  - Licence detection relies on crate metadata and on askalono's confidence.
  - Embedded assets that a crate does not declare are not detected.
  - `licenses.private` only checks that a member's licence is allowed. A member that writes `license = "MIT"` instead of inheriting the workspace value still passes.
- **Failure mode**: Fail-closed: a licence that is not listed fails the check.

### Commit-message hook and CI check against AI co-author mentions
- **Location**: `scripts/git-hooks/commit-msg:L1-L48`, `scripts/check_project_docs.py:L71-L77` (`COAUTHOR_RE`), `scripts/check_project_docs.py:L80-L83` (`git log` arguments), `scripts/check_project_docs.py:L537-L560`, `scripts/check_project_docs.py:L653-L664` (check 7, CI `docs` job, `.github/workflows/ci.yml:L42-L50`)
- **Language**: POSIX sh, sed and grep. Python 3.
- **Description**:
  - The hook is byte-identical to the §18.2 block (checked with `diff` on 2026-10-14).
  - It sets and exports `LC_ALL=C`, so sed and both greps match bytes on every platform. This resolves QA2-F1, where Git for Windows' grep missed the 4-byte emoji under git's UTF-8 hook locale.
  - It rejects, ignoring ASCII case:
    - `co-authored-by:`, `assisted-by:` or `generated-by:` followed by one of 12 AI tool names;
    - `generated with` followed by one of those names;
    - the robot emoji, matched on its UTF-8 bytes.
  - It fails closed (exit 1) when the message file is unreadable, or when sed or grep fails.
  - It ignores comment lines and everything below git's scissors line.
  - In CI, `check_project_docs.py` applies the same names, with the emoji as a code point, to `git log --encoding=UTF-8 --format=%an%n%ae%n%B -n 50`. When git fails it prints a `note:`.
  - The installed `.git/hooks/commit-msg` is identical to the source (verified with `cmp`).
  - The 15 `xtask/tests/hooks.rs` tests pass on Windows. They cover the UTF-8 locales and `git hook run`.
  - Probes on 2026-10-14: the robot emoji and a Claude trailer were rejected under `LC_ALL=en_US.UTF-8` (Git Bash) and `C.UTF-8` (WSL), and a clean message was accepted.
- **Rationale**: Project rule that commits are never co-signed by an AI tool (framework rule, E6). Resolves F4 and QA2-F1.
- **Limitations**:
  - The hook is client-side only: `--no-verify` bypasses it, and rebase and cherry-pick do not run it.
  - R2-11 (accepted, §18.2 wording): "Comment lines and a scissors line are ignored by the hook but kept in the commit when the message comes from `-m`/`-F`, `--cleanup=verbatim`/`whitespace`, or a non-default `core.commentChar`. Such a message passes locally and is caught by the CI `docs` job's log check after the push, which becomes a required check with branch protection."
  - A trailer folded onto a continuation line is not caught.
  - R3-4: the author lines in the CI log text can match only the emoji, and the committer is not included at all. A commit *authored* by an AI identity therefore passes both the hook, which cannot see the author, and the CI check. Examples: `Claude <noreply@anthropic.com>`, `copilot-swe-agent[bot]` (verified against `COAUTHOR_RE`).
  - The CI check covers only the last 50 commits (`fetch-depth: 50`). It blocks a merge only once branch protection exists (§17.17 item 3, not in place yet).
- **Failure mode**: The hook is fail-closed on errors. The CI check is fail-open (a note only) when git fails.

### Safe hook installation (`cargo xtask hooks [--force]`)
- **Location**: `xtask/src/hooks.rs:L49-L124` (hooks and common folders, containment), `xtask/src/hooks.rs:L126-L150` (backup names), `xtask/src/hooks.rs:L152-L236` (comparison, rename, `create_new`), `xtask/src/hooks.rs:L238-L250` (mode), `Makefile:L36-L37`. Tests in `xtask/tests/hooks.rs`.
- **Language**: Rust 1.99.0
- **Description**:
  - The hooks folder comes from `git rev-parse --git-path hooks`. It must lie inside the canonicalized common git folder (`--git-common-dir`). Missing path components are resolved lexically.
  - A `core.hooksPath` outside the repository is refused, even with `--force`.
  - An identical hook prints `already installed` and is left as is.
  - A different file, a symlink or a folder at the hook's place is refused without `--force`. With `--force`, it is first renamed to `commit-msg.bak.<UTC time>`, and `-2`, `-3`, … suffixes ensure no backup is ever overwritten.
  - The new hook is created with `create_new`, which never follows a link.
  - `make hooks` never passes `--force`.
  - The hooks tests pass on Windows (2026-10-14).
- **Rationale**: F6: the developer's own or global hook must never be overwritten silently, and the project policy must not leak into other repositories.
- **Limitations**:
  - A hooks path inside the worktree but outside `.git` (for example `core.hooksPath=.githooks`) is refused too.
  - The check and the write are not atomic. A local process that swaps the hooks folder in between could redirect the write.
  - On Windows the mode is not set (Git for Windows runs hooks through its own `sh`).
  - `git` is found through the search path (see the external processes entry).
- **Failure mode**: Fail-closed: any doubt leaves every file untouched and exits 1.

### CI workflow hardening
- **Location**:
  - triggers and the read-only token: `.github/workflows/ci.yml:L10-L17`;
  - `persist-credentials: false`: `ci.yml:L35`, `L48`, `L58`, `L80`;
  - actions pinned to commit SHAs: `ci.yml:L33`, `L36`, `L46`, `L56`, `L59`, `L78`, `L81`, `L85`;
  - cache saved only on `main`, in the `check` job only: `ci.yml:L88`;
  - `deny` job without a cache: `ci.yml:L52-L67`;
  - timeouts: `ci.yml:L31`, `L44`, `L54`, `L72`, `L96`.
- **Language**: GitHub Actions YAML
- **Description**:
  - The token is read-only at workflow level, and no job widens it. Checkout never persists credentials.
  - The workflow uses no `pull_request_target` and no secrets.
  - No `${{ … }}` expression appears inside a `run:` script. `runner.temp` is used only as `working-directory`, and the matrix value only in `name`, `runs-on` and the cache key. PR titles and branch names therefore cannot inject shell code.
  - Every action is pinned to a full commit SHA. Each pin was re-resolved upstream with `git ls-remote` on 2026-10-14:
    - `actions/checkout` v7.0.1 = `3d3c42e5aac5ba805825da76410c181273ba90b1`;
    - `Swatinem/rust-cache` v2.9.2 (peeled tag) = `6323deb102c322ba6fcbdcafc7e3dddab59af2b6`;
    - `dtolnay/rust-toolchain` master (also tag `v1`) = `e2a55d2ffb04f378e9626c28d38b36d230d1e12f`.
  - The Rust cache is used only by the `check` job and written only by `main` runs, so a pull request cannot poison it.
  - The `deny` job has no cache, and it builds cargo-deny outside the checkout on every run (R2-10 resolved).
- **Rationale**: Untrusted pull requests must not get write access, secrets or a poisoned cache. Mutable tags must not change what CI runs.
- **Limitations**:
  - In the `check` job, `rust-cache` restores `target/` and, since `cache-bin` defaults to `true`, `~/.cargo/bin`. The integrity of that job therefore rests on every earlier `main` run.
  - Branch protection, required status checks and CODEOWNERS do not exist yet.
  - Files that any pull request can change decide what CI runs: `ci.yml`, `.cargo/deny.toml`, the `xtask` alias and xtask itself. Only review protects them.
  - The `windows-latest` and `ubuntu-latest` images move.
  - `cancel-in-progress` can cancel a `main` run when a newer push arrives.
  - The workflow has not run on GitHub yet.
- **Failure mode**: Fail-closed on permissions (read-only token).

### External processes spawned without a shell (xtask and scripts)
- **Location**: `xtask/src/exec.rs:L1-L62`. Call sites:
  - `xtask/src/locate.rs:L31-L45`;
  - `xtask/src/ci.rs:L172-L174`;
  - `xtask/src/docs.rs:L30-L63`;
  - `xtask/src/hooks.rs:L51-L62`;
  - `xtask/src/bench/mod.rs:L259-L287`;
  - `xtask/src/bench/runner.rs:L117-L179` and `xtask/src/bench/runner.rs:L417-L431`;
  - `scripts/check_project_docs.py:L537-L560`;
  - `scripts/gen_agencement.py:L67-L95`.

  Makefile recipes: `Makefile:L31-L73`.
- **Language**: Rust 1.99.0. Python 3. GNU make 4.4.1.
- **Description**:
  - Every child process (cargo, git, Python, the measured GUI) is started with `std::process::Command` or `subprocess.run` and a fixed argument vector, never through a shell. No argument comes from untrusted data.
  - Failures to start, non-zero exits and non-UTF-8 output become errors with context. The Python git calls have a timeout.
  - Makefile recipes contain no shell syntax.
- **Rationale**: Architecture §9.5. No command injection, and identical behaviour under sh and cmd.exe.
- **Limitations**:
  - The program names `git`, `python` / `python3` and `$CARGO`, and the `$VA_PYTHON` override, are trusted as found. On Windows, Rust resolves a bare program name in the xtask executable's folder (`target/debug/`) before `PATH`.
  - Under sh, make passes `SCENARIO`, `RENDERER` and `RUNS` to the shell unquoted. That input comes from the developer's own command line and is not a trust boundary.
- **Failure mode**: Fail-closed: a child that cannot start or that fails stops the task with exit code 1.

### Bench child-process containment (deadlines, bounded reader shutdown, output caps)
- **Location**: `xtask/src/bench/runner.rs:L32-L58` (limits), `xtask/src/bench/runner.rs:L433-L548` (run engine), `xtask/src/bench/runner.rs:L550-L614` (reader threads), `xtask/src/bench/runner.rs:L616-L675` (deadline wait, bounded drain, kill and reap). GUI-side watchdog: `apps/visual-assembler/src/bench/recorder.rs:L47-L113`.
- **Language**: Rust 1.99.0
- **Description**:
  - Each measured run has stdin set to null. Stdout and stderr are piped and drained by reader threads.
  - The run deadline is 75 s for startup and N + 75 s for idle. Both are strictly above the GUI's own watchdog (60 s, and 60 + N + 3 s for idle), so the GUI's `bench timed out` (exit 4) surfaces first.
  - Past the deadline, the child is killed and reaped.
  - Once the child has exited or been killed, the readers get at most 5 s. Readers still blocked after that are detached, and the run fails with `output pipes still open after the GUI exited (a descendant process may hold them)`.
  - Only the last 64 KiB of stderr is kept. Stdout lines are capped at 1 MiB, and a truncated line never counts as the run line.
  - The 23 `bench_runner.rs` tests pass on Windows (2026-10-14).
- **Rationale**: A hung GPU or driver must not hang the developer tool or leave processes behind (F5, D07, D08).
- **Limitations**:
  - Accepted (§17.8): the kill reaches only the direct child (accepted risk F5 below).
  - The channel between the readers and the run loop is unbounded. Memory stays bounded only while the loop keeps up, and the producer is the project's own GUI.
  - The fake-GUI process tests (pipe join timeout, kill at the deadline, caps) run only on Unix with `sh`.
- **Failure mode**: Fail-closed: a timeout or failure aborts the bench with exit code 1, and no report is written.

### Bench reports never overwrite an existing file
- **Location**: `xtask/src/bench/report.rs:L408-L452`
- **Language**: Rust 1.99.0
- **Description**: Reports are created with `OpenOptions::create_new` (O_EXCL / CREATE_NEW). An existing file or symlink at the target name is never followed or overwritten. The name gets a `-2`, `-3`, … suffix instead, up to 1000 attempts.
- **Rationale**: Integrity of earlier measurements, and no clobbering through a planted link in `bench/results/`.
- **Limitations**:
  - `create_dir_all` follows a symlinked `bench/` or `bench/results/` folder.
  - Reports record the values of `WGPU_BACKEND`, `WGPU_ADAPTER_NAME`, `WGPU_POWER_PREF`, `WAYLAND_DISPLAY` and `DISPLAY`, plus GPU and driver names. These are not secrets, but they identify the host. `bench/results/` is gitignored, so reports are never committed by accident.
- **Failure mode**: Fail-closed: after 1000 name collisions, or on any I/O error, the command fails.

### Test temporary folders under `CARGO_TARGET_TMPDIR`
- **Location**: `xtask/tests/layering.rs:L71-L80`, `xtask/tests/ci.rs:L537-L546`, `xtask/tests/bench_report.rs:L121-L130`, `xtask/tests/bench_runner.rs:L418-L426`, `xtask/tests/hooks.rs:L21-L30`. Durable rule in architecture §8.
- **Language**: Rust 1.99.0
- **Description**:
  - Every xtask test folder is `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>`.
  - It is created with `fs::create_dir`, which fails if the path exists.
  - `grep` finds no `std::env::temp_dir` or `create_dir_all` in the xtask or app tests (2026-10-14).
  - The hooks tests also assert that each temporary repository is its own git repository, never the project's.
- **Rationale**: F7: predictable names in the shared system temp folder let another local user pre-create the folder, or race a symlink so fixtures are written elsewhere.
- **Limitations**:
  - Folders stay behind when a test panics.
  - `CARGO_TARGET_TMPDIR` lies inside the target folder, so whoever controls that folder controls them. That is the developer.
- **Failure mode**: Fail-closed: a pre-existing path makes the test fail rather than reuse it.

### Secret-file patterns in `.gitignore`
- **Location**: `.gitignore:L1-L7`
- **Language**: gitignore patterns
- **Description**: `.env`, `.env.*` (except `.env.example`), `*.pem`, `*.key` and `secrets/` are ignored by git.
- **Rationale**: Prevents committing local credentials by accident.
- **Limitations**: Matches names only. Secrets in other files are not detected, and CI has no secret scanning. M0 uses no secrets at all.
- **Failure mode**: Fail-open for any file name the patterns do not cover.

---

## Accepted limitations

### Not-blocking tripwire bypasses (§18.1, §18.5): recorded here only, never reopened

| ID | Bypass | Why it is Not blocking | What covers it |
|----|--------|------------------------|----------------|
| QA2-F3 | Raw-identifier attribute `#[r#path = "x.inc"] mod x;` escapes S8 | Deliberate obfuscation (raw identifier) | Review; QA test `raw_identifier_path_attribute_is_checked`, kept ignored as the record |
| QA2-F4, R2-2 | Renamed or aliased built-in macro: `use std::include as inc; inc!("x.inc")`. The `.inc` file is never scanned. | Deliberate obfuscation (renamed macro) | Review; QA test `renamed_include_macro_is_flagged`, kept ignored as the record |
| R2-3 | A member's `[lib]`, `[[bin]]` or `[[test]]` `path`, or `build =`, pointing outside the package folder. Cargo builds the file, and the scan never sees it. | Deliberate (out-of-tree target path) | Review of manifest changes |
| Crate aliasing | `package =` renaming in a manifest, or `extern crate std as s; s::net::…` without the S7 identifiers | Deliberate | Layering checks the real dependency name. Review covers the rest. |
| Input-ignoring macros | A macro that ignores its input, e.g. `m!(deny(unsafe_code))` expanding to an `allow` | Deliberate | S6 still flags `unsafe` and `unsafe_code` written in the body of a workspace macro. A macro from a dependency that emits them is a review matter. |
| `std::r#net` | A raw-identifier path segment does not match S7a | Deliberate | Review; `TcpStream`, `UdpSocket` and the other S7b names are still flagged |
| R2-7 | Unix-domain sockets (`std::os::unix::net::UnixStream`, `UnixListener`, `UnixDatagram`) match no pattern | They are local IPC, not network, so the offline rule does not cover them (§18.5) | Review |
| Exclude tricks | `[workspace] exclude` combined with path dependencies | Deliberate | Since §18.6, excluded packages inside the root are reported by check 3d |
| Config include, quoted keys | cargo config `include = […]`, escaped quoted keys | Listed as Not blocking in §18.1 | Caught anyway by the canonical comparison (§18.4) |
| R2-11 | "Comment lines and a scissors line are ignored by the hook but kept in the commit when the message comes from `-m`/`-F`, `--cleanup=verbatim`/`whitespace`, or a non-default `core.commentChar`. Such a message passes locally and is caught by the CI `docs` job's log check after the push, which becomes a required check with branch protection." | Accepted wording of §18.2 | CI `docs` job; branch protection once enabled |
| R3-1 | rustc treats U+200E and U+200F (invisible direction marks) as whitespace between tokens. The scan's helpers use `char::is_whitespace`, which does not include them. So `include\u{200E}!("x.inc")` escapes S8, and `std::\u{200E}net` escapes S7a. Verified: rustc 1.99.0 compiles both under `-D warnings`, and `scan_text` gives 0 violations. | Deliberate (invisible characters) | Review. `TcpStream` and the other S7b names are still flagged. Optional one-line hardening: reuse the existing `is_rustc_whitespace` in `skip_whitespace`, `previous_non_whitespace`, `identifier_before` and the S7 compaction (`xtask/src/layering.rs:L657-L683`, `L761`). |
| R3-5 | Spacing and look-alike variants of AI credits: `Co-Authored-By : Claude <…>` (a valid git trailer: `git interpret-trailers` normalizes it), or non-ASCII look-alike letters in a name | Deliberate; tools emit the canonical spelling | Review |

### Accepted risks

| Risk | Impact | Why accepted | Decision |
|------|--------|--------------|----------|
| F5: no process-group or Job Object kill in the bench runner | A descendant of the GUI outlives the run. Its reader threads stay detached until xtask exits. | Needs unsafe code or a new dependency, and the bench GUI spawns no child processes | Accepted (§17.8). The run still fails within the deadline + 5 s. |
| User-level cargo configuration (`$CARGO_HOME/config.toml`, a `.cargo/config.toml` in a parent of the checkout or inside a member folder) adding `build.rustflags`, `[patch]` or `paths` | Lints silenced, `+crt-static` dropped or a dependency replaced on one developer machine | `cargo config get` is unstable; step 0 can only read the repository's file | Accepted (§17.3.3). CI runners have no such file, and CI runs from the root. |
| F10: DLL planting on Windows. The release executables import non-KnownDLLs (`dxgi.dll`, `opengl32.dll`, `uiautomationcore.dll`, `dwmapi.dll`, `uxtheme.dll`, …; dumpbin, 2026-10-14). wgpu and glutin load drivers at run time. | Code execution when an executable is started from a folder an attacker can write to (for example Downloads) | Nothing is distributed before M8. The fix needs an extra allow-listed unsafe file. | Accepted until M8 (§17.15). Planned, **not implemented**: before any Windows artifact ships, `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)` as the first statement of both `main`s, `/DEPENDENTLOADFLAG:0x800` in the windows-msvc rustflags (`CANONICAL_CARGO_CONFIG` updated in the same commit), and a Process Monitor check. |
| zbus and X11 environment transports (Linux): a `tcp:` D-Bus/AT-SPI bus address, or `DISPLAY=host:N`, makes the GUI open TCP connections | The GUI talks over the network when the user's environment says so | Required by accessibility (AccessKit -> zbus) and by X11 support. `async-io` and `polling` cannot be banned. | Accepted (architecture §9.3). The product itself never chooses a network address. |
| Safe socket APIs of crates already in the Linux graph (`rustix` with `net`, `async-io`) once one becomes a direct dependency | Socket I/O with no flagged name | These crates cannot be banned without breaking zbus | Accepted. Every new direct dependency goes through R6 (one place, `[workspace.dependencies]`) and Security review. |
| Code in doctests and `#[doc]` strings is not scanned, and the workspace lints do not apply to doctests | Network or unsafe code in test-only doctests runs during `cargo test`, on developer machines and CI | Doc comments are blanked by design. Doctests are never shipped. | Accepted. Reviewers check examples that touch I/O or memory; such examples should be `no_run`. |
| Code run at build time by dependency build scripts and proc macros (downloads, arbitrary code on developer machines and CI) | Supply-chain compromise during the build | cargo has no build sandbox, and cargo-deny checks only metadata | Accepted for M0. The lock file limits drift. Consider `cargo vet` from M1. |
| A malicious version published under a pinned crate before it is locked (account takeover, typosquat) | Code execution in the product or the tooling | Nobody reviews each transitive crate | Accepted for M0. Review `Cargo.lock` diffs; add CODEOWNERS on `Cargo.lock` once the repository exists. |
| AT-SPI exposes UI text to any client on the local D-Bus session bus (Linux) | Local processes of the same user can read what the window shows | Accessibility requirement, by design | Accepted |

---

## Threats identified and not covered

| Threat | Impact | Why not covered | Decision |
|--------|--------|-----------------|----------|
| R3-2: step 0 removes whitespace inside quoted strings, so `"target-feature=+crt-static "` passes. rustc only warns and links the CRT DLLs. | `+crt-static` silently lost after an unlikely typo. Only the manual E4 check would notice. | §18.4 specifies "remove all whitespace from each remaining line" | Open, low (Architect, then WP4): remove whitespace only outside string literals, or compare string contents exactly. Add a test with a space inside the feature string. |
| R3-3: check 3d skips folders named like an `IGNORED_DIRS` entry that `.gitignore` does not ignore (`out`, `env`, `.tox`, …) | A committed `va-arch-*` package in such a folder escapes `[lints]`/`[deps]` (layering and S6 still apply) | The walk reuses the README/AGENCEMENT exclusions | Open, low (WP5): in `iter_nested_manifests`, skip only `.git`, the target folders and gitignored paths. Add a fixture test. |
| R3-4: the CI log check cannot see an AI tool as commit *author* or *committer* | An AI-authored commit reaches history unnoticed | `COAUTHOR_RE` only matches trailer forms, `generated with` and the emoji; `%cn`/`%ce` are not logged | Open, low (WP5): add `%cn%n%ce` to `GIT_LOG_ARGS`, and match identity lines against a narrow pattern (`[bot]` suffix, `@anthropic.com`, `@openai.com`, the 12 names as whole words). |
| R3-6: exact `=` pins (rule R6) are not enforced by any tool | A caret requirement lets `cargo update` drift within the range | The `[deps]` check only collects `[workspace.dependencies]` names | Open, low (WP5): report every non-path `[workspace.dependencies]` entry whose version does not start with `=`. |
| R3-7: the ignored-test command recorded in §17.11 D10 (`-- --ignored --nocapture`) now also runs the two ignored §18.1 record tests, which fail by design (verified: 2 failed, live check ok) | The live check looks broken and may stop being run (`cargo xtask ci` still runs the same check) | The record tests were marked `#[ignore]` by §18.5 | Open, low (Architect, WP7): document `cargo test -p xtask --test layering -- --ignored live_workspace --nocapture`. Fixed in this file. |
| R3-8: CI depends on files that any pull request can change, and no branch protection exists | Every CI check above is advisory: a red run can still be merged, and a PR can weaken `deny.toml`, `ci.yml` or xtask | The GitHub repository does not exist yet | Pending the user's consent (§17.17 item 3): require `fmt`, `docs`, `deny` and the four `check (…)` jobs on `main`; add CODEOWNERS for `.github/`, `.cargo/`, `xtask/`, `scripts/`, `Cargo.lock`. |
| R3-9: the `xtask` alias (`run --package xtask --`) has no `--locked`. In the CI `check` job it runs first and rewrites a stale `Cargo.lock`, so the job's later `--locked` clippy, test and release-build steps pass (verified with a probe workspace, cargo 1.99.0) | A pull request that edits a manifest without updating `Cargo.lock` is built and tested in the `check` job with freshly resolved, uncommitted versions. Only the `deny` job fails. | Spec §3.3/§18.4 fix the alias, and the canonical comparison freezes it | Open, low (Architect, then WP4 and WP5): either add `--locked` to the alias (`xtask = "run --locked --package xtask --"`, with `.cargo/config.toml`, `CANONICAL_CARGO_CONFIG` and §18.4 updated in the same commit), or run `cargo run --locked --package xtask -- ci --no-fmt` in `ci.yml`. Add a test that the CI command fails on a stale lock. |
| R3-10: the usage comment of `.cargo/deny.toml` (L5-L8) shows the cargo-deny command without `--locked` | A command copied from it re-resolves a stale lock instead of failing | The comment predates §18.7, and WP1 had no change in that wave | Open, low (WP1, comment only): replace it with the §18.7 command. |
| Untrusted inputs (S-records, project files, settings, `INCLUDE` paths). On Windows, a UNC path (`\\host\share\…`) opened through `std::fs` starts an SMB connection and can leak the user's NTLM hash. | Parser crashes, path traversal, network access and credential leak through file paths | No parser and no file opening exist in M0 | Planned for M7, **not implemented** (architecture §9.5): the `SourceProvider` sandbox must also reject UNC and device paths (`\\?\`, `\\.\`). |
| Release signing, SBOM, signed commits | Users cannot verify where binaries come from | Nothing is released in M0 | M8 packaging |

## Secrets and configuration

M0 uses no secrets. The CI workflow references no repository secret. Its `GITHUB_TOKEN` is read-only
and never persisted in the checkout (`persist-credentials: false`). The workflow never calls `gh`. The
`gh` CLI on the developer machine keeps its credentials in the OS keyring, outside the repository.

Environment variables read by the project are configuration, never credentials:
- `VA_PYTHON` (xtask `docs`; an empty value counts as unset);
- `CARGO` and `CARGO_TARGET_DIR` (xtask);
- `WGPU_BACKEND`, `WGPU_ADAPTER_NAME`, `WGPU_POWER_PREF`, `WAYLAND_DISPLAY` and `DISPLAY` (bench; their values are recorded in the gitignored reports);
- `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `CARGO_BUILD_RUSTFLAGS` and `CARGO_TARGET_*_RUSTFLAGS` are read only to refuse them (`cargo xtask ci` step 0). They must stay **unset**, which rules out an empty value too (§18.3): any of them replaces the `+crt-static` flags of `.cargo/config.toml`.

There is nothing to rotate. When a secret is first needed (M8 signing), it must come from GitHub
encrypted secrets, scoped to the release workflow only, and be recorded here with its rotation
procedure.
