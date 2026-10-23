//! Tests for `src/ci.rs`: the normative step list of `cargo xtask ci` (m0-foundations.md §6,
//! with step 0 of §17.3.2), the build-configuration checks of step 0 — `check_build_env` (set
//! variables, even empty, §18.3) and the canonical `.cargo/config.toml` comparison
//! `check_cargo_config` / `check_cargo_config_files` (§18.4) — the real `cargo xtask ci`
//! stopping at step 0, the path used to move the running executable aside on Windows, and the
//! §19.9 shape of `.github/workflows/ci.yml`, which runs `cargo xtask ci` (CR19-06, S19-5).
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
