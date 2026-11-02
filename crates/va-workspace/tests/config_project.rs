//! Tests of `.visualasm/project.toml` (m4-editor-shell.md §4.5, §10.1): the keys and their
//! defaults, the reserved tables kept raw, paths that leave the project (`..`, absolute, rooted,
//! drive prefixes) refused with `cfg-bad-path`, and the app's unknown-`arch` problem.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use va_core::diag::DiagArg;
use va_core::source::FileId;
use va_workspace::config::{
    ConfigProblem, PROJECT_FILE, Project, parse_project, project_arch_problem,
};
use va_workspace::i18n::{Lang, Localizer};

/// The English configuration messages.
const CONFIG_EN: &str = include_str!("../../../locales/en/config.ftl");
/// The French configuration messages.
const CONFIG_FR: &str = include_str!("../../../locales/fr/config.ftl");

/// The project root given to the parser (never read).
fn root() -> PathBuf {
    PathBuf::from("projects").join("tp1")
}

/// Parses `text` as the project file of [`root`].
fn parse(text: &str) -> (Project, Vec<ConfigProblem>) {
    parse_project(&root(), Some(text))
}

/// The project of [`root`] without a file.
fn empty_project() -> Project {
    Project {
        root: root(),
        arch: None,
        io_profile: None,
        entry: None,
        include_dirs: Vec::new(),
        reserved: BTreeMap::new(),
    }
}

/// A problem as `(code, line, column)`.
fn summary(problem: &ConfigProblem) -> (&'static str, u32, u32) {
    (problem.code.0, problem.at.line, problem.at.column)
}

/// The argument `name` of a problem.
fn arg<'p>(problem: &'p ConfigProblem, name: &str) -> &'p DiagArg {
    &problem
        .args
        .iter()
        .find(|(arg, _)| *arg == name)
        .unwrap_or_else(|| panic!("{} has no argument {name}", problem.code.0))
        .1
}

/// A string argument value.
fn string_arg(value: &str) -> DiagArg {
    DiagArg::Str(value.to_owned())
}

/// Writes `value` as a TOML basic string (backslashes doubled).
fn toml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\"))
}

#[test]
fn the_project_file_lives_in_the_visualasm_folder() {
    assert_eq!(PROJECT_FILE, ".visualasm/project.toml");
}

#[test]
fn a_missing_or_empty_file_gives_the_defaults() {
    assert_eq!(parse_project(&root(), None), (empty_project(), Vec::new()));
    for text in ["", "schema_version = 1\n", "[project]\n"] {
        assert_eq!(parse(text), (empty_project(), Vec::new()), "{text:?}");
    }
}

#[test]
fn every_key_is_read() {
    let text = "schema_version = 1\n[project]\narch = \"m68k\"\nio_profile = \"easy68k\"\nentry = \"src/main.asm\"\ninclude_dirs = [\"inc\", \"lib/macros\", \"lib\\\\win\"]\n";
    let (project, problems) = parse(text);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(
        project,
        Project {
            root: root(),
            arch: Some("m68k".to_owned()),
            io_profile: Some("easy68k".to_owned()),
            entry: Some(PathBuf::from("src/main.asm")),
            include_dirs: vec![
                PathBuf::from("inc"),
                PathBuf::from("lib/macros"),
                PathBuf::from("lib\\win")
            ],
            reserved: BTreeMap::new(),
        }
    );
    // `arch` and `io_profile` are stored as written: the app checks `arch`.
    let (project, problems) = parse("[project]\narch = \"Z80\"\nio_profile = \"\"\n");
    assert!(problems.is_empty());
    assert_eq!(project.arch.as_deref(), Some("Z80"));
    assert_eq!(project.io_profile.as_deref(), Some(""));
}

#[test]
fn reserved_tables_are_kept_raw_without_warnings() {
    let text = "[project]\narch = \"m68k\"\n[assembler]\nlisting = true\nlevel = 3\n[assembler.defines]\nDEBUG = 1\n[output]\nformat = \"srec\"\n[run]\nspeed = 1.5\nwhen = 1979-05-27\n[debug]\nbreak = [\"main\", \"loop\"]\n";
    let (project, problems) = parse(text);
    assert!(problems.is_empty(), "{problems:?}");
    let expected: toml::Table = text.parse().unwrap();
    let names: Vec<&str> = project.reserved.keys().map(String::as_str).collect();
    assert_eq!(names, ["assembler", "debug", "output", "run"]);
    for (name, table) in &project.reserved {
        assert_eq!(
            Some(table),
            expected[name.as_str()].as_table(),
            "{name} is kept as written"
        );
    }
    let assembler = &project.reserved["assembler"];
    assert_eq!(assembler["level"].as_integer(), Some(3));
    assert_eq!(assembler["defines"]["DEBUG"].as_integer(), Some(1));
}

#[test]
fn a_reserved_name_that_is_not_a_table_is_reported() {
    let (project, problems) = parse("run = \"fast\"\n");
    assert!(project.reserved.is_empty());
    assert_eq!(summary(&problems[0]), ("cfg-wrong-type", 1, 7));
    assert_eq!(arg(&problems[0], "expected"), &string_arg("table"));
}

#[test]
fn contained_relative_paths_are_accepted() {
    for value in [
        "main.asm",
        "src/main.asm",
        "src\\main.asm",
        "./main.asm",
        "a..b/c.asm",
        "...",
        "src/./x.asm",
        "dossier élève/main.asm",
    ] {
        let (project, problems) = parse(&format!("[project]\nentry = {}\n", toml_string(value)));
        assert!(problems.is_empty(), "{value}: {problems:?}");
        assert_eq!(project.entry, Some(PathBuf::from(value)), "{value}");
    }
}

#[test]
fn paths_that_leave_the_project_are_bad_paths() {
    for value in [
        "..",
        "../x.asm",
        "src/../../x.asm",
        "src/..",
        "a\\..\\b.asm",
        "..\\x.asm",
        "/abs/main.asm",
        "\\rooted\\main.asm",
        "\\\\server\\share\\main.asm",
        "//server/share/main.asm",
        "C:\\x\\main.asm",
        "c:/x/main.asm",
        "C:main.asm",
        "\\\\?\\C:\\main.asm",
        "",
    ] {
        let text = format!("[project]\nentry = {}\n", toml_string(value));
        let (project, problems) = parse(&text);
        assert_eq!(project.entry, None, "{value:?}");
        assert_eq!(problems.len(), 1, "{value:?}: {problems:?}");
        assert_eq!(summary(&problems[0]), ("cfg-bad-path", 2, 9), "{value:?}");
        assert_eq!(arg(&problems[0], "key"), &string_arg("project.entry"));
        assert_eq!(arg(&problems[0], "value"), &string_arg(value));
    }
}

#[test]
fn bad_include_folders_are_dropped_and_the_others_kept() {
    let text = "[project]\ninclude_dirs = [\"inc\", \"../shared\", 7, \"/usr/include\", \"lib\"]\n";
    let (project, problems) = parse(text);
    assert_eq!(
        project.include_dirs,
        [PathBuf::from("inc"), PathBuf::from("lib")]
    );
    let found: Vec<_> = problems.iter().map(summary).collect();
    assert_eq!(
        found,
        [
            ("cfg-bad-path", 2, 24),
            ("cfg-wrong-type", 2, 37),
            ("cfg-bad-path", 2, 40)
        ]
    );
    assert!(
        problems
            .iter()
            .all(|p| arg(p, "key") == &string_arg("project.include_dirs"))
    );
    assert_eq!(arg(&problems[1], "expected"), &string_arg("string"));
    // An empty list is no include folder; a value that is not a list keeps the default.
    let (project, problems) = parse("[project]\ninclude_dirs = []\n");
    assert!(problems.is_empty() && project.include_dirs.is_empty());
    let (project, problems) = parse("[project]\ninclude_dirs = \"inc\"\n");
    assert!(project.include_dirs.is_empty());
    assert_eq!(summary(&problems[0]), ("cfg-wrong-type", 2, 16));
    assert_eq!(arg(&problems[0], "expected"), &string_arg("array"));
}

#[test]
fn wrong_types_and_unknown_keys_keep_the_neighbours() {
    let text = "name = \"tp1\"\n[project]\narch = 68000\nio_profile = \"easy68k\"\nentry = 1\nauthor = \"me\"\n[projet]\narch = \"m68k\"\nproject = 3\n";
    let (project, problems) = parse(text);
    assert_eq!(project.arch, None);
    assert_eq!(project.io_profile.as_deref(), Some("easy68k"));
    assert_eq!(project.entry, None);
    let found: Vec<_> = problems.iter().map(summary).collect();
    assert_eq!(
        found,
        [
            ("cfg-unknown-key", 1, 1),
            ("cfg-wrong-type", 3, 8),
            ("cfg-wrong-type", 5, 9),
            ("cfg-unknown-key", 6, 1),
            ("cfg-unknown-key", 7, 2),
        ]
    );
    assert_eq!(arg(&problems[3], "key"), &string_arg("project.author"));
    assert_eq!(arg(&problems[4], "key"), &string_arg("projet"));
}

#[test]
fn another_schema_version_or_a_syntax_error_gives_the_defaults() {
    let (project, problems) = parse("schema_version = 2\n[project]\narch = \"m68k\"\n");
    assert_eq!(project, empty_project());
    assert_eq!(problems.len(), 1);
    assert_eq!(summary(&problems[0]), ("cfg-schema-version", 1, 18));
    let (project, problems) = parse("[project]\narch = m68k\n");
    assert_eq!(project, empty_project());
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].code.0, "cfg-syntax");
}

#[test]
fn problems_name_the_project_file() {
    let (_, problems) = parse("[project]\nentry = \"../x.asm\"\n");
    assert_eq!(problems[0].path, root().join(Path::new(PROJECT_FILE)));
}

#[test]
fn an_unknown_arch_is_reported_for_the_app() {
    let known = ["m68k"];
    let text = "[project]\nio_profile = \"easy68k\"\narch = \"z80\"\n";
    let problem = project_arch_problem(&root(), Some(text), &known).unwrap();
    assert_eq!(summary(&problem), ("cfg-invalid-value", 3, 8));
    assert_eq!(
        problem.args,
        [
            ("key", string_arg("project.arch")),
            ("value", string_arg("z80")),
            ("expected", string_arg("m68k")),
        ]
    );
    assert_eq!(problem.range.slice(text), Some("\"z80\""));
    assert_eq!(problem.path, root().join(PROJECT_FILE));
    // A known arch, no arch, no file, a bad file or a non-string arch: nothing to add.
    for text in [
        Some("[project]\narch = \"m68k\"\n"),
        Some("[project]\nentry = \"main.asm\"\n"),
        None,
        Some("[project\narch = \"z80\"\n"),
        Some("[project]\narch = 1\n"),
        Some("schema_version = 2\n[project]\narch = \"z80\"\n"),
    ] {
        assert_eq!(
            project_arch_problem(&root(), text, &known),
            None,
            "{text:?}"
        );
    }
    let problem = project_arch_problem(&root(), Some(text), &["m68k", "z80x"]).unwrap();
    assert_eq!(arg(&problem, "expected"), &string_arg("m68k, z80x"));
}

#[test]
fn project_problems_render_in_both_languages() {
    let text = "[project]\nentry = \"../x.asm\"\narch = \"z80\"\n";
    let (_, mut problems) = parse(text);
    problems.extend(project_arch_problem(&root(), Some(text), &["m68k"]));
    assert_eq!(problems.len(), 2);
    for (lang, primary) in [(Lang::En, CONFIG_EN), (Lang::Fr, CONFIG_FR)] {
        let localizer = Localizer::new(lang, &[primary], &[CONFIG_EN]).unwrap();
        for problem in &problems {
            let rendered = localizer.render(&problem.to_diagnostic(FileId(3)));
            assert!(!rendered.message.contains("{$"), "{}", rendered.message);
            assert!(!rendered.help.unwrap().contains("{$"));
        }
    }
    let french = Localizer::new(Lang::Fr, &[CONFIG_FR], &[CONFIG_EN]).unwrap();
    assert_eq!(
        french.render(&problems[0].to_diagnostic(FileId(3))).message,
        "« ../x.asm » ne convient pas pour project.entry : le chemin doit rester dans le dossier du projet ; il est ignoré"
    );
    let english = Localizer::new(Lang::En, &[CONFIG_EN], &[CONFIG_EN]).unwrap();
    assert_eq!(
        english
            .render(&problems[1].to_diagnostic(FileId(3)))
            .help
            .unwrap(),
        "The accepted values are: m68k. Write one of them in lowercase, between double quotes."
    );
}
