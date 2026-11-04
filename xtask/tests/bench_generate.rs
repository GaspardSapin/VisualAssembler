//! Tests for `src/bench/generate.rs`: the template lines of the generated bench inputs and
//! `ensure_inputs` (m4-editor-shell.md §7.1, §10.3): 100,000 and 5,000 lines, LF only,
//! deterministic, unique labels, regenerated on a version change or a missing file, kept
//! otherwise. Every test generates into its own `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` root.
#![allow(clippy::unwrap_used)]

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};

use xtask::bench::generate::{
    BenchInputs, GENERATOR_VERSION, InputsStatus, LARGE_FILE, LARGE_LINES, PROJECT_DIR,
    PROJECT_FILE_LINES, PROJECT_LINES, PROJECT_MAIN, PROJECT_PARTS, PROJECT_TOML,
    PROJECT_TOML_TEXT, TEMPLATE_CYCLE, VERSION_FILE, ensure_inputs, ensure_inputs_with_status,
    generated_dir, generated_files, include_line, inputs_are_current, main_text, part_name,
    template_line, template_text,
};

/// Returns a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>`, created with
/// `create_dir`, which fails if the path already exists.
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    dir
}

/// Reads `path` as UTF-8.
fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

/// Returns the labels a file defines: the column-1 word of each line that is neither a comment
/// nor empty, without its trailing colons (`l20`, `31$`).
fn labels(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.starts_with([';', '*', '\t', ' ']) && !line.is_empty())
        .map(|line| {
            let word = line.split(['\t', ' ']).next().unwrap();
            word.trim_end_matches(':').to_owned()
        })
        .collect()
}

/// Asserts that `text` is LF only, ends with one line break and holds `lines` lines.
fn assert_lf_lines(text: &str, lines: usize, what: &str) {
    assert!(!text.contains('\r'), "{what}: CR found");
    assert!(text.ends_with('\n'), "{what}: no final line break");
    assert!(!text.ends_with("\n\n"), "{what}: empty last line");
    assert_eq!(text.lines().count(), lines, "{what}");
}

#[test]
fn generator_constants_follow_the_spec() {
    assert_eq!(GENERATOR_VERSION, 1);
    assert_eq!(TEMPLATE_CYCLE, 20);
    assert_eq!(LARGE_FILE, "100k.asm");
    assert_eq!(LARGE_LINES, 100_000);
    assert_eq!(PROJECT_DIR, "5k");
    assert_eq!(PROJECT_MAIN, "main.asm");
    assert_eq!(PROJECT_PARTS, 9);
    assert_eq!(PROJECT_FILE_LINES, 500);
    assert_eq!(PROJECT_LINES, 5_000);
    assert_eq!(PROJECT_TOML, ".visualasm/project.toml");
    assert_eq!(VERSION_FILE, "VERSION");
    assert_eq!(
        generated_dir(Path::new("root")),
        Path::new("root").join("bench").join("generated")
    );
    assert_eq!(part_name(1), "part01.asm");
    assert_eq!(part_name(9), "part09.asm");
}

#[test]
fn template_lines_are_deterministic_single_lines_that_cycle() {
    for n in 0..200 {
        let line = template_line(n);
        assert_eq!(line, template_line(n), "line {n}");
        assert!(!line.is_empty(), "line {n}");
        assert!(!line.contains(['\n', '\r']), "line {n}: {line:?}");
        assert!(line.is_ascii(), "line {n}: {line:?}");
        // Lines of the same slot differ only by the numbers they carry.
        let without_digits = |text: &str| text.replace(|c: char| c.is_ascii_digit(), "");
        assert_eq!(
            without_digits(&line),
            without_digits(&template_line(n + TEMPLATE_CYCLE)),
            "slot {}",
            n % TEMPLATE_CYCLE
        );
    }
}

/// The exact first cycle: labels and local labels with the line number, the twelve addressing
/// modes, `DC.B` strings, both comment forms, the `SuspectComment` line and the invalid line.
#[test]
fn the_template_cycle_covers_the_required_constructs() {
    let cycle: Vec<String> = (0..TEMPLATE_CYCLE).map(template_line).collect();
    assert_eq!(
        cycle,
        [
            "l0:\tmoveq\t#42,d0\t; immediate data",
            "\tmovea.l\ta0,a1\t; address register direct",
            "\tmove.w\t(a0),d2\t; address register indirect",
            "\tmove.b\t(a1)+,d3\t; post-increment",
            "\tmove.l\td4,-(sp)\t; pre-decrement",
            "l5:\tmove.w\t8(a2),d5\t; 16-bit displacement",
            "\tmove.b\t2(a3,d1.w),d6\t; index and displacement",
            "\tmove.w\t$1000.w,d7\t; absolute short",
            "\tmove.l\t$00ff0000,d0\t; absolute long",
            "\tlea\tl0(pc),a0\t; program counter relative",
            "\tmove.w\tl0(pc,d0.w),d1\t; pc with index",
            "11$:\tsubq.w\t#1,d0\t; local label",
            "\tbne.s\t11$\t; back to the local label",
            "; line 13 is a comment of our own",
            "* a comment that starts in column one",
            "l15:\tdc.b\t'bench text',0",
            "\tdc.b\t\"line 16\",13,10,0",
            "\tdc.b\t1, 2",
            "\tmove.x\td0,d1",
            "\trts",
        ]
    );
    let text = cycle.join("\n");
    for mode in [
        ",d0",            // data register direct
        "a0,a1",          // address register direct
        "(a0),",          // address register indirect
        "(a1)+,",         // post-increment
        "-(sp)",          // pre-decrement
        "8(a2),",         // 16-bit displacement
        "2(a3,d1.w),",    // index
        "$1000.w,",       // absolute short
        "$00ff0000,",     // absolute long
        "l0(pc),",        // PC relative
        "l0(pc,d0.w),",   // PC relative with index
        "#42,",           // immediate
        "'bench text'",   // DC.B string, single quotes
        "\"line 16\"",    // DC.B string, double quotes
        "\tdc.b\t1, 2\n", // the SuspectComment line (lex-operands-in-comment)
        "move.x",         // the invalid line (lex-bad-size-suffix)
    ] {
        assert!(text.contains(mode), "{mode} missing");
    }
}

/// References name labels of the same cycle defined on earlier lines, so each file stays
/// self-consistent wherever it is cut.
#[test]
fn references_name_earlier_labels_of_the_same_cycle() {
    assert_eq!(
        template_line(99_989),
        "\tlea\tl99980(pc),a0\t; program counter relative"
    );
    assert_eq!(
        template_line(99_990),
        "\tmove.w\tl99980(pc,d0.w),d1\t; pc with index"
    );
    assert_eq!(
        template_line(99_991),
        "99991$:\tsubq.w\t#1,d0\t; local label"
    );
    assert_eq!(
        template_line(99_992),
        "\tbne.s\t99991$\t; back to the local label"
    );
    assert_eq!(
        template_line(99_980),
        "l99980:\tmoveq\t#42,d0\t; immediate data"
    );
}

#[test]
fn template_text_is_the_lines_with_lf_terminators() {
    assert_eq!(template_text(0), "");
    assert_eq!(
        template_text(2),
        format!("{}\n{}\n", template_line(0), template_line(1))
    );
}

#[test]
fn main_ends_with_one_include_line_per_part() {
    let text = main_text();
    assert_lf_lines(&text, PROJECT_FILE_LINES, "main.asm");
    let lines: Vec<&str> = text.lines().collect();
    let (body, includes) = lines.split_at(PROJECT_FILE_LINES - PROJECT_PARTS);
    for (n, line) in body.iter().enumerate() {
        assert_eq!(*line, template_line(n), "line {n}");
    }
    assert_eq!(
        includes,
        (1..=PROJECT_PARTS)
            .map(include_line)
            .collect::<Vec<_>>()
            .as_slice()
    );
    assert_eq!(include_line(3), "\tINCLUDE\t\"part03.asm\"");
}

#[test]
fn ensure_inputs_writes_100k_and_5k_lines_lf_only() {
    let root = temp_dir("generate-fresh");
    let (inputs, status) = ensure_inputs_with_status(&root).unwrap();
    assert_eq!(status, InputsStatus::Generated);
    let dir = generated_dir(&root);
    assert_eq!(
        inputs,
        BenchInputs {
            large: dir.join("100k.asm"),
            project: dir.join("5k"),
        }
    );

    let large = read(&inputs.large);
    assert_lf_lines(&large, LARGE_LINES, "100k.asm");
    assert_eq!(large, template_text(LARGE_LINES));
    let average = large.len() as f64 / LARGE_LINES as f64;
    assert!((30.0..=36.0).contains(&average), "{average} bytes per line");

    let project = &inputs.project;
    assert_eq!(
        read(&project.join(".visualasm").join("project.toml")),
        PROJECT_TOML_TEXT
    );
    assert_eq!(read(&project.join("main.asm")), main_text());
    let mut total = read(&project.join("main.asm")).lines().count();
    for index in 1..=PROJECT_PARTS {
        let part = read(&project.join(part_name(index)));
        assert_lf_lines(&part, PROJECT_FILE_LINES, &part_name(index));
        assert_eq!(part, template_text(PROJECT_FILE_LINES));
        total += part.lines().count();
    }
    assert_eq!(total, 5_000);

    let mut asm: Vec<String> = fs::read_dir(project)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".asm"))
        .collect();
    asm.sort();
    let mut expected = vec!["main.asm".to_owned()];
    expected.extend((1..=PROJECT_PARTS).map(part_name));
    assert_eq!(asm, expected, "the .asm files the idle scenario opens");
    assert_eq!(read(&dir.join("VERSION")), "1\n");
    assert!(inputs_are_current(&dir));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn every_generated_file_defines_unique_labels() {
    let root = temp_dir("generate-labels");
    let inputs = ensure_inputs(&root).unwrap();
    let mut files = vec![inputs.large.clone(), inputs.project.join("main.asm")];
    files.extend((1..=PROJECT_PARTS).map(|index| inputs.project.join(part_name(index))));
    for file in files {
        let found = labels(&read(&file));
        let unique: HashSet<&String> = found.iter().collect();
        assert_eq!(unique.len(), found.len(), "{}", file.display());
        assert!(found.iter().all(|label| {
            let digits = label.trim_start_matches('l').trim_end_matches('$');
            !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
        }));
    }
    let large = labels(&read(&inputs.large));
    // Four labels per cycle: l<n> on slots 0, 5 and 15, the local label <n>$ on slot 11.
    assert_eq!(large.len(), LARGE_LINES / TEMPLATE_CYCLE * 4);
    assert_eq!(large[..4], ["l0", "l5", "11$", "l15"]);
    assert_eq!(large.last().unwrap(), "l99995");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn generation_is_byte_identical_in_two_roots() {
    let first = temp_dir("generate-first");
    let second = temp_dir("generate-second");
    ensure_inputs(&first).unwrap();
    ensure_inputs(&second).unwrap();
    let files = generated_files(&generated_dir(&first));
    assert_eq!(files.len(), 3 + PROJECT_PARTS);
    for file in files {
        let relative = file.strip_prefix(&first).unwrap();
        assert_eq!(
            fs::read(&file).unwrap(),
            fs::read(second.join(relative)).unwrap(),
            "{}",
            relative.display()
        );
    }
    fs::remove_dir_all(&first).unwrap();
    fs::remove_dir_all(&second).unwrap();
}

#[test]
fn current_inputs_are_kept() {
    let root = temp_dir("generate-kept");
    let inputs = ensure_inputs(&root).unwrap();
    let marker = "; kept by ensure_inputs\n";
    fs::write(&inputs.large, marker).unwrap();
    fs::write(inputs.project.join("part04.asm"), marker).unwrap();
    let (again, status) = ensure_inputs_with_status(&root).unwrap();
    assert_eq!(status, InputsStatus::Current);
    assert_eq!(again, inputs);
    assert_eq!(read(&inputs.large), marker);
    assert_eq!(read(&inputs.project.join("part04.asm")), marker);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_different_or_missing_version_regenerates_everything() {
    let root = temp_dir("generate-version");
    let inputs = ensure_inputs(&root).unwrap();
    let version = generated_dir(&root).join("VERSION");
    for stale in [Some("0\n"), Some("2"), Some(""), Some("1.0\n"), None] {
        fs::write(&inputs.large, "stale\n").unwrap();
        fs::write(inputs.project.join("extra.asm"), "stale\n").unwrap();
        match stale {
            Some(text) => fs::write(&version, text).unwrap(),
            None => fs::remove_file(&version).unwrap(),
        }
        assert!(!inputs_are_current(&generated_dir(&root)), "{stale:?}");
        let (_, status) = ensure_inputs_with_status(&root).unwrap();
        assert_eq!(status, InputsStatus::Generated, "{stale:?}");
        assert_eq!(read(&inputs.large), template_text(LARGE_LINES), "{stale:?}");
        assert!(
            !inputs.project.join("extra.asm").exists(),
            "a stale project file stays after {stale:?}"
        );
        assert_eq!(read(&version), "1\n", "{stale:?}");
    }
    // Surrounding whitespace in VERSION is accepted (an editor may add a CRLF).
    fs::write(&version, " 1\r\n").unwrap();
    assert!(inputs_are_current(&generated_dir(&root)));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_missing_file_regenerates_everything() {
    let root = temp_dir("generate-missing");
    let inputs = ensure_inputs(&root).unwrap();
    let dir = generated_dir(&root);
    for file in generated_files(&dir) {
        fs::write(&inputs.large, "stale\n").unwrap();
        fs::remove_file(&file).unwrap();
        assert!(!inputs_are_current(&dir), "{}", file.display());
        let (_, status) = ensure_inputs_with_status(&root).unwrap();
        assert_eq!(status, InputsStatus::Generated, "{}", file.display());
        assert!(file.is_file(), "{}", file.display());
        assert_eq!(read(&inputs.large), template_text(LARGE_LINES));
    }
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_file_in_place_of_the_project_folder_is_replaced() {
    let root = temp_dir("generate-blocked");
    let dir = generated_dir(&root);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("5k"), "not a folder").unwrap();
    let inputs = ensure_inputs(&root).unwrap();
    assert!(inputs.project.is_dir());
    assert!(inputs_are_current(&dir));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn an_unwritable_generated_folder_fails_with_context() {
    let root = temp_dir("generate-unwritable");
    fs::write(root.join("bench"), "not a folder").unwrap();
    let error = format!("{:#}", ensure_inputs(&root).unwrap_err());
    assert!(error.contains("cannot create"), "{error}");
    assert!(error.contains("generated"), "{error}");
    fs::remove_dir_all(&root).unwrap();
}
