//! Tests for `src/ci.rs`: the normative step list of `cargo xtask ci` (m0-foundations.md §6,
//! with step 0 of §17.3.2), the build-configuration checks of step 0 — `check_build_env` (set
//! variables, even empty, §18.3) and the canonical `.cargo/config.toml` comparison
//! `check_cargo_config` / `check_cargo_config_files` (§18.4) — the real `cargo xtask ci`
//! stopping at step 0, the path used to move the running executable aside on Windows, the
//! §19.9 shape of `.github/workflows/ci.yml`, which runs `cargo xtask ci` (CR19-06, S19-5), and
//! its §20 runner labels (Security S20-1), each check proven by mutated in-memory copies.
#![allow(clippy::unwrap_used)]

use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU32, Ordering},
};

use xtask::ci::{
    CANONICAL_CARGO_CONFIG, CiArgs, Step, check_build_configuration, check_build_env,
    check_cargo_config, check_cargo_config_files, parked_path, replaces_config_rustflags, steps,
};

/// Returns the labels of the steps selected by `no_fmt`.
fn labels(no_fmt: bool) -> Vec<String> {
    steps(&CiArgs { no_fmt }).iter().map(Step::label).collect()
}

/// The `.cargo/config.toml` of the repository as specified (m0-foundations.md §3.3).
const SPEC_CONFIG: &str = r#"# Cargo configuration. Normative content: docs/specs/m0-foundations.md §3.3.
# The RUSTFLAGS env var replaces the `rustflags` below entirely: never set it in CI, scripts or the
# Makefile.

[alias]
xtask = "run --package xtask --"

# Static MSVC runtime: Windows binaries need no VC++ redistributable.
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]

[target.aarch64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]

# aarch64 Linux: cross builds from x86_64 (apt: gcc-aarch64-linux-gnu).
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
"#;

/// Line number of the first `rustflags` line of [`SPEC_CONFIG`] (under the x86_64 msvc table).
const SPEC_RUSTFLAGS_LINE: usize = 10;

/// Line number of the `xtask` alias of [`SPEC_CONFIG`].
const SPEC_ALIAS_LINE: usize = 6;

/// Returns [`SPEC_CONFIG`] with `line` inserted so that it becomes line `number` (1-based).
fn spec_with_line_inserted(number: usize, line: &str) -> String {
    let mut lines: Vec<&str> = SPEC_CONFIG.lines().collect();
    lines.insert(number - 1, line);
    lines.join("\n") + "\n"
}

/// Returns [`SPEC_CONFIG`] with line `number` (1-based) replaced by `line`.
fn spec_with_line_replaced(number: usize, line: &str) -> String {
    let mut lines: Vec<&str> = SPEC_CONFIG.lines().collect();
    lines[number - 1] = line;
    lines.join("\n") + "\n"
}

/// Returns [`SPEC_CONFIG`] with `line` appended, and the number of that last line.
fn spec_with_line_appended(line: &str) -> (String, usize) {
    (
        format!("{SPEC_CONFIG}{line}\n"),
        SPEC_CONFIG.lines().count() + 1,
    )
}

/// Asserts that `problems` is exactly one message about line `number` that quotes `line` and
/// says `what` ("is not in the canonical configuration" or "differs from the canonical line").
fn assert_single_line_problem(problems: &[String], number: usize, line: &str, what: &str) {
    assert_eq!(problems.len(), 1, "{line}: {problems:?}");
    let expected = format!(".cargo/config.toml:{number}: `{}` {what}", line.trim());
    assert!(
        problems[0].starts_with(&expected),
        "{line}: expected `{expected}…`, got {problems:?}"
    );
}

/// Message part of an extra line.
const EXTRA: &str = "is not in the canonical configuration";

/// Message part of a changed line.
const CHANGED: &str = "differs from the canonical line";

/// §17.3.2 changed the step order: step 0 "build configuration" now comes first. §19.6 added
/// step 3b, the wgpu-only clippy run, right after the glow-only one. m1 §12.7 (R12-4) added step
/// 3c, clippy on `va` without any CPU, after it, and step 5b, the `va` tests without any CPU,
/// after the workspace tests.
#[test]
fn steps_match_the_spec_in_order() {
    assert_eq!(
        labels(false),
        [
            "build configuration",
            "cargo fmt --all -- --check",
            "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings",
            "cargo clippy -p visual-assembler --all-targets --no-default-features --features renderer-glow --locked -- -D warnings",
            "cargo clippy -p visual-assembler --all-targets --no-default-features --features renderer-wgpu --locked -- -D warnings",
            "cargo clippy -p va --all-targets --no-default-features --locked -- -D warnings",
            "layering and source checks",
            "cargo test --workspace --all-features --locked",
            "cargo test -p va --no-default-features --locked",
        ]
    );
}

/// m1 §12.7 (R12-4): `va` without any `arch-*` feature is linted once and tested once, both with
/// the default features off and no feature enabled, on `va` alone (neither step rebuilds xtask).
#[test]
fn va_without_architecture_is_linted_and_tested() {
    use xtask::ci::{CLIPPY_VA_NO_ARCH, TEST_VA_NO_ARCH};
    let all = steps(&CiArgs::default());
    for (args, command) in [(CLIPPY_VA_NO_ARCH, "clippy"), (TEST_VA_NO_ARCH, "test")] {
        let runs = all
            .iter()
            .filter(|step| **step == Step::Cargo(args))
            .count();
        assert_eq!(runs, 1, "{args:?}");
        assert_eq!(args.first(), Some(&command), "{args:?}");
        assert!(args.contains(&"--no-default-features"), "{args:?}");
        assert!(!args.contains(&"--features"), "{args:?}");
        assert!(!args.contains(&"--all-features"), "{args:?}");
        let package = args.iter().position(|arg| *arg == "-p").unwrap();
        assert_eq!(args[package + 1], "va", "{args:?}");
        assert!(!args.contains(&"--workspace"), "{args:?}");
        assert!(!Step::Cargo(args).rebuilds_xtask(), "{args:?}");
    }
    assert!(CLIPPY_VA_NO_ARCH.contains(&"--all-targets"));
}

/// §17.3.2: step 0 also runs with `--no-fmt`; only the format check is skipped.
#[test]
fn no_fmt_skips_only_the_format_check() {
    let all = labels(false);
    let mut expected = vec![all[0].clone()];
    expected.extend_from_slice(&all[2..]);
    assert_eq!(labels(true), expected);
    assert_eq!(labels(true)[0], "build configuration");
}

/// §19.6: both single-renderer builds are linted, each with exactly one renderer feature and the
/// default features off, so neither can hide behind the other renderer's code.
#[test]
fn each_single_renderer_build_is_linted_once() {
    for renderer in ["renderer-glow", "renderer-wgpu"] {
        let runs: Vec<&'static [&'static str]> = steps(&CiArgs::default())
            .into_iter()
            .filter_map(|step| match step {
                Step::Cargo(args) if args.contains(&renderer) => Some(args),
                _ => None,
            })
            .collect();
        assert_eq!(runs.len(), 1, "{renderer}: {runs:?}");
        let args = runs[0];
        assert_eq!(args.first(), Some(&"clippy"), "{renderer}");
        assert!(args.contains(&"--no-default-features"), "{renderer}");
        let features = args.iter().position(|arg| *arg == "--features").unwrap();
        assert_eq!(args[features + 1], renderer);
        assert!(args.contains(&"--all-targets"), "{renderer}");
    }
    assert_eq!(xtask::ci::CLIPPY_WGPU_ONLY[6], "renderer-wgpu");
    assert_eq!(xtask::ci::CLIPPY_GLOW_ONLY[6], "renderer-glow");
}

#[test]
fn only_the_test_step_rebuilds_xtask() {
    let rebuilding: Vec<String> = steps(&CiArgs::default())
        .into_iter()
        .filter(Step::rebuilds_xtask)
        .map(|step| step.label())
        .collect();
    assert_eq!(
        rebuilding,
        ["cargo test --workspace --all-features --locked"]
    );
}

#[test]
fn parked_executable_keeps_its_name_with_a_suffix() {
    let parked = parked_path(Path::new("target/debug/xtask.exe"));
    assert_eq!(parked, Path::new("target/debug/xtask.exe.old"));
}

// QA edge cases (audit round 1).

#[test]
fn parked_path_keeps_unicode_folders_with_spaces() {
    let executable = Path::new("Code Info").join("tärget 文件").join("xtask.exe");
    let parked = parked_path(&executable);
    assert_eq!(parked.parent(), executable.parent());
    assert_eq!(
        parked.file_name().unwrap().to_string_lossy(),
        "xtask.exe.old"
    );
}

#[test]
fn every_cargo_step_is_locked_and_denies_warnings() {
    for step in steps(&CiArgs { no_fmt: true }) {
        if let Step::Cargo(args) = step {
            assert!(args.contains(&"--locked"), "{}", step.label());
            if args.first() == Some(&"clippy") {
                assert!(
                    args.ends_with(&["--", "-D", "warnings"]),
                    "{}",
                    step.label()
                );
            }
        }
    }
}

// Step 0 (§17.3.2): build environment.

#[test]
fn rustflags_variables_are_reported_with_the_spec_message() {
    let problems = check_build_env([("RUSTFLAGS", "-Aunsafe_code")]);
    assert_eq!(
        problems,
        [
            "RUSTFLAGS is set: it replaces .cargo/config.toml (+crt-static) and can silence lints; unset it"
        ]
    );
}

#[test]
fn every_rustflags_variable_is_reported_in_name_order() {
    let problems = check_build_env([
        ("RUSTFLAGS", "-Aunsafe_code"),
        ("PATH", "/usr/bin"),
        ("CARGO_ENCODED_RUSTFLAGS", "-Aunsafe_code"),
        ("CARGO_BUILD_RUSTFLAGS", "-Awarnings"),
        (
            "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS",
            "-Ctarget-feature=-crt-static",
        ),
        (
            "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
            "-Aunsafe_code",
        ),
        ("CARGO_TARGET_DIR", "target/wp4"),
        ("RUSTDOCFLAGS", "--cfg docsrs"),
    ]);
    let names: Vec<&str> = problems
        .iter()
        .map(|problem| problem.split(' ').next().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "CARGO_BUILD_RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
            "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS",
            "RUSTFLAGS",
        ]
    );
}

/// §18.3 split the former `clean_or_empty_build_environment_passes`: cargo 1.99 treats an empty
/// `RUSTFLAGS` as an override that drops `+crt-static`, so an empty value is now a problem.
#[test]
fn empty_build_variable_is_rejected() {
    assert_eq!(
        check_build_env([("RUSTFLAGS", "")]),
        [
            "RUSTFLAGS is set: it replaces .cargo/config.toml (+crt-static) and can silence lints; unset it"
        ]
    );
    for name in [
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS",
        "cargo_target_aarch64_unknown_linux_gnu_rustflags",
    ] {
        let problems = check_build_env([(name, ""), ("PATH", "/usr/bin")]);
        assert_eq!(problems.len(), 1, "{name}: {problems:?}");
        assert!(
            problems[0].starts_with(&format!("{name} is set: ")),
            "{problems:?}"
        );
    }
}

/// §18.3: an environment without any rustflags-type variable passes, whatever else is set.
#[test]
fn clean_build_environment_passes() {
    assert_eq!(
        check_build_env([
            ("PATH", "/usr/bin"),
            ("CARGO_TARGET_DIR", "target/wp4"),
            ("CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER", "lld-link"),
            ("RUSTDOCFLAGS", ""),
            ("MY_RUSTFLAGS", ""),
        ]),
        Vec::<String>::new()
    );
    assert_eq!(
        check_build_env(Vec::<(String, String)>::new()),
        Vec::<String>::new()
    );
}

/// A variable name that is not valid Unicode is compared in its lossy form: it can only add
/// problems, never hide one.
#[cfg(unix)]
#[test]
fn non_unicode_rustflags_variable_name_is_still_reported() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt as _};
    let name = OsString::from_vec(b"CARGO_TARGET_\xff_RUSTFLAGS".to_vec());
    let problems = check_build_env([(name, OsString::from("-Aunsafe_code"))]);
    assert_eq!(problems.len(), 1, "{problems:?}");
}

#[test]
fn rustflags_variable_names_ignore_ascii_case() {
    assert!(replaces_config_rustflags("rustflags"));
    assert!(replaces_config_rustflags(
        "Cargo_Target_x86_64_pc_windows_msvc_RustFlags"
    ));
    assert!(!replaces_config_rustflags("CARGO_TARGET_RUSTFLAGS_EXTRA"));
    assert!(!replaces_config_rustflags("MY_RUSTFLAGS"));
    assert!(!replaces_config_rustflags("CARGO_TARGET_RUSTFLAGS"));
}

// Step 0 (§18.4, replacing the §17.3.2 b whitelist): `.cargo/config.toml` must equal
// `CANONICAL_CARGO_CONFIG` once blank lines, full-line comments and whitespace are dropped. The
// whitelist tests below keep their inputs; §18.4 changed what they expect: every line outside
// the canonical file is now a violation, and a lone fragment of the file lacks canonical lines.

/// §18.4: the constant is the normative block, line for line.
#[test]
fn canonical_constant_matches_the_spec_block() {
    assert_eq!(
        CANONICAL_CARGO_CONFIG,
        [
            "[alias]",
            "xtask=\"run--packagextask--\"",
            "[target.x86_64-pc-windows-msvc]",
            "rustflags=[\"-C\",\"target-feature=+crt-static\"]",
            "[target.aarch64-pc-windows-msvc]",
            "rustflags=[\"-C\",\"target-feature=+crt-static\"]",
            "[target.aarch64-unknown-linux-gnu]",
            "linker=\"aarch64-linux-gnu-gcc\"",
        ]
    );
}

#[test]
fn the_specified_cargo_config_passes() {
    assert_eq!(check_cargo_config(SPEC_CONFIG), Vec::<String>::new());
    let spaced = concat!(
        "\u{20}\u{20}[ alias ]\r\n",
        "xtask\t=\t\"run --package xtask --\"\r\n",
        "\r\n",
        "  # a comment on its own line, indented\n",
        "[target.x86_64-pc-windows-msvc]\n",
        "  rustflags=[ \"-C\" ,\t\"target-feature=+crt-static\" ]  \n",
        "[target.aarch64-pc-windows-msvc]\n",
        "rustflags = [\"-C\", \"target-feature=+crt-static\"]\n",
        "[target . aarch64-unknown-linux-gnu]\n",
        "linker = \"aarch64-linux-gnu-gcc\"",
    );
    assert_eq!(check_cargo_config(spaced), Vec::<String>::new());
}

/// §18.4 changed this expectation (the former whitelist accepted an empty text): an empty file
/// lacks every canonical line.
#[test]
fn empty_cargo_config_lacks_every_canonical_line() {
    for text in ["", "\n\n", "# only a comment\n"] {
        let problems = check_cargo_config(text);
        assert_eq!(problems.len(), CANONICAL_CARGO_CONFIG.len(), "{problems:?}");
        assert!(
            problems.iter().all(|problem| problem
                .starts_with(".cargo/config.toml: the canonical line `")
                && problem.ends_with("is missing (expected at the start of the file)")),
            "{problems:?}"
        );
    }
}

#[test]
fn a_missing_canonical_line_is_reported_with_the_line_it_follows() {
    let text = SPEC_CONFIG.replace("linker = \"aarch64-linux-gnu-gcc\"\n", "");
    assert_eq!(
        check_cargo_config(&text),
        [
            ".cargo/config.toml: the canonical line `linker=\"aarch64-linux-gnu-gcc\"` is missing (expected after line 16)"
        ]
    );
}

#[test]
fn the_live_cargo_config_passes() {
    let root = xtask::locate::workspace_root().unwrap();
    let text = std::fs::read_to_string(root.join(".cargo").join("config.toml")).unwrap();
    assert_eq!(check_cargo_config(&text), Vec::<String>::new());
    assert_eq!(check_cargo_config_files(&root), Vec::<String>::new());
}

#[test]
fn unlisted_tables_are_reported_with_their_line() {
    for header in [
        "[build]",
        "[target.'cfg(windows)']",
        "[target.x86_64-unknown-linux-gnu]",
        "[profile.dev]",
        "[ alias ]",
        "[alias] # aliases",
        "[[target]]",
        "[env]",
    ] {
        let (text, number) = spec_with_line_appended(header);
        assert_single_line_problem(&check_cargo_config(&text), number, header, EXTRA);
        // The former fragment form is still rejected and names the header's line.
        let fragment = format!("[alias]\nxtask = \"run --package xtask --\"\n{header}\n");
        let problems = check_cargo_config(&fragment);
        assert!(
            problems
                .iter()
                .any(|problem| problem.starts_with(".cargo/config.toml:3: `")),
            "{header}: {problems:?}"
        );
    }
}

#[test]
fn build_and_target_key_lines_are_reported() {
    for line in [
        "build.rustflags = [\"-Aunsafe_code\"]",
        "build.target-dir = \"elsewhere\"",
        "target.x86_64-pc-windows-msvc.linker = \"evil\"",
        "build = { jobs = 1 }",
        "\"build\".jobs = 1",
        "'target' = {}",
    ] {
        let text = spec_with_line_inserted(1, line);
        assert_single_line_problem(&check_cargo_config(&text), 1, line, EXTRA);
        let problems = check_cargo_config(&format!("{line}\n"));
        assert!(
            problems
                .iter()
                .any(|problem| problem.starts_with(".cargo/config.toml:1: `")),
            "{line}: {problems:?}"
        );
    }
    // §18.4 changed this expectation: lines the former whitelist accepted as harmless are not in
    // the canonical file, so they are violations too.
    for harmless in ["targets = 1", "builder = 2", "xtask2 = \"build\""] {
        let text = spec_with_line_inserted(SPEC_ALIAS_LINE + 1, harmless);
        assert_single_line_problem(
            &check_cargo_config(&text),
            SPEC_ALIAS_LINE + 1,
            harmless,
            EXTRA,
        );
    }
}

#[test]
fn rustflags_lines_must_be_exactly_crt_static() {
    for line in [
        "rustflags = [\"-C\", \"target-feature=+crt-static\", \"-A\", \"unsafe_code\"]",
        "rustflags = [\"-Aunsafe_code\"]",
        "rustflags = [",
        "rustflags = [\"-C\", \"target-feature=+crt-static\"] # trailing comment",
        "\"rustflags\" = [\"-C\", \"target-feature=+crt-static\"]",
    ] {
        let text = spec_with_line_replaced(SPEC_RUSTFLAGS_LINE, line);
        let problems = check_cargo_config(&text);
        assert_single_line_problem(&problems, SPEC_RUSTFLAGS_LINE, line, CHANGED);
        assert!(
            problems[0].ends_with("`rustflags=[\"-C\",\"target-feature=+crt-static\"]`"),
            "{problems:?}"
        );
    }
}

#[test]
fn rustflags_must_sit_under_a_windows_msvc_table() {
    let line = "rustflags = [\"-C\", \"target-feature=+crt-static\"]";
    // Before every table, right under `[alias]`, and under the aarch64 Linux table.
    for number in [1, SPEC_ALIAS_LINE, 17] {
        let text = spec_with_line_inserted(number, line);
        assert_single_line_problem(&check_cargo_config(&text), number, line, EXTRA);
    }
    // A `[build]` table slipped between a windows-msvc header and its rustflags line.
    let text = spec_with_line_inserted(SPEC_RUSTFLAGS_LINE, "[build]");
    assert_single_line_problem(
        &check_cargo_config(&text),
        SPEC_RUSTFLAGS_LINE,
        "[build]",
        EXTRA,
    );
    // The former fragment forms are still rejected.
    for fragment in [
        format!("{line}\n"),
        format!("[alias]\n{line}\n"),
        format!("[target.aarch64-unknown-linux-gnu]\n{line}\n"),
        format!("[target.x86_64-pc-windows-msvc]\n[build]\n{line}\n"),
    ] {
        assert!(!check_cargo_config(&fragment).is_empty(), "{fragment}");
    }
    // Comments and blank lines between a header and its rustflags line are allowed.
    let ok = spec_with_line_inserted(SPEC_RUSTFLAGS_LINE, "# comment\n\n   ");
    assert_eq!(check_cargo_config(&ok), Vec::<String>::new());
}

#[test]
fn an_alias_smuggling_rustflags_is_reported() {
    let line = "ci = \"--config build.rustflags=['-Aunsafe_code'] xtask ci\"";
    let text = spec_with_line_inserted(SPEC_ALIAS_LINE + 1, line);
    assert_single_line_problem(&check_cargo_config(&text), SPEC_ALIAS_LINE + 1, line, EXTRA);
}

// The §18.4 list: one test each for `include`, an escaped quoted key, `runner`, a modified
// `xtask` alias, a trailing comment, and an extension-less `.cargo/config` (further below).

#[test]
fn canonical_config_rejects_an_include_key() {
    for line in [
        "include = [\"x.toml\"]",
        "include = [{ path = \"x.toml\", optional = true }]",
    ] {
        let text = spec_with_line_inserted(1, line);
        assert_single_line_problem(&check_cargo_config(&text), 1, line, EXTRA);
    }
}

#[test]
fn canonical_config_rejects_an_escaped_quoted_build_rustflags_key() {
    let line = "\"build\".\"rustflags\" = [\"--cap-lints\", \"allow\"]";
    let (text, number) = spec_with_line_appended(line);
    assert_single_line_problem(&check_cargo_config(&text), number, line, EXTRA);
}

#[test]
fn canonical_config_rejects_a_runner_line() {
    let line = "runner = \"wrapper.exe\"";
    let text = spec_with_line_inserted(SPEC_RUSTFLAGS_LINE + 1, line);
    assert_single_line_problem(
        &check_cargo_config(&text),
        SPEC_RUSTFLAGS_LINE + 1,
        line,
        EXTRA,
    );
}

#[test]
fn canonical_config_rejects_a_modified_xtask_alias() {
    for line in [
        "xtask = \"run --package xtask --release --\"",
        "xtask = [\"run\", \"--package\", \"xtask\", \"--\"]",
        "xtask = \"run --package other --\"",
    ] {
        let text = spec_with_line_replaced(SPEC_ALIAS_LINE, line);
        let problems = check_cargo_config(&text);
        assert_single_line_problem(&problems, SPEC_ALIAS_LINE, line, CHANGED);
        assert!(
            problems[0].ends_with("`xtask=\"run--packagextask--\"`"),
            "{problems:?}"
        );
    }
}

#[test]
fn canonical_config_rejects_a_trailing_comment() {
    for (number, line) in [
        (5, "[alias] # aliases"),
        (
            SPEC_ALIAS_LINE,
            "xtask = \"run --package xtask --\" # the task runner",
        ),
        (17, "linker = \"aarch64-linux-gnu-gcc\"#"),
    ] {
        let text = spec_with_line_replaced(number, line);
        assert_single_line_problem(&check_cargo_config(&text), number, line, CHANGED);
    }
}

// ---------------------------------------------------------------------------------------------
// QA audit round 1 (continuation): TOML spellings that cargo reads like the forbidden keys and
// tables, and the file-level part of step 0 (`check_build_configuration`), which had no test.
// ---------------------------------------------------------------------------------------------

/// Returns a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (§17.9): created with
/// `create_dir`, which fails if the path already exists.
fn temp_root(tag: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    dir
}

/// TOML allows whitespace around the dots of a dotted key and quoted segments; cargo reads
/// these lines exactly like `build.jobs`, `target.<triple>.linker` and `build.target-dir`.
#[test]
fn spaced_and_quoted_spellings_of_forbidden_keys_are_reported() {
    for line in [
        "build . jobs = 1",
        "target . \"x86_64-pc-windows-msvc\" . linker = \"x\"",
        "build.\"target-dir\" = \"crates/va-core/src\"",
        "'build'.incremental = false",
    ] {
        let text = spec_with_line_inserted(1, line);
        assert_single_line_problem(&check_cargo_config(&text), 1, line, EXTRA);
        let problems = check_cargo_config(&format!("{line}\n"));
        assert!(
            problems
                .iter()
                .any(|problem| problem.starts_with(".cargo/config.toml:1: `")),
            "{line}: {problems:?}"
        );
    }
}

/// A quoted spelling of a canonical table header is a different text: it fails closed, in place
/// or added. §18.4 removes all whitespace before comparing, so the spaced spelling
/// `[target . x86_64-pc-windows-msvc]` now equals the canonical header in place (cargo reads it
/// as the same table); added as a second table, it is reported like any other extra line.
#[test]
fn equivalent_spellings_of_allowed_headers_fail_closed() {
    let quoted = "[target.\"x86_64-pc-windows-msvc\"]";
    let text = spec_with_line_replaced(9, quoted);
    assert_single_line_problem(&check_cargo_config(&text), 9, quoted, CHANGED);
    let spaced = "[target . x86_64-pc-windows-msvc]";
    assert_eq!(
        check_cargo_config(&spec_with_line_replaced(9, spaced)),
        Vec::<String>::new()
    );
    for header in [quoted, spaced, "[unstable]", "[registries.x]"] {
        let (text, number) = spec_with_line_appended(header);
        assert_single_line_problem(&check_cargo_config(&text), number, header, EXTRA);
        // Alone, each fragment is rejected (the spaced header aligns with the canonical header,
        // so the problems are the canonical lines it lacks).
        assert!(
            !check_cargo_config(&format!("{header}\n")).is_empty(),
            "{header}"
        );
    }
}

/// Writes `.cargo/config.toml` (when `config` is given) under a fresh root and returns the root.
fn root_with_config(tag: &str, config: Option<&[u8]>) -> PathBuf {
    let root = temp_root(tag);
    std::fs::create_dir(root.join(".cargo")).unwrap();
    if let Some(bytes) = config {
        std::fs::write(root.join(".cargo").join("config.toml"), bytes).unwrap();
    }
    root
}

/// Cargo reads the extension-less `.cargo/config` instead of `.cargo/config.toml` when both
/// exist, so step 0 must reject it even when `config.toml` is clean (§18.4).
#[test]
fn extension_less_cargo_config_next_to_a_clean_config_toml_is_reported() {
    let root = root_with_config("ci-legacy-config", Some(SPEC_CONFIG.as_bytes()));
    std::fs::write(
        root.join(".cargo").join("config"),
        "[build]\nrustflags = [\"-Aunsafe_code\"]\n",
    )
    .unwrap();
    let problems = check_cargo_config_files(&root);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with(".cargo/config exists"),
        "{problems:?}"
    );
    assert!(
        check_build_configuration(&root)
            .iter()
            .any(|problem| problem.starts_with(".cargo/config exists")),
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// §18.4 changed the form, not the outcome: a missing `.cargo/config.toml` was an error and is now
/// a reported problem; either way step 0 fails, it never passes.
#[test]
fn missing_cargo_config_toml_is_a_violation_not_a_pass() {
    let root = temp_root("ci-missing-config");
    let problems = check_cargo_config_files(&root);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with(".cargo/config.toml cannot be read ("),
        "{problems:?}"
    );
    assert!(!check_build_configuration(&root).is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}

/// An unreadable `.cargo/config.toml` (not UTF-8, or a folder) is a problem too (§18.4).
#[test]
fn unreadable_cargo_config_toml_is_a_violation() {
    let root = root_with_config("ci-non-utf8-config", Some(b"[alias]\n\xff\xfe = 1\n"));
    let problems = check_cargo_config_files(&root);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with(".cargo/config.toml cannot be read ("),
        "{problems:?}"
    );
    std::fs::remove_dir_all(&root).unwrap();

    let root = root_with_config("ci-folder-config", None);
    std::fs::create_dir(root.join(".cargo").join("config.toml")).unwrap();
    let problems = check_cargo_config_files(&root);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with(".cargo/config.toml cannot be read ("),
        "{problems:?}"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn canonical_cargo_config_files_pass() {
    let root = root_with_config("ci-canonical-config", Some(SPEC_CONFIG.as_bytes()));
    assert_eq!(check_cargo_config_files(&root), Vec::<String>::new());
    std::fs::remove_dir_all(&root).unwrap();
}

// ---------------------------------------------------------------------------------------------
// The real `cargo xtask ci` (row 18-4 acceptance): a set rustflags-type variable, even empty,
// stops the command at step 0 with exit code 1, before any cargo step runs.
// ---------------------------------------------------------------------------------------------

/// Runs the built xtask binary as `xtask ci --no-fmt` with `name=value` added to the
/// environment, and returns its exit code and stderr. Step 0 fails before any cargo command,
/// so this never builds or tests anything.
fn ci_with_variable(name: &str, value: &str) -> (Option<i32>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["ci", "--no-fmt"])
        .env(name, value)
        .output()
        .unwrap();
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn ci_stops_at_step_0_when_a_rustflags_variable_is_set_even_empty() {
    for (name, value) in [
        ("RUSTFLAGS", ""),
        ("RUSTFLAGS", "-Aunsafe_code"),
        ("CARGO_ENCODED_RUSTFLAGS", ""),
        ("CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS", ""),
    ] {
        let (code, stderr) = ci_with_variable(name, value);
        assert_eq!(code, Some(1), "{name}={value:?}: {stderr}");
        assert!(
            stderr.starts_with("==> build configuration\n")
                || stderr.starts_with("==> build configuration\r\n"),
            "{name}={value:?}: {stderr}"
        );
        assert!(
            stderr.contains(&format!(
                "{name} is set: it replaces .cargo/config.toml (+crt-static) and can silence lints; unset it"
            )),
            "{name}={value:?}: {stderr}"
        );
        assert_eq!(
            stderr.matches("==> ").count(),
            1,
            "{name}={value:?}: a later step ran: {stderr}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// QA, final M0 pass (after §18): values and spellings a shell profile or an editor produces.
// ---------------------------------------------------------------------------------------------

/// A value made of whitespace only is still a set variable: cargo reads it as an (empty)
/// override and drops `+crt-static`, exactly like the empty value of §18.3.
#[test]
fn whitespace_only_rustflags_values_are_rejected() {
    for value in [" ", "\t", "  \t "] {
        for name in [
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_BUILD_RUSTFLAGS",
        ] {
            let problems = check_build_env([(name, value)]);
            assert_eq!(problems.len(), 1, "{name}={value:?}: {problems:?}");
            assert!(
                problems[0].starts_with(&format!("{name} is set: ")),
                "{problems:?}"
            );
        }
    }
}

/// Every target triple the project builds for, in the spelling cargo documents
/// (`CARGO_TARGET_<TRIPLE>_RUSTFLAGS`, dashes as underscores), plus a lower-case spelling.
#[test]
fn per_target_rustflags_variables_of_every_project_triple_are_rejected() {
    for name in [
        "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS",
        "CARGO_TARGET_AARCH64_PC_WINDOWS_MSVC_RUSTFLAGS",
        "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
        "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
        "cargo_target_x86_64_unknown_linux_gnu_rustflags",
    ] {
        assert!(replaces_config_rustflags(name), "{name}");
        assert_eq!(check_build_env([(name, "")]).len(), 1, "{name}");
    }
}

/// A TOML formatter (taplo, an editor's "format document") may expand the rustflags array over
/// several lines. Cargo reads the same value, but §18.4 compares line for line: the file must be
/// reported, never accepted, and the first problem names the line that opens the array.
#[test]
fn formatter_expanded_rustflags_array_is_reported_not_accepted() {
    let expanded = SPEC_CONFIG.replacen(
        "rustflags = [\"-C\", \"target-feature=+crt-static\"]",
        "rustflags = [\n    \"-C\",\n    \"target-feature=+crt-static\",\n]",
        1,
    );
    let problems = check_cargo_config(&expanded);
    assert!(!problems.is_empty(), "{expanded}");
    assert!(
        problems[0].starts_with(&format!(
            ".cargo/config.toml:{SPEC_RUSTFLAGS_LINE}: `rustflags = [`"
        )),
        "{problems:?}"
    );
}

/// A canonical line duplicated (a copy-paste slip) or two canonical blocks swapped is reported:
/// the comparison is in order, line for line.
#[test]
fn duplicated_or_reordered_canonical_lines_are_reported() {
    let (duplicated, number) =
        spec_with_line_appended("rustflags = [\"-C\", \"target-feature=+crt-static\"]");
    assert_single_line_problem(
        &check_cargo_config(&duplicated),
        number,
        "rustflags = [\"-C\", \"target-feature=+crt-static\"]",
        EXTRA,
    );
    let swapped = SPEC_CONFIG
        .replace("[alias]\nxtask = \"run --package xtask --\"\n", "")
        .replace(
            "linker = \"aarch64-linux-gnu-gcc\"\n",
            "linker = \"aarch64-linux-gnu-gcc\"\n[alias]\nxtask = \"run --package xtask --\"\n",
        );
    assert!(!check_cargo_config(&swapped).is_empty(), "{swapped}");
}

/// Row 18-4 with the other variable families and spellings: the real `cargo xtask ci` stops at
/// step 0 with exit code 1 for each of them (names compare ignoring case on every OS, §18.3).
#[test]
fn ci_stops_at_step_0_for_every_variable_family_and_spelling() {
    for (name, value) in [
        ("CARGO_BUILD_RUSTFLAGS", ""),
        ("RUSTFLAGS", " "),
        ("rustflags", ""),
        (
            "CARGO_TARGET_AARCH64_PC_WINDOWS_MSVC_RUSTFLAGS",
            "-Ctarget-feature=-crt-static",
        ),
        ("cargo_target_x86_64_unknown_linux_gnu_rustflags", ""),
    ] {
        let (code, stderr) = ci_with_variable(name, value);
        assert_eq!(code, Some(1), "{name}={value:?}: {stderr}");
        assert!(
            stderr.contains(&format!("{name} is set: ")),
            "{name}={value:?}: {stderr}"
        );
        assert_eq!(
            stderr.matches("==> ").count(),
            1,
            "{name}={value:?}: a later step ran: {stderr}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// QA, round post-measure: a UTF-8 byte-order mark, which Windows editors may write. Cargo 1.99
// reads `.cargo/config.toml` with a leading BOM exactly like the same file without it (checked
// with `cargo <alias>` on a scratch project). Whatever step 0 does with the mark itself, it must
// never hide a forbidden line that follows it.
// ---------------------------------------------------------------------------------------------

/// The UTF-8 byte-order mark.
const BOM: &str = "\u{feff}";

#[test]
fn a_byte_order_mark_never_hides_a_forbidden_line() {
    // A forbidden line deeper in the file keeps its own report.
    let runner = "runner = \"x\"";
    let number = SPEC_ALIAS_LINE + 1;
    let text = format!("{BOM}{}", spec_with_line_inserted(number, runner));
    let problems = check_cargo_config(&text);
    assert!(
        problems.iter().any(|problem| problem
            .starts_with(&format!(".cargo/config.toml:{number}: `{runner}` {EXTRA}"))),
        "{problems:?}"
    );

    // A forbidden table right after the mark, on line 1, is reported, with its key line.
    let text = format!("{BOM}[build]\nrustflags = [\"-Aunsafe_code\"]\n{SPEC_CONFIG}");
    let problems = check_cargo_config(&text);
    for line in [1, 2] {
        assert!(
            problems
                .iter()
                .any(|problem| problem.starts_with(&format!(".cargo/config.toml:{line}: `"))),
            "line {line}: {problems:?}"
        );
    }

    // The same on disk, through the file-level check that step 0 runs.
    let root = root_with_config("ci-bom-config", Some(text.as_bytes()));
    let problems = check_cargo_config_files(&root);
    assert!(
        problems
            .iter()
            .any(|problem| problem.starts_with(".cargo/config.toml:2: `rustflags")),
        "{problems:?}"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

// ---------------------------------------------------------------------------------------------
// QA (§19.9 re-check, row 19-9 a, CR19-06 and S19-5): `cargo xtask ci` is what the `check` job of
// `.github/workflows/ci.yml` runs, and that file is normative. Its second Windows smoke step is
// exactly the §19.9 block, the runner expression stays in `env:` and never reaches a `run:`
// value, and every action stays pinned to a full commit SHA.
// ---------------------------------------------------------------------------------------------

/// The `ci.yml` block of m0-foundations.md §19.9.6 (row 19-9 a), verbatim.
const FALLBACK_SMOKE_STEP: &str = "      # Section 19.9 (CR19-06): fail the first renderer of this runner's auto order on purpose, so\n      # the log documents a fallback on both images (x64: glow then wgpu; ARM64: wgpu then glow).\n      - name: GUI renderer fallback smoke (informational, never a support claim)\n        if: runner.os == 'Windows'\n        continue-on-error: true\n        timeout-minutes: 3\n        shell: bash\n        env:\n          VA_TEST_FAIL_RENDERER: ${{ runner.arch == 'ARM64' && 'wgpu' || 'glow' }}\n        run: ./target/release/visual-assembler.exe --bench startup\n";

/// The repository's workflow file.
fn workflow() -> String {
    let root = xtask::locate::workspace_root().unwrap();
    std::fs::read_to_string(root.join(".github").join("workflows").join("ci.yml")).unwrap()
}

/// Every `run:` value of a workflow, a `|` or `>` block scalar joined into one string (its lines
/// are those indented deeper than the `run:` key).
fn run_values(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut block: Option<(usize, String)> = None;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        if let Some((key_indent, body)) = block.as_mut() {
            if line.trim().is_empty() || indent > *key_indent {
                body.push_str(line.trim());
                body.push('\n');
                continue;
            }
            values.push(std::mem::take(body));
            block = None;
        }
        let item = line.trim_start().trim_start_matches("- ");
        if let Some(value) = item.strip_prefix("run:") {
            let value = value.trim();
            if value.starts_with('|') || value.starts_with('>') {
                block = Some((line.len() - item.len(), String::new()));
            } else {
                values.push(value.to_owned());
            }
        }
    }
    values.extend(block.map(|(_, body)| body));
    values
}

/// CR19-06: the second smoke step is the §19.9 block, once, and it is the only place that arms
/// the test hook (the first smoke step stays a plain `auto` startup).
#[test]
fn renderer_fallback_smoke_step_is_the_spec_block() {
    let text = workflow();
    assert_eq!(
        text.matches(FALLBACK_SMOKE_STEP).count(),
        1,
        "ci.yml lacks the §19.9 block:\n{text}"
    );
    assert_eq!(text.matches("VA_TEST_FAIL_RENDERER").count(), 1);
    assert_eq!(
        text.matches("run: ./target/release/visual-assembler.exe --bench startup\n")
            .count(),
        2
    );
}

/// S19-5 and §8: no `run:` value holds a `${{ … }}` expression (the block scalar of the deny job
/// included), and every `uses:` line names `owner/repo@<40 lowercase hex>`.
#[test]
fn workflow_run_values_hold_no_expression_and_actions_are_sha_pinned() {
    let text = workflow();
    let runs = run_values(&text);
    assert!(
        runs.iter()
            .any(|run| run.contains("cargo install cargo-deny") && run.contains("GITHUB_PATH")),
        "the block scalar of the deny job was not read: {runs:?}"
    );
    assert!(runs.len() >= 8, "{runs:?}");
    for run in &runs {
        assert!(!run.contains("${{"), "expression inside run: {run:?}");
    }

    let mut pinned = 0;
    for line in text.lines() {
        let item = line.trim_start().trim_start_matches("- ");
        let Some(value) = item.strip_prefix("uses:") else {
            continue;
        };
        let reference = value.split_whitespace().next().unwrap_or_default();
        let (action, sha) = reference.split_once('@').unwrap_or_default();
        assert!(action.contains('/'), "{line}");
        assert!(
            sha.len() == 40
                && sha
                    .chars()
                    .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "not pinned to a commit SHA: {line}"
        );
        pinned += 1;
    }
    assert!(pinned >= 4, "{pinned} uses: lines");
}

// ---------------------------------------------------------------------------------------------
// QA (m0 §20, Security S20-1): every `runs-on` value and matrix entry of a workflow names one OS
// release, with no `-latest` label and no exception (§20.1, §20.2). The required check names
// guard only the `check` matrix labels (§20.3); these tests also guard `fmt`, `docs` and `deny`.
// Each check is a function of the workflow text, so the mutation tests feed it in-memory copies
// of `ci.yml` and never edit the real file.
// ---------------------------------------------------------------------------------------------

/// The four runner labels of m0 §20.2, in the order of the `check` matrix.
const SECTION_20_LABELS: [&str; 4] = [
    "windows-2025",
    "windows-11-arm",
    "ubuntu-26.04",
    "ubuntu-24.04-arm",
];

/// The `runs-on` value of the `check` job (§8, kept by §20).
const MATRIX_RUNNER: &str = "${{ matrix.os }}";

/// `line` without its YAML comment and trailing whitespace. A `#` starts a comment at the start
/// of the line or after whitespace, outside a quoted scalar (a quote opens one only at the start
/// of a value, so the apostrophe of `runner's` does not).
fn without_comment(line: &str) -> &str {
    let mut quote = None;
    let mut previous = ' ';
    for (at, c) in line.char_indices() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None if (c == '"' || c == '\'') && " \t[{,:".contains(previous) => quote = Some(c),
            None if c == '#' && previous.is_whitespace() => return line[..at].trim_end(),
            None => {}
        }
        previous = c;
    }
    line.trim_end()
}

/// A line of a workflow that YAML reads (blank and comment-only lines are left out).
struct WorkflowLine<'a> {
    /// 1-based line number in the file.
    number: usize,
    /// Leading spaces of the line.
    indent: usize,
    /// Column of the line's first key, past any `- ` list markers.
    key_indent: usize,
    /// The line from that column on, without its comment.
    item: &'a str,
}

/// The lines of `text` that YAML reads, comments dropped.
fn workflow_lines(text: &str) -> Vec<WorkflowLine<'_>> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let code = without_comment(line);
            let content = code.trim_start();
            if content.is_empty() {
                return None;
            }
            let mut item = content;
            while let Some(rest) = item
                .strip_prefix('-')
                .filter(|rest| rest.is_empty() || rest.starts_with([' ', '\t']))
            {
                item = rest.trim_start();
            }
            Some(WorkflowLine {
                number: index + 1,
                indent: code.len() - content.len(),
                key_indent: code.len() - item.len(),
                item,
            })
        })
        .collect()
}

/// The value of `key` when it is the first key of `line`: the rest of the item, trimmed (empty
/// for a block value).
fn key_value<'a>(line: &WorkflowLine<'a>, key: &str) -> Option<&'a str> {
    let rest = line.item.strip_prefix(key)?.strip_prefix(':')?;
    (rest.is_empty() || rest.starts_with([' ', '\t'])).then(|| rest.trim())
}

/// `value` without one pair of surrounding YAML quotes.
fn scalar(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}

/// End (exclusive index into `lines`) of the block under the first key of `lines[at]`: the
/// following lines indented deeper than that key, plus the `- ` entries of a block sequence
/// written at the key's own indent (`os:` then `- windows-2025` in the same column).
fn block_end(lines: &[WorkflowLine<'_>], at: usize) -> usize {
    let key = lines[at].key_indent;
    lines[at + 1..]
        .iter()
        .position(|line| {
            line.indent < key || (line.indent == key && line.key_indent == line.indent)
        })
        .map_or(lines.len(), |offset| at + 1 + offset)
}

/// The jobs of a workflow as `(id, lines)`: each key of the top-level `jobs:` block, with the
/// lines of its own block.
fn workflow_jobs<'b, 'a>(lines: &'b [WorkflowLine<'a>]) -> Vec<(&'a str, &'b [WorkflowLine<'a>])> {
    let Some(start) = lines
        .iter()
        .position(|line| line.indent == 0 && key_value(line, "jobs") == Some(""))
    else {
        return Vec::new();
    };
    let end = block_end(lines, start);
    let mut jobs = Vec::new();
    let mut at = start + 1;
    while at < end {
        let job_end = block_end(lines, at).min(end);
        let id = lines[at].item.split(':').next().unwrap_or_default();
        jobs.push((id, &lines[at + 1..job_end]));
        at = job_end;
    }
    jobs
}

/// The `runs-on` values of `job` (unquoted), in file order; empty when the job is missing.
fn job_runners<'a>(jobs: &[(&'a str, &[WorkflowLine<'a>])], job: &str) -> Vec<&'a str> {
    jobs.iter()
        .filter(|(id, _)| *id == job)
        .flat_map(|(_, block)| block.iter())
        .filter_map(|line| key_value(line, "runs-on").map(scalar))
        .collect()
}

/// m0 §20.2 (a): every `runs-on` value of the `ci.yml` text that is neither one of the four §20
/// labels nor `${{ matrix.os }}` in the `check` job, one problem each. It fails closed: a line
/// that mentions `runs-on` other than as the first key of a job line (a flow mapping, a key
/// outside `jobs:`), or no `runs-on` key at all, is a problem too.
fn runs_on_problems(text: &str) -> Vec<String> {
    let lines = workflow_lines(text);
    let mut problems = Vec::new();
    let mut keys = 0;
    for (job, block) in workflow_jobs(&lines) {
        for line in block {
            let Some(value) = key_value(line, "runs-on").map(scalar) else {
                continue;
            };
            keys += 1;
            let allowed =
                SECTION_20_LABELS.contains(&value) || (job == "check" && value == MATRIX_RUNNER);
            if !allowed {
                problems.push(format!(
                    "ci.yml:{}: job `{job}` has `runs-on: {value}`, which is neither a m0 §20 label ({}) nor `{MATRIX_RUNNER}` in job `check`",
                    line.number,
                    SECTION_20_LABELS.join(", ")
                ));
            }
        }
    }
    let mentions = lines
        .iter()
        .filter(|line| line.item.contains("runs-on"))
        .count();
    if mentions != keys {
        problems.push(format!(
            "ci.yml: {mentions} lines mention `runs-on` but {keys} are `runs-on:` keys of a job (m0 §20: one plain label per key)"
        ));
    }
    if keys == 0 {
        problems.push("ci.yml: no `runs-on:` key was read (m0 §20)".to_owned());
    }
    problems
}

/// The labels of the `check` matrix, given its `matrix:` key line and that key's block, or a
/// problem when the matrix holds anything but one `os` key with a list (`[…]` or a block
/// sequence): another key such as `include` could add a runner the `os` list does not show.
fn matrix_os(matrix: &WorkflowLine<'_>, block: &[WorkflowLine<'_>]) -> Result<Vec<String>, String> {
    let shape = |line: &WorkflowLine<'_>| {
        format!(
            "ci.yml:{}: `{}` in the `check` matrix: m0 §20.2 allows only the `os` list",
            line.number, line.item
        )
    };
    if key_value(matrix, "matrix") != Some("") {
        return Err(shape(matrix));
    }
    let Some((os, entries)) = block.split_first() else {
        return Err(shape(matrix));
    };
    let value = key_value(os, "os").ok_or_else(|| shape(os))?;
    if let Some(list) = value
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    {
        if let Some(extra) = entries.first() {
            return Err(shape(extra));
        }
        return Ok(list
            .split(',')
            .map(|label| scalar(label.trim()).to_owned())
            .collect());
    }
    if !value.is_empty() {
        return Err(shape(os));
    }
    entries
        .iter()
        .map(|entry| {
            if entry.key_indent > entry.indent && !entry.item.contains(':') {
                Ok(scalar(entry.item).to_owned())
            } else {
                Err(shape(entry))
            }
        })
        .collect()
}

/// m0 §20.2 (b): the `check` matrix of the `ci.yml` text is exactly one `os` list equal to
/// [`SECTION_20_LABELS`], in that order; one problem otherwise.
fn matrix_problems(text: &str) -> Vec<String> {
    let lines = workflow_lines(text);
    let jobs = workflow_jobs(&lines);
    let Some((_, check)) = jobs.iter().find(|(id, _)| *id == "check") else {
        return vec!["ci.yml has no `check` job (m0 §20.2)".to_owned()];
    };
    let matrices: Vec<usize> = (0..check.len())
        .filter(|&at| key_value(&check[at], "matrix").is_some())
        .collect();
    let &[at] = matrices.as_slice() else {
        return vec![format!(
            "ci.yml: job `check` has {} `matrix:` keys; m0 §20.2 requires one",
            matrices.len()
        )];
    };
    match matrix_os(&check[at], &check[at + 1..block_end(check, at)]) {
        Err(problem) => vec![problem],
        Ok(labels) if labels == SECTION_20_LABELS => Vec::new(),
        // `matrix_os` returns labels only when the block's first line, `check[at + 1]`, is `os:`.
        Ok(labels) => vec![format!(
            "ci.yml:{}: the `check` matrix `os` is {labels:?}; m0 §20.2 requires exactly {SECTION_20_LABELS:?}, in this order",
            check[at + 1].number
        )],
    }
}

/// m0 §20.2 (c): `fmt`, `docs` and `deny` each have exactly one `runs-on: ubuntu-26.04`, and
/// `check` exactly one `runs-on: ${{ matrix.os }}`; one problem per job that differs (a missing
/// job or `runs-on` key included).
fn job_runner_problems(text: &str) -> Vec<String> {
    let lines = workflow_lines(text);
    let jobs = workflow_jobs(&lines);
    [
        ("fmt", "ubuntu-26.04"),
        ("docs", "ubuntu-26.04"),
        ("deny", "ubuntu-26.04"),
        ("check", MATRIX_RUNNER),
    ]
    .into_iter()
    .filter_map(|(job, expected)| {
        let runners = job_runners(&jobs, job);
        (runners != [expected]).then(|| {
            format!(
                "ci.yml: job `{job}` runs on {runners:?}; m0 §20.2 requires exactly [{expected:?}]"
            )
        })
    })
    .collect()
}

/// m0 §20.1 (d): every line of the workflow `file` (its `text`) that holds a `-latest` label in
/// a runner position: the value of a `runs-on:` or `matrix:` key, inline (a flow mapping
/// included) or in the block under the line's first key. Comments are ignored; letter case is
/// not, since GitHub matches runner labels ignoring case.
fn latest_label_problems(file: &str, text: &str) -> Vec<String> {
    let lines = workflow_lines(text);
    let mut in_scope = vec![false; lines.len()];
    for (at, line) in lines.iter().enumerate() {
        if ["runs-on:", "matrix:"]
            .iter()
            .any(|key| line.item.contains(key))
        {
            in_scope[at..block_end(&lines, at)].fill(true);
        }
    }
    lines
        .iter()
        .zip(in_scope)
        .filter(|(line, scoped)| *scoped && line.item.to_ascii_lowercase().contains("-latest"))
        .map(|(line, _)| {
            format!(
                "{file}:{}: `{}` names a `-latest` runner label; m0 §20 requires one OS release per label",
                line.number, line.item
            )
        })
        .collect()
}

/// Fails with every problem, one per line, under a heading that names m0 §20.
fn assert_section_20(problems: &[String], what: &str) {
    assert!(
        problems.is_empty(),
        "m0 §20: {what}:\n{}",
        problems.join("\n")
    );
}

/// m0 §20.2 (a): every `runs-on` value of `ci.yml` is one of the four §20 labels, or exactly
/// `${{ matrix.os }}` in the `check` job.
#[test]
fn every_runs_on_value_is_a_section_20_label() {
    assert_section_20(
        &runs_on_problems(&workflow()),
        "a `runs-on` value of ci.yml is not a §20.2 label",
    );
}

/// m0 §20.2 (b): the `check` matrix is exactly `os: [windows-2025, windows-11-arm,
/// ubuntu-26.04, ubuntu-24.04-arm]`, in that order, with no other matrix key.
#[test]
fn check_matrix_is_the_section_20_label_list_in_order() {
    assert_section_20(
        &matrix_problems(&workflow()),
        "the check matrix of ci.yml is not the §20.2 list",
    );
}

/// m0 §20.2 (c): `fmt`, `docs` and `deny` run on `ubuntu-26.04` (their labels are not part of
/// a required check name, S20-1), and `check` on its matrix label.
#[test]
fn fmt_docs_and_deny_run_on_ubuntu_26_04() {
    assert_section_20(
        &job_runner_problems(&workflow()),
        "a job of ci.yml runs on another label than §20.2 gives it",
    );
}

/// m0 §20.1 (d): no workflow under `.github/workflows/` (`.yml` or `.yaml`, the two extensions
/// GitHub reads) has a `-latest` label in a `runs-on` value or a matrix entry.
#[test]
fn no_workflow_uses_a_latest_runner_label() {
    let folder = xtask::locate::workspace_root()
        .unwrap()
        .join(".github")
        .join("workflows");
    let mut files = Vec::new();
    let mut problems = Vec::new();
    for entry in std::fs::read_dir(&folder).unwrap() {
        let path = entry.unwrap().path();
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if extension != "yml" && extension != "yaml" {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let text = std::fs::read_to_string(&path).unwrap();
        problems.extend(latest_label_problems(&name, &text));
        files.push(name);
    }
    assert!(
        files.iter().any(|name| name == "ci.yml"),
        "m0 §20: ci.yml was not read from {folder:?}: {files:?}"
    );
    assert_section_20(&problems, "a workflow uses a `-latest` runner label");
}

// Mutation tests: each check above reports an in-memory copy of `ci.yml` changed the way a
// slipped edit would change it, so none of them can pass vacuously.

/// `text` with the first `from` replaced by `to`; asserts that `from` occurs, so a mutation never
/// silently tests the unchanged file.
fn mutated(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "mutation target not in ci.yml: {from:?}"
    );
    text.replacen(from, to, 1)
}

/// 1-based number of the first line of `text` that contains `needle`.
fn line_of(text: &str, needle: &str) -> usize {
    text.lines().position(|line| line.contains(needle)).unwrap() + 1
}

/// Asserts that `problems` (for the mutation `case`) is not empty, names m0 §20 in every
/// problem, and mentions `fragment` in one of them.
fn assert_reported(problems: &[String], case: &str, fragment: &str) {
    assert!(!problems.is_empty(), "{case}: not reported");
    assert!(
        problems.iter().all(|problem| problem.contains("m0 §20")),
        "{case}: {problems:?}"
    );
    assert!(
        problems.iter().any(|problem| problem.contains(fragment)),
        "{case}: no problem mentions {fragment:?}: {problems:?}"
    );
}

/// The `runs-on` line of the `fmt` job, with its job key.
const FMT_RUNNER: &str = "  fmt:\n    runs-on: ubuntu-26.04\n";

/// The `runs-on` line of the `docs` job, with its job key.
const DOCS_RUNNER: &str = "  docs:\n    runs-on: ubuntu-26.04\n";

/// The `runs-on` line of the `deny` job, with its job key.
const DENY_RUNNER: &str = "  deny:\n    runs-on: ubuntu-26.04\n";

/// The `os:` line of the `check` matrix (m0 §20.2).
const OS_LINE: &str =
    "        os: [windows-2025, windows-11-arm, ubuntu-26.04, ubuntu-24.04-arm]\n";

/// Check (a) on mutated copies: a `-latest` label, a release outside §20, a label list, a block
/// value, an expression, the matrix expression outside `check`, a flow mapping.
#[test]
fn runs_on_check_reports_a_label_outside_section_20() {
    let text = workflow();
    for (case, from, to, fragment) in [
        (
            "fmt on ubuntu-latest",
            FMT_RUNNER,
            "  fmt:\n    runs-on: ubuntu-latest\n",
            "job `fmt` has `runs-on: ubuntu-latest`",
        ),
        (
            "docs on ubuntu-24.04, one release but not a §20 label",
            DOCS_RUNNER,
            "  docs:\n    runs-on: ubuntu-24.04\n",
            "job `docs` has `runs-on: ubuntu-24.04`",
        ),
        (
            "deny on a label list",
            DENY_RUNNER,
            "  deny:\n    runs-on: [self-hosted, ubuntu-26.04]\n",
            "job `deny` has `runs-on: [self-hosted, ubuntu-26.04]`",
        ),
        (
            "deny on a block value",
            DENY_RUNNER,
            "  deny:\n    runs-on:\n      - ubuntu-latest\n",
            "job `deny` has `runs-on: `",
        ),
        (
            "check on a repository variable",
            "runs-on: ${{ matrix.os }}",
            "runs-on: ${{ vars.CHECK_RUNNER }}",
            "job `check` has `runs-on: ${{ vars.CHECK_RUNNER }}`",
        ),
        (
            "fmt on the matrix expression",
            FMT_RUNNER,
            "  fmt:\n    runs-on: ${{ matrix.os }}\n",
            "job `fmt` has `runs-on: ${{ matrix.os }}`",
        ),
        (
            "a runs-on inside a flow mapping",
            FMT_RUNNER,
            "  fmt:\n    runs-on: ubuntu-26.04\n    defaults: { runs-on: ubuntu-latest }\n",
            "lines mention `runs-on` but",
        ),
    ] {
        assert_reported(&runs_on_problems(&mutated(&text, from, to)), case, fragment);
    }
    // The problem names the line of the value.
    let copy = mutated(&text, FMT_RUNNER, "  fmt:\n    runs-on: ubuntu-latest\n");
    let problems = runs_on_problems(&copy);
    let line = line_of(&copy, "runs-on: ubuntu-latest");
    assert!(
        problems[0].starts_with(&format!("ci.yml:{line}: ")),
        "{problems:?}"
    );
    // A quoted label is the same YAML value.
    let quoted = mutated(&text, FMT_RUNNER, "  fmt:\n    runs-on: \"ubuntu-26.04\"\n");
    assert_eq!(runs_on_problems(&quoted), Vec::<String>::new());
}

/// Check (b) on mutated copies: a `-latest` label, a new order, a label added or dropped, an
/// `include` entry, a block sequence, an expression. The same labels as a block sequence pass.
#[test]
fn matrix_check_reports_a_changed_list_or_an_extra_key() {
    let text = workflow();
    for (case, to, fragment) in [
        (
            "windows-latest back in the list",
            "        os: [windows-latest, windows-11-arm, ubuntu-26.04, ubuntu-24.04-arm]\n",
            "is [\"windows-latest\", \"windows-11-arm\"",
        ),
        (
            "two labels swapped",
            "        os: [windows-11-arm, windows-2025, ubuntu-26.04, ubuntu-24.04-arm]\n",
            "is [\"windows-11-arm\", \"windows-2025\"",
        ),
        (
            "a fifth label",
            "        os: [windows-2025, windows-11-arm, ubuntu-26.04, ubuntu-24.04-arm, macos-15]\n",
            "\"macos-15\"]",
        ),
        (
            "a label dropped",
            "        os: [windows-2025, windows-11-arm, ubuntu-26.04]\n",
            "is [\"windows-2025\", \"windows-11-arm\", \"ubuntu-26.04\"];",
        ),
        (
            "an include entry beside the list",
            "        os: [windows-2025, windows-11-arm, ubuntu-26.04, ubuntu-24.04-arm]\n        include:\n          - os: ubuntu-latest\n",
            "`include:` in the `check` matrix",
        ),
        (
            "a block sequence with a -latest label",
            "        os:\n        - windows-2025\n        - windows-11-arm\n        - ubuntu-latest\n        - ubuntu-24.04-arm\n",
            "\"ubuntu-latest\"",
        ),
        (
            "the matrix from an expression",
            "        os: ${{ fromJSON(vars.RUNNERS) }}\n",
            "`os: ${{ fromJSON(vars.RUNNERS) }}` in the `check` matrix",
        ),
    ] {
        assert_reported(
            &matrix_problems(&mutated(&text, OS_LINE, to)),
            case,
            fragment,
        );
    }
    // The problem names the line of the `os` list.
    let copy = mutated(
        &text,
        OS_LINE,
        "        os: [windows-latest, windows-11-arm, ubuntu-26.04, ubuntu-24.04-arm]\n",
    );
    let problems = matrix_problems(&copy);
    let line = line_of(&copy, "os: [windows-latest");
    assert!(
        problems[0].starts_with(&format!("ci.yml:{line}: ")),
        "{problems:?}"
    );
    // The same labels as a block sequence are the same value: the check reads values, not layout.
    let block = mutated(
        &text,
        OS_LINE,
        "        os:\n          - windows-2025\n          - windows-11-arm\n          - ubuntu-26.04\n          - ubuntu-24.04-arm\n",
    );
    assert_eq!(matrix_problems(&block), Vec::<String>::new());
}

/// Check (c) on mutated copies: `fmt` or `docs` moved to another §20 label (which check (a)
/// accepts), `deny` without `runs-on`, `check` on one fixed label.
#[test]
fn job_runner_check_reports_a_job_on_another_label() {
    let text = workflow();
    for (case, from, to, fragment) in [
        (
            "fmt moved to another §20 label",
            FMT_RUNNER,
            "  fmt:\n    runs-on: windows-2025\n",
            "job `fmt` runs on [\"windows-2025\"]",
        ),
        (
            "docs moved to the ARM64 label",
            DOCS_RUNNER,
            "  docs:\n    runs-on: ubuntu-24.04-arm\n",
            "job `docs` runs on [\"ubuntu-24.04-arm\"]",
        ),
        (
            "deny without runs-on",
            DENY_RUNNER,
            "  deny:\n",
            "job `deny` runs on []",
        ),
        (
            "check on one fixed label",
            "runs-on: ${{ matrix.os }}",
            "runs-on: ubuntu-26.04",
            "job `check` runs on [\"ubuntu-26.04\"]",
        ),
    ] {
        let copy = mutated(&text, from, to);
        assert_reported(&job_runner_problems(&copy), case, fragment);
    }
    // Moving `fmt` to another §20 label passes check (a): only check (c) pins the job's label.
    let moved = mutated(&text, FMT_RUNNER, "  fmt:\n    runs-on: windows-2025\n");
    assert_eq!(runs_on_problems(&moved), Vec::<String>::new());
}

/// Check (d) on mutated copies and on another workflow: `-latest` in a `runs-on` value (any
/// case, quoted, a block sequence) or a matrix entry (the list, an `include`, a flow mapping)
/// is reported; a comment or a `run:` value that mentions `-latest` is not.
#[test]
fn latest_label_check_reports_runner_positions_and_ignores_comments() {
    let text = workflow();
    for (case, from, to, fragment) in [
        (
            "fmt on ubuntu-latest",
            FMT_RUNNER,
            "  fmt:\n    runs-on: ubuntu-latest\n",
            "`runs-on: ubuntu-latest` names",
        ),
        (
            "an upper-case label",
            DOCS_RUNNER,
            "  docs:\n    runs-on: Ubuntu-Latest\n",
            "`runs-on: Ubuntu-Latest`",
        ),
        (
            "a quoted label",
            DENY_RUNNER,
            "  deny:\n    runs-on: \"windows-latest\"\n",
            "`runs-on: \"windows-latest\"`",
        ),
        (
            "a runs-on block sequence at the key's indent",
            DENY_RUNNER,
            "  deny:\n    runs-on:\n    - ubuntu-latest\n",
            "`ubuntu-latest` names",
        ),
        (
            "a matrix entry",
            OS_LINE,
            "        os: [windows-latest, windows-11-arm, ubuntu-26.04, ubuntu-24.04-arm]\n",
            "`os: [windows-latest,",
        ),
        (
            "a matrix include entry",
            OS_LINE,
            "        os: [windows-2025, windows-11-arm, ubuntu-26.04, ubuntu-24.04-arm]\n        include:\n          - os: macos-latest\n",
            "`os: macos-latest` names",
        ),
    ] {
        let copy = mutated(&text, from, to);
        assert_reported(&latest_label_problems("ci.yml", &copy), case, fragment);
    }
    // The problem names the file and the line.
    let copy = mutated(&text, FMT_RUNNER, "  fmt:\n    runs-on: ubuntu-latest\n");
    let problems = latest_label_problems("ci.yml", &copy);
    let line = line_of(&copy, "runs-on: ubuntu-latest");
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with(&format!("ci.yml:{line}: ")),
        "{problems:?}"
    );
    // Another workflow file, with the matrix in a flow mapping.
    let release = "on: push\njobs:\n  build:\n    strategy: { matrix: { os: [windows-2025, ubuntu-latest] } }\n    runs-on: ${{ matrix.os }}\n";
    assert_reported(
        &latest_label_problems("release.yml", release),
        "a flow-mapping matrix in another workflow",
        "release.yml:4: `strategy: { matrix: { os: [windows-2025, ubuntu-latest] } }`",
    );
    // Comments may mention `-latest`, and so may text outside runner positions.
    for (case, from, to) in [
        (
            "a trailing comment",
            FMT_RUNNER,
            "  fmt:\n    runs-on: ubuntu-26.04 # was ubuntu-latest before m0 §20\n",
        ),
        (
            "a comment line inside the matrix",
            OS_LINE,
            "        # ubuntu-latest moved to 26.04 (runner-images#14748)\n        os: [windows-2025, windows-11-arm, ubuntu-26.04, ubuntu-24.04-arm]\n",
        ),
        (
            "a run value",
            "      - run: cargo fmt --all -- --check\n",
            "      - run: echo \"not on ubuntu-latest\"\n",
        ),
    ] {
        let copy = mutated(&text, from, to);
        assert_eq!(
            latest_label_problems("ci.yml", &copy),
            Vec::<String>::new(),
            "{case}"
        );
    }
}
