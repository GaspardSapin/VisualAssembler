//! Generated bench inputs (m4-editor-shell.md §7.1): a 100,000-line file for the `open`, `scroll`
//! and `type` scenarios and a 5,000-line project for `idle`, written under `bench/generated/`
//! (gitignored, owned by M4, §14.2).
//!
//! Every line comes from [`template_line`]: 20 fixed 68000 lines of our own, cycled, with the line
//! number in every label so that the labels of a file are unique. The cycle covers labels and local
//! labels, the twelve addressing modes, `DC.B` strings, both comment forms, one `SuspectComment`
//! line (`dc.b 1, 2`) and one invalid line, for about 33 bytes per line. Files are LF only and
//! deterministic: nothing depends on the clock, the host or a random source.
//!
//! `bench/generated/VERSION` holds [`GENERATOR_VERSION`]. [`ensure_inputs`] keeps the files when
//! that version matches and every file exists, and regenerates all of them otherwise. `VERSION` is
//! removed first and written last, so an interrupted generation is redone by the next bench.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};

/// Version of the generated content; changing [`template_line`] or the file set needs a new one.
pub const GENERATOR_VERSION: u32 = 1;

/// Name of the file under `bench/generated/` that holds [`GENERATOR_VERSION`].
pub const VERSION_FILE: &str = "VERSION";

/// Name of the large file opened by `open`, `scroll` and `type`.
pub const LARGE_FILE: &str = "100k.asm";

/// Lines of [`LARGE_FILE`].
pub const LARGE_LINES: usize = 100_000;

/// Name of the project folder opened by `idle`.
pub const PROJECT_DIR: &str = "5k";

/// The project's entry file, which includes every part.
pub const PROJECT_MAIN: &str = "main.asm";

/// Number of part files (`part01.asm` … `part09.asm`).
pub const PROJECT_PARTS: usize = 9;

/// Lines of each project file, `main.asm` included.
pub const PROJECT_FILE_LINES: usize = 500;

/// Lines of the whole project: `main.asm` plus the parts.
pub const PROJECT_LINES: usize = PROJECT_FILE_LINES * (PROJECT_PARTS + 1);

/// Path of the project file inside the project folder (m4-editor-shell.md §4.5).
pub const PROJECT_TOML: &str = ".visualasm/project.toml";

/// Content of [`PROJECT_TOML`].
pub const PROJECT_TOML_TEXT: &str =
    "schema_version = 1\n\n[project]\narch = \"m68k\"\nentry = \"main.asm\"\n";

/// Number of template lines in one cycle.
pub const TEMPLATE_CYCLE: usize = 20;

/// Returns the folder of the generated inputs, `<root>/bench/generated`.
pub fn generated_dir(root: &Path) -> PathBuf {
    root.join("bench").join("generated")
}

/// Returns line `n` (0-based) of every generated file, without its line break.
///
/// The line is slot `n % 20` of the template cycle. Labels carry `n` (`l<n>:`, local `<n>$:`);
/// the PC-relative lines refer to the label of the cycle's first line, and the branch refers to
/// the local label of the line just before it, so every reference names a label of the same file
/// defined earlier.
pub fn template_line(n: usize) -> String {
    let cycle_start = n - n % TEMPLATE_CYCLE;
    match n % TEMPLATE_CYCLE {
        0 => format!("l{n}:\tmoveq\t#42,d0\t; immediate data"),
        1 => "\tmovea.l\ta0,a1\t; address register direct".to_owned(),
        2 => "\tmove.w\t(a0),d2\t; address register indirect".to_owned(),
        3 => "\tmove.b\t(a1)+,d3\t; post-increment".to_owned(),
        4 => "\tmove.l\td4,-(sp)\t; pre-decrement".to_owned(),
        5 => format!("l{n}:\tmove.w\t8(a2),d5\t; 16-bit displacement"),
        6 => "\tmove.b\t2(a3,d1.w),d6\t; index and displacement".to_owned(),
        7 => "\tmove.w\t$1000.w,d7\t; absolute short".to_owned(),
        8 => "\tmove.l\t$00ff0000,d0\t; absolute long".to_owned(),
        9 => format!("\tlea\tl{cycle_start}(pc),a0\t; program counter relative"),
        10 => format!("\tmove.w\tl{cycle_start}(pc,d0.w),d1\t; pc with index"),
        11 => format!("{n}$:\tsubq.w\t#1,d0\t; local label"),
        12 => format!("\tbne.s\t{}$\t; back to the local label", n - 1),
        13 => format!("; line {n} is a comment of our own"),
        14 => "* a comment that starts in column one".to_owned(),
        15 => format!("l{n}:\tdc.b\t'bench text',0"),
        16 => format!("\tdc.b\t\"line {n}\",13,10,0"),
        17 => "\tdc.b\t1, 2".to_owned(),
        18 => "\tmove.x\td0,d1".to_owned(),
        _ => "\trts".to_owned(),
    }
}

/// Returns the text made of template lines `0..lines`, each ending with `\n`.
pub fn template_text(lines: usize) -> String {
    let mut text = String::with_capacity(lines * 36);
    for n in 0..lines {
        text.push_str(&template_line(n));
        text.push('\n');
    }
    text
}

/// Returns the name of part `index` (1-based): `part01.asm` … `part09.asm`.
pub fn part_name(index: usize) -> String {
    format!("part{index:02}.asm")
}

/// Returns the `INCLUDE` line of part `index` (1-based), without its line break.
pub fn include_line(index: usize) -> String {
    format!("\tINCLUDE\t\"{}\"", part_name(index))
}

/// Returns the text of the project's `main.asm`: template lines, then one `INCLUDE` line per part,
/// [`PROJECT_FILE_LINES`] lines in all.
pub fn main_text() -> String {
    let mut text = template_text(PROJECT_FILE_LINES - PROJECT_PARTS);
    for index in 1..=PROJECT_PARTS {
        text.push_str(&include_line(index));
        text.push('\n');
    }
    text
}

/// Returns the paths of every generated file under `dir` (`bench/generated`), `VERSION` excluded,
/// in writing order.
pub fn generated_files(dir: &Path) -> Vec<PathBuf> {
    let project = dir.join(PROJECT_DIR);
    let mut files = vec![
        dir.join(LARGE_FILE),
        project.join(PROJECT_TOML),
        project.join(PROJECT_MAIN),
    ];
    files.extend((1..=PROJECT_PARTS).map(|index| project.join(part_name(index))));
    files
}

/// The generated inputs: the large file and the project folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchInputs {
    /// `bench/generated/100k.asm`, for `open`, `scroll` and `type`.
    pub large: PathBuf,
    /// `bench/generated/5k`, the project folder for `idle`.
    pub project: PathBuf,
}

/// What [`ensure_inputs_with_status`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputsStatus {
    /// `VERSION` matched and every file existed: nothing was written.
    Current,
    /// The inputs were (re)generated.
    Generated,
}

/// Tells whether the inputs under `dir` (`bench/generated`) can be kept: `VERSION` holds
/// [`GENERATOR_VERSION`] and every file of [`generated_files`] exists.
pub fn inputs_are_current(dir: &Path) -> bool {
    let version = fs::read_to_string(dir.join(VERSION_FILE));
    version.is_ok_and(|text| text.trim() == GENERATOR_VERSION.to_string())
        && generated_files(dir).iter().all(|file| file.is_file())
}

/// Makes sure the generated inputs under `<root>/bench/generated` exist at the current
/// [`GENERATOR_VERSION`] and returns their paths.
pub fn ensure_inputs(root: &Path) -> Result<BenchInputs> {
    ensure_inputs_with_status(root).map(|(inputs, _)| inputs)
}

/// Same as [`ensure_inputs`], and tells whether the files were kept or (re)generated.
pub fn ensure_inputs_with_status(root: &Path) -> Result<(BenchInputs, InputsStatus)> {
    let dir = generated_dir(root);
    let inputs = BenchInputs {
        large: dir.join(LARGE_FILE),
        project: dir.join(PROJECT_DIR),
    };
    if inputs_are_current(&dir) {
        return Ok((inputs, InputsStatus::Current));
    }
    generate(&dir)?;
    Ok((inputs, InputsStatus::Generated))
}

/// Writes every input under `dir`: removes `VERSION`, writes the large file, replaces the project
/// folder, then writes `VERSION`.
fn generate(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let version = dir.join(VERSION_FILE);
    match fs::remove_file(&version) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("cannot remove {}", version.display()));
        }
    }
    write(&dir.join(LARGE_FILE), &template_text(LARGE_LINES))?;

    let project = dir.join(PROJECT_DIR);
    remove_entry(&project)?;
    let config_dir = project.join(".visualasm");
    fs::create_dir_all(&config_dir)
        .with_context(|| format!("cannot create {}", config_dir.display()))?;
    write(&project.join(PROJECT_TOML), PROJECT_TOML_TEXT)?;
    write(&project.join(PROJECT_MAIN), &main_text())?;
    let part = template_text(PROJECT_FILE_LINES);
    for index in 1..=PROJECT_PARTS {
        write(&project.join(part_name(index)), &part)?;
    }
    write(&version, &format!("{GENERATOR_VERSION}\n"))
}

/// Removes `path` when it exists: a folder with its content, anything else as a file. A symbolic
/// link is removed itself, never followed.
fn remove_entry(path: &Path) -> Result<()> {
    let removed = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => Err(error),
    };
    removed.with_context(|| format!("cannot remove the stale {}", path.display()))
}

/// Writes `text` to `path`, replacing any previous content.
fn write(path: &Path, text: &str) -> Result<()> {
    fs::write(path, text).with_context(|| format!("cannot write {}", path.display()))
}
