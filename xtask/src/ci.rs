//! `cargo xtask ci`: the single source of the CI check list (m0-foundations.md §6, amended by
//! §17.3, §18.3 and §18.4).
//!
//! Steps run in order and the command stops at the first failure. `==> <step>` is printed on
//! stderr before each step. Step 0, the build configuration check, always runs first, also with
//! `--no-fmt`: it fails when a `RUSTFLAGS`-type variable is set (even to an empty value), when
//! `.cargo/config.toml` differs from [`CANONICAL_CARGO_CONFIG`], or when the extension-less
//! `.cargo/config` exists (architecture R8).
//!
//! Step 0 is a tripwire against accidental or careless build-flag overrides, not a sandbox
//! against deliberate evasion (m0-foundations.md §18.1, architecture §9.3/§9.4).
//!
//! Windows only: before the test step, the running xtask executable is renamed aside (see
//! [`park_running_executable`]), because that step rebuilds xtask and cargo must replace the file
//! this process runs from.

use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context as _, Result, bail};

use crate::{exec, layering, locate};

/// Arguments of `cargo xtask ci`.
#[derive(Debug, Clone, Default, clap::Args)]
pub struct CiArgs {
    /// Skip `cargo fmt --all -- --check` (the CI workflow runs it once, in its own job).
    #[arg(long)]
    pub no_fmt: bool,
}

/// Step 1: formatting check.
pub const FMT: &[&str] = &["fmt", "--all", "--", "--check"];

/// Step 2: clippy on the whole workspace with every feature, warnings denied.
pub const CLIPPY_ALL_FEATURES: &[&str] = &[
    "clippy",
    "--workspace",
    "--all-targets",
    "--all-features",
    "--locked",
    "--",
    "-D",
    "warnings",
];

/// Step 3: clippy on the GUI built with the glow renderer only, warnings denied.
pub const CLIPPY_GLOW_ONLY: &[&str] = &[
    "clippy",
    "-p",
    "visual-assembler",
    "--all-targets",
    "--no-default-features",
    "--features",
    "renderer-glow",
    "--locked",
    "--",
    "-D",
    "warnings",
];

/// Step 5: every test of the workspace with every feature.
pub const TEST_ALL: &[&str] = &["test", "--workspace", "--all-features", "--locked"];

/// Environment variables that replace the `rustflags` of `.cargo/config.toml` (§17.3.2 a,
/// §18.3). `CARGO_TARGET_<triple>_RUSTFLAGS` is matched with [`TARGET_RUSTFLAGS_PREFIX`] and
/// [`TARGET_RUSTFLAGS_SUFFIX`].
pub const RUSTFLAGS_VARIABLES: &[&str] = &[
    "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
];

/// Prefix of the per-target rustflags variables `CARGO_TARGET_<triple>_RUSTFLAGS`.
pub const TARGET_RUSTFLAGS_PREFIX: &str = "CARGO_TARGET_";

/// Suffix of the per-target rustflags variables `CARGO_TARGET_<triple>_RUSTFLAGS`.
pub const TARGET_RUSTFLAGS_SUFFIX: &str = "_RUSTFLAGS";

/// The cargo configuration checked by step 0, relative to the workspace root.
pub const CARGO_CONFIG: &str = ".cargo/config.toml";

/// The extension-less cargo configuration, which cargo reads instead of [`CARGO_CONFIG`] when
/// both exist. Step 0 rejects it.
pub const LEGACY_CARGO_CONFIG: &str = ".cargo/config";

/// The canonical content of [`CARGO_CONFIG`] (m0-foundations.md §18.4): once blank lines and
/// full-line `#` comments are dropped and all whitespace is removed from each remaining line,
/// the file must equal these lines, in this order. Every other line is a violation.
///
/// Any intended change to `.cargo/config.toml` (e.g. M8's `/DEPENDENTLOADFLAG:0x800`) updates the
/// file and this constant in the same commit.
pub const CANONICAL_CARGO_CONFIG: &[&str] = &[
    "[alias]",
    r#"xtask="run--packagextask--""#,
    "[target.x86_64-pc-windows-msvc]",
    r#"rustflags=["-C","target-feature=+crt-static"]"#,
    "[target.aarch64-pc-windows-msvc]",
    r#"rustflags=["-C","target-feature=+crt-static"]"#,
    "[target.aarch64-unknown-linux-gnu]",
    r#"linker="aarch64-linux-gnu-gcc""#,
];

/// One step of `cargo xtask ci`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Step 0: build flags come only from `.cargo/config.toml` (§17.3.2, architecture R8).
    BuildConfiguration,
    /// A cargo invocation; the slice holds the arguments that follow `cargo`.
    Cargo(&'static [&'static str]),
    /// The in-process layering and source checks of `layering.rs` (§6.3, §17.2, §17.4).
    Layering,
}

impl Step {
    /// Tells whether the step links a new xtask binary into the target directory.
    ///
    /// `cargo test --workspace --all-features` unifies dependency features across the whole
    /// workspace (eframe enables more `windows-sys` features than xtask alone), so xtask and its
    /// clap/anstream chain get new hashes and `<target>/debug/xtask.exe` is replaced.
    pub fn rebuilds_xtask(&self) -> bool {
        *self == Self::Cargo(TEST_ALL)
    }

    /// Returns the label printed after `==> ` before the step runs.
    pub fn label(&self) -> String {
        match self {
            Self::BuildConfiguration => "build configuration".to_owned(),
            Self::Cargo(args) => format!("cargo {}", args.join(" ")),
            Self::Layering => "layering and source checks".to_owned(),
        }
    }
}

/// Returns the steps of `cargo xtask ci`, in execution order.
pub fn steps(args: &CiArgs) -> Vec<Step> {
    let mut steps = Vec::with_capacity(6);
    steps.push(Step::BuildConfiguration);
    if !args.no_fmt {
        steps.push(Step::Cargo(FMT));
    }
    steps.extend([
        Step::Cargo(CLIPPY_ALL_FEATURES),
        Step::Cargo(CLIPPY_GLOW_ONLY),
        Step::Layering,
        Step::Cargo(TEST_ALL),
    ]);
    steps
}

/// Runs every step from `root`, stopping at the first failure.
pub fn run(root: &Path, args: &CiArgs) -> Result<()> {
    let cargo = locate::cargo();
    for step in steps(args) {
        eprintln!("==> {}", step.label());
        match step {
            Step::BuildConfiguration => run_build_configuration(root)?,
            Step::Cargo(cargo_args) => {
                if cfg!(windows) && step.rebuilds_xtask() {
                    let target_dir = locate::target_directory(root)?;
                    if let Some(parked) = park_running_executable(&target_dir)? {
                        eprintln!(
                            "note: moved the running xtask executable to {} so cargo can rebuild it",
                            parked.display()
                        );
                    }
                }
                let mut cmd = Command::new(&cargo);
                cmd.current_dir(root).args(cargo_args);
                exec::run(&mut cmd)?;
            }
            Step::Layering => run_layering(root)?,
        }
    }
    Ok(())
}

/// Tells whether the environment variable `name` replaces the `rustflags` of
/// `.cargo/config.toml`. Names are compared ignoring ASCII case on every OS (fail-closed,
/// m0-foundations.md §18.3).
pub fn replaces_config_rustflags(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    RUSTFLAGS_VARIABLES.contains(&name.as_str())
        || (name.len() >= TARGET_RUSTFLAGS_PREFIX.len() + TARGET_RUSTFLAGS_SUFFIX.len()
            && name.starts_with(TARGET_RUSTFLAGS_PREFIX)
            && name.ends_with(TARGET_RUSTFLAGS_SUFFIX))
}

/// Step 0 (a): returns one message per variable of `vars` that is set and replaces the
/// `rustflags` of `.cargo/config.toml` (`RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`,
/// `CARGO_BUILD_RUSTFLAGS`, `CARGO_TARGET_*_RUSTFLAGS`), sorted by name.
///
/// A variable set to an **empty value** is reported too (§18.3): cargo 1.99 treats an empty
/// `RUSTFLAGS` as an override and drops `+crt-static`. A name that is not valid Unicode is
/// compared in its lossy form, so it can only add problems. Pure: pass `std::env::vars_os()`
/// for the real environment.
pub fn check_build_env<I, K, V>(vars: I) -> Vec<String>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    let mut names: Vec<String> = vars
        .into_iter()
        .map(|(name, _)| name.as_ref().to_string_lossy().into_owned())
        .filter(|name| replaces_config_rustflags(name))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            format!(
                "{name} is set: it replaces .cargo/config.toml (+crt-static) and can silence lints; unset it"
            )
        })
        .collect()
}

/// One significant line of `.cargo/config.toml` (neither blank nor a full-line `#` comment).
struct ConfigLine<'a> {
    /// 1-based line number in the file.
    number: usize,
    /// The line without its surrounding whitespace, quoted in messages.
    text: &'a str,
    /// The line with all whitespace removed, compared with [`CANONICAL_CARGO_CONFIG`].
    compact: String,
}

/// Returns the significant lines of `text`: blank lines and full-line `#` comments are dropped
/// (§18.4). A trailing comment stays part of its line, so it makes the line differ.
fn significant_config_lines(text: &str) -> Vec<ConfigLine<'_>> {
    text.lines()
        .enumerate()
        .filter_map(|(index, raw)| {
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            Some(ConfigLine {
                number: index + 1,
                text: trimmed,
                compact: trimmed.chars().filter(|c| !c.is_whitespace()).collect(),
            })
        })
        .collect()
}

/// Lines of one stretch of `.cargo/config.toml` that lies between two lines matching
/// [`CANONICAL_CARGO_CONFIG`] (or before the first one, or after the last one).
#[derive(Default)]
struct ConfigGap<'a> {
    /// Number of the last matching file line before the stretch, `None` at the start.
    after: Option<usize>,
    /// File lines of the stretch that match no canonical line.
    extra: Vec<&'a ConfigLine<'a>>,
    /// Canonical lines that the stretch lacks.
    missing: Vec<&'static str>,
}

impl ConfigGap<'_> {
    /// Appends one message per problem of the stretch to `problems`, then empties the stretch.
    ///
    /// Extra and missing lines are paired in order, so a changed line gives one message; the
    /// rest are reported as extra lines (with their number) or missing canonical lines.
    fn report(&mut self, problems: &mut Vec<String>) {
        let paired = self.extra.len().min(self.missing.len());
        for (line, expected) in self.extra.iter().zip(&self.missing) {
            problems.push(format!(
                "{CARGO_CONFIG}:{}: `{}` differs from the canonical line `{expected}`",
                line.number, line.text
            ));
        }
        for line in &self.extra[paired..] {
            problems.push(format!(
                "{CARGO_CONFIG}:{}: `{}` is not in the canonical configuration",
                line.number, line.text
            ));
        }
        for expected in &self.missing[paired..] {
            let place = self.after.map_or_else(
                || "at the start of the file".to_owned(),
                |number| format!("after line {number}"),
            );
            problems.push(format!(
                "{CARGO_CONFIG}: the canonical line `{expected}` is missing (expected {place})"
            ));
        }
        self.extra.clear();
        self.missing.clear();
    }
}

/// Step 0 (b): compares the text of `.cargo/config.toml` with [`CANONICAL_CARGO_CONFIG`]
/// (m0-foundations.md §18.4) and returns one message per problem, in file order.
///
/// Blank lines and full-line `#` comments are dropped and all whitespace is removed from each
/// remaining line; the result must equal the canonical lines, line for line and in order. The
/// two are aligned on their longest common subsequence, so an inserted line (a top-level key,
/// `include`, a quoted key, `runner`, an extra alias) is reported alone with its line number, a
/// changed line (an alias value, a trailing comment) gives one message naming the expected line,
/// and a missing canonical line is reported with the line it should follow. An empty text lacks
/// every canonical line.
pub fn check_cargo_config(text: &str) -> Vec<String> {
    let lines = significant_config_lines(text);
    let canonical = CANONICAL_CARGO_CONFIG;
    let (n, m) = (lines.len(), canonical.len());
    // common[i][j]: length of the longest common subsequence of lines[i..] and canonical[j..].
    let mut common = vec![vec![0_usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            common[i][j] = if lines[i].compact == canonical[j] {
                common[i + 1][j + 1] + 1
            } else {
                common[i + 1][j].max(common[i][j + 1])
            };
        }
    }
    let mut problems = Vec::new();
    let mut gap = ConfigGap::default();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && lines[i].compact == canonical[j] {
            gap.report(&mut problems);
            gap.after = Some(lines[i].number);
            i += 1;
            j += 1;
        } else if i < n && (j == m || common[i + 1][j] >= common[i][j + 1]) {
            gap.extra.push(&lines[i]);
            i += 1;
        } else {
            gap.missing.push(canonical[j]);
            j += 1;
        }
    }
    gap.report(&mut problems);
    problems
}

/// Step 0 (b) on the files of the workspace at `root`: `.cargo/config.toml` must be readable and
/// canonical ([`check_cargo_config`]), and the extension-less [`LEGACY_CARGO_CONFIG`] must not
/// exist (§18.4).
///
/// Every failure is a problem, never a pass: a missing or unreadable `.cargo/config.toml`, and a
/// `.cargo/config` that cannot be inspected, are reported like any other problem.
pub fn check_cargo_config_files(root: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    let config = root.join(CARGO_CONFIG);
    match fs::read_to_string(&config) {
        Ok(text) => problems.extend(check_cargo_config(&text)),
        Err(error) => problems.push(format!(
            "{CARGO_CONFIG} cannot be read ({error}); restore the canonical file (CANONICAL_CARGO_CONFIG in xtask/src/ci.rs)"
        )),
    }
    let legacy = root.join(LEGACY_CARGO_CONFIG);
    match fs::symlink_metadata(&legacy) {
        Ok(_) => problems.push(format!(
            "{LEGACY_CARGO_CONFIG} exists: cargo reads it instead of {CARGO_CONFIG}; delete it"
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => problems.push(format!(
            "{LEGACY_CARGO_CONFIG} cannot be inspected ({error}); cargo would read it instead of {CARGO_CONFIG}"
        )),
    }
    problems
}

/// Runs step 0 on the workspace at `root` and returns its problems: the process environment
/// ([`check_build_env`]), then the cargo configuration files ([`check_cargo_config_files`]).
pub fn check_build_configuration(root: &Path) -> Vec<String> {
    let mut problems = check_build_env(std::env::vars_os());
    problems.extend(check_cargo_config_files(root));
    problems
}

/// Runs step 0, prints each problem on stderr, fails if any exists.
fn run_build_configuration(root: &Path) -> Result<()> {
    let problems = check_build_configuration(root);
    if problems.is_empty() {
        eprintln!("build configuration: ok");
        return Ok(());
    }
    for problem in &problems {
        eprintln!("{problem}");
    }
    bail!(
        "{} build configuration problem(s): no RUSTFLAGS-type variable may be set, even empty, and \
         .cargo/config.toml must equal CANONICAL_CARGO_CONFIG in xtask/src/ci.rs once blank lines, \
         full-line comments and whitespace are dropped (architecture R8, m0-foundations.md §18.3 \
         and §18.4)",
        problems.len()
    );
}

/// Runs the layering and source checks, prints each violation on stderr, fails if any exists.
fn run_layering(root: &Path) -> Result<()> {
    let violations = layering::check_workspace(root)?;
    if violations.is_empty() {
        eprintln!("layering and source checks: no violation");
        return Ok(());
    }
    for violation in &violations {
        eprintln!("{violation}");
    }
    bail!(
        "{} layering or source violation(s) (rules: architecture.md §2 and §9, m0-foundations.md §17.2 and §17.4)",
        violations.len()
    );
}

/// Suffix of the renamed executable: `xtask.exe` becomes `xtask.exe.old`.
pub const PARKED_SUFFIX: &str = ".old";

/// Returns the path the running executable is moved to.
pub fn parked_path(executable: &Path) -> PathBuf {
    let mut name = executable.as_os_str().to_owned();
    name.push(PARKED_SUFFIX);
    PathBuf::from(name)
}

/// Renames the running executable to [`parked_path`] when it lives under `target_dir`, and
/// returns the new path; returns `None` when it lives elsewhere (cargo will not replace it).
///
/// Windows refuses to delete a running executable but allows renaming it, so cargo can then
/// link the new `xtask.exe`. A file parked by an earlier run is no longer running and is
/// replaced. The next `cargo xtask` restores `xtask.exe` (cargo re-links fresh binaries).
/// Fails with context when the rename fails, e.g. while another `cargo xtask ci` runs from the
/// same target directory.
pub fn park_running_executable(target_dir: &Path) -> Result<Option<PathBuf>> {
    let executable =
        std::env::current_exe().context("cannot locate the running xtask executable")?;
    let resolved_executable = fs::canonicalize(&executable)
        .with_context(|| format!("cannot resolve {}", executable.display()))?;
    let resolved_target = fs::canonicalize(target_dir).with_context(|| {
        format!(
            "cannot resolve the target directory {}",
            target_dir.display()
        )
    })?;
    if !resolved_executable.starts_with(&resolved_target) {
        return Ok(None);
    }
    let parked = parked_path(&executable);
    fs::rename(&executable, &parked).with_context(|| {
        format!(
            "cannot move the running {} aside to {} (is another `cargo xtask ci` running?)",
            executable.display(),
            parked.display()
        )
    })?;
    Ok(Some(parked))
}
