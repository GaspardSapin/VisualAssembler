# Security

This file lists the protections that are actually in place. The Security agent adds one entry per
protection when it is implemented. It is not a list of good intentions: anything not implemented is
left out, and planned work appears only in the tables at the end.

Last audit: the two M1-S10 passes of 2026-10-23. The first triaged the yanked `clipboard-win`
5.4.1 that turned the CI `deny` job red on `8994d44` (crates.io and GitHub read with `gh api`, the
5.4.1 and 5.4.3 sources reviewed read-only, cargo-deny run on a scratch export). The second recorded
the `Cargo.lock` bump to 5.4.3 that DevOps applied in the working tree (`git diff`, a read-only
cargo-deny run). They re-checked only that entry and the lock change. Before them: M1 recording
pass after the M1 fix wave, 2026-10-23, on `15e9fa4`. The fixes of
M1-S1, M1-S3 and M1-S4 (`a1f4294`), the advisory ignores of M1-S2 (`215e407`), the spacing change
`e764313` and the CI steps of `15e9fa4` were re-read. The release `va` was rebuilt with `--locked`,
and the escape, echo, fuzz and timing probes of the M1 round were run again (each entry says what
was run). Before it: M1 (core contracts, 68000 ISA tables, lexer, Fluent, `va lex` and `va isa`), Security
wave 3 of m1-core-isa-lexer.md §1, 2026-10-17 and 2026-10-18, on the M1 diff `cc827b0..9f86ae0`
(`9f86ae0` is a spec-only commit on top of `5d2d205`). The M1 crates were re-read, a release `va`
was built with `--locked` on Windows x64 and in WSL, and crafted inputs were run through it (each
M1 entry below says what was run). QA's uncommitted test additions in the tree were not audited.
Before it: M0 (foundations), the §19.9 recording pass (Security row of the m0-foundations.md
§19.9.6 fix wave), 2026-10-16, on the uncommitted §19 working tree with fix-wave rows 19-8 to 19-11
landed. A second recording pass the same day, after the final QA certification, re-checked the WP7
pilot-file statements against `README.md`, `CLAUDE.md` and `DEPENDANCE.md`, and read the branch
protection of `main` on GitHub (`gh api`, read-only). A third pass the same day, after the §19 work
was committed and pushed (`main` on GitHub is `6ed43c8`), recorded the CI state from GitHub Actions
run 38969165969 and re-read the branch protection (`gh run view`, `gh api`, read-only). Before them: audit round 1 of the §19 renderer
change (row 19-7), 2026-10-15, on rows 19-1 to 19-4; and security audit round 3, the first after the
fix wave of §18, 2026-10-14. Every entry below
was checked against the code at the date it gives. Where a check was actually run, the entry says
so. Bypasses are classified under the tripwire threat model below (§18.1).

- Findings of the M1 round are numbered M1-S1 to M1-S10. M1-S1 and M1-S2 are medium; the others
  are low. None is critical or high, so none is blocking. None is a tripwire bypass. All of them
  are listed under "Threats identified and not covered":
  - M1-S1, M1-S3 and M1-S4 (source text, C1 and bidi characters, and echoed values reaching the
    terminal raw) are fixed by `a1f4294` (`report::terminal_safe`). They are kept there as history;
  - M1-S2 (three RustSec "unmaintained" advisories of the zbus family, M0's Linux GUI graph) is
    accepted by `215e407` (m1 §12.8 F-2) and listed under "Accepted risks". `make security` exits
    0 again;
  - M1-S5 (output amplification, M2), M1-S6 (blocking inputs and UNC paths, M7) and M1-S7
    (no-panic guarantee tested only, M7) stay open;
  - M1-S8 and M1-S9 come from the recording pass of 2026-10-23 and are open: clap's usage errors
    echo the offending argument raw, and `terminal_safe` lets U+061C (ARABIC LETTER MARK) through;
  - M1-S10 (the yanked `clipboard-win` 5.4.1 of the Windows GUI graph, which turned the CI `deny`
    job red on `8994d44`) comes from the passes of 2026-10-23 and is fixed by the `Cargo.lock`
    bump to 5.4.3 (entry "Yanked `clipboard-win` 5.4.1 in the Windows GUI graph"). The fix is in
    the commit that bumps `Cargo.lock`; CI confirmation is pending. It is kept there as history.
- Findings of the §19 round are numbered S19-1 to S19-5: S19-1 is medium, the others are low. None
  is critical or high, so none is blocking. None is a tripwire bypass, so the §18.1 classes do not
  apply. §19.9.4 settled them:
  - S19-1: accepted for M0 (m0 §19.9 S19-1), listed under "Accepted risks", with M4 follow-ups;
  - S19-2, S19-3 and S19-4: fixed by rows 19-8, 19-9(b), 19-10 and 19-11, described in the entries
    they touch, and kept as history under "Threats identified and not covered";
  - S19-5: accepted limitation (m0 §19.9 S19-5), with three compensating controls.
- Round-3 findings are numbered R3-1 to R3-10. All are low and none is blocking.
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
- branch protection with required CI (§17.17 item 3). `main` requires `fmt`, `docs`, `deny` and the
  four `check (…)` jobs, but not for administrators (`enforce_admins` is off). The owner, the only
  collaborator, can therefore push to `main` directly or merge past a red check, so for the owner's
  own changes every CI check below stays advisory (branch protection entry, R3-8).

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
- **Location**: `.cargo/deny.toml:L57-L113` (bans and the `mio` feature ban), `.cargo/deny.toml:L10-L17` (graph: 4 targets, all features), `.cargo/deny.toml:L23-L34` (advisories), `.cargo/deny.toml:L115-L123` (workspace dependencies and sources), `.github/workflows/ci.yml:L52-L67` (CI `deny` job), `Makefile:L55-L58` (`make security`)
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
  - The `deny` job in `ci.yml` is identical to the §18.7 block. It has been green in every GitHub run so far: on the pre-§19 commit `c326afd` (run 38635672228, 2026-10-14), then on `646b6f7` (run 38968085878) and `6ed43c8` (run 38969165969), both 2026-10-16 (read with `gh run view` on 2026-10-16).

  Re-run on 2026-10-15 on the §19 tree (direct `winit` dependency, both renderers by default): the same command gave `advisories ok, bans ok, licenses ok, sources ok`. Re-run on 2026-10-16, after rows 19-8 to 19-11: same result.

  Re-run on 2026-10-18 on the M1 tree (`9f86ae0`, `make security`): `advisories FAILED, bans ok, licenses ok, sources ok`. The bans, licence and source checks pass with the M1 crates, so M1 adds no banned crate. The advisory failure is M1-S2: RUSTSEC-2026-0370 (`zvariant` 5.15.0), RUSTSEC-2026-0371 (`zbus_names` 4.3.4) and RUSTSEC-2026-0372 (`zvariant_derive` 5.15.0), all `unmaintained` (merged into zbus 6), reached through `accesskit_unix` 0.21.1 from eframe. The same three versions are in the `cc827b0` lock file, so the advisories are newer than the last green `deny` job (2026-10-16), not caused by M1. No advisory names an M1 crate.

  Re-run on 2026-10-23 on `15e9fa4`, after `215e407` added exactly those three ids to `[advisories] ignore` (`.cargo/deny.toml:L27-L34`, accepted risk M1-S2): `make security` gives `advisories ok, bans ok, licenses ok, sources ok`. `unmaintained = "all"`, `unsound = "all"` and `yanked = "deny"` are unchanged (`L24-L26`).
- **Rationale**: Architecture goal G4 and §9.3: an offline product with no telemetry, enforced by tooling rather than by promise. Resolves F3, F9 and R2-10.
- **Limitations**:
  - The list matches crate names, so a network-capable crate that is not named gets through. Examples: `nix` or `rustix` with their socket features, bindings to C networking libraries such as `zmq`, or `dns-lookup`.
  - `async-io` and `polling` cannot be banned, because zbus needs them. In the Linux graph, `rustix` 1.1.5 is built with its `net` feature. A workspace crate that adds `rustix` as a direct dependency gets safe socket APIs that neither cargo-deny nor the source scan sees (accepted risk below).
  - Transports that the environment selects are already in the Linux graph and are accepted: zbus 5.19 and x11rb 0.13.2 (accepted risk below).
  - `Cargo.lock` lists `tokio` 1.53.2, with feature `rt` only, for wgpu's `wasm32`-only `wasm-bindgen-futures`. cargo-deny prunes it, because `[graph] targets` has no wasm target. Adding a wasm target would make the ban fire.
  - Path sources are never checked by cargo-deny. The `[patch]`/`[replace]` and unlisted-package checks below close the realistic routes for such a source.
  - The check does not see what dependency build scripts and proc macros do at build time.
  - `cargo xtask ci` does not run cargo-deny. Only the CI `deny` job and `make security` do, and E1 runs `make security` on Windows only (§17.14).
  - `[bans.workspace-dependencies] unused = "deny"` (`.cargo/deny.toml:L115-L118`) is graph-based: it only checks that a pinned name, source and version exist somewhere in the resolved graph. A `[workspace.dependencies]` pin that no member declares still passes it when the crate is already a transitive dependency, and an internal path pin (M1+) matches its own member package (S19-3, 2026-10-15 probe under "Threats identified and not covered"). That stays true of cargo-deny. Since rows 19-9(b), 19-10 and 19-11, the member side of rule R6 ("only crates declared by at least one member") is enforced by the reverse half of check 3b of `scripts/check_project_docs.py` (supply-chain entry below), so S19-3 is fixed.
  - R3-10: the usage comment at `.cargo/deny.toml:L5-L8` predates §18.7 and shows the command without `--locked`. Someone who copies it gets a run that re-resolves a stale `Cargo.lock` instead of failing. The Makefile and CI use the full command.
  - The CI job finds `cargo-deny` through `PATH`:
    - `$RUNNER_TEMP/cargo-deny/bin` is prepended through `GITHUB_PATH`;
    - `~/.cargo/bin` is already on `PATH`, so cargo keeps the `PATH` order;
    - calling the binary by its absolute path would remove that dependency on order.
  - On a runner that has no rustup, `dtolnay/rust-toolchain` downloads `rustup-init` over HTTPS without a pinned checksum (seen in its `action.yml` at the pinned SHA).
- **Failure mode**: Fail-closed for the named crates and features, for unknown sources and for a stale lock file. Fail-open for crates that are not named.

### eframe built without its default features (no URL opening)
- **Location**: `Cargo.toml:L34`, `apps/visual-assembler/Cargo.toml:L23-L26`
- **Language**: TOML (Cargo manifest), Rust 1.99.0
- **Description**: eframe is pinned with `default-features = false`. It keeps only `accesskit`, `default_fonts`, `wayland` and `x11`, plus the renderer features of the build. Since §19.1 the shipped default build enables both `glow` and `wgpu`; single-renderer builds exist only for the CI clippy steps. So the `links` feature, which opens URLs in a browser through the `webbrowser` crate, is not compiled in. `webbrowser` is not in `Cargo.lock` (verified 2026-10-14 and again on 2026-10-15). eframe's `persistence` feature is off too, so eframe stores no app state on disk.
- **Rationale**: Offline product (G4). No UI path can open a web page.
- **Limitations**: Re-enabling eframe's default features would restore URL opening without any check failing. So would adding a crate that turns `links` back on through feature unification. No `[[bans.features]]` rule guards this.
- **Failure mode**: Fail-open on a manifest change. The protection holds only while the manifests stay as they are.

### Layering rules and GUI-crate families (rules R1 to R5 and R7)
- **Location**: `xtask/src/layering.rs:L68-L95` (GUI families, exact names, the `va-ui` allowance), `xtask/src/layering.rs:L136-L360` (crate map and `check_layering`). Run by `xtask/src/ci.rs:L416-L430`. Tests in `xtask/tests/layering.rs`.
- **Language**: Rust 1.99.0 (edition 2024)
- **Description**: The check runs on the output of `cargo metadata --no-deps`.
  - Every workspace package must have a rule in the §17.2 table. A package without one is a violation.
  - Every dependency kind is checked under the crate's real name, so a `package =` rename does not hide it.
  - Apps and xtask are leaves: no package may depend on them.
  - GUI crates are matched by family (§18.5, resolves R2-13). A crate counts as GUI when its normalized name (lower case, `_` and `-` equal) starts with `egui`, `eframe`, `epaint`, `emath`, `ecolor`, `wgpu`, `naga`, `winit`, `glutin`, `glow` or `accesskit`, or equals `raw-window-handle`. Only `visual-assembler` may use any of them. `va-ui` may use exactly `egui` and `egui_kittest`.
  - A malformed metadata document is a violation.

  The tests pass on Windows (2026-10-14, again on 2026-10-15). The §19 direct dependency on `winit` (GUI family) is declared only by `visual-assembler`, which the rule allows; the live check reports 0 violations (2026-10-15).
- **Rationale**: Rules R3 and R4. Besides the GUI stack, this keeps two network-capable stacks out of the headless CLI's dependency closure: AccessKit's zbus and the X11 client stack.
- **Limitations**: Only direct dependencies are checked. Two kinds of crate still pass: a third-party egui add-on whose name lies outside the families (for example `<x>-egui`), and a different GUI toolkit. R7 and review cover them. §18.5 rejected a resolved-graph check.
- **Failure mode**: Fail-closed: a package without a rule, a forbidden edge or a malformed document fails the step.

### Source scan of every workspace member (rules S1 and S3 to S9)
- **Location**: `xtask/src/layering.rs:L31-L66` (allow-lists and patterns), `xtask/src/layering.rs:L362-L634` (code view S5, shebang rule), `xtask/src/layering.rs:L636-L741` (token helpers), `xtask/src/layering.rs:L757-L817` (network, S7), `xtask/src/layering.rs:L843-L935` (inclusion, S8), `xtask/src/layering.rs:L984-L1022` (per-file rules), `xtask/src/layering.rs:L1024-L1155` (walk: S1, S3, S4, S9), `xtask/src/layering.rs:L1157-L1204` (live workspace). Run by `xtask/src/ci.rs:L416-L430`: the "layering and source checks" step of `cargo xtask ci`, in the CI `check` job on 4 runners. Regression tests in `xtask/tests/layering.rs`.
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

  Verified on 2026-10-14, on Windows, and re-run on 2026-10-15 on the §19 tree:
  - `cargo test -p xtask --locked`: 89 layering tests pass and 3 are ignored (2026-10-15 count).
  - `cargo test -p xtask --test layering -- --ignored live_workspace` reports 0 violations on the live workspace (both dates).
  - Keep the name filter in that command. Without it, `-- --ignored` and `--include-ignored` also run the two §18.1 record tests, which fail by design (R3-7).
  - 17 edge cases were run through the real `scan_text` in a scratch crate: escaped quotes, byte, char, raw and C strings, nested comments, raw lifetimes, BOM plus shebang, `#!` with an attribute on the next line, `include!(concat!(env!("OUT_DIR"), …))`, and `cfg_attr(…, path = …)`. Each one gave the expected result.

  Verified on 2026-10-18 on the M1 tree (`9f86ae0`, Windows):
  - `cargo test --locked -p xtask --test layering -- --ignored live_workspace` reports 0 violations. The walk now covers the three M1 library crates and the new `va` sources.
  - `git diff cc827b0..9f86ae0` touches neither `xtask/`, `scripts/`, `.cargo/` nor `.github/`. The allow-lists are unchanged.
  - The M1 code has no network API, no `std::process` and no file write. The only file read is `File::open` in `apps/va/src/input.rs:L55` (line as of `15e9fa4`). The only environment reads are `VA_LANG`, `LC_ALL`, `LC_MESSAGES` and `LANG` (`apps/va/src/commands/mod.rs:L41-L43` as of `15e9fa4`).
  - `arch/m68k/src/locales.rs:L10-L19` embeds the four `.ftl` files with `include_str!("../locales/…")`. S8 covers `include!` and `#[path]` only, so these `..` paths are not checked. They embed data, not code (m1 §6.1).
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
- **Location**: `Cargo.toml:L45-L56` (`unsafe_code = "deny"`, `clippy::undocumented_unsafe_blocks`), `xtask/src/layering.rs:L31-L36` (`UNSAFE_ALLOWLIST`), `xtask/src/layering.rs:L819-L841` (S6), `xtask/src/ci.rs:L39-L84` (clippy with `-D warnings`: all features, then glow only, then wgpu only (step 3b, §19.6); tests)
- **Language**: Rust 1.99.0 (edition 2024), TOML
- **Description**: Every member inherits `unsafe_code = "deny"`. rustc 1.99.0 also reports `global_asm!` under this lint (verified in round 2). Outside the three allow-listed files (`apps/visual-assembler/src/bench/process_windows.rs`, `process_linux.rs` and `platform.rs`), S6 flags:
  - every whole-word `unsafe` token in the code view, including edition-2024 forms such as `unsafe extern` and `#[unsafe(no_mangle)]`;
  - every `unsafe_code` token whose innermost enclosing group, of any kind, is not introduced by `deny(` or `forbid(` (§18.5, CR1-02).

  More rules:
  - Wrapped, macro-passed, `expect(…)` and `cfg_attr(…, allow(unsafe_code))` forms therefore all fail.
  - Clippy with `-D warnings` stops `warn(unsafe_code)` and any unsafe block without a `// SAFETY:` comment.
  - Rule R8 is enforced by the `[lints]` check and by step 0 (entries below).
  - In M0 the workspace has 5 unsafe blocks, all in the allow-listed files. The §19 change adds none, and `UNSAFE_ALLOWLIST`, `NET_ALLOWLIST` and `INCLUDE_ALLOWLIST` are unchanged (`xtask/src/layering.rs` has no diff; 2026-10-15). The three clippy configurations of the GUI (all features, glow only, wgpu only) pass with `-D warnings` (2026-10-15).
  - `platform.rs` allows `unsafe_code` on `gl_string` only, so the compiler still rejects unsafe code anywhere else in that file. The two process files allow it for the whole file.
  - M1 (checked 2026-10-18 on `cc827b0..9f86ae0`): the diff adds no `unsafe` token, no `unsafe_code` attribute and no allow-list entry (E8). The live S6 scan reports 0 violations. The allocation-free lexer claim (m1 §3.4) is reviewed by eye only, because a counting allocator would need `unsafe` (m1 §12.4, M7).
- **Rationale**: Architecture §9.4. Memory safety by default, with every exception visible and reviewed (F1, F2).
- **Limitations**:
  - The scan bypasses under "Accepted limitations" hide `#![allow(unsafe_code)]` and `unsafe` from S6. The compiler lint stays the primary guard.
  - The allow-list works per file. Unsafe code can be added anywhere in the two process files without the scan noticing, and inside `gl_string`.
  - Unsafe code emitted by a proc macro is not seen by the scan.
  - Unsafe code in dependencies (wgpu, winit, windows-sys, glow and others) is not audited. Only RustSec `unsound` advisories cover it.
- **Failure mode**: Fail-closed through the compiler lint and S6 for ordinary code.

### Workspace lint inheritance check (rule R8, lint part)
- **Location**: `scripts/check_project_docs.py:L89-L90`, `scripts/check_project_docs.py:L469-L516` (`[lints]`), `scripts/check_project_docs.py:L310-L353` (member list). Run by the CI `docs` job (`.github/workflows/ci.yml:L42-L50`) and by `cargo xtask docs` (`xtask/src/docs.rs:L43-L63`).
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
- **Location**: `scripts/check_project_docs.py:L102-L113` (constants), `scripts/check_project_docs.py:L519-L606` (`path_identity`, `iter_nested_manifests`, `check_implicit_members`, `check_patch_tables`), `scripts/check_project_docs.py:L693-L697` (calls). Run by the CI `docs` job and by `cargo xtask docs`.
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
  - In CI the problem is found after the push. The `docs` job is a required check on `main`, so it blocks a pull-request merge, but not a direct push or a bypass by the owner (`enforce_admins` off, R3-8).
- **Failure mode**: Fail-closed for walked manifests, parse errors and unlistable folders. Fail-open for the skipped folder names.

### Build-flag integrity check (`cargo xtask ci` step 0, rule R8 build part)
- **Location**: `xtask/src/ci.rs:L86-L123` (variable names, `CANONICAL_CARGO_CONFIG`), `xtask/src/ci.rs:L200-L239` (environment), `xtask/src/ci.rs:L241-L359` (canonical comparison), `xtask/src/ci.rs:L361-L387` (files), `xtask/src/ci.rs:L389-L414` (step 0), `xtask/src/ci.rs:L156-L171` (step order). Tests in `xtask/tests/ci.rs`.
- **Language**: Rust 1.99.0
- **Description**: Step 0 runs before every other step, also with `--no-fmt`. `cargo xtask ci` fails with exit 1 when any of these holds:
  - `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `CARGO_BUILD_RUSTFLAGS` or any `CARGO_TARGET_*_RUSTFLAGS` is **set, even to an empty value** (§18.3, resolves R2-6). Names are compared ignoring ASCII case on every OS, and a non-Unicode name is compared in its lossy form.
  - `.cargo/config.toml` differs from `CANONICAL_CARGO_CONFIG` (8 lines) after normalization: blank lines and full-line `#` comments are dropped, and all whitespace is removed from each remaining line. Every extra, missing, changed or reordered line is reported with its line number (§18.4, resolves R2-5). That covers top-level keys, `include`, quoted keys, `runner`, alias changes and trailing comments.
  - `.cargo/config.toml` is missing or unreadable.
  - The extension-less `.cargo/config` exists, or cannot be inspected.

  Verified on 2026-10-14:
  - The 33 `ci.rs` tests pass on Windows. They include the live configuration and the binary test that sets each variable to an empty value. On 2026-10-15, with step 3b added by §19.6, the file has 40 tests and all pass on Windows; step 0 still comes first.
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

  §19 (2026-10-15): the three files are unchanged. `gl_string` is compiled only with `renderer-glow`, so it is now part of the shipped default binary, where it runs on every glow bench run. It never runs in a wgpu attempt, because `RendererInfo::from_frame` calls it only when `frame.gl()` is `Some`. So the GL context that a failed glow attempt leaves current (see the fallback entry) is never queried. The 7 `bench_process` tests pass on Windows on the §19 tree.
- **Rationale**: Soundness of the only unsafe code in the product (architecture §9.4).
- **Limitations**:
  - The `catch_unwind` works only with `panic = "unwind"`. Nothing enforces the root `[profile]` values, so a future `panic = "abort"` profile would turn a driver that returns NULL into a crash.
  - The struct layouts are those of windows-sys and libc and were not checked independently.
  - The ARM64 Windows and ARM64 Linux paths run only on the CI runners. Their 7 `bench_process` tests passed there on the pre-§19 commit `c326afd` (run 38635672228) and again on `6ed43c8` (run 38969165969: `test result: ok. 7 passed` in jobs `check (windows-11-arm)` and `check (ubuntu-24.04-arm)`; logs read with `gh run view` on 2026-10-16).
- **Failure mode**: Fail-safe: a failed OS query yields `null` in the bench line, never undefined behaviour.

### Renderer failure-injection hook `VA_TEST_FAIL_RENDERER` (§19.3): bounded effect, warned, recorded
- **Location**:
  - name: `apps/visual-assembler/src/cli.rs:L32-L33`;
  - parsing and startup warning: `cli.rs:L382-L455` (`InjectedFailures`, `startup_warning`, `parse_injected_failures`);
  - the injected error: `cli.rs:L496-L514` (`RendererInitError::Injected`);
  - the only read of the variable, and its warnings: `apps/visual-assembler/src/lib.rs:L134-L139`;
  - the failing app creator: `lib.rs:L264-L281` (`app_creator`);
  - bench recording: `apps/visual-assembler/src/bench/recorder.rs:L70-L80` (`injected_failure`), `xtask/src/bench/report.rs:L42-L53` (report `env` map);
  - CI: `.github/workflows/ci.yml:L99-L108`.

  Tests: `tests/cli.rs` (`injection_list_parses`, `injection_items_are_checked_one_by_one`, `non_unicode_injection_variable_is_ignored_as_a_whole`, `injection_values_are_echoed_escaped`, `non_unicode_values_are_echoed_escaped`, `fallback_eligibility`), `tests/binary.rs` (Linux only: `platform_failure_never_falls_back`, `selection_warnings_come_first_and_in_order`, `control_characters_in_selection_variables_reach_stderr_escaped`), `tests/fallback.rs` (10 tests, 8 of them compiled on Windows x64, all `#[ignore]`, need a display).
- **Language**: Rust 1.99.0, GitHub Actions YAML
- **Description**: The hook is compiled into every build, release included (a §19.3 decision). What bounds it:
  - **Environment only.** The variable is read once, with `std::env::var_os`, in `select`. There is no command-line flag, file or setting for it. A grep on 2026-10-15 finds no other read in `apps/`, `xtask/`, `crates/` or `arch/`.
  - **Strict values.** It takes a comma-separated list of exactly `glow` and `wgpu`. Items are trimmed, and empty items are skipped. Any other item, case variants such as `GLOW` included, prints `warning: ignoring unknown VA_TEST_FAIL_RENDERER value '<v>' …` and injects nothing. A non-Unicode value is ignored as a whole, with one warning.
  - **Bounded effect.** When a listed renderer's app creator runs, it returns `RendererInitError::Injected` instead of the app. The real OpenGL or wgpu initialization has already happened at that point. So the hook can only do two things: make `auto` take its single fallback, or make a forced renderer (or both renderers) exit 3. Everything it reaches is also reached by `--renderer wgpu` after a genuine glow failure. It changes no path, library, backend or window system. It does not turn a platform failure (no display) into a fallback (tested).
  - **Warned.** When at least one renderer is injected, `warning: VA_TEST_FAIL_RENDERER=<list> is set: simulating a renderer initialization failure (test hook)` goes to stderr before any window opens.
  - **Escaped (S19-2, fixed by row 19-8 a).** Every echoed value goes through `str::escape_debug` of its lossy text: the startup warning (`cli.rs:L407-L410`), the unknown-item warning (of the trimmed item, `cli.rs:L447-L450`) and the non-Unicode warning (`cli.rs:L430-L433`). A stray CR, newline or escape sequence shows as `\r`, `\n` or `\u{1b}` instead of breaking the line or driving the terminal. `InjectedFailures::raw`, reported as `injected_failure`, stays unescaped, and serde_json escapes it in the bench line.
  - **Recorded.** The bench line records the raw value (`injected_failure`) and the fallback (`fallback_from`, `fallback_reason`). The xtask report records the variable in its `env` map and counts fallbacks. Its progress lines print `renderer wgpu (fallback from glow)`. A forced bench run that falls back is rejected.
  - **CI.** Since row 19-9(a), the variable is set only through the step-level `env:` of one `continue-on-error` step (`ci.yml:L106-L107`), from `${{ runner.arch == 'ARM64' && 'wgpu' || 'glow' }}`: the first renderer of the runner's `auto` order. No other step sees it (CI hardening entry). Observed in run 38969165969 (`6ed43c8`, 2026-10-16; smoke-step logs read with `gh run view`): on both Windows images the startup warning was printed and `injected_failure` recorded the value. On `windows-11-arm` (`wgpu`) the injected error was reached: `the wgpu renderer failed to initialize: injected renderer failure (VA_TEST_FAIL_RENDERER); falling back to glow`. On `windows-latest` (`glow`) it was not: glow failed genuinely first (`egui_glow requires opengl 2.0+`), so `fallback_reason` is that GL error, and the step took the same fallback as the plain `auto` step.

  Verified on 2026-10-15:
  - The headless tests pass: on Windows, `tests/cli.rs` 31, `tests/binary.rs` 12 and `tests/renderer_backend.rs` 7, with `tests/fallback.rs` 4 ignored; in WSL, 32, 17 and 5.
  - A WSL probe without a display, with `VA_TEST_FAIL_RENDERER=" glow ,GLOW,<ESC>[2Jz"`, injected glow only. It warned for `GLOW` and for the escape item, printed the startup warning, then exited 3 with the window error and no fallback.
  - The §19.3 local tests (real windows) were run by WP2 on Windows and WSLg X11. Security did not re-run them.

  Verified on 2026-10-16, after row 19-8: the code was re-read, and the headless tests pass: on Windows, `tests/cli.rs` 45, `tests/binary.rs` 13 and `tests/renderer_backend.rs` 10, with `tests/fallback.rs` 8 ignored; in WSL, 46, 22 and 8. In WSL, `control_characters_in_selection_variables_reach_stderr_escaped` replays the 2026-10-15 S19-2 probe on the binary: stderr holds exactly three escaped warnings and the window error, with no control character other than the line ends. Security did not re-run the ignored tests.
- **Rationale**: §19.3: the fallback path of the shipped binary must be testable and measurable (N4). Footgun analysis: setting a process's environment already gives full control of that process, so the hook grants nothing new. Its effect is bounded and fail-safe. The loud warning keeps it from going unnoticed on a console; on the Windows release build, bench JSON and, from M4, the status bar do (§19.9 S19-1).
- **Limitations**:
  - S19-1 (medium), accepted for M0 (m0 §19.9 S19-1): on Windows, the release GUI uses the GUI subsystem, so stderr exists only when it is redirected. When the app is started from Explorer, the Start menu or a PowerShell prompt, the "loud" warning is invisible, and so are the §19.4 fallback and error messages. If the variable is in the session or the user environment (`$env:` or `setx`), every launch silently runs wgpu (about 342 MiB instead of 93 MiB, §19.7). With `glow,wgpu`, the app exits 3 with no window and no message. M0 shows nothing in the window. Accepted because nothing is distributed before M8, whoever sets the environment already controls the process, and bench JSON records `renderer`, `fallback_from`, `fallback_reason` and `injected_failure`. Deferred to M4, **not implemented**: the status bar shows a fallback and an armed hook, and M4 decides how an exit-3 startup failure reaches a user with no console. Written by the WP7 row of the §19.9.6 fix wave (checked 2026-10-16): the root README says that the release `visual-assembler.exe` has no console and that every warning and error is visible only when stderr is redirected (Git Bash, a pipe, the debug build) (`README.md:L240-L243`, "Renderer selection", "On Windows"). The window-title suffix was rejected. See "Accepted risks".
  - An injected glow failure leaves glutin's destroyed GL context current on the main thread, as a genuine late GL failure does. The fallback entry below describes how wgpu's GL backend is kept out of that state.
  - Child processes inherit the variable. The M0 GUI starts none.
  - With the variable exported, `cargo xtask bench --renderer auto` measures the fallback path. It is visible in the progress lines, in `fallbacks` and in the `env` map.
- **Failure mode**: Fail-safe: an unknown, malformed or non-Unicode value injects nothing. The worst possible effect is exit 3 at startup.

### Renderer selection from the environment: `VA_RENDERER` and `VA_WAYLAND` (§19.1)
- **Location**: `apps/visual-assembler/src/cli.rs:L29-L36` (names), `cli.rs:L84-L92` (`RendererChoice::from_name`, exact match), `cli.rs:L302-L380` (`auto_order`, `resolve_renderer`; the escaped warning at `L342-L345`), `cli.rs:L457-L494` (`window_system_policy`), `apps/visual-assembler/src/lib.rs:L115-L156` (`select`: the only reads and the warnings), `lib.rs:L365-L391` (X11 event-loop hook, `apply_window_system`), `apps/visual-assembler/src/bench/platform.rs:L65-L84` (`is_wsl`). Tests in `tests/cli.rs` (`renderer_precedence`, `non_unicode_renderer_variable_is_ignored_with_a_warning`, `variable_auto_is_an_explicit_choice`, `an_overridden_variable_is_never_checked_against_the_build`, `wsl_uses_x11_unless_va_wayland_is_1`, `non_unicode_va_wayland_is_not_1`, `invalid_renderer_variable_is_echoed_escaped`, `non_unicode_values_are_echoed_escaped`), `tests/binary.rs` (Linux only: `selection_warnings_come_first_and_in_order`, `control_characters_in_selection_variables_reach_stderr_escaped`) and `tests/renderer_backend.rs` (`x11_hook_follows_the_window_system_policy`).
- **Language**: Rust 1.99.0
- **Description**:
  - `VA_RENDERER` accepts exactly `auto`, `glow` or `wgpu`: case-sensitive and not trimmed. Any other value, an empty or non-Unicode one included, prints `warning: ignoring VA_RENDERER='<v>' (expected auto, glow or wgpu)` and is ignored. `<v>` is `str::escape_debug` of the lossy text (S19-2, fixed by row 19-8 a). The warning is printed even when `--renderer` overrides the variable (m0 §19.9 WP2-5). Selection then falls to the next level, today the built-in `auto`. An invalid value is never an error and never selects another renderer.
  - `--renderer` comes first. It is a clap value enum, so an invalid value exits 2.
  - A renderer forced through `VA_RENDERER` never falls back. A failure exits 3 with `(forced by VA_RENDERER; try --renderer auto)`.
  - `VA_WAYLAND` is compared, as a raw `OsStr`, with exactly `1`. Under WSL, any other value, `true`, ` 1` and non-Unicode included, keeps the X11 default (verified: `VA_WAYLAND=true` gave winit's "Failed to open connection to X server", the X11 path). Outside WSL the variable has no effect. With `1` and a plan that includes wgpu, a warning is printed.
  - The values only choose between compiled code paths. No path, library name, URL or command is built from them. They are never passed to a shell or to a file API.

  Verified on 2026-10-16, after row 19-8: the code was re-read, and `tests/cli.rs`, `tests/binary.rs` and `tests/renderer_backend.rs` pass (`cargo test --locked -p visual-assembler`): 45, 13 and 10 tests on Windows; 46, 22 and 8 in WSL.
- **Rationale**: §19.1 and architecture §6/§9.8. Every bad value falls back to the safe default (auto, X11 under WSL), so a typo never crashes the WSLg compositor or disables the fallback.
- **Limitations**:
  - S19-2 (low), fixed by row 19-8(a) (m0 §19.9 S19-2): invalid values were echoed unescaped to stderr. They now go through `escape_debug`, so a CR, newline or escape sequence shows as `\r`, `\n` or `\u{1b}`. The 2026-10-15 probe is kept as history under "Threats identified and not covered".
  - S19-1, accepted for M0 (m0 §19.9 S19-1): on a Windows release build, a `VA_RENDERER=glow` left in the user environment silently turns the fallback off. A GL failure then ends with exit 3 and no visible message. M4 decides how such an exit-3 startup failure reaches a user with no console (**not implemented**). See "Accepted risks".
  - `VA_WAYLAND=1` with wgpu under WSLg is only warned about, not refused. A weston 9 crash takes down every WSLg window of the user's session. The user opted in.
  - `is_wsl()` returns `false` when `/proc/sys/kernel/osrelease` cannot be read. winit then keeps its default (Wayland first) under WSLg, which brings back the compositor-crash case for wgpu.
  - The `settings.toml` level (`[render] renderer`) comes in M4 and is **not implemented**.
- **Failure mode**: Fail-safe: an invalid value degrades to `auto` and, under WSL, to X11.

### In-process renderer fallback: at most once, only before the first frame, panics not caught (§19.2)
- **Location**: `apps/visual-assembler/src/lib.rs:L178-L262` (`launch`), `lib.rs:L283-L363` (`native_options`, `wgpu_backend_env_is_effective`, `exclude_wgpu_gl_backend`), `lib.rs:L393-L412` (DX12 pin), `apps/visual-assembler/src/cli.rs:L516-L540` (`is_renderer_init_error`, `is_fallback_eligible`), `cli.rs:L554-L633` (`AttemptFailure`, `classify_failure`), `apps/visual-assembler/src/spike.rs:L36-L42` and `L62-L68` (`ui_started`), `apps/visual-assembler/src/bench/recorder.rs:L142-L146` (`record_fallback`). Tests in `tests/cli.rs` (`fallback_eligibility`, `auto_falls_back_once_with_the_spec_warning`, `second_attempt_failures_never_fall_back_again`, `forced_renderer_error_after_the_first_frame_keeps_the_window_message`, `late_or_foreign_errors_keep_the_window_message`, `an_attempt_outside_the_plan_is_fatal`), `tests/renderer_backend.rs` (`wgpu_backend_env_is_effective_only_for_a_non_empty_backend_set`, `wgpu_gl_backend_is_left_out_after_a_glow_failure`, `dx12_pin_applies_to_every_wgpu_attempt`, `attempt_options_follow_every_kind_of_wgpu_backend_value`, `every_attempt_option_combination_keeps_the_fallback_preconditions`), `tests/fallback.rs` (ignored).
- **Language**: Rust 1.99.0 (eframe 0.36.2, winit 0.30.13)
- **Description**:
  - A plan holds one renderer when forced, or at most the two distinct renderers of `auto_order`. After a failed attempt, `classify_failure` falls back only when all three conditions hold:
    - the plan is `auto` and has a next renderer;
    - `ui` never ran in this attempt. A fresh `Arc<AtomicBool>` is made per attempt and set at the top of `SpikeApp::ui`;
    - the error is a renderer initialization error: `Glutin`, `NoGlutinConfigs`, `OpenGL`, `Wgpu`, or `AppCreation` carrying `RendererInitError`.

    `Winit` and `WinitEventLoop` errors, foreign `AppCreation` errors and errors after the first frame exit 3. So a process falls back at most once (`both_failures_exit_3` counts exactly one `falling back`).
  - Each attempt gets fresh `NativeOptions` and a fresh app creator.
  - The bench recorder lives in `launch`, and each attempt's app only borrows it (`Option<&mut BenchRecorder>`). The borrow checker therefore guarantees that the failed attempt's app is gone before the next attempt starts. No `unsafe` was added: the live S6 scan reports 0 violations and the allow-lists are unchanged.
  - **Panics are not caught.** `lib.rs` has no `catch_unwind`. A panic in either attempt, for example eframe's `expect` on an empty glutin config list or a wgpu-hal panic, propagates out of `run_native` and ends the process with exit 101. So no attempt ever runs on winit or eframe state left by an unwind (§19.2). The release profile keeps `panic = "unwind"`, the default.
  - A wgpu attempt that follows a failed glow attempt leaves out wgpu's GL backend (`exclude_wgpu_gl_backend`), unless `WGPU_BACKEND` is effective (valid Unicode and at least one backend wgpu recognizes; `wgpu_backend_env_is_effective`, m0 §19.9 S19-4). glutin leaves the destroyed GL context current, and wgpu-hal 30.0.1 then panics on `EGL_BAD_ACCESS` (WP2 observation under WSLg X11). On Windows, the DX12 pin applies to every wgpu attempt, forced or fallback, unless `WGPU_BACKEND` is effective.
  - The first attempt fixes the window system: the X11 hook is installed on every attempt, and eframe uses it only when it creates its thread-local event loop (m0 §19.9 WP2-4).
  - `startup_ms` keeps counting from `main()`, and the watchdog limit is not reset.

  Verified on 2026-10-15: the code was re-read, and the tests above pass on Windows and in WSL (headless). The dual-renderer release binary imports the same DLLs as the earlier wgpu-only one (static CRT entry).

  Verified on 2026-10-16, after row 19-8: the code was re-read; `tests/cli.rs` and `tests/renderer_backend.rs` pass (45 and 10 on Windows, with `tests/fallback.rs` 8 ignored; 46 and 8 in WSL, with 9 ignored); `lib.rs` and `cli.rs` contain no `unsafe` and no `catch_unwind`; `xtask/src/layering.rs` has no diff against `HEAD`, and `cargo test --locked -p xtask --test layering -- --ignored live_workspace` reports 0 violations. Security did not re-run the ignored tests.
- **Rationale**: §19.2, architecture §9.8. The fallback must never hide a failure that the other renderer would also hit, never run twice, and never resume after undefined event-loop state.
- **Limitations**:
  - Leaving out wgpu's GL backend after a glow failure was a WP2 addition. Ratified by m0 §19.9 WP2-1. Accepted consequences:
    - a Linux host where GL is wgpu's only usable backend (no Vulkan driver, not even Mesa's lavapipe) exits 3 after a glow failure, instead of retrying the OpenGL driver that has just failed;
    - `WGPU_BACKEND=gl`, or any effective value whose backend set includes GL, brings wgpu-GL back at the user's risk, and with it the panic path (exit 101).
  - S19-4 (low), fixed by row 19-8(b) (m0 §19.9 S19-4). Before the fix, `WGPU_BACKEND` counted as set as soon as `std::env::var_os` found it, while wgpu reads it with `std::env::var(..).ok()`. `wgpu_backend_env_is_effective` (`lib.rs:L326-L338`) now holds exactly when the value is valid Unicode and `Backends::from_comma_list` of it is not empty. The DX12 pin and the GL exclusion step aside only for an effective value. An empty or non-Unicode value therefore no longer drops the pin or brings back the `EGL_BAD_ACCESS` panic. `attempt_options_follow_every_kind_of_wgpu_backend_value` re-runs the env-dependent tests in a child process under ten values (unset; empty, blank, `,`, `bogus`, non-Unicode; `vulkan`, `gl`, `DX12`, ` vk , bogus`). Accepted residue: on Linux, an empty or unrecognized `WGPU_BACKEND` still leaves wgpu with no backend, so forced wgpu, or the fallback, exits 3. That is wgpu's own reading of a variable that asked for no backend. On Windows the pin applies in that case.
  - A native driver crash (SIGSEGV or abort, for example glow's GLX path under `LIBGL_ALWAYS_INDIRECT=1`, exit 139 under WSLg) and any panic end the process. Neither can fall back (accepted limitation, m0 §19.9 WP2-13). Written by the WP7 row of the §19.9.6 fix wave (checked 2026-10-16): the README's "Renderer selection" section lists a panic (exit 101) and a crash inside the graphics driver among the cases with no fallback, gives the `LIBGL_ALWAYS_INDIRECT=1` example, and tells the user to start with `--renderer wgpu` (or `glow`) instead (`README.md:L194-L199`, "Fallback (`auto` only)").
  - After a glow failure, the destroyed GL context stays current for the rest of the process. No project code issues GL calls in a wgpu attempt (see the unsafe FFI entry).
  - Interactive (non-bench) launches have no watchdog. A driver hang during the first attempt hangs the process (unchanged since M0).
  - S19-5 (low), accepted limitation (m0 §19.9 S19-5): the real retry needs a window, so it runs only in the ignored local tests and in the informational CI step. No gating test exercises it. Compensating controls:
    1. Since row 19-9(a), the second Windows smoke step injects a failure into the first renderer of the runner's `auto` order: glow on x64 and wgpu on `windows-11-arm`. ARM64 now exercises wgpu to glow. The step stays `continue-on-error` (CI hardening entry).
    2. Binding QA rule, procedural and enforced by no tool: run `cargo test -p visual-assembler --test fallback -- --ignored --test-threads=1` on Windows and under WSLg X11 whenever `Cargo.lock` changes the version of eframe, egui, egui-wgpu, egui_glow, wgpu, glutin or winit, and before each milestone closes.
    3. The headless tests cover the classification, the §19.4 messages and every attempt's options (`tests/cli.rs`, `tests/renderer_backend.rs`).

    An assertion inside the smoke step was rejected, because a `continue-on-error` step cannot gate. m0 §19.9 S19-5 says to revisit this "when branch protection makes CI required (§17.17 item 3)". That condition is now met: the four `check (…)` jobs are required on `main` (branch protection entry, read 2026-10-16). The revisit has not happened yet (Architect). Until it does, both smoke steps stay `continue-on-error`, so they still cannot fail a required check.

    First CI observation of the real retry, informational only: run 38969165969 (`6ed43c8`, 2026-10-16; smoke-step logs read with `gh run view`):
    - `windows-latest`, plain `auto`: glow failed genuinely (`egui_glow requires opengl 2.0+`), one `falling back to wgpu` warning, then wgpu on DX12 (adapter `Microsoft Basic Render Driver`, WARP), one frame painted, exit 0. The injected-glow step took the same path (hook entry).
    - `windows-11-arm`, plain `auto`: wgpu first, no fallback, exit 0. With `VA_TEST_FAIL_RENDERER=wgpu`: the injected wgpu failure, one `falling back to glow` warning, then a genuine glow failure, `error: no renderer could be initialized (tried wgpu, then glow): …` and the README note (§19.4), exit 3, not a panic (101) or a timeout.
    - The ARM64 step is `continue-on-error`, so `check (windows-11-arm)` stayed green. While that image has no OpenGL 2.0+, exit 3 is the expected outcome of the step, so the outcome alone cannot tell a regression from it; only the log lines can. This is S19-5 itself.
- **Failure mode**: Fail-closed: anything outside the three conditions exits 3, and a panic exits 101.

### `winit` as a direct Linux dependency (X11 event-loop hook, §19.1)
- **Location**: `Cargo.toml:L41-L43`, `apps/visual-assembler/Cargo.toml:L39-L44`, `apps/visual-assembler/src/lib.rs:L365-L384` (`apply_window_system`, Linux), `Cargo.lock` (one new dependency edge), `scripts/check_project_docs.py:L387-L466` (reverse half of check 3b, which keeps the pin declared)
- **Language**: TOML, Rust 1.99.0
- **Description**:
  - `winit = { version = "=0.30.13", default-features = false, features = ["x11"] }` is pinned once in `[workspace.dependencies]`. `visual-assembler` declares it only under `[target.'cfg(target_os = "linux")'.dependencies]`, with `workspace = true`.
  - It is the version and checksum that eframe already pulled. The `Cargo.lock` diff adds only the dependency edge: no new package, source or licence (Apache-2.0, allowed).
  - eframe's `x11` feature already enabled winit's `x11` feature, so the resolved feature set is unchanged. `cargo tree --target x86_64-unknown-linux-gnu -e features` on 2026-10-15 lists `ahash, bytemuck, memmap2, percent-encoding, rwh_06, sctk, wayland, wayland-backend, wayland-client, wayland-protocols, wayland-protocols-plasma, x11, x11-dl, x11rb`.
  - It is used for one call, `EventLoopBuilderExtX11::with_x11`. winit is in a GUI family, so layering allows it only in `visual-assembler`.
- **Rationale**: §17.13 item 8 and §19.1. A single window system per process (X11 under WSL) keeps the in-process fallback safe and avoids the weston crash. Fix-wave row 19-7: no new network surface.
- **Limitations**:
  - The X11 transport is chosen by `DISPLAY`. `DISPLAY=host:N` opens a TCP connection, which is the accepted zbus/X11 risk below. Since X11 is now forced under WSL, a setup where `DISPLAY` names a TCP host uses it even when `WAYLAND_DISPLAY` is also set.
  - S19-3 (low), fixed by rows 19-9(b), 19-10 and 19-11 (m0 §19.9 S19-3): the 19-1 acceptance check (cargo-deny `unused`) is graph-based and would also have passed with no member declaring winit. Row 19-2's Linux target dependency proved the declaration. The reverse half of check 3b now keeps it: without that declaration, the docs check reports `[deps] Cargo.toml: workspace dependency 'winit' is not declared by any member (rule R6: remove the pin or declare it with workspace = true)`. A declaration in a `[target.<cfg>]` table counts (`workspace_dependency_used_only_in_a_target_table_counts` in `xtask/tests/docs.rs`). Check 3b proves a declaration, not use in code; the one use is the `with_x11` call.
  - Every winit API is now callable directly from the app. winit exposes no socket API.
- **Failure mode**: Fail-closed through cargo-deny (licence, source, advisories, bans), layering and check 3b (an undeclared pin).

### Static MSVC C runtime for Windows binaries
- **Location**: `.cargo/config.toml:L9-L13`, guarded by step 0 (`xtask/src/ci.rs:L86-L414`)
- **Language**: TOML (Cargo configuration), rustc 1.99.0 MSVC
- **Description**: `-C target-feature=+crt-static` is set for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`. The C runtime is linked into the executable, so no `VCRUNTIME140.dll` or `api-ms-win-crt-*` DLL is loaded. On 2026-10-14, `dumpbin /dependents` ran on three binaries: the local x64 release `va.exe` and `visual-assembler.exe` (built 2026-10-13 from the current configuration), and the debug `xtask.exe`. None imports a CRT DLL. On 2026-10-15 it ran on the §19 shipped binary (x64 release `visual-assembler.exe` with both renderers, 15,528,448 bytes, built with `--locked`): no CRT DLL, and exactly the same 21 imports as the earlier wgpu-only binary. Step 0 rejects every `RUSTFLAGS`-type variable and every non-canonical config line that would replace these flags.
- **Rationale**: Architecture §9.2. No VC++ redistributable to install, and the binary cannot pick up a CRT DLL from the search path or from a stale system copy.
- **Limitations**:
  - R3-2: a stray space inside the feature string drops the flag while step 0 passes.
  - E4 stays a manual QA step. The ARM64 Windows build was not inspected.
  - Static linking does not protect the other DLLs the binaries load (F10, accepted until M8).
- **Failure mode**: Fail-closed for the checked variables and config lines. Fail-open for R3-2, which only the manual E4 check detects.

### Supply-chain pinning: committed lock file, `--locked` CI steps, pinned toolchain
- **Location**: `Cargo.toml:L31-L43`, `Cargo.lock`, `rust-toolchain.toml:L3-L6`, `xtask/src/ci.rs:L39-L84` (`--locked` on the three clippy runs and on test), `.github/workflows/ci.yml:L92` (`--locked` release build), `.github/workflows/ci.yml:L65` and `L67` (`cargo install --locked`, `cargo deny --locked`), `Makefile:L58`, `.cargo/config.toml:L5-L6` (the `xtask` alias, without `--locked`), `.cargo/deny.toml:L23-L34` (advisories), `.cargo/deny.toml:L115-L123` (sources), `scripts/check_project_docs.py:L95-L100` (dependency tables), `scripts/check_project_docs.py:L114-L118` (reverse-check message), `scripts/check_project_docs.py:L356-L466` (rule R6 in both directions: `dependency_tables`, `inherits_from_workspace`, `undeclared_workspace_dependencies`, `check_member_dependencies`), `scripts/check_project_docs.py:L686-L688` (check 3b call), `scripts/check_project_docs.py:L597-L606` (`[patch]`/`[replace]`). Tests in `xtask/tests/docs.rs:L782-L840` (row 19-10) and `xtask/tests/docs.rs:L1056-L1140` (QA additions).
- **Language**: TOML, Rust 1.99.0, Python 3
- **Description**:
  - Every external dependency is declared once in `[workspace.dependencies]`, today with an exact `=` pin. The `[deps]` check (check 3b) makes every member dependency use `workspace = true`. M1 (checked 2026-10-18): `fluent-bundle = "=0.16.0"` and `unic-langid = "=0.9.6"` are exact pins (`Cargo.toml:L37`, `L42`). The three internal crates are path entries (`L43-L45`). Every M1 member declares them with `workspace = true`.
  - Reverse half of check 3b (S19-3, rows 19-9(b), 19-10 and 19-11):
    - Every key of the root `[workspace.dependencies]`, path entries included, must be declared with `workspace = true` by at least one member, in any dependency table. That covers `dependencies`, `dev-dependencies`, `build-dependencies`, their legacy underscore spellings and their `target.<cfg>` variants. When the root manifest has `[package]`, the root package's own tables count too.
    - Keys compare exactly. Each undeclared key is reported once, in sorted order, as `[deps] Cargo.toml: workspace dependency '<name>' is not declared by any member (rule R6: remove the pin or declare it with workspace = true)`.
    - The reverse check is skipped when a member manifest or one of its tables is unusable. That manifest or table is already reported, so the run still fails.
    - Fixtures in `xtask/tests/docs.rs`: `unused_workspace_dependency_is_reported`, `workspace_dependency_used_only_in_a_target_table_counts` and `unused_path_workspace_dependency_is_reported` (row 19-10). QA added `undeclared_pins_are_reported_once_in_sorted_order_with_exact_keys`, `root_package_tables_count_for_the_reverse_check` and `reverse_check_is_skipped_when_a_member_table_is_unusable`.
    - Verified on 2026-10-16: `cargo test --locked -p xtask --test docs` passes 22 tests on Windows and in WSL. On Windows, `python scripts/check_project_docs.py .` reports no `[deps]` problem on the real repository (exit 0).
    - The root `Cargo.toml` comment at L28 now reads exactly `# Exact pins, each declared by at least one member (rule R6, enforced by check_project_docs.py check 3b).`
    - cargo-deny's `unused = "deny"` stays graph-based: it only proves that each pin's name, source and version exist in the resolved graph (network-crate ban entry above).
  - `Cargo.lock`, which carries the crates.io checksums, is committed. `--locked` is passed to:
    - clippy and test inside `cargo xtask ci`, and the CI release build;
    - the cargo-deny install and the cargo-deny run;
    - `make security`.
  - In CI, only the `deny` job fails on a lock file that does not match the manifests (R3-9). The `check` job first runs the `xtask` alias, `run --package xtask --`, which has no `--locked`. That run rewrites a stale `Cargo.lock` on the runner, so the later `--locked` steps of the same job see a matching lock and pass. Verified on 2026-10-14 with a probe workspace on cargo 1.99.0: `cargo metadata --locked` failed on the stale lock, the alias run succeeded and rewrote it, and `cargo build --locked` then passed.
  - The toolchain is pinned to 1.99.0.
  - cargo-deny denies yanked crates and reports every `unsound` advisory. `unmaintained = "all"` covers transitive crates. RustSec vulnerability advisories fail the check. Since `215e407` (2026-10-23) the `ignore` list holds exactly the three `unmaintained` ids of M1-S2 (accepted risk), and nothing else.
  - Root `[patch]` and `[replace]` tables are rejected.
- **Rationale**: Reproducible builds, and no silent upgrade to a compromised or vulnerable version.
- **Limitations**:
  - R3-9: the `check` job, and `cargo xtask ci` locally, build and test against a lock file that the unlocked alias run may just have rewritten. With a stale lock, that job builds dependency versions that nobody committed. Only the `deny` job reports the mismatch. `deny` is a required check on `main`, so that failure blocks a pull-request merge, but not a direct push or a bypass by the owner (`enforce_admins` off, R3-8).
  - R3-6: no tool enforces the exact `=` pins. A caret requirement in `[workspace.dependencies]` passes every check, because cargo-deny denies only `*`. The committed lock file still fixes what CI builds, because the required `deny` job fails on a lock that does not match the manifests (with the same owner exception, R3-8).
  - S19-3 (low), fixed by rows 19-9(b), 19-10 and 19-11 (m0 §19.9 S19-3). The pilot files followed in the WP7 row (checked 2026-10-16): DEPENDANCE.md (`L7-L10`) and CLAUDE.md (`L53-L56`) now name check 3b as the enforcement and say that cargo-deny's `unused` rule only proves the pinned version is in the resolved graph. Before the fix, no tool enforced that every `[workspace.dependencies]` pin is declared by a member: cargo-deny's `unused` rule passes a pin of any crate already in the graph, and check 3b only went from members to the workspace. What remains:
    - Check 3b proves a declaration, not use in code. A declared crate that no code uses still passes, because no `unused_crate_dependencies` lint is enabled.
    - It runs in the CI `docs` job and in `cargo xtask docs`, not in `cargo xtask ci`.
    - Without tomllib it is skipped with a warning, and the CI `docs` job does not pass `--strict`.
    - In CI it runs in the `docs` job, a required check on `main`: it blocks a pull-request merge, but not a direct push or a bypass by the owner (R3-8).
  - Nobody reviews the transitive versions one by one: there is no `cargo vet` or `cargo crev`.
  - The local `make test` and `make build`, `cargo xtask bench` and the `xtask` alias omit `--locked`.
  - A malicious version already present when the lock file is written is not detected.
  - Advisories are only as fresh as the last database fetch. A fresh fetch can fail a tree that nothing changed. That happened on 2026-10-18: `unmaintained = "all"` reported three zbus-family crates of M0's Linux GUI graph, and `make security` exited 1 (M1-S2). `215e407` settled it with three scoped ignores (accepted risk); `make security` exits 0 again (2026-10-23).
- **Failure mode**: Fail-closed in CI through the `deny` job (lock mismatch, yanked crate, advisory) and the `docs` job (patch table, member dependency without `workspace = true`, undeclared pin). The `check` job is fail-open on a stale lock (R3-9). Fail-open locally without `--locked`.

### Licence policy
- **Location**: `.cargo/deny.toml:L36-L55`, `Cargo.toml:L25-L26`, `license.workspace = true` in the 10 member manifests, `LICENSE-MIT`, `LICENSE-APACHE`
- **Language**: TOML (cargo-deny 0.20.2)
- **Description**:
  - Allow-list of licences: Apache-2.0, MIT, BSL-1.0, ISC, Unicode-3.0 and Zlib. OFL-1.1 and Ubuntu-font-1.0 are allowed for `epaint_default_fonts` only. The confidence threshold is 0.93.
  - The project is licensed `MIT OR Apache-2.0`. `[licenses.private] ignore = false`, so cargo-deny checks the workspace crates too.
  - §19 adds no package: `winit` 0.30.13 (Apache-2.0) was already in the graph through eframe, and the shipped binary's extra glow crates were already in the all-features graph that cargo-deny checks. `licenses ok` on 2026-10-15.
  - M1 adds 11 packages to `Cargo.lock` (dependency entry below). Their licences are allowed: `Apache-2.0 OR MIT` for the Fluent and `unic-langid` crates, `MIT OR Apache-2.0` for `displaydoc`, `Unicode-3.0` for `tinystr`, `zerovec` and `zerofrom`. `self_cell` 1.3.0 (`Apache-2.0 OR GPL-2.0-only`) and `memchr` 2.8.3 (`Unlicense OR MIT`) were already in the graph and are now also linked into `va`. cargo-deny takes the allowed side of each `OR` (Apache-2.0, MIT). The GPL side is never chosen. `licenses ok` on 2026-10-18 (`make security`).
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
- **Location**: `scripts/git-hooks/commit-msg:L1-L48`, `scripts/check_project_docs.py:L82-L88` (`COAUTHOR_RE`), `scripts/check_project_docs.py:L91-L94` (`git log` arguments), `scripts/check_project_docs.py:L609-L632`, `scripts/check_project_docs.py:L734-L745` (check 7, CI `docs` job, `.github/workflows/ci.yml:L42-L50`)
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
  - The `xtask/tests/hooks.rs` tests pass on Windows (15 on 2026-10-14; 18 on 2026-10-15). They cover the UTF-8 locales and `git hook run`.
  - Probes on 2026-10-14: the robot emoji and a Claude trailer were rejected under `LC_ALL=en_US.UTF-8` (Git Bash) and `C.UTF-8` (WSL), and a clean message was accepted.
- **Rationale**: Project rule that commits are never co-signed by an AI tool (framework rule, E6). Resolves F4 and QA2-F1.
- **Limitations**:
  - The hook is client-side only: `--no-verify` bypasses it, and rebase and cherry-pick do not run it.
  - R2-11 (accepted, §18.2 wording): "Comment lines and a scissors line are ignored by the hook but kept in the commit when the message comes from `-m`/`-F`, `--cleanup=verbatim`/`whitespace`, or a non-default `core.commentChar`. Such a message passes locally and is caught by the CI `docs` job's log check after the push, which becomes a required check with branch protection." Since §17.17 item 3, the `docs` job is a required check on `main` (branch protection entry), with the owner exception of R3-8.
  - A trailer folded onto a continuation line is not caught.
  - R3-4: the author lines in the CI log text can match only the emoji, and the committer is not included at all. A commit *authored* by an AI identity therefore passes both the hook, which cannot see the author, and the CI check. Examples: `Claude <noreply@anthropic.com>`, `copilot-swe-agent[bot]` (verified against `COAUTHOR_RE`).
  - The CI check covers only the last 50 commits (`fetch-depth: 50`). It runs in the `docs` job, a required check on `main`, so it blocks a pull-request merge. It does not block a direct push or a bypass by the owner (`enforce_admins` off, R3-8): such a commit is reported only after the push, by the `push` run on `main`.
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
  - timeouts: `ci.yml:L31`, `L44`, `L54`, `L72`, `L96`, `L104`;
  - the two informational GUI smoke steps (Windows only, `continue-on-error`): `ci.yml:L93-L98` and, since §19.3 as rewritten by row 19-9(a) (m0 §19.9 CR19-06), `ci.yml:L99-L108` (an injected failure of the first renderer of the runner's `auto` order), with its step-level `env:` at `ci.yml:L106-L107`.
- **Language**: GitHub Actions YAML
- **Description**:
  - The token is read-only at workflow level, and no job widens it. Checkout never persists credentials.
  - The workflow uses no `pull_request_target` and no secrets.
  - No `${{ … }}` expression appears inside a `run:` script. `runner.temp` is used only as `working-directory`, the matrix value only in `name`, `runs-on` and the cache key, and `runner.arch` only in the step-level `env:` of the §19.3 step. PR titles and branch names therefore cannot inject shell code.
  - Since row 19-9(a), the §19.3 step sets `VA_TEST_FAIL_RENDERER` through its step-level `env:`, from `${{ runner.arch == 'ARM64' && 'wgpu' || 'glow' }}`. It is no longer a command prefix. The rule above still holds: the expression is in `env:`, not in `run:`; `runner.arch` is set by the runner, not by a pull request, and the expression can only yield `wgpu` or `glow`; and the variable is scoped to that one step. Re-read on 2026-10-16, against the row 19-9(a) block of m0 §19.9.6:
    - the step is identical to that block;
    - the YAML parses (PyYAML);
    - `grep 'run:.*${{'` finds nothing;
    - every `uses:` line is still pinned to a 40-hex SHA.
  - Every action is pinned to a full commit SHA. Each pin was re-resolved upstream with `git ls-remote` on 2026-10-14:
    - `actions/checkout` v7.0.1 = `3d3c42e5aac5ba805825da76410c181273ba90b1`;
    - `Swatinem/rust-cache` v2.9.2 (peeled tag) = `6323deb102c322ba6fcbdcafc7e3dddab59af2b6`;
    - `dtolnay/rust-toolchain` master (also tag `v1`) = `e2a55d2ffb04f378e9626c28d38b36d230d1e12f`.
  - The Rust cache is used only by the `check` job and written only by `main` runs, so a pull request cannot poison it.
  - The `deny` job has no cache, and it builds cargo-deny outside the checkout on every run (R2-10 resolved).
- **Rationale**: Untrusted pull requests must not get write access, secrets or a poisoned cache. Mutable tags must not change what CI runs.
- **Limitations**:
  - In the `check` job, `rust-cache` restores `target/` and, since `cache-bin` defaults to `true`, `~/.cargo/bin`. The integrity of that job therefore rests on every earlier `main` run.
  - Branch protection on `main` requires the 7 jobs of this workflow, but not for administrators, and no CODEOWNERS file exists (branch protection entry below, R3-8).
  - Files that any pull request can change decide what CI runs: `ci.yml`, `.cargo/deny.toml`, the `xtask` alias and xtask itself. Only review protects them. The required checks are matched by job name, so a pull request that rewrites those jobs still produces checks with the required names.
  - The `windows-latest` and `ubuntu-latest` images move.
  - `cancel-in-progress` can cancel a `main` run when a newer push arrives.
  - The workflow has run three times on GitHub, each a `push` run on `main` (read with `gh run view` on 2026-10-16):
    - run 38635672228 on the pre-§19 commit `c326afd` (2026-10-14): all 7 jobs green;
    - run 38968085878 on `646b6f7` (2026-10-16), the first §19 run: only `check (windows-11-arm)` failed, in its `clippy, layering, tests` step (`struct KillOnDrop is never constructed`, a test helper of `tests/fallback.rs`, under `-D warnings`);
    - run 38969165969 on `6ed43c8` (2026-10-16), which gates that helper: all 7 jobs green, the first fully green run of the §19 tree, the rewritten §19.3 step included.

    `main` on GitHub is `6ed43c8` (`gh api`, 2026-10-16).
  - Both GUI smoke steps are `continue-on-error`. A regression of the renderer fallback (a panic, exit 3 or a hang) never fails CI. S19-5 is an accepted limitation (m0 §19.9 S19-5), with the three compensating controls listed in the fallback entry. Since row 19-9(a), `windows-11-arm` injects wgpu, its first renderer, so it exercises wgpu to glow instead of injecting a renderer that never runs. Its glow fallback may itself fail when the hosted image has no usable OpenGL; the §19.4 lines are then the record. Both outcomes are informational. In run 38969165969, `windows-latest` fell back from a genuine glow failure to wgpu (exit 0) in both steps, and the `windows-11-arm` injected step fell back from wgpu to glow, which also failed, so it exited 3 while the job stayed green (fallback entry).
- **Failure mode**: Fail-closed on permissions (read-only token).

### Branch protection on `main` with the 7 CI jobs required (§17.17 item 3)
- **Location**: GitHub settings of the public repository `GaspardSapin/VisualAssembler`, outside the tree. Read with `"C:/Program Files/GitHub CLI/gh.exe" api repos/GaspardSapin/VisualAssembler/branches/main/protection` (the command of `.github/workflows/README.md:L65`). Described in `.github/workflows/README.md:L55-L56` and `CLAUDE.md:L157-L159`. The required names are the jobs of `.github/workflows/ci.yml:L29` (`fmt`), `L42` (`docs`), `L52` (`deny`) and `L69-L76` (`check (${{ matrix.os }})` on 4 images).
- **Language**: GitHub branch protection rule (classic; the repository has no ruleset), read through the GitHub REST API
- **Description**: Read on 2026-10-16 with `gh api` (read-only):
  - Required status checks: `fmt`, `docs`, `deny`, `check (windows-latest)`, `check (windows-11-arm)`, `check (ubuntu-latest)` and `check (ubuntu-24.04-arm)`. Each one is bound to app id 15368 (GitHub Actions), so a status with the same name posted by another app, or through the commit-status API, does not satisfy it.
  - `allow_force_pushes` and `allow_deletions` are `false`: `main` cannot be force-pushed or deleted.
  - This makes the policy jobs blocking for a pull-request merge: cargo-deny (`deny`), the docs, `[lints]`, `[deps]`, `[patch]` and AI co-author log checks (`docs`), and step 0, clippy, the layering and source scan, the tests and the release build on the 4 runners (`check (…)`). The two GUI smoke steps are `continue-on-error`, so they never fail `check (…)`. This is what §17.17 item 3 asked for, so that the AI co-author log check and the policy jobs block a merge (Security F4).
- **Rationale**: §18.1: the tripwires catch accidents, and deliberate evasion is left to review, cargo-deny and required CI. Without required checks, a red CI run could be merged.
- **Limitations**:
  - `enforce_admins` is `false`. GitHub does not apply the rule to repository administrators. The owner, the only collaborator (role `admin`), can push to `main` directly and can merge a pull request whose required checks failed or never ran. For the owner's own changes, every check stays advisory, and a problem is reported only by the `push` run after the fact (R3-8).
  - `strict` is `false`: a pull request does not have to be up to date with `main` before it merges, so the merged result can differ from the commit CI tested.
  - No approving review is required (the rule has no `required_pull_request_reviews`), and no CODEOWNERS file exists (none in the tree; `.github/CODEOWNERS`, `CODEOWNERS` and `docs/CODEOWNERS` give 404 on GitHub).
  - The checks are matched by job name. A pull request that edits `ci.yml` decides what those jobs run, so it can satisfy them with weakened jobs. Only review catches that (CI hardening entry).
  - The rule lives in the repository settings, not in the tree. No check in the tree reads it, so a later change of the settings is seen only by reading it again with `gh api`.
  - No change has gone through the rule as a pull request yet: the repository has no pull request, and `646b6f7` and `6ed43c8` have none associated (`gh api …/commits/<sha>/pulls`, 2026-10-16). Both reached `main` by direct pushes from the owner (runs 38968085878 and 38969165969, `push` events, actor `GaspardSapin`). `646b6f7` was on `main` with a failing required check, `check (windows-11-arm)`, reported only by its `push` run after the fact, until `6ed43c8` (run 38969165969, all 7 jobs green). This is the R3-8 exception in practice. Re-read with `gh api` after these pushes (2026-10-16): the rule is unchanged.
- **Failure mode**: Fail-closed for a pull-request merge by a non-administrator (today there is none): blocked until the 7 checks pass. Fail-open for an administrator (direct push or bypass), because `enforce_admins` is off.

### External processes spawned without a shell (xtask and scripts)
- **Location**: `xtask/src/exec.rs:L1-L62`. Call sites:
  - `xtask/src/locate.rs:L31-L45`;
  - `xtask/src/ci.rs:L190-L192`;
  - `xtask/src/docs.rs:L30-L63`;
  - `xtask/src/hooks.rs:L51-L62`;
  - `xtask/src/bench/mod.rs:L295-L323`;
  - `xtask/src/bench/runner.rs:L123-L180` (shipped build), `xtask/src/bench/runner.rs:L194-L214` (GUI arguments) and `xtask/src/bench/runner.rs:L473-L487`;
  - `scripts/check_project_docs.py:L609-L632`;
  - `scripts/gen_agencement.py:L67-L95`.

  Makefile recipes: `Makefile:L31-L73`.
- **Language**: Rust 1.99.0. Python 3. GNU make 4.4.1.
- **Description**:
  - Every child process (cargo, git, Python, the measured GUI) is started with `std::process::Command` or `subprocess.run` and a fixed argument vector, never through a shell. No argument comes from untrusted data. Since §19.5 the GUI gets `--renderer <auto|glow|wgpu> --bench <scenario> [--duration-secs N]`, built from clap value enums and a range-checked integer.
  - Failures to start, non-zero exits and non-UTF-8 output become errors with context. The Python git calls have a timeout.
  - Makefile recipes contain no shell syntax.
- **Rationale**: Architecture §9.5. No command injection, and identical behaviour under sh and cmd.exe.
- **Limitations**:
  - The program names `git`, `python` / `python3` and `$CARGO`, and the `$VA_PYTHON` override, are trusted as found. On Windows, Rust resolves a bare program name in the xtask executable's folder (`target/debug/`) before `PATH`.
  - Under sh, make passes `SCENARIO`, `RENDERER` and `RUNS` to the shell unquoted. That input comes from the developer's own command line and is not a trust boundary.
- **Failure mode**: Fail-closed: a child that cannot start or that fails stops the task with exit code 1.

### Bench child-process containment (deadlines, bounded reader shutdown, output caps)
- **Location**: `xtask/src/bench/runner.rs:L39-L65` (limits), `xtask/src/bench/runner.rs:L489-L595` (run engine), `xtask/src/bench/runner.rs:L222-L265` (run-line validation, §19.5), `xtask/src/bench/runner.rs:L598-L660` (reader threads), `xtask/src/bench/runner.rs:L664-L723` (deadline wait, bounded drain, kill and reap). GUI-side watchdog: `apps/visual-assembler/src/bench/recorder.rs:L52-L140`.
- **Language**: Rust 1.99.0
- **Description**:
  - Each measured run has stdin set to null. Stdout and stderr are piped and drained by reader threads.
  - The run deadline is 75 s for startup and N + 75 s for idle. Both are strictly above the GUI's own watchdog (60 s, and 60 + N + 3 s for idle), so the GUI's `bench timed out` (exit 4) surfaces first.
  - Past the deadline, the child is killed and reaped.
  - Once the child has exited or been killed, the readers get at most 5 s. Readers still blocked after that are detached, and the run fails with `output pipes still open after the GUI exited (a descendant process may hold them)`.
  - Only the last 64 KiB of stderr is kept. Stdout lines are capped at 1 MiB, and a truncated line never counts as the run line.
  - §19 (2026-10-15): the limits, the reader threads and the deadline, drain, kill and reap code are byte-identical to the round-3 version (diffed against `HEAD`). Only the run-line check changed. It now requires `scenario` and `renderer_requested` to match the request, and a forced `glow` or `wgpu` run is rejected when it reports a fallback or another renderer. Under `auto`, any renderer is accepted and a fallback is counted (§19.5).
  - The GUI watchdog is created once, before the first renderer attempt. A fallback neither drops it nor resets its limit, so a renderer initialization that hangs is still caught (exit 4).
  - The `bench_runner.rs` tests pass on Windows (23 on 2026-10-14; 30 on 2026-10-15).
- **Rationale**: A hung GPU or driver must not hang the developer tool or leave processes behind (F5, D07, D08).
- **Limitations**:
  - Accepted (§17.8): the kill reaches only the direct child (accepted risk F5 below).
  - The channel between the readers and the run loop is unbounded. Memory stays bounded only while the loop keeps up, and the producer is the project's own GUI.
  - The fake-GUI process tests (pipe join timeout, kill at the deadline, caps) run only on Unix with `sh`.
- **Failure mode**: Fail-closed: a timeout or failure aborts the bench with exit code 1, and no report is written.

### Bench reports never overwrite an existing file
- **Location**: `xtask/src/bench/report.rs:L463-L507`
- **Language**: Rust 1.99.0
- **Description**: Reports are created with `OpenOptions::create_new` (O_EXCL / CREATE_NEW). An existing file or symlink at the target name is never followed or overwritten. The name gets a `-2`, `-3`, … suffix instead, up to 1000 attempts. The code is byte-identical to the round-3 version (diffed on 2026-10-15); the report schema is now `va.bench.report/2`.
- **Rationale**: Integrity of earlier measurements, and no clobbering through a planted link in `bench/results/`.
- **Limitations**:
  - `create_dir_all` follows a symlinked `bench/` or `bench/results/` folder.
  - Reports record the values of `WGPU_BACKEND`, `WGPU_ADAPTER_NAME`, `WGPU_POWER_PREF`, `WAYLAND_DISPLAY`, `DISPLAY` and, since §19.5, `VA_RENDERER`, `VA_TEST_FAIL_RENDERER` and `VA_WAYLAND`, plus GPU and driver names. Each run line also carries the raw `VA_TEST_FAIL_RENDERER` value (`injected_failure`) and the first line of a renderer error (`fallback_reason`), which can contain driver text. These are not secrets, but they identify the host. `serde_json` escapes them, and none of them appears in the markdown table. `bench/results/` is gitignored, so reports are never committed by accident.
- **Failure mode**: Fail-closed: after 1000 name collisions, or on any I/O error, the command fails.

### Test temporary folders under `CARGO_TARGET_TMPDIR`
- **Location**: `xtask/tests/layering.rs:L71-L80`, `xtask/tests/ci.rs:L563-L572`, `xtask/tests/bench_report.rs:L128-L137`, `xtask/tests/bench_runner.rs:L639-L647`, `xtask/tests/hooks.rs:L21-L30`. Durable rule in architecture §8.
- **Language**: Rust 1.99.0
- **Description**:
  - Every xtask test folder is `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>`.
  - It is created with `fs::create_dir`, which fails if the path exists.
  - `grep` finds no `std::env::temp_dir` or `create_dir_all` in the xtask or app tests (2026-10-14, again on 2026-10-15; the helpers named `temp_dir` are the project's own). The new `apps/visual-assembler/tests/fallback.rs` creates no folder.
  - The hooks tests also assert that each temporary repository is its own git repository, never the project's.
- **Rationale**: F7: predictable names in the shared system temp folder let another local user pre-create the folder, or race a symlink so fixtures are written elsewhere.
- **Limitations**:
  - Folders stay behind when a test panics.
  - `CARGO_TARGET_TMPDIR` lies inside the target folder, so whoever controls that folder controls them. That is the developer.
- **Failure mode**: Fail-closed: a pre-existing path makes the test fail rather than reuse it.

### Source input limits: 16 MiB per input, 64 KiB per lexed line (m1 §3.2, §5.2, §7.2)
- **Location**:
  - limits: `crates/va-core/src/source.rs:L12-L18` (`MAX_SOURCE_BYTES`, `MAX_LINE_BYTES`);
  - size check: `source.rs:L132-L151` (`check_size`, `SourceText::from_bytes`, `from_string`);
  - bounded read: `apps/va/src/input.rs:L24-L26` (`READ_LIMIT`), `input.rs:L44-L85` (`read`), `input.rs:L87-L92` (`read_limited`);
  - line limit: `arch/m68k/src/syntax/lexer.rs:L42-L45`.

  Tests: `crates/va-core/tests/source.rs` (`limits_have_the_specified_values`, `size_limit_accepts_exactly_the_maximum`, `size_limit_rejects_one_byte_over`, `size_limit_counts_the_bom`, `size_is_checked_before_utf8`), `apps/va/tests/lex.rs` (`input_limit_is_16_mib`), `arch/m68k/tests/syntax/robustness.rs` (`a_line_over_the_limit_is_one_invalid_token`).
- **Language**: Rust 1.99.0
- **Description**:
  - `va lex` reads its file, or stdin for `-`, through `Read::take(MAX_SOURCE_BYTES + 1)`. An endless or oversized input is never read past 16 MiB + 1 byte.
  - `SourceText::from_bytes` checks the size before decoding. More than 16,777,216 bytes is `SourceError::TooLarge`, so `va` exits 3 with `input-too-large` before printing anything. A BOM counts toward the limit (m1 §12.2).
  - A line longer than 64 KiB is never lexed: `lex_line` returns one `Invalid(lex-line-too-long)` token over the whole line.
  - Because of the limit, every byte offset fits in a `u32`. Span arithmetic saturates anyway (`TextRange::shifted`), and line numbers and columns convert with `try_from` and saturate at `u32::MAX`.

  Verified on 2026-10-18 with the release `va` (Windows x64, `--locked`, and WSL):
  - exactly 16 MiB (`;` repeated), and a BOM plus 16 MiB − 3 bytes: accepted (one `lex-line-too-long` token, exit 1);
  - 16 MiB + 1 byte, and a BOM plus 16 MiB − 2 bytes: exit 3, `input-too-large`, nothing on stdout but the `va.error/1` line in JSON mode;
  - `/dev/zero` and `/dev/urandom` (WSL): exit 3, `input-too-large`;
  - a 65,543-byte line: one invalid token;
  - peak working set during the largest run below: about 33 MB (`tasklist`).
- **Rationale**: m1 §7.2 and §10 F8. An input cannot exhaust memory, and the lexer's work per line is bounded.
- **Limitations**:
  - The limit bounds the input, not the output (M1-S5). A 16 MiB input of `^a` pairs in operand fields gave 516 MB on stdout and 1.47 GB of diagnostics on stderr in 72 s in text mode, and 3.77 GB of JSON in 376 s. 16 MiB of empty lines gave 783 MB of JSON, and 16 MiB of `1,` operands 1.2 GB of JSON. m1 §10 F8 says about 20×; the measured worst case is about 225× (JSON).
  - A read that never ends before 16 MiB (a FIFO with no writer, `CON`, `/dev/tty`, a stalled network share) blocks forever: there is no timeout (M1-S6). Verified in WSL with a FIFO: killed by `timeout 5` (exit 124).
  - The 64 KiB line limit lives in the m68k lexer. Another `SyntaxProvider` must apply `MAX_LINE_BYTES` itself, since `crates/va-core` only defines the constant.
- **Failure mode**: Fail-closed: an oversized input is refused with exit 3, and an oversized line is one invalid token, never a partial lex.

### Strict UTF-8 decoding and BOM handling of source input (m1 §3.2, §7.2)
- **Location**: `crates/va-core/src/source.rs:L141-L166` (`from_bytes`, `without_bom`), `apps/va/src/input.rs:L70-L83` (error mapping). Tests: `crates/va-core/tests/source.rs` (`not_utf8_reports_the_first_invalid_input_byte`, `from_bytes_strips_exactly_one_bom`, `a_bom_after_the_start_is_kept`, `nul_bytes_are_valid_utf8`), `apps/va/tests/lex.rs` (`non_utf8_input_exits_3`, `bom_is_stripped_before_lexing`, `crlf_and_cr_give_the_same_tokens_as_lf`).
- **Language**: Rust 1.99.0
- **Description**:
  - Bytes are decoded with `String::from_utf8`, never lossily. Invalid UTF-8 is `SourceError::NotUtf8` with the offset of the first invalid byte, and `va` exits 3 with `input-not-utf8`.
  - Exactly one leading BOM is stripped. Offsets are relative to the text after it.
  - Lines end at LF, CRLF or a lone CR, the same way in `lines` and `LineIndex`, so a CR can never hide text from one consumer and show it to another.

  Verified on 2026-10-18 with the release `va`:
  - a Latin-1 `é` byte: offset 11. A UTF-16 file: offset 0. An overlong encoding (`C0 AF`) and a UTF-16 surrogate encoded in UTF-8 (`ED A0 80`): offset 8. All exit 3;
  - invalid stdin (`FF`): exit 3, `standard input is not valid UTF-8`;
  - a BOM-only file: one empty line, exit 0. A second BOM is text (an invalid label, exit 1);
  - BOM, then `  dc.b 1` CRLF `  nop` CR: three lines at offsets 0, 10 and 16, the last one empty, so offsets start after the BOM.
- **Rationale**: Rust strings must be valid UTF-8. Lossy decoding would change bytes and offsets silently, and overlong forms are a classic filter bypass.
- **Limitations**:
  - Valid UTF-8 still carries control characters (NUL, ESC, DEL), C1 controls (U+0080 to U+009F) and bidi controls (U+202A to U+202E, U+2066 to U+2069). The lexer handles them as invalid or comment text. The terminal writers do not neutralize them (M1-S1, M1-S3).
  - Windows-1252 sources are refused rather than converted (m1 §7.2, an M4 editor feature).
- **Failure mode**: Fail-closed: anything that is not UTF-8 is refused with exit 3 and the byte offset.

### Lexer robustness: total, panic-free and linear on any line (m1 §3.4, §5.6, §9.1)
- **Location**: `arch/m68k/src/syntax/lexer.rs:L31-L66` (`lex_line`), `arch/m68k/src/syntax/literals.rs` (scanners), `arch/m68k/src/syntax/spacing.rs` (§5.7 rules and quick fix), `crates/va-core/src/syntax.rs:L188-L213` (`check_line_tokens`), `syntax.rs:L215-L263` (`text_arg`, `token_diagnostics`), `crates/va-core/src/source.rs:L23-L82` (saturating offsets, `TextRange`). Tests: `arch/m68k/tests/syntax/main.rs:L65-L77` (every test lexes through `check_line_tokens`), `tests/syntax/coverage.rs` (`all_short_ascii_strings_are_covered`, `random_lines_are_covered`, `lexing_is_stateless`, `returned_state_is_always_initial`), `tests/syntax/robustness.rs` (`pathological_lines_are_linear`, `carriage_return_and_line_feed_inside_a_line`, `nul_inside_a_line`, `four_byte_chars_in_comments_strings_and_elsewhere`), `crates/va-core/tests/source.rs` (`a_mismatched_text_never_panics`).
- **Language**: Rust 1.99.0
- **Description**:
  - Contract (m1 §3.4): for any `&str`, `lex_line` appends tokens that cover the line exactly, contiguous and on char boundaries, never panics and runs in O(len). `check_line_tokens` checks the cover. The suite runs it on every lexed line, so every test also checks the property.
  - Implementation: one forward pass over the bytes with bounded look-backs. Scanners stop only at ASCII bytes or consume whole non-ASCII chars. Slicing uses `get`, `TextRange::slice` returns `None` instead of panicking, and offsets saturate. A `\r` or `\n` inside a line is an invalid token. Diagnostics truncate the source text to 32 chars plus `…`.
  - The test suite: every ASCII string of length ≤ 3 over 24 interesting chars; 100,000 pseudo-random lines; 40 pathological 64 KiB lines in debug under 30 s (the CI guard against quadratic time); NUL, CR, LF and 4-byte chars.
  - Verified on 2026-10-18:
    - `cargo test --locked -p va-core -p va-arch-m68k -p va-workspace` passes on Windows (syntax suite 54 tests, QA's uncommitted additions included, in 1.45 s debug).
    - A scratch fuzz harness outside the repository, built in release with `overflow-checks` and `debug-assertions`, ran 3,000,000 random lines. The alphabet: ASCII, NUL, ESC, DEL, CR, LF, `é`, U+202E, U+009B, an emoji, U+FEFF, U+2028 and keyword fragments; one line in 1000 is up to about 28 KB. Each line went through `lex_line`, `check_line_tokens` and `line_diagnostics` with a line start near `u32::MAX`, and 10,714,135 diagnostics were rendered in English and French. It also ran 200,000 random `describe` queries and every spelling with eight suffixes. No panic and no invariant failure, in 6.7 s.
    - 300 random source files through the release `va`, in text, JSON and `--lang fr`: exit 0 or 1, never 101.
    - 16 pathological 64 KiB lines through `va --json lex` (release): at most 0.16 s each.
    - The JSON mode lexes twice (§12.7), from the same in-memory text, so the two passes cannot disagree because of a file change.
  - Re-verified on 2026-10-23 on `15e9fa4`, after `e764313` changed `spacing.rs` (a quote in the trailing text opens a string only when the line closes it, m1 §12.8 N-7; no-op fixes dropped, N-1). The new `closing_quote` scan (`arch/m68k/src/syntax/spacing.rs:L129-L165`) stays linear: a failed search for one delimiter proves that no later byte is that delimiter, so at most two searches fail per trailing text, and a successful one jumps past the closing quote. The fuzz harness, rebuilt on that code, ran the same 3,000,000 lines with no panic or invariant failure (10,714,135 diagnostics rendered, 2,713 with a fix). Six 64 KiB quote-heavy trailing texts (`'` runs, `'"` pairs, `a '` and ` " ` runs, `x'y"z;` runs) took at most 0.039 s each in text and JSON mode. The 316 earlier CLI inputs gave exit 0 or 1, never 101.
- **Rationale**: The lexer is the first code that sees untrusted text (course files, files from classmates). A panic would be a denial of service of the CLI and, from M4, of the editor's highlighter. Quadratic time would be one too.
- **Limitations**:
  - The property is tested, not proven (M1-S7). No fuzzer runs in CI. The workspace denies neither `clippy::indexing_slicing` nor `clippy::arithmetic_side_effects`, and the lexer indexes slices directly (`bytes[at]`), guarded by explicit bounds checks that review must keep.
  - The release profile has no `overflow-checks`. An arithmetic bug would wrap silently in a release build instead of panicking.
  - Panics are not caught. A lexer bug ends `va` with exit 101 (fail-closed; m0 §5).
  - The CI guard is coarse: it allows 30 s in debug, while the whole syntax suite, that test included, ran in 1.45 s. A mildly superlinear regression would pass it.
  - Allocation-freedom (m1 §3.4) is not measured (m1 §12.4, M7).
  - Linear time is a lexer property. `va lex` computes each diagnostic's column by counting chars from the line start (`apps/va/src/commands/lex.rs:L114-L125`), which is O(diagnostics × line length) per line. The 64 KiB line limit bounds it: a 64 KiB line with 32,000 diagnostics took 0.085 s in text mode and 0.087 s in JSON mode (release, 2026-10-18), and the time grows linearly with the number of such lines.
- **Failure mode**: Fail-closed: a broken invariant fails the tests, and a panic exits 101 without output past the failing line.

### `va` output encoding: JSON through serde_json, terminal text through `report::terminal_safe`
- **Location**: `apps/va/src/report.rs:L250-L295` (`is_unsafe_for_terminal`, `terminal_safe`), `report.rs:L218-L237` (`print_error_line`, `print_warning_line`), `apps/va/src/commands/lex.rs:L147-L172` (`write_text`, file prefix at `L151`), `lex.rs:L174-L201` (`write_token_lines`), `lex.rs:L203-L242` (`write_diagnostic_text`), `lex.rs:L312-L418` (`write_value`, `write_json`), `apps/va/src/commands/isa.rs:L253-L260` (`write_line`), `report.rs:L172-L216` (`va.error/1`). Tests: `apps/va/tests/lex.rs` (`text_mode_never_writes_terminal_controls_raw`, `echoed_file_names_are_terminal_safe`, `qa_text_mode_token_texts_stay_json_strings`, `json_tokens_match_every_golden_fixture`, helper `assert_terminal_safe` at `L42-L52`), `apps/va/tests/isa.rs` (`echoed_query_and_suffix_are_terminal_safe`, `va_lang_with_control_characters_gives_one_safe_warning_line`, `move_json_has_the_documented_keys_in_order`).
- **Language**: Rust 1.99.0 (serde_json 1.0.151)
- **Description**:
  - JSON: every string in `va.lex/1`, `va.isa/1`, `va.isa.list/1` and `va.error/1` (file name, token text, message, help, fix replacement, query) goes through `serde_json`. The hand-written parts of the streamed `va.lex/1` line carry only fixed keys and integers. A source file therefore cannot break the JSON structure or inject keys.
  - Terminal text (fixed by `a1f4294`, M1-S1, M1-S3, M1-S4): `terminal_safe` rewrites every char of a fixed set into its `char::escape_debug` form (`\t`, `\u{1b}`, `\u{9b}`, `\u{202e}`, …), falling back to `escape_unicode` for any char of the set that `escape_debug` would keep. The set: U+0000–U+001F (tab included), U+007F–U+009F (DEL and the C1 controls), U+200E, U+200F, U+2028, U+2029, U+202A–U+202E and U+2066–U+2069. Other text, accents and backslashes included, is kept, and nothing is allocated when no char needs rewriting.
  - It is applied to every human line that carries outside text:
    - `va lex` text mode: each token text after JSON quoting (stdout), and the file prefix, message, `help`, `note` and `fix` line of each diagnostic (stderr);
    - every `error:` and `warning:` line (`print_error_line`, `print_warning_line`): the `VA_LANG` warning, file names in `cannot read '<name>'`, and the `va isa` query and suffix.
  - Text that carries no outside input is printed as is: the `va isa` documentation (the embedded Fluent texts and the tables) and `va info`.

  Verified on 2026-10-23 with the release `va` built on `15e9fa4` (byte dumps, Windows):
  - the five M1 probe files (OSC title plus `ESC[2J` in an unterminated string; `ESC[31m` in an operand run; OSC 52 behind `;` on a dangling-comma line, echoed by the `fix:` line; U+202E; U+009B) give no ESC, BEL or CR byte and no raw C1 or bidi char on stdout or stderr. For example, the fix line now reads `fix:   dc.b 1,2\u{1b}[7m ;\u{1b}]52;c;ZWNobyBoaQo=\u{7}`;
  - `VA_LANG=$'x\e[2J\ry'`, the query `MO<ESC>[2JV`, the suffix `.<ESC>]0;T<BEL>` and the file name `no<ESC>[31mpe` each give one escaped line (`ignoring VA_LANG=x\u{1b}[2J\ry`, `unknown mnemonic 'MO\u{1b}[2JV'`, …);
  - `--json` output is unchanged: the `va.error/1` message keeps the query (`\u001b` from serde_json), and U+202E stays raw in `va.lex/1` by design.
- **Rationale**: The JSON documents are the machine interface (M5 tooling, scripts), so their structure must not depend on the input. Source files cross a trust boundary, so their text must not drive the user's terminal.
- **Limitations**:
  - JSON documents carry DEL, C1 and bidi characters raw (valid JSON, decoded as in the source; the test pins it). A consumer that prints them to a terminal must escape them itself.
  - M1-S8 (low, open): clap's own usage errors (`apps/va/src/main.rs:L43-L56`, `error.print()`) bypass `terminal_safe`. With colour on (a terminal; `CLICOLOR_FORCE=1` in the probe), `va $'\e[2J'` prints the raw `ESC[2J`, and `va lex $'--x\e]0;T\a'` the raw OSC. With colour off (a pipe, `NO_COLOR`, which the tests set), anstream strips ESC sequences and BEL but keeps CR, LF, U+202E and U+009B.
  - M1-S9 (low, open): the set leaves out U+061C (ARABIC LETTER MARK), an implicit bidi mark like U+200E and U+200F. It reached stderr raw (probe of 2026-10-23). Invisible chars without a control or reordering effect (U+200B–U+200D, U+2060, U+FEFF) are kept by design.
  - The escaped form is not reversible: a source that holds the literal text `\u{1b}` prints the same as one that holds ESC. A tab in a `fix:` line prints as `\t`, so that line can no longer be pasted back as is.
- **Failure mode**: Fail-closed for the JSON structure and for the characters of the set on every terminal line except clap's usage errors (M1-S8).

### Fluent localizer: embedded resources only, arguments inserted verbatim, no isolation marks (m1 §6.2)
- **Location**: `crates/va-workspace/src/i18n/localizer.rs:L103-L196` (`new`, `for_architecture`, `message`, `render`, `lookup`), `localizer.rs:L207-L229` (`build_bundle`, `set_use_isolating(false)` at `L210`), `localizer.rs:L264-L302` (`fluent_args`, `fluent_value`, `format_pattern`), `arch/m68k/src/locales.rs:L10-L29` (`include_str!` resources), `apps/va/src/commands/mod.rs:L55-L56` (`locale-invalid`). Tests: `crates/va-workspace/tests/i18n_localizer.rs` (`text_arguments_are_verbatim_and_the_last_repeat_wins`, `missing_argument_gives_best_effort_text`, `no_isolation_marks`, `render_without_attributes_or_message`, `parse_errors_name_the_language_resource_and_line`, `duplicate_ids_are_rejected_within_one_language`), `arch/m68k/tests/locales.rs` (parse and completeness, m1 §6.3).
- **Language**: Rust 1.99.0 (fluent-bundle 0.16.0)
- **Description**:
  - The only Fluent sources are the four `.ftl` files compiled into `va-arch-m68k` with `include_str!`. No `.ftl` file is read at run time. A broken resource is a build defect: `va` exits 3 with `locale-invalid`, and `tests/locales.rs` parses every resource first.
  - Source text reaches Fluent only as the value of `$text`, a `FluentValue::String`. Fluent inserts a value as is and never parses it as Fluent syntax. Verified on 2026-10-18: an unterminated string `'{ $cause } { -x } {"A"}` rendered those characters literally in English and French. The `$cause` selector only ever gets one of three fixed ids from code (`spacing.rs:L39-L45`).
  - A missing message renders as its code and a missing variable as `{$name}`. A resolver error never panics and never fails the command.
- **Rationale**: Architecture §5. Translations are data, and user text must never become a template.
- **Limitations**:
  - `set_use_isolating(false)` is a decision (m1 §6.2: no U+2068/U+2069 in terminals or egui). Bidi controls in `$text` are therefore not isolated. In the CLI, `terminal_safe` rewrites them before they reach the terminal (output-encoding entry, M1-S1 fixed by `a1f4294`), except U+061C (M1-S9).
  - The localizer does not escape. Each sink must: the CLI does through `terminal_safe` since `a1f4294`; the M4 GUI must decide for its Problems panel and hover (**not implemented**).
- **Failure mode**: Fail-safe: missing text degrades to ids, and resource defects fail the build tests or exit 3.

### Language selection from the environment: `VA_LANG` and the locale variables (m1 §6.2, §7.1)
- **Location**: `crates/va-workspace/src/i18n/lang.rs:L38-L42` (`Lang::parse`), `lang.rs:L73-L121` (`resolve_lang`, `auto_lang`), `apps/va/src/commands/mod.rs:L40-L48` (the `var_os` closure and the warning, printed through `report::print_warning_line`), `apps/va/src/cli.rs` (`--lang`, a clap possible-values parser). Tests: `crates/va-workspace/tests/i18n_lang.rs` (10 tests, e.g. `parse_accepts_only_exact_lowercase_codes`, `flag_wins_and_reads_no_variable`, `invalid_va_lang_is_returned_and_auto_applies`), `apps/va/tests/isa.rs` (`va_lang_selects_the_language_and_bad_values_are_reported`, `va_lang_with_control_characters_gives_one_safe_warning_line`).
- **Language**: Rust 1.99.0
- **Description**:
  - `--lang` takes exactly `en` or `fr` (any other value, `auto` included, exits 2). With the flag, no variable is read.
  - `VA_LANG` selects only on an exact `en`, `fr` or `auto`. Any other value, empty, mixed case or non-Unicode (decoded lossily) included, is ignored with a warning, and `auto` applies.
  - `auto` reads the first non-empty of `LC_ALL`, `LC_MESSAGES` and `LANG`. A value starting with `fr` gives French, anything else English.
  - The values only choose one of two compiled-in resource lists. No path, file name or command is built from them.

  Verified on 2026-10-18: `VA_LANG=f<FF>r` (WSL) warned with U+FFFD and fell back to `auto`.
- **Rationale**: m1 §6.2. A bad value degrades to a safe default and never selects resources from outside the binary.
- **Limitations**:
  - M1-S4 (low), fixed by `a1f4294` (m1 §12.8 N-2): the warning went raw to stderr; `VA_LANG=$'x\e[2J\ry'` sent ESC and CR (byte dump, 2026-10-18). It now passes through `terminal_safe` and stays one line: `warning: ignoring VA_LANG=x\u{1b}[2J\ry (expected en, fr or auto)` (2026-10-23). The GUI uses `escape_debug` for the same class (S19-2, `apps/visual-assembler/src/cli.rs:L342-L345`). Both escape every control char; they differ only outside that set (`escape_debug` also escapes other non-printable chars and `\`, `'` and `"`).
  - The Windows UI language is not read before M4 (m1 §6.2).
- **Failure mode**: Fail-safe: an invalid value degrades to `auto`, then to English.

### `va lex` file access: one read-only path from the command line (m1 §7.2)
- **Location**: `apps/va/src/input.rs:L44-L92`, `apps/va/src/cli.rs` (`Lex { file: OsString }`), `apps/va/src/report.rs:L57-L102` (codes and exit codes), `report.rs:L239-L248` (`english_io`). Tests: `apps/va/tests/lex.rs` (`unreadable_paths_exit_3`, `closed_stdout_pipe_is_io_error`, `closed_stderr_pipe_with_diagnostics_is_io_error`, `missing_or_extra_file_argument_is_a_usage_error`, `qa_stdout_failure_message_is_english`, `echoed_file_names_are_terminal_safe`), `apps/va/tests/binary.rs` (`unwritable_stdout_is_io_error`, `usage_error_on_a_closed_stderr_pipe_is_io_error`).
- **Language**: Rust 1.99.0
- **Description**:
  - `va` opens at most one file, read-only, with `File::open`: the `FILE` argument, taken as an `OsString` (non-UTF-8 names work), or stdin for `-`. It never writes, creates or deletes a file.
  - No path is derived from file content. `INCLUDE` and `INCBIN` operands are lexed as `String` tokens and never opened (M2/M7 decide include handling).
  - A missing file, a directory, a permission error or a failed read is `read-failed` (exit 3). The reason is the English `io::ErrorKind` plus the OS error number, never a localized OS message (§12.7). Since `a1f4294` (m1 §12.8 F-1) every `io::Error` that `va` prints goes through `english_io`, the stdout and stderr failures and `cannot print the command-line message` included. Before it, a stdout failure printed the Windows-localized text (`Le canal de communication a été fermé. (os error 109)`, 2026-10-18): a wording issue, not a leak. A directory says `it is a directory` on both systems.
  - A stdout failure (closed pipe, full disk) and a stderr failure while text-mode diagnostics are written exit 3.

  Verified on 2026-10-18:
  - Windows: a directory, a missing file (`entity not found (os error 2)`) and `NUL` (empty, exit 0).
  - WSL: `/tmp`, a symlink to a folder, a dangling symlink, a mode-000 file (`permission denied (os error 13)`), `/dev/null`, a FIFO with a writer, a non-UTF-8 file name (U+FFFD in the JSON `file` field), `/dev/full` as stdout (exit 3), and `/dev/full` as stderr with a diagnostic (exit 3).
- **Rationale**: A local tool that reads exactly what the user names, with no surprising side effect.
- **Limitations**:
  - Symlinks are followed, like any reader.
  - M1-S6 (low, open): a path is opened as given. On Windows, a UNC path (`\\host\share\f.asm`) makes the OS connect to that host over SMB and can send the user's NTLM credentials. Only the user can type such a path in M1, but scripts that pass untrusted names inherit this. Device and pipe paths (`CON`, `\\.\pipe\…`) are read too, and reads have no timeout (input-limits entry). The `SourceProvider` sandbox planned for M7 (architecture §9.5) is **not implemented**.
  - M1-S4 (low), fixed by `a1f4294`: the file name was echoed raw in `cannot read '<name>'` and as the prefix of every text-mode diagnostic. Both now go through `terminal_safe` (`report.rs:L224-L227`, `lex.rs:L151`). The JSON `file` field and the `va.error/1` message keep the name as given (serde escapes it). Verified on 2026-10-23: `no<ESC>[31mpe` gives `error: cannot read 'no\u{1b}[31mpe': invalid filename (os error 123)`. With fd 1 already closed (`>&-`), Rust's stdout discards output silently and `va` exits 0 (WSL, 2026-10-18). That is standard-library behaviour, distinct from the closed-pipe case.
- **Failure mode**: Fail-closed: every access failure exits 3 with a stable code.

### M1 dependencies: `fluent-bundle` 0.16.0, `unic-langid` 0.9.6 and their transitive packages (m1 §8)
- **Location**: `Cargo.toml:L37`, `L42-L45`, `crates/va-workspace/Cargo.toml:L11-L14`, `arch/m68k/Cargo.toml:L12-L16` (`fluent-bundle` as a dev-dependency only), `apps/va/Cargo.toml:L17-L25`, `Cargo.lock`.
- **Language**: TOML, Rust 1.99.0
- **Description**:
  - Two external crates with exact pins: `fluent-bundle = "=0.16.0"` and `unic-langid = "=0.9.6"`. `va-workspace` uses both. `va-arch-m68k` uses `fluent-bundle` only in its tests.
  - `Cargo.lock` gains 11 packages, all from crates.io with checksums: `fluent-bundle` 0.16.0, `fluent-langneg` 0.13.1, `fluent-syntax` 0.12.0, `intl-memoizer` 0.5.3, `intl_pluralrules` 7.0.2, `unic-langid` 0.9.6, `unic-langid-impl` 0.9.6, `tinystr` 0.8.4, `displaydoc` 0.2.7 (proc macro), `zerovec` 0.11.8 and `zerofrom` 0.1.8.
  - `self_cell` 1.3.0, `type-map` 0.5.1, `smallvec` 1.16.3, `rustc-hash` 2.1.3, `memchr` 2.8.3 and `thiserror` 2.0.21 were already in the GUI graph. They are now also linked into the headless `va`.
  - `zerovec` and `zerofrom` are lock-only: `cargo tree --target all -i zerovec -e all` finds no path, and the release `va` build folder holds no `zerovec` or `zerofrom` artifact (checked 2026-10-18). m1 §8 says they come "through tinystr's default features", which is not what the resolver does today.
  - The normal closure of `va` on Windows is now clap, serde and serde_json with their helpers, plus the Fluent stack above (`cargo tree --locked -p va -e normal`, 2026-10-18). It holds no network-capable crate and no GUI crate (layering R4).
  - Licences: licence entry above. cargo-deny on 2026-10-18: `bans ok, licenses ok, sources ok`, and no advisory against any M1 crate (`self_cell` 1.3.0 is past RUSTSEC-2023-0070, fixed in 1.0.2). On 2026-10-23 (`15e9fa4`): all four checks ok, and the three ignored ids (M1-S2) name no M1 crate.
- **Rationale**: m1 §6.2 and §8. Fluent supplies plurals, selectors and typed arguments, which std does not.
- **Limitations**:
  - `self_cell`, `tinystr`, `smallvec` and `type-map` contain `unsafe` code that nobody here audited. Only RustSec `unsound` advisories cover it (unsafe-policy entry).
  - `displaydoc` is a proc macro that runs at build time (accepted build-time risk).
  - The `va` CLI is no longer a minimal binary: a Fluent parser for embedded, trusted resources now ships in it.
- **Failure mode**: Fail-closed through cargo-deny (licences, bans, sources, advisories) and the lock file.

### Yanked `clipboard-win` 5.4.1 in the Windows GUI graph (M1-S10): bump to 5.4.3, FIXED
- **Location**: `Cargo.lock:L673-L680` (`clipboard-win` 5.4.3, checksum `052af7878466feccda12ba7a6a67d019d712f296809559b47c12c8cc196d3a9a`; was 5.4.1, checksum `bde03770…7e84d4`), `.cargo/deny.toml:L26` (`yanked = "deny"`), `.github/workflows/ci.yml:L52-L67` (CI `deny` job), `DEPENDANCE.md:L74` (licence line naming the version). Dependency path, Windows targets only (`cfg(windows)`): `clipboard-win` 5.4.3 (was 5.4.1) → `arboard` 3.6.1 → `egui-winit` 0.36.2 → eframe 0.36.2 → `visual-assembler`. `va` does not contain it.
- **Language**: TOML (Cargo lock file, cargo-deny 0.20.2 policy). The reviewed crate is Rust (edition 2018, `no_std` with an `std` feature).
- **Description**:
  - Found: CI run 40023018227 on `8994d44` (started 2026-10-23 20:56 UTC) is red only in the `deny` job, with `error[yanked]: detected yanked crate (try `cargo update -p clipboard-win`)`. The last green run, 38970636341 on `cc827b0` (2026-10-16), locked the same 5.4.1 entry and checksum, so the yank alone turned the job red. Reproduced on a scratch `git archive` export of `HEAD`: `advisories FAILED, bans ok, licenses ok, sources ok`.
  - Why it was yanked (read on 2026-10-23 with `gh api` and the crates.io API, read-only):
    - Upstream issue DoumanAsh/clipboard-win#42, opened 2026-10-23 04:43 UTC: `raw::get_html` checked only that `EndFragment - StartFragment` fits in `GlobalSize`, not that the fragment ends inside the data. It then copied `size` bytes from `data.as_ptr().add(start_idx)` with `ptr::copy_nonoverlapping`. Any program on the same desktop can put an "HTML Format" entry whose header points past its end. Reading it crashes the reader (access violation) or returns bytes read past the clipboard data (heap over-read).
    - The maintainer fixed it in commit `38c3c3f` ("5.4.2: Fix handling of invalid html fragments"), published 5.4.2 at 05:26:39 UTC, and closed #42 at 05:37:37 ("Released 5.4.2 with this fix"). Between 05:38:19 and 05:38:27 the same account yanked 5.4.1, 5.4.0, 5.3.1, 5.3.0 and 5.2.0. These are exactly the releases that contain `get_html`: the HTML format arrived in 5.2.0 (commit `fdc4c8e`, "Implement HTML format (#32)"). 5.0.0 and 5.1.0, which predate it, are not yanked. 5.4.3 followed at 05:43:13 (commit `ed27008`).
    - No RustSec advisory exists for `clipboard-win`: rustsec/advisory-db has no `crates/clipboard-win` folder, and none of its pull requests names the crate. No GitHub advisory exists either, global or in the repository (2026-10-23).
  - Owner and publisher: crates.io lists one owner, the user `DoumanAsh`, and no team. Every listed version from 4.4.2 to 5.4.3 was published by `DoumanAsh`, and the crates.io audit actions show that the five yanks were made by the same account. No trusted publishing. crates.io shows no owner history, so "unchanged" here means the same single account that published every earlier release.
  - Diff review of the published sources (extracted read-only into a scratch folder, never built, run or imported):
    - The sha256 of each `.crate` equals its index checksum: `bde03770…7e84d4` for 5.4.1 (also in `Cargo.lock`), `052af787…6d3a9a` for 5.4.3.
    - Both archives hold the same 17 regular files. No `build.rs` (`build = false`), no proc macro, no binary target. Dependencies and features are unchanged: `error-code` `^3`, and the optional `windows-win` `^3` behind `monitor`, both `cfg(windows)`. arboard enables only `std`, so `windows-win` stays out of the graph. The licence stays BSL-1.0, already allowed.
    - Code changes from 5.4.1 to 5.4.3: (1) in `raw::get_html`, the raw copy (`ptr::copy_nonoverlapping`, `spare_capacity_mut`, `set_len`) is replaced by `data.get(start_idx..start_idx+size)` and `extend_from_slice`, a bounds-checked safe copy that returns `invalid_data()` when the range is outside the data. `start_idx + size` equals the already-parsed `end_idx`, so it cannot overflow. (2) In `Clipboard::new_attempts_for`, `Sleep(0)` becomes `Sleep(5)` between open attempts (Chromium's value). (3) `#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]` becomes `#![allow(clippy::style)]`. The rest is documentation (a doc link in angle brackets, a sample string), version numbers and one new test, `should_set_get_invalid_html`.
    - `unsafe` appears 73 times in both versions. The fix removes three unsafe operations inside an existing block and adds none. The FFI declarations of `src/sys.rs` are byte-identical. No process, network, file or environment API, no `include_*!`, no `asm!`, and no non-ASCII byte in `src/`.
    - The published sources equal the repository. Every non-generated file of each archive (14 files; `Cargo.toml.orig` compared with the repository's `Cargo.toml`) has the git blob SHA-1 of the upstream tree at the commit its `.cargo_vcs_info.json` names: `3b27cf2` for 5.4.1, `ed27008` for 5.4.3. `ed27008` is the current upstream head, three commits ahead of `3b27cf2`. The upstream has no tags or releases, and these commits are unsigned.
  - Reachability in the product: `egui-winit` 0.36.2 (`src/clipboard.rs`) calls only arboard's `get_text`, `set_text` and `set_image`, and no workspace crate depends on `arboard` or `clipboard-win`. arboard reaches `get_html` only through `Get::html()`, which nothing calls. arboard 3.6.1 opens the clipboard with its own retry loop (`Clipboard::new()` and a 5 ms `thread::sleep`), never `new_attempts`. So the 5.4.1 bug was never reachable in `visual-assembler`, and 5.4.3 changes no behaviour of our binary.
  - Compatibility: arboard 3.6.1 requires `clipboard-win` `^5.3.1` (`cfg(windows)`, feature `std`), which 5.4.3 satisfies. arboard 3.7.0, the newest, has the same requirement, so no arboard, egui-winit or eframe bump is involved. On `8994d44`, `cargo update -p clipboard-win --precise 5.4.3 --dry-run` printed only `Updating clipboard-win v5.4.1 -> v5.4.3`, and the dry run left the repository's `Cargo.lock` unchanged.
  - Pre-check of the fix: on the scratch export, with exactly that lock change (the `version` and `checksum` lines of `clipboard-win`), `cargo deny --locked --manifest-path Cargo.toml --config .cargo/deny.toml check` gives `advisories ok, bans ok, licenses ok, sources ok`, exit 0. Since `--locked` passes, the edited lock matches the manifests.
  - Decision: option (a), `cargo update -p clipboard-win --precise 5.4.3`. Security verdict on this option: non-blocking. **Applied** by the DevOps step on 2026-10-23, in the working tree on top of `8994d44`.
  - Fix, as checked by Security on 2026-10-23 (read-only):
    - `git diff Cargo.lock` changes exactly two lines, both in the `clipboard-win` entry (`L673-L680`): `version` 5.4.1 → 5.4.3, and `checksum` `bde03770…7e84d4` → `052af7878466feccda12ba7a6a67d019d712f296809559b47c12c8cc196d3a9a`. The new checksum is the one checked against the 5.4.3 `.crate` above. No other package moved, and `Cargo.toml` and `.cargo/deny.toml` are untouched.
    - `DEPENDANCE.md:L74` names `clipboard-win 5.4.3`.
    - `cargo deny --locked --manifest-path Cargo.toml --config .cargo/deny.toml check` on the working tree gives `advisories ok, bans ok, licenses ok, sources ok`, exit 0. DevOps reports the same result for `make security`.
    - The fix is in the commit that bumps `Cargo.lock`, together with `DEPENDANCE.md`. That commit did not exist yet when this was recorded. CI confirmation is pending: no `deny` job has run on the bumped lock.
  - Severity:
    - Before the fix (5.4.1 locked): low for the product, because the yanked code is not reachable (above). The gate, though, was broken: the required `deny` check is red on `main` at `8994d44` (run 40023018227) and would fail every pull request built on that lock (E2).
    - Option (a), applied: low. What remains is the general supply-chain risk of any new version (limitations below).
    - Option (b), *rejected*: an `[advisories] ignore` entry `{ crate = "clipboard-win@5.4.1", reason = "…" }` in the normative `.cargo/deny.toml`. Medium. It would knowingly keep a release that its author withdrew for a memory-safety bug while a drop-in fix exists, against the `.cargo/deny.toml` rule that advisories are never silenced to make CI green. The bug would become live as soon as any code calls arboard's `html()` (rich paste, for example). And it needs a spec revision.
    - Option (c), *rejected*: 5.1.0 (not yanked, no HTML code) does not satisfy arboard's `^5.3.1`. Bumping arboard or eframe is out of scope and would not help, since the newest arboard has the same requirement. A `[patch]` table is rejected by the docs check.
  - Not verified: the published 5.4.2 archive was not downloaded. Its source is upstream `38c3c3f`, the parent of `ed27008`. 5.4.3 is preferred: it is the newest, it is what `cargo update` picks, and its archive was checked against git.
- **Rationale**: `yanked = "deny"` exists to stop exactly this, a release that its author withdrew. The chosen version is a small, reviewed change from the same maintainer. It removes unsafe code, adds no dependency and leaves the policy unchanged.
- **Limitations**:
  - The review trusts the sole maintainer account. The signs point to a genuine fix rather than a compromise: the change matches a public issue's analysis and removes unsafe code, the account published every release since 4.x, the archives equal the repository, and dependencies and features are unchanged. But the commits are unsigned and there is no trusted publishing, so an account takeover could produce a matching repository and archive. The control that counts is the line-by-line diff review above, which found nothing that could do harm.
  - The reporter of #42 is a GitHub account created on 2026-10-11, with one public repository. The code we take was written and published by the maintainer and reviewed here, so this does not change the decision.
  - Nobody checked whether `get_html` is linked into the release binary. Since the bug is unreachable, it does not matter.
  - The bump does not trigger the S19-5 fallback-test rule: `clipboard-win` is not one of the listed crates. The crate builds only for Windows targets, so its build and test coverage is the two Windows `check` jobs and `make ci` on Windows. `make security` exits 0 after the bump (above). Security ran no build or test of the bumped lock: QA's `make ci` on Windows was still running when this was recorded, and the two Windows `check` jobs have not run on it.
  - CI has not confirmed the fix. Until the commit that bumps `Cargo.lock` is pushed and its `deny` job passes, the last CI result on `main` is the red run 40023018227 on `8994d44`.
  - If a RustSec advisory is later filed for `get_html` (versions below 5.4.2), 5.4.3 is outside its range.
- **Failure mode**: Fail-closed: `yanked = "deny"` caught the yank on the first CI run after it, as designed. Now that 5.4.3 is locked, a later yank of 5.4.3 fails the `deny` job again for a new triage. Fail-open for a malicious version that is neither yanked nor under an advisory: only a manual review like this one catches it (accepted supply-chain risk).

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
| R2-11 | "Comment lines and a scissors line are ignored by the hook but kept in the commit when the message comes from `-m`/`-F`, `--cleanup=verbatim`/`whitespace`, or a non-default `core.commentChar`. Such a message passes locally and is caught by the CI `docs` job's log check after the push, which becomes a required check with branch protection." | Accepted wording of §18.2 | CI `docs` job, a required check on `main` since branch protection was enabled (not for the owner's direct pushes, R3-8) |
| R3-1 | rustc treats U+200E and U+200F (invisible direction marks) as whitespace between tokens. The scan's helpers use `char::is_whitespace`, which does not include them. So `include\u{200E}!("x.inc")` escapes S8, and `std::\u{200E}net` escapes S7a. Verified: rustc 1.99.0 compiles both under `-D warnings`, and `scan_text` gives 0 violations. | Deliberate (invisible characters) | Review. `TcpStream` and the other S7b names are still flagged. Optional one-line hardening: reuse the existing `is_rustc_whitespace` in `skip_whitespace`, `previous_non_whitespace`, `identifier_before` and the S7 compaction (`xtask/src/layering.rs:L657-L683`, `L761`). |
| R3-5 | Spacing and look-alike variants of AI credits: `Co-Authored-By : Claude <…>` (a valid git trailer: `git interpret-trailers` normalizes it), or non-ASCII look-alike letters in a name | Deliberate; tools emit the canonical spelling | Review |

### Accepted risks

| Risk | Impact | Why accepted | Decision |
|------|--------|--------------|----------|
| F5: no process-group or Job Object kill in the bench runner | A descendant of the GUI outlives the run. Its reader threads stay detached until xtask exits. | Needs unsafe code or a new dependency, and the bench GUI spawns no child processes | Accepted (§17.8). The run still fails within the deadline + 5 s. |
| User-level cargo configuration (`$CARGO_HOME/config.toml`, a `.cargo/config.toml` in a parent of the checkout or inside a member folder) adding `build.rustflags`, `[patch]` or `paths` | Lints silenced, `+crt-static` dropped or a dependency replaced on one developer machine | `cargo config get` is unstable; step 0 can only read the repository's file | Accepted (§17.3.3). CI runners have no such file, and CI runs from the root. |
| F10: DLL planting on Windows. The release executables import non-KnownDLLs (`dxgi.dll`, `opengl32.dll`, `uiautomationcore.dll`, `dwmapi.dll`, `uxtheme.dll`, …; dumpbin, 2026-10-14). The §19 dual-renderer binary imports exactly the same list (dumpbin, 2026-10-15). wgpu and glutin load drivers at run time, and since §19 glutin does so on every default launch. | Code execution when an executable is started from a folder an attacker can write to (for example Downloads) | Nothing is distributed before M8. The fix needs an extra allow-listed unsafe file. | Accepted until M8 (§17.15). Planned, **not implemented**: before any Windows artifact ships, `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)` as the first statement of both `main`s, `/DEPENDENTLOADFLAG:0x800` in the windows-msvc rustflags (`CANONICAL_CARGO_CONFIG` updated in the same commit), and a Process Monitor check. |
| zbus and X11 environment transports (Linux): a `tcp:` D-Bus/AT-SPI bus address, or `DISPLAY=host:N`, makes the GUI open TCP connections | The GUI talks over the network when the user's environment says so | Required by accessibility (AccessKit -> zbus) and by X11 support. `async-io` and `polling` cannot be banned. | Accepted (architecture §9.3). The product itself never chooses a network address. Since §19.1, X11 is forced under WSL unless `VA_WAYLAND=1`, so a TCP `DISPLAY` is used there even when `WAYLAND_DISPLAY` is set; WSLg's own `DISPLAY=:0` is a local socket. |
| GPU driver selection by the environment (Linux libglvnd `__GLX_VENDOR_LIBRARY_NAME`, `__EGL_VENDOR_LIBRARY_FILENAMES`, Mesa overrides, Vulkan loader `VK_ICD_FILENAMES`/`VK_LAYER_PATH`) loads native libraries into the GUI. Since §19 the glow path (libGL/libEGL) runs on every default launch. | Code execution through the environment | Anyone who sets the process environment already controls it (`LD_PRELOAD`). The §19.3 genuine-failure command uses exactly these variables. | Accepted, same reasoning as the `VA_TEST_FAIL_RENDERER` footgun analysis. |
| Safe socket APIs of crates already in the Linux graph (`rustix` with `net`, `async-io`) once one becomes a direct dependency | Socket I/O with no flagged name | These crates cannot be banned without breaking zbus | Accepted. Every new direct dependency goes through R6 (one place, `[workspace.dependencies]`) and Security review. |
| Code in doctests and `#[doc]` strings is not scanned, and the workspace lints do not apply to doctests | Network or unsafe code in test-only doctests runs during `cargo test`, on developer machines and CI | Doc comments are blanked by design. Doctests are never shipped. | Accepted. Reviewers check examples that touch I/O or memory; such examples should be `no_run`. |
| Code run at build time by dependency build scripts and proc macros (downloads, arbitrary code on developer machines and CI) | Supply-chain compromise during the build | cargo has no build sandbox, and cargo-deny checks only metadata | Accepted for M0. The lock file limits drift. Consider `cargo vet` from M1. |
| A malicious version published under a pinned crate before it is locked (account takeover, typosquat) | Code execution in the product or the tooling | Nobody reviews each transitive crate | Accepted for M0. Review `Cargo.lock` diffs. The repository now exists, but no CODEOWNERS file does (checked 2026-10-16), so CODEOWNERS on `Cargo.lock` stays open with R3-8. |
| AT-SPI exposes UI text to any client on the local D-Bus session bus (Linux) | Local processes of the same user can read what the window shows | Accessibility requirement, by design | Accepted |
| S19-1 (medium, Security round 1 of §19, 2026-10-15): on Windows the release GUI uses the GUI subsystem, so stderr exists only when it is redirected. The `VA_TEST_FAIL_RENDERER` startup warning, the selection warnings and every §19.4 message (fallback warning, forced or all-renderers failure) are invisible when the app starts from Explorer, the Start menu or a PowerShell prompt. The §19.3 footgun analysis relied on that warning. | With the variable in the session or user environment, every launch silently runs wgpu (about 3.7 times the memory, §19.7). With `glow,wgpu`, or a forced `VA_RENDERER` whose renderer fails, the app exits 3 with no window and no message. | Nothing is distributed before M8. Whoever sets the environment already controls the process. Bench JSON records `renderer`, `fallback_from`, `fallback_reason` and `injected_failure`, so no measurement is misattributed. | Accepted for M0 (m0 §19.9 S19-1). The last sentence of the §19.3 footgun analysis now ends "on a console; on the Windows release build, bench JSON and, from M4, the status bar do" (hook entry). Deferred to M4, **not implemented**: the status bar shows a fallback and an armed `VA_TEST_FAIL_RENDERER` hook, and M4 decides how an exit-3 startup failure reaches a user with no console. Written (WP7, checked 2026-10-16): the root README says these messages are visible only when stderr is redirected (Git Bash, a pipe, the debug build), at `README.md:L240-L243`. *Rejected:* a ` (test hook active)` window-title suffix in M0, which would widen the spike.rs i18n exception for a test-only path. |
| M1-S2 (medium, M1 round of 2026-10-18; accepted by `215e407`, m1 §12.8 F-2): three RustSec `unmaintained` advisories are ignored in `.cargo/deny.toml:L27-L34`: RUSTSEC-2026-0370 (`zvariant` 5.15.0), RUSTSEC-2026-0371 (`zbus_names` 4.3.4) and RUSTSEC-2026-0372 (`zvariant_derive` 5.15.0). They reach only Linux builds of `visual-assembler`, through eframe 0.36.2 → egui-winit → accesskit_winit → accesskit_unix 0.21.1 → zbus 5 / atspi (AT-SPI over the local D-Bus session bus). `va` does not contain them. | A bug found later in the zbus 5 crates may get no fix there. `zvariant` decodes D-Bus messages from the session bus, so the attacker would be a local process of the same user. No vulnerability or `unsound` advisory exists for these versions (2026-10-23). | The advisories mean "merged into zbus 6", not a known flaw. There is no safe upgrade: zbus 5 has no successor, and the locally cached index lists `accesskit_unix` up to 0.24.0, all on zbus `^5` (2026-10-18). Leaving zbus 5 means an eframe/egui/accesskit change, which also triggers the S19-5 fallback-test rule. Dropping AccessKit is rejected: accessibility is a V1 requirement (M7). | Accepted with Security's conditions (checked against `.cargo/deny.toml` on 2026-10-23): (1) `unmaintained = "all"`, `unsound = "all"` and `yanked = "deny"` stay as they are (`L24-L26`); (2) exactly these three ids are ignored, each with a `reason`, and the ignores are never extended to a vulnerability or `unsound` advisory. Such an advisory against the same crates has another id, so it still fails `make security` and the CI `deny` job; (3) revisit at the next eframe/egui bump, and at the latest in M8 (release); (4) drop an entry as soon as eframe or accesskit allows zbus 6, or as soon as cargo-deny stops reporting it. No tool enforces (2) to (4): they rest on review and on this entry. `make security` exits 0 again on `15e9fa4`. |

---

## Threats identified and not covered

| Threat | Impact | Why not covered | Decision |
|--------|--------|-----------------|----------|
| R3-2: step 0 removes whitespace inside quoted strings, so `"target-feature=+crt-static "` passes. rustc only warns and links the CRT DLLs. | `+crt-static` silently lost after an unlikely typo. Only the manual E4 check would notice. | §18.4 specifies "remove all whitespace from each remaining line" | Open, low (Architect, then WP4): remove whitespace only outside string literals, or compare string contents exactly. Add a test with a space inside the feature string. |
| R3-3: check 3d skips folders named like an `IGNORED_DIRS` entry that `.gitignore` does not ignore (`out`, `env`, `.tox`, …) | A committed `va-arch-*` package in such a folder escapes `[lints]`/`[deps]` (layering and S6 still apply) | The walk reuses the README/AGENCEMENT exclusions | Open, low (WP5): in `iter_nested_manifests`, skip only `.git`, the target folders and gitignored paths. Add a fixture test. |
| R3-4: the CI log check cannot see an AI tool as commit *author* or *committer* | An AI-authored commit reaches history unnoticed | `COAUTHOR_RE` only matches trailer forms, `generated with` and the emoji; `%cn`/`%ce` are not logged | Open, low (WP5): add `%cn%n%ce` to `GIT_LOG_ARGS`, and match identity lines against a narrow pattern (`[bot]` suffix, `@anthropic.com`, `@openai.com`, the 12 names as whole words). |
| R3-6: exact `=` pins (rule R6) are not enforced by any tool | A caret requirement lets `cargo update` drift within the range | The `[deps]` check only collects `[workspace.dependencies]` names | Open, low (WP5): report every non-path `[workspace.dependencies]` entry whose version does not start with `=`. |
| R3-7: the ignored-test command recorded in §17.11 D10 (`-- --ignored --nocapture`) now also runs the two ignored §18.1 record tests, which fail by design (verified: 2 failed, live check ok) | The live check looks broken and may stop being run (`cargo xtask ci` still runs the same check) | The record tests were marked `#[ignore]` by §18.5 | Open, low (Architect): §17.11 D10 still records `-- --ignored --nocapture`. The by-name command `cargo test -p xtask --test layering -- --ignored live_workspace --nocapture` is documented in this file, in `CLAUDE.md:L43` and `L131-L132`, and in `xtask/tests/README.md:L27-L29` and `L40` (checked 2026-10-16). |
| R3-8: CI depends on files that any pull request can change, and branch protection does not apply to administrators | For the owner (the only collaborator, `admin`), every CI check above is advisory: a direct push to `main`, or a merge past a red check, is allowed. Observed on 2026-10-16: `646b6f7` reached `main` by a direct push, and its failing `check (windows-11-arm)` was reported only afterwards (run 38968085878; fixed by `6ed43c8`, run 38969165969). A pull request can still weaken `deny.toml`, `ci.yml` or xtask, and the jobs it rewrites keep the required names. | Partly covered (§17.17 item 3, read with `gh api` on 2026-10-16): the public repository exists, and `main` requires `fmt`, `docs`, `deny` and the four `check (…)` jobs, with force pushes and deletion disabled. `enforce_admins` is off, `strict` is off, no review is required, and no CODEOWNERS file exists (branch protection entry). | Required checks: **done**. Open, low (user decision): turn `enforce_admins` on if the owner's own pushes must be gated; add CODEOWNERS for `.github/`, `.cargo/`, `xtask/`, `scripts/`, `Cargo.lock` (it only adds review requests, unless reviews become required). |
| R3-9: the `xtask` alias (`run --package xtask --`) has no `--locked`. In the CI `check` job it runs first and rewrites a stale `Cargo.lock`, so the job's later `--locked` clippy, test and release-build steps pass (verified with a probe workspace, cargo 1.99.0) | A pull request that edits a manifest without updating `Cargo.lock` is built and tested in the `check` job with freshly resolved, uncommitted versions. Only the `deny` job fails. | Spec §3.3/§18.4 fix the alias, and the canonical comparison freezes it | Open, low (Architect, then WP4 and WP5): either add `--locked` to the alias (`xtask = "run --locked --package xtask --"`, with `.cargo/config.toml`, `CANONICAL_CARGO_CONFIG` and §18.4 updated in the same commit), or run `cargo run --locked --package xtask -- ci --no-fmt` in `ci.yml`. Add a test that the CI command fails on a stale lock. |
| R3-10: the usage comment of `.cargo/deny.toml` (L5-L8) shows the cargo-deny command without `--locked` | A command copied from it re-resolves a stale lock instead of failing | The comment predates §18.7, and WP1 had no change in that wave | Open, low (WP1, comment only): replace it with the §18.7 command. |
| S19-2 (low), as found on 2026-10-15: `VA_RENDERER` and `VA_TEST_FAIL_RENDERER` values were echoed to stderr unescaped (`to_string_lossy` in `resolve_renderer`, `parse_injected_failures` and `InjectedFailures::startup_warning`). A WSL probe on 2026-10-15 with a newline and an ESC sequence printed a forged `error: forged' …` line and raw `ESC[2J`. | Misleading console or log output, terminal escape sequences, also inside the 64 KiB stderr tail that xtask reports | Not a trust boundary: whoever sets the variable controls the process | **Fixed** by row 19-8(a) (m0 §19.9 S19-2), verified 2026-10-16. Every echoed `VA_RENDERER` and `VA_TEST_FAIL_RENDERER` value, the startup warning included, goes through `str::escape_debug` of its lossy text (`cli.rs:L342-L345`, `L407-L410`, `L430-L433`, `L447-L450`). `injected_failure` stays raw, and serde_json escapes it. Tests: `tests/cli.rs` `invalid_renderer_variable_is_echoed_escaped`, `injection_values_are_echoed_escaped`, `non_unicode_values_are_echoed_escaped`; `tests/binary.rs` `control_characters_in_selection_variables_reach_stderr_escaped` (Linux), which replays the 2026-10-15 probe on the binary. The exact-text tests pass unchanged. |
| S19-3 (low), as found on 2026-10-15: cargo-deny 0.20.2 `[bans.workspace-dependencies] unused = "deny"` is graph-based. A probe on 2026-10-15 used a copy of `HEAD` with the §19 root `Cargo.toml` (winit pinned, no member declaring it): `cargo deny check bans` gave `bans ok`. Adding `wgpu-types = "=24.0.0"`, a version not in the graph, gave `error[unused-workspace-dependency]`. | Rule R6 ("only crates used by a member") was not enforced for crates already in the graph. The root `Cargo.toml` comment (L28), DEPENDANCE.md line 8, CLAUDE.md and the §19.8 row 19-1 acceptance wording overstated it. An unused pin compiles nothing, so the security impact is small. | `check_project_docs.py` checked members against the workspace, not the reverse | **Fixed** by rows 19-9(b), 19-10 and 19-11 (m0 §19.9 S19-3), verified 2026-10-16. The reverse half of check 3b in `scripts/check_project_docs.py` (`L387-L466`) reports `[deps] Cargo.toml: workspace dependency '<name>' is not declared by any member (rule R6: remove the pin or declare it with workspace = true)`. Fixtures in `xtask/tests/docs.rs`: `unused_workspace_dependency_is_reported`, `workspace_dependency_used_only_in_a_target_table_counts`, `unused_path_workspace_dependency_is_reported`, plus three QA cases (supply-chain entry). cargo-deny `unused` stays graph-based and is documented as such. Root `Cargo.toml` L28 now reads `# Exact pins, each declared by at least one member (rule R6, enforced by check_project_docs.py check 3b).` The DEPENDANCE.md (`L7-L10`) and CLAUDE.md (`L53-L56`) wording was corrected by the WP7 row: both name check 3b and say that cargo-deny's `unused` rule only proves the pinned version is in the graph (checked 2026-10-16). |
| S19-4 (low), as found on 2026-10-15: `pin_windows_wgpu_backend` and `exclude_wgpu_gl_backend` counted `WGPU_BACKEND` as set when `var_os` found it, while wgpu-types 30.0.1 reads it with `std::env::var(..).ok()` and parses an empty string to an empty backend set | An empty value (for example exported by a shell profile) dropped the DX12 pin and gave wgpu no backend: forced wgpu, or the fallback, exited 3. A non-Unicode value brought the GL backend back after a glow failure, so the `EGL_BAD_ACCESS` panic (exit 101) returned under WSLg. | Mirrored the §17.13 pin check, which predates §19 | **Fixed** by row 19-8(b) (m0 §19.9 S19-4), verified 2026-10-16. `wgpu_backend_env_is_effective` (`lib.rs:L326-L338`) is true exactly when the value is valid Unicode and `Backends::from_comma_list` of it is not empty. The DX12 pin and the GL exclusion step aside only for an effective value, so an empty or non-Unicode value no longer drops the pin or brings back the `EGL_BAD_ACCESS` panic. Tests in `tests/renderer_backend.rs`: `wgpu_backend_env_is_effective_only_for_a_non_empty_backend_set`, the env-dependent `dx12_pin_applies_to_every_wgpu_attempt` and `wgpu_gl_backend_is_left_out_after_a_glow_failure`, and `attempt_options_follow_every_kind_of_wgpu_backend_value` (child processes, ten values). Accepted residue: on Linux, an empty or unrecognized `WGPU_BACKEND` still leaves wgpu with no backend (exit 3). That is wgpu's own reading. |
| S19-5 (low): the in-process retry, meaning event-loop reuse, the GL-backend exclusion and the borrowed recorder, is exercised only by the `#[ignore]`d tests of `tests/fallback.rs` (10, 8 of them compiled on Windows x64) and by the `continue-on-error` CI smoke step. Until row 19-9(a), that step injected glow on `windows-11-arm` too, where `auto` never reaches it. | A dependency bump or refactor that makes the fallback panic, hang or exit 3 passes every required check | GUI windows cannot open on the headless runners, except the informational Windows smoke steps (architecture §9.1: never evidence) | Accepted limitation (m0 §19.9 S19-5), with three compensating controls (fallback entry). (1) The second smoke step injects the first renderer of the runner's order: glow on x64, wgpu on `windows-11-arm`, so ARM64 now exercises wgpu to glow (row 19-9 a). (2) Binding QA rule, procedural: run `cargo test -p visual-assembler --test fallback -- --ignored --test-threads=1` on Windows and WSLg X11 whenever `Cargo.lock` changes eframe, egui, egui-wgpu, egui_glow, wgpu, glutin or winit, and before each milestone closes. (3) Headless classification and options tests. *Rejected:* the in-step assertion. Revisit with branch protection (§17.17 item 3): that condition is now met, since the `check (…)` jobs are required on `main` (read 2026-10-16), and the revisit is open (Architect). The smoke steps are still `continue-on-error`, so they cannot gate. First CI observation, run 38969165969 (`6ed43c8`, 2026-10-16): x64, genuine glow failure to wgpu, exit 0; ARM64, injected wgpu failure to glow, which genuinely failed, exit 3 with the §19.4 messages, and the job stayed green (fallback entry). |
| M1-S1 (medium, M1 round), as found on 2026-10-18: `va lex` text mode writes source-derived text to stderr unescaped. That covers the Fluent `$text` argument in each diagnostic message (32 chars of source; `crates/va-core/src/syntax.rs:L254-L256`, rendered without isolation marks, `localizer.rs:L210`) and the `  fix:` line, which is the whole source line with the fix applied, up to 64 KiB (`apps/va/src/commands/lex.rs:L129-L137`, `L207-L226`). Verified by byte dump: `ESC]0;…BEL` (title), `ESC[2J` (clear screen), an OSC 52 clipboard write (`ESC]52;c;<base64>BEL`) behind a `;` on a line whose operands end with `,`, U+202E and U+009B, all raw on stderr. | A crafted source file (a course file, a classmate's file) can retitle or clear the terminal, forge or erase earlier output lines (for example a fake "no errors"), and, on terminals that honour OSC 52 writes, replace the clipboard with a command the student may paste. The same class as S19-2, but here the input crosses a trust boundary. | Not specified: m1 §7.3 defines the text layout and §5.2 accepts control characters in strings and comments | **Fixed** by `a1f4294` (m1 §12.8; owner WP-F), verified 2026-10-23 on `15e9fa4`. `report::terminal_safe` (`apps/va/src/report.rs:L250-L295`) rewrites exactly the recommended set into its `escape_debug` form. `write_diagnostic_text` (`apps/va/src/commands/lex.rs:L203-L242`) applies it to the message, `help`, `note` and fix line, and `write_text` to the file prefix (`L151`). The five probe files of 2026-10-18 now give no ESC, BEL or CR byte and no raw C1 or bidi char on stderr (byte dumps). JSON is unchanged. Tests in `apps/va/tests/lex.rs`: `text_mode_never_writes_terminal_controls_raw` (OSC title, `ESC[2J`, OSC 52, U+202E and U+009B in comments, strings, invalid runs and suspect comments; JSON keeps the text) and `echoed_file_names_are_terminal_safe`. Residues: M1-S8 (clap usage errors) and M1-S9 (U+061C), output-encoding entry. |
| M1-S2 (medium, gate; not caused by M1), as found on 2026-10-18: `make security` exited 1. cargo-deny 0.20.2 reports `advisories FAILED` for RUSTSEC-2026-0370 (`zvariant` 5.15.0), RUSTSEC-2026-0371 (`zbus_names` 4.3.4) and RUSTSEC-2026-0372 (`zvariant_derive` 5.15.0). All three are `unmaintained` (merged into zbus 6). They reach the Linux GUI through `accesskit_unix` 0.21.1 and eframe 0.36.2 (verified 2026-10-18, same versions at `cc827b0`). | Exit criterion E1 fails, and the required CI `deny` job will fail on the next push (E2). `zvariant` decodes D-Bus messages from the session bus (AT-SPI), so a future bug in the 5.x line may not get a fix. No vulnerability advisory exists. The CLI `va` does not contain these crates. | `.cargo/deny.toml:L19-L34`: an advisory is fixed by a bump, or by an ignore with Security's approval and a SECURITY.md entry. No bump exists: the locally cached index lists `accesskit_unix` up to 0.24.0, all on zbus `^5` (2026-10-18). | **Accepted** by `215e407` (m1 §12.8 F-2), with Security's conditions; see "Accepted risks". `.cargo/deny.toml:L27-L34` ignores exactly these three ids, each with a `reason`. `make security` gives `advisories ok, bans ok, licenses ok, sources ok` (2026-10-23, `15e9fa4`). |
| M1-S3 (low), as found on 2026-10-18: text-mode token lines on stdout quote each token with `serde_json`, which escapes only C0 controls, `"` and `\` (`lex.rs:L175-L191`). DEL, C1 controls (U+009B is CSI, which some terminals honour) and bidi controls stay raw (verified 2026-10-18). | Same effects as M1-S1, limited to terminals that act on C1 or bidi controls | serde_json follows RFC 8259 | **Fixed** by `a1f4294`, verified 2026-10-23. `write_token_lines` (`apps/va/src/commands/lex.rs:L174-L201`) passes the JSON-quoted text through `terminal_safe`, so U+202E and U+009B print as `\u{202e}` and `\u{9b}`. Tests in `apps/va/tests/lex.rs`: `text_mode_never_writes_terminal_controls_raw`, `qa_text_mode_token_texts_stay_json_strings`. |
| M1-S4 (low), as found on 2026-10-18: echoes of user-supplied values are unescaped on stderr: the `VA_LANG` warning (`apps/va/src/commands/mod.rs:L42-L46`), the file name in `cannot read '<name>'` and in each diagnostic prefix (`apps/va/src/input.rs:L41-L47`), and the query in `unknown mnemonic '<q>'` and `does not accept the size suffix '.<s>'` (`apps/va/src/commands/isa.rs:L120-L149`). Verified 2026-10-18: ESC and CR reached stderr raw. | Misleading console output. File names from an untrusted archive reach it through `find -exec va lex` or scripts. | Not a trust boundary for values the user types. The GUI already escapes the same class (S19-2), so the two binaries differ. | **Fixed** by `a1f4294` (m1 §12.8 N-2), verified 2026-10-23. `print_error_line` and `print_warning_line` (`apps/va/src/report.rs:L218-L237`) pass every `error:` and `warning:` line through `terminal_safe`: the `VA_LANG` warning, the file name, the `va isa` query and suffix. The `va lex` file prefix is escaped once in `write_text`. `va.error/1` keeps the raw text, which serde escapes. Probes: `VA_LANG=$'x\e[2J\ry'`, `MO<ESC>[2JV`, `.<ESC>]0;T<BEL>` and `no<ESC>[31mpe` each give one escaped line. Tests: `apps/va/tests/isa.rs` `echoed_query_and_suffix_are_terminal_safe` and `va_lang_with_control_characters_gives_one_safe_warning_line`; `apps/va/tests/lex.rs` `echoed_file_names_are_terminal_safe`. |
| M1-S5 (low): output amplification. The 16 MiB input limit does not bound the output. Measured on 2026-10-18 (release, Windows): `^a` pairs in operand fields gave 516 MB on stdout plus 1.47 GB on stderr in 72 s (text), and 3.77 GB of JSON in 376 s. Empty lines gave 783 MB of JSON, and `1,` operands 1.2 GB of JSON. Memory stayed at about 33 MB. | A local denial of service by disk or terminal flooding. m1 §10 F8 says about 20×; the worst case measured is about 225× (JSON). | Output is streamed by design (§7.3), so memory is bounded | Open, low (Architect, M2, the first milestone that adds diagnostics): optionally cap the diagnostics per file (for example 1000, then one `too many errors` note, as compilers do), and correct F8. Unchanged on 2026-10-23. |
| M1-S6 (low): `va lex` opens the path as given, with no timeout. A FIFO with no writer, `CON`, `/dev/tty` or a stalled share blocks forever (WSL FIFO verified). On Windows, a UNC path starts an SMB connection and can send the user's NTLM credentials. | Hang, or a credential leak when a script passes an attacker-chosen path. In M1 no path comes from file content. | Local tool. The M7 `SourceProvider` sandbox owns path policy. | Open, low. M1 (Documentarian, WP-G): say in `apps/va/README.md` that `va lex` opens the path as given, with no timeout. On 2026-10-23 that README still dated from M0. M7 (Architect): the `SourceProvider` sandbox (next row) rejects UNC and device paths for every path the tools open, the CLI argument included. |
| M1-S7 (low, hardening): the lexer's no-panic and linear-time contract (m1 §3.4) rests on tests only. CI runs no fuzzer. The workspace denies neither `clippy::indexing_slicing` nor `clippy::arithmetic_side_effects`. The release profile has no overflow checks. The CI timing guard (40 lines under 30 s in debug) catches only gross quadratic behaviour. | A future lexer edit could add a panic (exit 101) or a slow path that the suite misses. The 2026-10-18 fuzz run (3,000,000 lines, overflow checks on) found none. | Spec choice: no fuzz or criterion tooling before M7 (m1 §5.6) | Open, low (Architect, M7): add a long-running fuzz or property target (ignored test, or `cargo fuzz` in xtask) over `lex_line` + `line_diagnostics` + `Localizer::render`, and consider `#![deny(clippy::indexing_slicing)]` in `arch/m68k/src/syntax/`. Re-run on 2026-10-23 after `e764313`: the same 3,000,000-line fuzz run found nothing (lexer-robustness entry). |
| M1-S8 (low, recording pass, 2026-10-23): clap's usage errors echo the offending argument raw. `print_parse_outcome` (`apps/va/src/main.rs:L43-L56`) calls `clap::Error::print()`, which bypasses `terminal_safe`. Probes on the release `va` of `15e9fa4`, with colour forced (`CLICOLOR_FORCE=1`, as on a terminal): `va $'\e[2J'` printed `error: unrecognized subcommand '` followed by a raw `ESC[2J`; `va lex $'--x\e]0;T\a'` printed the raw OSC and BEL; CR, U+202E and U+009B passed too. With colour off (a pipe, or `NO_COLOR`, which the tests set), anstream strips ESC sequences and BEL, but CR, LF, U+202E and U+009B stay raw. | The class of M1-S4: misleading or terminal-driving output. The text comes only from an argument of the `va` command line itself, so whoever passes it already runs `va`. A script that passes an untrusted name starting with `-` reaches it. | clap renders its own messages. `a1f4294` covered the lines that `va` prints itself. | Open, low (Developer, WP-F scope). Exact change in `apps/va/src/main.rs`: add `fn print_usage_error(error: &clap::Error) -> io::Result<()> { let text = error.render().to_string(); let safe = text.split('\n').map(report::terminal_safe).collect::<Vec<_>>().join("\n"); let mut stderr = io::stderr().lock(); stderr.write_all(safe.as_bytes())?; stderr.flush() }` and call it in `print_parse_outcome` instead of `error.print()` when `error.use_stderr()` is true (usage errors). `--help` and `--version` keep `error.print()` on stdout. The exit code (`error.exit_code()`) and the write-failure branch stay as they are. `StyledStr`'s `Display` drops clap's styling, so usage errors lose their colours. It also strips ESC sequences and BEL from the quoted argument, and `terminal_safe` then escapes CR, C1 and bidi chars. Checked on 2026-10-23 with a scratch build of this exact function against clap 4.6.7, colour forced: `a<ESC>[2Jb` and `a<ESC>]0;T<BEL>b` print as `'ab'`, `a<CR>b` with an escaped CR, U+202E as `\u{202e}` and U+009B as `\u{9b}`; `--version` is unchanged. Test in `apps/va/tests/binary.rs`: run `va` with each of the arguments `a\u{1b}[2Jb`, `a\rb`, `a\u{202e}b` and `a\u{9b}b` (Rust escapes), with `CLICOLOR_FORCE=1` and without `NO_COLOR`. Assert exit 2, no byte `1B`, `07`, `0D`, `C2 9B` or `E2 80 AE` on stderr, and `'ab'`, `a\rb`, `a\u{202e}b` and `a\u{9b}b` (literal backslashes) in it. Accepted residue after the fix: an LF inside an argument still starts a new line of the message, because it cannot be told apart from clap's own layout. |
| M1-S9 (low, recording pass, 2026-10-23): `is_unsafe_for_terminal` (`apps/va/src/report.rs:L254-L266`) leaves out U+061C ARABIC LETTER MARK, the third implicit bidi mark next to U+200E and U+200F. A probe line with U+061C in an operand and in an unterminated string printed it raw on stderr. | It can change the display direction of the neutral chars around it (quotes, punctuation, digits) in a diagnostic line. It cannot embed or override like U+202A–U+202E, and it has no control function. | The fix wave took the set recommended for M1-S1, which missed U+061C. | Open, low (Developer, WP-F scope): add `'\u{61C}'` as one more alternative of the `matches!` pattern in `is_unsafe_for_terminal`, update its doc comment, and add U+061C to `HOSTILE_SOURCE` and `TERMINAL_UNSAFE` (bytes `D8 9C`) in `apps/va/tests/lex.rs`. Zero-width chars without a bidi or control effect (U+200B–U+200D, U+2060, U+FEFF) stay as they are. |
| M1-S10 (low, recording pass, 2026-10-23), as found on 2026-10-23: `Cargo.lock` locked `clipboard-win` 5.4.1 (Windows GUI only: `arboard` 3.6.1 → `egui-winit` 0.36.2 → eframe 0.36.2; `va` does not contain it), which its sole maintainer yanked on 2026-10-23 with 5.4.0, 5.3.1, 5.3.0 and 5.2.0. They are the releases with `raw::get_html`, whose unchecked fragment offsets allow a heap over-read (upstream issue #42, fixed in 5.4.2). CI run 40023018227 on `8994d44` is red only in the `deny` job (`error[yanked]`), because `.cargo/deny.toml:L26` sets `yanked = "deny"`. No RustSec or GitHub advisory exists. | The required `deny` check is red on `main` and fails every pull request built on that lock (E2). The bug itself is not reachable: `egui-winit` calls only arboard's `get_text`, `set_text` and `set_image`, and nothing calls arboard's `html()`. | The yank was published after the last green run (38970636341, `cc827b0`, 2026-10-16), which locked the same 5.4.1 entry and checksum. | **Fixed** by `cargo update -p clipboard-win --precise 5.4.3` (option (a), DevOps, 2026-10-23), verified read-only by Security on 2026-10-23 in the working tree on top of `8994d44`. `git diff Cargo.lock` changes only the `version` and `checksum` lines of the entry (`Cargo.lock:L673-L680`, now 5.4.3, checksum `052af7878466feccda12ba7a6a67d019d712f296809559b47c12c8cc196d3a9a`), `DEPENDANCE.md:L74` names 5.4.3, and `cargo deny --locked --manifest-path Cargo.toml --config .cargo/deny.toml check` gives `advisories ok, bans ok, licenses ok, sources ok`. The 5.4.1 → 5.4.3 source diff was reviewed line by line (yanked-`clipboard-win` entry). The fix is in the commit that bumps `Cargo.lock`; CI confirmation is pending. *Rejected:* an `[advisories] ignore` entry for 5.4.1, and 5.1.0, which does not satisfy arboard's `^5.3.1`. |
| Untrusted inputs (S-records, project files, settings, `INCLUDE` paths). On Windows, a UNC path (`\\host\share\…`) opened through `std::fs` starts an SMB connection and can leak the user's NTLM hash. | Parser crashes, path traversal, network access and credential leak through file paths | M1 (2026-10-18): untrusted source text is now lexed. The lexer side is covered by the input-limit, UTF-8 and lexer-robustness entries. `va lex` opens only the path on its command line, and `INCLUDE`/`INCBIN` operands are never opened (file-access entry, M1-S6). No other parser and no other file opening exist. | Planned for M7, **not implemented** (architecture §9.5): the `SourceProvider` sandbox must also reject UNC and device paths (`\\?\`, `\\.\`). |
| Release signing, SBOM, signed commits | Users cannot verify where binaries come from | Nothing is released in M0 | M8 packaging |

## Secrets and configuration

M0 and M1 use no secrets. The CI workflow references no repository secret. Its `GITHUB_TOKEN` is read-only
and never persisted in the checkout (`persist-credentials: false`). The workflow never calls `gh`. The
`gh` CLI on the developer machine keeps its credentials in the OS keyring, outside the repository.

Environment variables read by the project are configuration, never credentials:
- `VA_PYTHON` (xtask `docs`; an empty value counts as unset);
- `CARGO` and `CARGO_TARGET_DIR` (xtask);
- `WGPU_BACKEND`, `WGPU_ADAPTER_NAME`, `WGPU_POWER_PREF`, `WAYLAND_DISPLAY` and `DISPLAY` (bench; their values are recorded in the gitignored reports);
- `VA_RENDERER`, `VA_TEST_FAIL_RENDERER` and `VA_WAYLAND` (GUI renderer selection, §19.1 and §19.3; read once at startup by `select`, also recorded in bench reports). `VA_TEST_FAIL_RENDERER` is a test hook: it only makes renderer initialization fail on purpose. Leave it unset outside tests (S19-1);
- `VA_LANG`, then `LC_ALL`, `LC_MESSAGES` and `LANG` (M1: language of `va lex` and `va isa`, read only when `--lang` is absent, never by `va info`; m1 §6.2, §12.7). They only choose between the two embedded languages;
- `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `CARGO_BUILD_RUSTFLAGS` and `CARGO_TARGET_*_RUSTFLAGS` are read only to refuse them (`cargo xtask ci` step 0). They must stay **unset**, which rules out an empty value too (§18.3): any of them replaces the `+crt-static` flags of `.cargo/config.toml`.

There is nothing to rotate. When a secret is first needed (M8 signing), it must come from GitHub
encrypted secrets, scoped to the release workflow only, and be recorded here with its rotation
procedure.
