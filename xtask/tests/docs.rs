//! Tests for `src/docs.rs` (`cargo xtask docs`, m0-foundations.md §6) and for the generator it
//! runs first, `scripts/gen_agencement.py`.
//!
//! QA audit, M0 round 2 (regression for QA-M0-F1): on Windows the generator wrote
//! `AGENCEMENT.md` in text mode, so every line ended in CRLF. `.gitattributes` asks for LF
//! everywhere (§3.4), so `make docs` left a fresh checkout dirty and the file differed between
//! Windows and Linux. The tests run the project's real generator, copied into a fresh git
//! repository under `CARGO_TARGET_TMPDIR` (§17.9), with the interpreter `cargo xtask docs` uses.
#![allow(clippy::unwrap_used)]

use std::{
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU32, Ordering},
};

use xtask::docs::{CHECK_SCRIPT, GENERATE_SCRIPT, PYTHON_ENV, python_program};

/// The file the generator writes, relative to the root it is given.
const AGENCEMENT: &str = "AGENCEMENT.md";

/// The marker the generator puts on entries that still need a hand-written annotation.
const TODO: &str = "<!-- annotate -->";

/// Returns a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (§17.9): created with
/// `create_dir`, which fails if the path already exists.
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    dir
}

/// Runs `git <args>` in `dir` and asserts that it succeeds; returns its stdout.
fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Writes `content` to `root/relative`, creating the missing folders one level at a time.
fn write_file(root: &Path, relative: &str, content: &[u8]) {
    let path = root.join(relative);
    let mut folder = root.to_path_buf();
    let parent = Path::new(relative).parent().unwrap();
    for component in parent.components() {
        folder.push(component);
        match std::fs::create_dir(&folder) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => panic!("cannot create {}: {error}", folder.display()),
        }
    }
    std::fs::write(path, content).unwrap();
}

/// Creates a fresh git repository holding a small project: the workspace's real
/// `.gitattributes` and generator, a `.gitignore`, a few source and data files, and one
/// gitignored output file. Asserts that git sees it as its own repository, never the project's.
fn project(tag: &str) -> PathBuf {
    let root = temp_dir(tag);
    git(&root, &["init", "--quiet"]);
    let toplevel = git(&root, &["rev-parse", "--show-toplevel"]);
    assert_eq!(
        std::fs::canonicalize(toplevel.trim()).unwrap(),
        std::fs::canonicalize(&root).unwrap(),
        "{} is not its own git repository",
        root.display()
    );
    let workspace = xtask::locate::workspace_root().unwrap();
    for relative in [GENERATE_SCRIPT, ".gitattributes"] {
        write_file(
            &root,
            relative,
            &std::fs::read(workspace.join(relative)).unwrap(),
        );
    }
    write_file(&root, ".gitignore", b"/bench/results/\n");
    write_file(&root, "Cargo.toml", b"[workspace]\nmembers = []\n");
    write_file(&root, "crates/va-core/src/lib.rs", b"//! Core.\n");
    write_file(&root, "docs/notes.md", "# Notes \u{2713}\n".as_bytes());
    write_file(&root, "bench/results/run.json", b"{}\n");
    root
}

/// Runs the generator exactly as `cargo xtask docs` does (`<python> scripts/gen_agencement.py .`
/// from the root, `$VA_PYTHON` honoured), plus `extra` arguments. Returns `None` after printing
/// `skipped: python not found` when the interpreter cannot be found.
fn generate(root: &Path, extra: &[&str]) -> Option<Output> {
    let python: OsString = python_program(std::env::var_os(PYTHON_ENV));
    match Command::new(&python)
        .current_dir(root)
        .arg(GENERATE_SCRIPT)
        .arg(".")
        .args(extra)
        .output()
    {
        Ok(output) => Some(output),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            println!("skipped: python not found ({})", python.to_string_lossy());
            None
        }
        Err(error) => panic!("cannot run {}: {error}", python.to_string_lossy()),
    }
}

/// Asserts that a generator run succeeded.
fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "gen_agencement.py failed ({}): {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Returns the bytes of the generated file.
fn agencement(root: &Path) -> Vec<u8> {
    std::fs::read(root.join(AGENCEMENT)).unwrap()
}

/// Asserts that `bytes` holds only LF line endings: no carriage return at all, and a final LF.
fn assert_lf_only(bytes: &[u8], context: &str) {
    let carriage_returns = bytes.iter().filter(|&&byte| byte == b'\r').count();
    assert_eq!(
        carriage_returns, 0,
        "{context}: {AGENCEMENT} holds {carriage_returns} carriage return(s); \
         .gitattributes asks for LF everywhere (m0-foundations.md §3.4)"
    );
    assert_eq!(bytes.last(), Some(&b'\n'), "{context}: no final LF");
}

/// Replaces the annotation marker of the `name` entry with `note` (a hand-written annotation).
fn annotate(text: &str, name: &str, note: &str) -> String {
    let mut found = false;
    let lines: Vec<String> = text
        .split('\n')
        .map(|line| {
            let entry = line.split("  # ").next().unwrap_or_default();
            if !found && entry.trim_end().ends_with(name) && line.ends_with(TODO) {
                found = true;
                line.replace(TODO, note)
            } else {
                line.to_owned()
            }
        })
        .collect();
    assert!(found, "no entry `{name}` marked {TODO} in:\n{text}");
    lines.join("\n")
}

#[test]
fn python_program_honours_a_non_empty_override_only() {
    let default = if cfg!(windows) { "python" } else { "python3" };
    assert_eq!(python_program(None), OsString::from(default));
    assert_eq!(
        python_program(Some(OsString::new())),
        OsString::from(default)
    );
    assert_eq!(
        python_program(Some(OsString::from("C:/Py 3/python.exe"))),
        OsString::from("C:/Py 3/python.exe")
    );
}

/// QA-M0-F1: the generated file uses LF only on every OS, a second run rewrites the same bytes,
/// and `--check` agrees with what was written.
#[test]
fn generated_agencement_has_lf_line_endings_and_is_byte_stable() {
    let root = project("docs-agencement-lf");
    let Some(first) = generate(&root, &[]) else {
        std::fs::remove_dir_all(&root).unwrap();
        return;
    };
    assert_success(&first);
    let written = agencement(&root);
    assert_lf_only(&written, "first generation");
    let text = String::from_utf8(written.clone()).unwrap();
    for entry in [
        "crates/",
        "lib.rs",
        "notes.md",
        "gen_agencement.py",
        AGENCEMENT,
    ] {
        assert!(text.contains(entry), "`{entry}` is missing from:\n{text}");
    }
    assert!(
        !text.contains("run.json") && !text.contains("results/"),
        "a gitignored path is listed:\n{text}"
    );

    let second = generate(&root, &[]).unwrap();
    assert_success(&second);
    assert_eq!(
        agencement(&root),
        written,
        "a second generation changed the bytes of {AGENCEMENT}"
    );

    let check = generate(&root, &["--check"]).unwrap();
    assert!(
        check.status.success(),
        "--check disagrees with the file just written: {}",
        String::from_utf8_lossy(&check.stdout)
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// A hand annotation survives regeneration, and a file that a Windows editor (or a checkout
/// without the attributes) saved with CRLF is rewritten with LF, annotation kept.
#[test]
fn regeneration_keeps_annotations_and_turns_a_crlf_file_back_to_lf() {
    let root = project("docs-agencement-crlf");
    let Some(first) = generate(&root, &[]) else {
        std::fs::remove_dir_all(&root).unwrap();
        return;
    };
    assert_success(&first);
    let text = String::from_utf8(agencement(&root)).unwrap();
    let note = "core crate, caf\u{e9} \u{2713}";
    let annotated = annotate(&text, "lib.rs", note);

    // Reference: the annotated file saved with LF, then regenerated.
    std::fs::write(root.join(AGENCEMENT), annotated.as_bytes()).unwrap();
    assert_success(&generate(&root, &[]).unwrap());
    let expected = agencement(&root);
    assert_lf_only(&expected, "regeneration of an annotated LF file");
    let expected_text = String::from_utf8(expected.clone()).unwrap();
    assert!(
        expected_text
            .lines()
            .any(|line| line.contains("lib.rs") && line.ends_with(&format!("  # {note}"))),
        "the hand annotation was lost:\n{expected_text}"
    );

    // The same annotated file saved with CRLF must give the same bytes.
    std::fs::write(
        root.join(AGENCEMENT),
        annotated.replace('\n', "\r\n").as_bytes(),
    )
    .unwrap();
    assert_success(&generate(&root, &[]).unwrap());
    let rewritten = agencement(&root);
    assert_lf_only(&rewritten, "regeneration of a CRLF file");
    assert_eq!(
        String::from_utf8(rewritten).unwrap(),
        expected_text,
        "a CRLF copy of the file must regenerate to the same LF bytes"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// The QA-M0-F1 scenario end to end: commit the generated file, check it out afresh (LF, from
/// `.gitattributes`), regenerate it as `make docs` does, and the working tree must stay clean.
#[test]
fn regenerating_after_a_fresh_checkout_leaves_the_tree_clean() {
    let root = project("docs-agencement-checkout");
    let Some(first) = generate(&root, &[]) else {
        std::fs::remove_dir_all(&root).unwrap();
        return;
    };
    assert_success(&first);
    git(&root, &["add", "--all"]);
    git(
        &root,
        &[
            "-c",
            "user.name=qa",
            "-c",
            "user.email=qa@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "Add the fixture project",
        ],
    );
    let eol = git(&root, &["ls-files", "--eol", AGENCEMENT]);
    assert!(eol.starts_with("i/lf"), "committed blob is not LF: {eol}");

    // A fresh checkout of every file, as a clone would produce.
    git(&root, &["rm", "--quiet", "--cached", "-r", "."]);
    git(&root, &["reset", "--quiet", "--hard"]);
    assert_lf_only(&agencement(&root), "fresh checkout");
    assert_eq!(git(&root, &["status", "--porcelain"]), "");

    assert_success(&generate(&root, &[]).unwrap());
    assert_lf_only(&agencement(&root), "regeneration after a fresh checkout");
    let eol = git(&root, &["ls-files", "--eol", AGENCEMENT]);
    assert!(eol.contains("w/lf"), "working copy is not LF: {eol}");
    assert_eq!(
        git(&root, &["status", "--porcelain"]),
        "",
        "regenerating {AGENCEMENT} after a fresh checkout dirtied the tree"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

// ---------------------------------------------------------------------------------------------
// QA audit, M0 round 3: edge cases around the QA-M0-F1 fix. The generator must write the same
// LF bytes on every OS, whatever line endings or byte-order mark the previous AGENCEMENT.md had
// (Windows editors and Windows PowerShell 5.1 `Set-Content -Encoding UTF8` write a BOM and CRLF),
// and `check_project_docs.py` must agree with what it wrote.
// ---------------------------------------------------------------------------------------------

/// The UTF-8 byte-order mark that Windows PowerShell 5.1 (`Set-Content -Encoding UTF8`,
/// `Out-File -Encoding utf8`) and older Notepad versions put at the start of a text file.
const BOM: &[u8] = b"\xEF\xBB\xBF";

/// The start of the message `check_project_docs.py` prints when AGENCEMENT.md does not match the
/// tree (ASCII only: Windows Python writes a piped stdout in the ANSI code page).
const OUT_OF_SYNC: &str = "[layout] AGENCEMENT.md is out of sync";

/// Runs the documentation check exactly as `cargo xtask docs` does (`<python>
/// scripts/check_project_docs.py .` from the root) and returns its stdout. The project fixtures
/// have no pilot files, so the exit code is not asserted. Returns `None` after printing
/// `skipped: python not found` when the interpreter cannot be found.
fn check_docs(root: &Path) -> Option<String> {
    let python: OsString = python_program(std::env::var_os(PYTHON_ENV));
    match Command::new(&python)
        .current_dir(root)
        .arg(CHECK_SCRIPT)
        .arg(".")
        .output()
    {
        Ok(output) => Some(String::from_utf8_lossy(&output.stdout).into_owned()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            println!("skipped: python not found ({})", python.to_string_lossy());
            None
        }
        Err(error) => panic!("cannot run {}: {error}", python.to_string_lossy()),
    }
}

/// Generates the file, annotates the `lib.rs` entry with `note`, saves that with LF and
/// regenerates it. Returns the annotated LF text and the regenerated bytes (the reference every
/// other spelling of the same file must regenerate to), or `None` when python is not found.
fn annotated_reference(root: &Path, note: &str) -> Option<(String, Vec<u8>)> {
    assert_success(&generate(root, &[])?);
    let text = String::from_utf8(agencement(root)).unwrap();
    let annotated = annotate(&text, "lib.rs", note);
    std::fs::write(root.join(AGENCEMENT), annotated.as_bytes()).unwrap();
    assert_success(&generate(root, &[]).unwrap());
    let reference = agencement(root);
    assert_lf_only(&reference, "regeneration of an annotated LF file");
    let reference_text = String::from_utf8(reference.clone()).unwrap();
    assert!(
        reference_text
            .lines()
            .any(|line| line.contains("lib.rs") && line.ends_with(&format!("  # {note}"))),
        "the hand annotation was lost:\n{reference_text}"
    );
    Some((annotated, reference))
}

/// A file saved by Windows PowerShell 5.1 (`Set-Content -Encoding UTF8`): byte-order mark plus
/// CRLF. `--check` compares the tree only, so it passes and writes nothing; regeneration drops
/// the BOM and the CRs and keeps the hand annotation, giving the LF reference bytes exactly.
#[test]
fn byte_order_mark_and_crlf_from_windows_tools_are_dropped_and_annotations_survive() {
    let root = project("docs-agencement-bom");
    let note = "core crate, annotated in PowerShell";
    let Some((annotated, reference)) = annotated_reference(&root, note) else {
        std::fs::remove_dir_all(&root).unwrap();
        return;
    };

    let mut windows = BOM.to_vec();
    windows.extend_from_slice(annotated.replace('\n', "\r\n").as_bytes());
    std::fs::write(root.join(AGENCEMENT), &windows).unwrap();

    let check = generate(&root, &["--check"]).unwrap();
    assert!(
        check.status.success(),
        "--check must compare the tree, not the BOM or the line endings: {}",
        String::from_utf8_lossy(&check.stdout)
    );
    assert_eq!(
        agencement(&root),
        windows,
        "--check must not write {AGENCEMENT}"
    );

    assert_success(&generate(&root, &[]).unwrap());
    let rewritten = agencement(&root);
    assert!(
        !rewritten.starts_with(BOM),
        "regeneration kept the byte-order mark"
    );
    assert_lf_only(&rewritten, "regeneration of a BOM + CRLF file");
    assert_eq!(
        String::from_utf8(rewritten).unwrap(),
        String::from_utf8(reference).unwrap(),
        "a BOM + CRLF copy must regenerate to the same LF bytes as the LF original"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// Line endings mixed inside one file (an editor that keeps the ending of each line it did not
/// touch) and a missing final newline regenerate to the LF reference bytes.
#[test]
fn mixed_line_endings_and_a_missing_final_newline_regenerate_to_the_lf_reference() {
    let root = project("docs-agencement-mixed");
    let note = "core crate";
    let Some((annotated, reference)) = annotated_reference(&root, note) else {
        std::fs::remove_dir_all(&root).unwrap();
        return;
    };

    let mut mixed = String::new();
    for (index, line) in annotated.lines().enumerate() {
        if index > 0 {
            mixed.push_str(if index % 2 == 0 { "\r\n" } else { "\n" });
        }
        mixed.push_str(line);
    }
    assert!(mixed.contains("\r\n") && !mixed.ends_with('\n'));
    std::fs::write(root.join(AGENCEMENT), mixed.as_bytes()).unwrap();

    assert_success(&generate(&root, &[]).unwrap());
    let rewritten = agencement(&root);
    assert_lf_only(&rewritten, "regeneration of a mixed-ending file");
    assert_eq!(
        String::from_utf8(rewritten).unwrap(),
        String::from_utf8(reference).unwrap(),
        "a mixed-ending copy must regenerate to the same LF bytes as the LF original"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// `--output FILE` goes through the same write path: LF only, the file lists itself, and
/// AGENCEMENT.md is not created.
#[test]
fn output_option_writes_lf_and_leaves_agencement_alone() {
    let root = project("docs-agencement-output");
    let Some(output) = generate(&root, &["--output", "LAYOUT.md"]) else {
        std::fs::remove_dir_all(&root).unwrap();
        return;
    };
    assert_success(&output);
    let bytes = std::fs::read(root.join("LAYOUT.md")).unwrap();
    assert_lf_only(&bytes, "--output LAYOUT.md");
    let text = String::from_utf8(bytes).unwrap();
    assert!(
        text.contains("LAYOUT.md"),
        "the output file is missing from its own tree:\n{text}"
    );
    assert!(
        !root.join(AGENCEMENT).exists(),
        "--output LAYOUT.md also created {AGENCEMENT}"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// The tree block that [`golden_project`] must produce on every OS: folders first, then files,
/// each level sorted ignoring case; `/` after folder names; the `#` column two spaces after the
/// longest label; the gitignored `bench/results/` left out while `bench/README.md` stays (the
/// layout of §2.1), and `logs/`, which holds only gitignored content, left out entirely.
const GOLDEN_TREE: &str = "\
├── alpha/                 # <!-- annotate -->\n\
│   └── x.rs               # <!-- annotate -->\n\
├── bench/                 # <!-- annotate -->\n\
│   └── README.md          # <!-- annotate -->\n\
├── café/                  # <!-- annotate -->\n\
│   └── notes.md           # <!-- annotate -->\n\
├── scripts/               # <!-- annotate -->\n\
│   └── gen_agencement.py  # <!-- annotate -->\n\
├── Zeta/                  # <!-- annotate -->\n\
│   └── y.txt              # <!-- annotate -->\n\
├── .gitattributes         # <!-- annotate -->\n\
├── .gitignore             # <!-- annotate -->\n\
├── a.md                   # <!-- annotate -->\n\
├── AGENCEMENT.md          # <!-- annotate -->\n\
└── B.md                   # <!-- annotate -->\n";

/// Creates a fresh git repository holding exactly the tree of [`GOLDEN_TREE`] (plus the
/// gitignored `bench/results/run.json` and `logs/run.json`), with the workspace's real generator
/// and attributes.
fn golden_project() -> PathBuf {
    let root = temp_dir("docs-agencement-golden");
    git(&root, &["init", "--quiet"]);
    let toplevel = git(&root, &["rev-parse", "--show-toplevel"]);
    assert_eq!(
        std::fs::canonicalize(toplevel.trim()).unwrap(),
        std::fs::canonicalize(&root).unwrap(),
        "{} is not its own git repository",
        root.display()
    );
    let workspace = xtask::locate::workspace_root().unwrap();
    for relative in [GENERATE_SCRIPT, ".gitattributes"] {
        write_file(
            &root,
            relative,
            &std::fs::read(workspace.join(relative)).unwrap(),
        );
    }
    write_file(&root, ".gitignore", b"/bench/results/\nlogs/\n");
    write_file(&root, "a.md", b"a\n");
    write_file(&root, "B.md", b"b\n");
    write_file(&root, "alpha/x.rs", b"//! x\n");
    write_file(&root, "Zeta/y.txt", b"y\n");
    write_file(&root, "caf\u{e9}/notes.md", b"notes\n");
    write_file(&root, "bench/README.md", b"# Bench\n");
    write_file(&root, "bench/results/run.json", b"{}\n");
    write_file(&root, "logs/run.json", b"{}\n");
    root
}

/// QA-M0-F1 said the file "differed between Windows and Linux". This pins the whole document,
/// byte for byte, for a tree that exercises ordering (case, folders first, dot files), a
/// non-ASCII name and a gitignored folder: the same assertion runs on Windows and on Linux, so
/// both must write identical bytes.
#[test]
fn generated_document_is_byte_identical_on_every_os() {
    let root = golden_project();
    let Some(output) = generate(&root, &[]) else {
        std::fs::remove_dir_all(&root).unwrap();
        return;
    };
    assert_success(&output);
    let root_name = root.file_name().unwrap().to_str().unwrap();
    let expected = format!(
        "# Project layout\n\
         \n\
         Generated by `python scripts/gen_agencement.py .`.\n\
         Hand-written annotations are preserved; fill in the entries marked `{TODO}` before closing the task.\n\
         \n\
         ```\n\
         {root_name}/\n\
         {GOLDEN_TREE}\
         ```\n\
         \n\
         _15 entries \u{b7} 15 to annotate._\n"
    );
    assert_eq!(
        String::from_utf8(agencement(&root)).unwrap(),
        expected,
        "the generated document differs from the OS-independent reference"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// `check_project_docs.py` (the CI `docs` job, after `make docs`) agrees with the generator:
/// the generated LF file is in sync, and so are a CRLF copy and a BOM + CRLF copy of it; adding
/// a file makes it out of sync, so the check is not vacuous.
#[test]
fn check_script_agrees_with_the_generator_whatever_the_line_endings() {
    let root = project("docs-agencement-check");
    let workspace = xtask::locate::workspace_root().unwrap();
    write_file(
        &root,
        CHECK_SCRIPT,
        &std::fs::read(workspace.join(CHECK_SCRIPT)).unwrap(),
    );
    let Some(first) = generate(&root, &[]) else {
        std::fs::remove_dir_all(&root).unwrap();
        return;
    };
    assert_success(&first);
    let generated = agencement(&root);
    let crlf = String::from_utf8(generated.clone())
        .unwrap()
        .replace('\n', "\r\n");
    let mut bom_crlf = BOM.to_vec();
    bom_crlf.extend_from_slice(crlf.as_bytes());

    for (label, bytes) in [
        ("generated LF", generated.clone()),
        ("CRLF copy", crlf.into_bytes()),
        ("BOM + CRLF copy", bom_crlf),
    ] {
        std::fs::write(root.join(AGENCEMENT), &bytes).unwrap();
        let report = check_docs(&root).unwrap();
        assert!(
            !report.contains(OUT_OF_SYNC),
            "{label}: check_project_docs.py reports the layout out of sync:\n{report}"
        );
    }

    std::fs::write(root.join(AGENCEMENT), &generated).unwrap();
    write_file(&root, "docs/extra.md", b"# Extra\n");
    let report = check_docs(&root).unwrap();
    assert!(
        report.contains(OUT_OF_SYNC),
        "a new file did not make the layout out of sync:\n{report}"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

// ---------------------------------------------------------------------------------------------
// QA, round post-measure: the manifest checks of `check_project_docs.py` that §18.6 added (row
// 18-2), as committed regression tests. Until now their acceptance check was a set of scratch
// fixtures outside the repository. Each test runs the real script on a fresh git repository
// that holds a minimal clean workspace, then adds one realistic accident.
// ---------------------------------------------------------------------------------------------

/// Message of an unlisted package (§18.6 item 1), after `[deps] <path>: `.
const IMPLICIT_MEMBER: &str = "package inside the workspace is not listed in [workspace].members";

/// Whole message of a root `[patch]` or `[replace]` table (§18.6 item 2).
const PATCH_TABLES: &str = "[deps] Cargo.toml: [patch]/[replace] tables are not allowed (they bypass cargo-deny's source checks)";

/// Root manifest of the minimal clean workspace: one member, the lint table that check 3c needs.
const CLEAN_ROOT_MANIFEST: &str = "[workspace]\nmembers = [\"crates/va-core\"]\nresolver = \"3\"\n\n[workspace.lints.rust]\nunsafe_code = \"deny\"\n";

/// A package manifest named `name`, with the `[lints]` table that check 3c accepts.
fn package_manifest(name: &str) -> String {
    format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lints]\nworkspace = true\n"
    )
}

/// Creates a fresh git repository holding both documentation scripts, a `.gitignore` and a clean
/// one-member Cargo workspace, with `root_manifest` as its root `Cargo.toml`.
fn manifest_project(tag: &str, root_manifest: &str) -> PathBuf {
    let root = temp_dir(tag);
    git(&root, &["init", "--quiet"]);
    let workspace = xtask::locate::workspace_root().unwrap();
    for relative in [GENERATE_SCRIPT, CHECK_SCRIPT] {
        write_file(
            &root,
            relative,
            &std::fs::read(workspace.join(relative)).unwrap(),
        );
    }
    write_file(&root, ".gitignore", b"target/\nxtask/cache/\n");
    write_file(&root, "Cargo.toml", root_manifest.as_bytes());
    write_file(
        &root,
        "crates/va-core/Cargo.toml",
        package_manifest("va-core").as_bytes(),
    );
    write_file(&root, "crates/va-core/src/lib.rs", b"//! Core.\n");
    root
}

/// Runs the check on `root` and returns its `[deps]` and `[lints]` lines, or `None` when python
/// is not found.
fn manifest_problems(root: &Path) -> Option<Vec<String>> {
    let report = check_docs(root)?;
    Some(
        report
            .lines()
            .filter(|line| line.starts_with("[deps]") || line.starts_with("[lints]"))
            .map(str::to_owned)
            .collect(),
    )
}

/// The fixture itself is clean, so every problem the other tests see comes from what they add.
#[test]
fn clean_manifest_fixture_has_no_manifest_problem() {
    let root = manifest_project("docs-manifest-clean", CLEAN_ROOT_MANIFEST);
    if let Some(problems) = manifest_problems(&root) {
        assert_eq!(problems, Vec::<String>::new());
    }
    std::fs::remove_dir_all(&root).unwrap();
}

/// §18.6 item 1: a package folder that `[workspace].members` forgets, in the spellings an editor
/// or an old template produces (`cargo.toml` saved on a case-insensitive file system, the legacy
/// `[project]` table), is reported once, with its path.
#[test]
fn unlisted_in_tree_packages_are_reported_as_implicit_members() {
    for (tag, path, manifest) in [
        (
            "docs-manifest-unlisted",
            "crates/va-extra/Cargo.toml",
            package_manifest("va-extra"),
        ),
        (
            "docs-manifest-lower-case",
            "crates/va-extra/cargo.toml",
            package_manifest("va-extra"),
        ),
        (
            "docs-manifest-legacy-project",
            "arch/z80/Cargo.toml",
            "[project]\nname = \"va-arch-z80\"\nversion = \"0.1.0\"\n".to_owned(),
        ),
    ] {
        let root = manifest_project(tag, CLEAN_ROOT_MANIFEST);
        write_file(&root, path, manifest.as_bytes());
        if let Some(problems) = manifest_problems(&root) {
            assert_eq!(
                problems,
                [format!("[deps] {path}: {IMPLICIT_MEMBER}")],
                "{path}"
            );
        }
        std::fs::remove_dir_all(&root).unwrap();
    }
}

/// A member listed with another spelling of its folder (`./` prefix, trailing slash) is still a
/// listed member: no false implicit-member report.
#[test]
fn other_spellings_of_a_listed_member_are_not_implicit_members() {
    for members in [
        "[\"./crates/va-core\"]",
        "[\"crates/va-core/\"]",
        "[\"crates/*\"]",
    ] {
        let manifest = CLEAN_ROOT_MANIFEST.replace("[\"crates/va-core\"]", members);
        let root = manifest_project("docs-manifest-spelling", &manifest);
        if let Some(problems) = manifest_problems(&root) {
            assert_eq!(problems, Vec::<String>::new(), "members = {members}");
        }
        std::fs::remove_dir_all(&root).unwrap();
    }
}

/// Packages that cargo itself writes under a `target` folder (`cargo package` copies the
/// manifest to `target/package/<name>-<version>/Cargo.toml`) and packages under a gitignored
/// folder are not part of the source tree, so they are not reported (§18.6: the walk skips the
/// paths that are already skipped).
#[test]
fn packages_under_target_and_gitignored_folders_are_not_reported() {
    let root = manifest_project("docs-manifest-skipped", CLEAN_ROOT_MANIFEST);
    write_file(
        &root,
        "target/package/va-core-0.1.0/Cargo.toml",
        package_manifest("va-core").as_bytes(),
    );
    write_file(
        &root,
        "crates/va-core/target/package/va-core-0.1.0/Cargo.toml",
        package_manifest("va-core").as_bytes(),
    );
    write_file(
        &root,
        "xtask/cache/vendored/Cargo.toml",
        package_manifest("vendored").as_bytes(),
    );
    if let Some(problems) = manifest_problems(&root) {
        assert_eq!(problems, Vec::<String>::new());
    }
    std::fs::remove_dir_all(&root).unwrap();
}

/// §18.6 item 2: a root `[patch]` for any registry or git source, and a root `[replace]`, each
/// give exactly the specified message, as does a `[patch]` left behind next to a clean workspace.
#[test]
fn root_patch_and_replace_tables_are_reported() {
    for table in [
        "[patch.crates-io]\nserde = { path = \"../serde\" }\n",
        "[patch.\"https://github.com/example/x\"]\nx = { path = \"x\" }\n",
        "[patch.crates-io.serde]\npath = \"../serde\"\n",
        "[replace]\n\"serde:1.0.229\" = { path = \"../serde\" }\n",
    ] {
        let manifest = format!("{CLEAN_ROOT_MANIFEST}\n{table}");
        let root = manifest_project("docs-manifest-patch", &manifest);
        if let Some(problems) = manifest_problems(&root) {
            assert_eq!(problems, [PATCH_TABLES], "{table}");
        }
        std::fs::remove_dir_all(&root).unwrap();
    }
}
