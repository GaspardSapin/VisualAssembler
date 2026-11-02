//! Tests of `settings.toml` and the configuration problems (m4-editor-shell.md §4.2, §4.3,
//! §10.1): defaults, the three `DEFAULT_SETTINGS_TOML` checks, every key valid, of the wrong type,
//! out of range or invalid (with its line and column), unknown keys, schema versions, syntax
//! errors, `read_config_text` limits, and the `diag-cfg-*` messages of `locales/*/config.ftl`.

#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use va_core::diag::{DiagArg, Severity};
use va_core::source::{FileId, LineCol, TextRange};
use va_workspace::config::{
    AutosaveMode, CONFIG_CODES, ConfigProblem, DEFAULT_SETTINGS_TOML, EolKind, MAX_CONFIG_BYTES,
    RendererKind, Settings, ThemeKind, parse_settings, read_config_text,
};
use va_workspace::i18n::{Lang, Localizer};

/// The English configuration messages.
const CONFIG_EN: &str = include_str!("../../../locales/en/config.ftl");
/// The French configuration messages.
const CONFIG_FR: &str = include_str!("../../../locales/fr/config.ftl");

/// The path given to the parser (never read).
fn settings_path() -> PathBuf {
    PathBuf::from("config").join("settings.toml")
}

/// Parses `text` as `settings.toml`.
fn parse(text: &str) -> (Settings, Vec<ConfigProblem>) {
    parse_settings(&settings_path(), Some(text))
}

/// A problem as `(code, line, column)`.
fn summary(problem: &ConfigProblem) -> (&'static str, u32, u32) {
    (problem.code.0, problem.at.line, problem.at.column)
}

/// The string argument `name` of a problem.
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

// -------------------------------------------------------------------------------------------
// Defaults and the default file
// -------------------------------------------------------------------------------------------

/// The keys of the §4.3 table, in its order, with their table.
const SCHEMA_ROWS: [(&str, &str); 18] = [
    ("ui", "language"),
    ("ui", "theme"),
    ("ui", "native_dialogs"),
    ("editor", "font_size"),
    ("editor", "font_file"),
    ("editor", "tab_width"),
    ("editor", "insert_spaces"),
    ("editor", "cursor_blink"),
    ("editor", "line_numbers"),
    ("editor", "highlight_current_line"),
    ("editor", "diagnostics_delay_ms"),
    ("files", "hot_exit"),
    ("files", "backup_delay_ms"),
    ("files", "autosave"),
    ("files", "autosave_delay_ms"),
    ("files", "eol"),
    ("explorer", "exclude"),
    ("render", "renderer"),
];

/// The key of a line matching `^# [a-z_]+ = `, if it is one.
fn commented_key(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("# ")?;
    let end = rest
        .find(|c: char| !(c.is_ascii_lowercase() || c == '_'))
        .unwrap_or(rest.len());
    (end > 0 && rest[end..].starts_with(" = ")).then(|| &rest[..end])
}

#[test]
fn a_missing_file_gives_the_defaults() {
    let (settings, problems) = parse_settings(&settings_path(), None);
    assert_eq!(settings, Settings::default());
    assert!(problems.is_empty());
    let defaults = Settings::default();
    assert_eq!(defaults.ui.language, None);
    assert_eq!(defaults.editor.font_size, 14.0);
    assert_eq!(defaults.editor.tab_width, 8);
    assert!(!defaults.editor.insert_spaces);
    assert_eq!(defaults.files.autosave, AutosaveMode::Off);
    assert_eq!(defaults.explorer.exclude, [".git"]);
    // An empty file and a file with only the schema version are the defaults too.
    for text in ["", "schema_version = 1\n", "# only a comment\n"] {
        assert_eq!(parse(text), (Settings::default(), Vec::new()), "{text:?}");
    }
}

#[test]
fn the_default_file_parses_to_the_defaults_without_problem() {
    assert_eq!(
        parse(DEFAULT_SETTINGS_TOML),
        (Settings::default(), Vec::new())
    );
    assert!(DEFAULT_SETTINGS_TOML.is_ascii());
    assert!(!DEFAULT_SETTINGS_TOML.contains('\r'));
    assert!(DEFAULT_SETTINGS_TOML.ends_with('\n') && !DEFAULT_SETTINGS_TOML.ends_with("\n\n"));
}

#[test]
fn the_default_file_lists_every_key_once_in_table_order() {
    let mut table = "";
    let mut found = Vec::new();
    for line in DEFAULT_SETTINGS_TOML.lines() {
        if let Some(header) = line.strip_prefix('[') {
            table = header.trim_end_matches(']');
        } else if let Some(key) = commented_key(line) {
            found.push((table, key));
        }
    }
    assert_eq!(found, SCHEMA_ROWS);
}

#[test]
fn the_default_file_uncommented_still_parses_to_the_defaults() {
    let mut uncommented = 0;
    let text: String = DEFAULT_SETTINGS_TOML
        .lines()
        .map(|line| {
            if commented_key(line).is_some() {
                uncommented += 1;
                format!("{}\n", &line[2..])
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    assert_eq!(uncommented, SCHEMA_ROWS.len());
    assert_eq!(parse(&text), (Settings::default(), Vec::new()));
}

// -------------------------------------------------------------------------------------------
// Valid values
// -------------------------------------------------------------------------------------------

#[test]
fn every_key_takes_a_valid_value() {
    let text = r#"schema_version = 1
[ui]
language = "fr"
theme = "dark"
native_dialogs = false
[editor]
font_size = 18.5
font_file = "fonts/Code.TTF"
tab_width = 4
insert_spaces = true
cursor_blink = false
line_numbers = false
highlight_current_line = false
diagnostics_delay_ms = 50
[files]
hot_exit = false
backup_delay_ms = 60000
autosave = "after-delay"
autosave_delay_ms = 250
eol = "crlf"
[explorer]
exclude = ["build", ".git", "node_modules"]
[render]
renderer = "wgpu"
"#;
    let (settings, problems) = parse(text);
    assert!(problems.is_empty(), "{problems:?}");
    let mut expected = Settings::default();
    expected.ui.language = Some(Lang::Fr);
    expected.ui.theme = Some(ThemeKind::Dark);
    expected.ui.native_dialogs = false;
    expected.editor.font_size = 18.5;
    expected.editor.font_file = Some(PathBuf::from("fonts/Code.TTF"));
    expected.editor.tab_width = 4;
    expected.editor.insert_spaces = true;
    expected.editor.cursor_blink = false;
    expected.editor.line_numbers = false;
    expected.editor.highlight_current_line = false;
    expected.editor.diagnostics_delay_ms = 50;
    expected.files.hot_exit = false;
    expected.files.backup_delay_ms = 60_000;
    expected.files.autosave = AutosaveMode::AfterDelay;
    expected.files.autosave_delay_ms = 250;
    expected.files.eol = Some(EolKind::CrLf);
    expected.explorer.exclude = vec!["build".into(), ".git".into(), "node_modules".into()];
    expected.render.renderer = Some(RendererKind::Wgpu);
    assert_eq!(settings, expected);
}

#[test]
fn the_other_ids_parse_too() {
    let (settings, problems) = parse(
        "[ui]\nlanguage = \"en\"\ntheme = \"light\"\n[files]\nautosave = \"on-focus-change\"\neol = \"lf\"\n[render]\nrenderer = \"glow\"\n",
    );
    assert!(problems.is_empty());
    assert_eq!(settings.ui.language, Some(Lang::En));
    assert_eq!(settings.ui.theme, Some(ThemeKind::Light));
    assert_eq!(settings.files.autosave, AutosaveMode::OnFocusChange);
    assert_eq!(settings.files.eol, Some(EolKind::Lf));
    assert_eq!(settings.render.renderer, Some(RendererKind::Glow));
    let (settings, problems) = parse("[files]\nautosave = \"off\"\nautosave = \"x\"\n");
    assert_eq!(problems.len(), 1, "a duplicate key is a syntax error");
    assert_eq!(problems[0].code.0, "cfg-syntax");
    assert_eq!(settings, Settings::default());
}

#[test]
fn auto_gives_none_without_a_problem() {
    let (settings, problems) = parse(
        "[ui]\nlanguage = \"auto\"\ntheme = \"auto\"\n[files]\neol = \"auto\"\n[render]\nrenderer = \"auto\"\n",
    );
    assert!(problems.is_empty());
    assert_eq!(settings, Settings::default());
    // `autosave` has no `auto`.
    let (settings, problems) = parse("[files]\nautosave = \"auto\"\n");
    assert_eq!(settings.files.autosave, AutosaveMode::Off);
    assert_eq!(summary(&problems[0]), ("cfg-invalid-value", 2, 12));
}

#[test]
fn ranges_are_inclusive() {
    let text = "[editor]\nfont_size = 8.0\ntab_width = 16\ndiagnostics_delay_ms = 5000\n[files]\nbackup_delay_ms = 250\nautosave_delay_ms = 60000\n";
    let (settings, problems) = parse(text);
    assert!(problems.is_empty());
    assert_eq!(settings.editor.font_size, 8.0);
    assert_eq!(settings.editor.tab_width, 16);
    assert_eq!(settings.editor.diagnostics_delay_ms, 5000);
    assert_eq!(settings.files.backup_delay_ms, 250);
    assert_eq!(settings.files.autosave_delay_ms, 60_000);
    let (settings, problems) = parse("[editor]\nfont_size = 40.0\ntab_width = 1\n");
    assert!(problems.is_empty());
    assert_eq!(settings.editor.font_size, 40.0);
    assert_eq!(settings.editor.tab_width, 1);
}

#[test]
fn a_float_key_takes_an_integer_and_an_integer_key_never_a_float() {
    for (value, size) in [("16", 16.0), ("0x10", 16.0), ("1_2", 12.0), ("9.5", 9.5)] {
        let (settings, problems) = parse(&format!("[editor]\nfont_size = {value}\n"));
        assert!(problems.is_empty(), "{value}: {problems:?}");
        assert_eq!(settings.editor.font_size, size, "{value}");
    }
    let (settings, problems) = parse("[editor]\ntab_width = 4.0\n");
    assert_eq!(settings.editor.tab_width, 8);
    assert_eq!(summary(&problems[0]), ("cfg-wrong-type", 2, 13));
    assert_eq!(arg(&problems[0], "expected"), &string_arg("integer"));
}

// -------------------------------------------------------------------------------------------
// Bad values
// -------------------------------------------------------------------------------------------

/// Each key, a value of the wrong type for it, and the `expected` id.
const WRONG_TYPES: [(&str, &str, &str, &str); 18] = [
    ("ui", "language", "1", "string"),
    ("ui", "theme", "true", "string"),
    ("ui", "native_dialogs", "\"yes\"", "boolean"),
    ("editor", "font_size", "\"14\"", "number"),
    ("editor", "font_file", "[]", "string"),
    ("editor", "tab_width", "\"8\"", "integer"),
    ("editor", "insert_spaces", "1", "boolean"),
    ("editor", "cursor_blink", "\"true\"", "boolean"),
    ("editor", "line_numbers", "0", "boolean"),
    ("editor", "highlight_current_line", "{}", "boolean"),
    ("editor", "diagnostics_delay_ms", "300.0", "integer"),
    ("files", "hot_exit", "1979-05-27", "boolean"),
    ("files", "backup_delay_ms", "\"1500\"", "integer"),
    ("files", "autosave", "false", "string"),
    ("files", "autosave_delay_ms", "[1000]", "integer"),
    ("files", "eol", "10", "string"),
    ("explorer", "exclude", "\".git\"", "array"),
    ("render", "renderer", "[\"glow\"]", "string"),
];

#[test]
fn a_value_of_the_wrong_type_keeps_the_default() {
    for (table, key, value, expected) in WRONG_TYPES {
        let text = format!("# settings\n[{table}]\n{key} = {value}\n");
        let (settings, problems) = parse(&text);
        assert_eq!(settings, Settings::default(), "{key}");
        assert_eq!(problems.len(), 1, "{key}: {problems:?}");
        let problem = &problems[0];
        let column = u32::try_from(key.len()).unwrap() + 4;
        assert_eq!(summary(problem), ("cfg-wrong-type", 3, column), "{key}");
        assert_eq!(arg(problem, "key"), &text_arg(table, key), "{key}");
        assert_eq!(arg(problem, "expected"), &string_arg(expected), "{key}");
        let start = text.find(&format!("{key} = ")).unwrap() + key.len() + 3;
        assert_eq!(problem.range.start as usize, start, "{key}");
        assert_eq!(
            problem.range.slice(&text),
            Some(value),
            "{key}: the range covers the value"
        );
        assert_eq!(problem.path, settings_path());
    }
}

/// The `key` argument `<table>.<key>`.
fn text_arg(table: &str, key: &str) -> DiagArg {
    string_arg(&format!("{table}.{key}"))
}

#[test]
fn a_table_of_the_wrong_type_is_reported() {
    let (settings, problems) = parse("ui = 3\neditor = \"big\"\n");
    assert_eq!(settings, Settings::default());
    let found: Vec<_> = problems.iter().map(summary).collect();
    assert_eq!(found, [("cfg-wrong-type", 1, 6), ("cfg-wrong-type", 2, 10)]);
    assert_eq!(arg(&problems[0], "expected"), &string_arg("table"));
    assert_eq!(arg(&problems[1], "key"), &string_arg("editor"));
}

/// Each numeric key, values outside its range, and the bounds.
const OUT_OF_RANGE: [(&str, &str, &[&str], i64, i64); 5] = [
    (
        "editor",
        "font_size",
        &[
            "7.99", "40.01", "7", "41", "nan", "inf", "-inf", "+nan", "-8.0",
        ],
        8,
        40,
    ),
    ("editor", "tab_width", &["0", "17", "-1", "256"], 1, 16),
    (
        "editor",
        "diagnostics_delay_ms",
        &["49", "5001", "9223372036854775807"],
        50,
        5000,
    ),
    ("files", "backup_delay_ms", &["249", "60001"], 250, 60_000),
    (
        "files",
        "autosave_delay_ms",
        &["0", "1_000_000"],
        250,
        60_000,
    ),
];

#[test]
fn a_number_out_of_range_keeps_the_default() {
    for (table, key, values, min, max) in OUT_OF_RANGE {
        for value in values {
            let text = format!("[{table}]\n{key} = {value}\n");
            let (settings, problems) = parse(&text);
            assert_eq!(settings, Settings::default(), "{key} = {value}");
            assert_eq!(problems.len(), 1, "{key} = {value}: {problems:?}");
            let problem = &problems[0];
            let column = u32::try_from(key.len()).unwrap() + 4;
            assert_eq!(
                summary(problem),
                ("cfg-out-of-range", 2, column),
                "{key} = {value}"
            );
            assert_eq!(arg(problem, "key"), &text_arg(table, key));
            assert_eq!(arg(problem, "min"), &DiagArg::Int(min));
            assert_eq!(arg(problem, "max"), &DiagArg::Int(max));
        }
    }
}

/// Each string key with an id, values that are not allowed, and the `expected` list.
const INVALID_VALUES: [(&str, &str, &[&str], &str); 5] = [
    (
        "ui",
        "language",
        &["EN", "english", " en", "fr_FR", "de", ""],
        "auto, en, fr",
    ),
    (
        "ui",
        "theme",
        &["Dark", "black", "AUTO"],
        "auto, light, dark",
    ),
    (
        "files",
        "autosave",
        &["on", "after_delay", "Off"],
        "off, after-delay, on-focus-change",
    ),
    ("files", "eol", &["LF", "cr", "\\n"], "auto, lf, crlf"),
    (
        "render",
        "renderer",
        &["Glow", " glow", "glow ", "opengl", "Auto"],
        "auto, glow, wgpu",
    ),
];

#[test]
fn a_string_outside_the_allowed_values_keeps_the_default() {
    for (table, key, values, expected) in INVALID_VALUES {
        for value in values {
            let text = format!("[{table}]\n{key} = \"{value}\"\n");
            let (settings, problems) = parse(&text);
            assert_eq!(settings, Settings::default(), "{key} = {value:?}");
            assert_eq!(problems.len(), 1, "{key} = {value:?}: {problems:?}");
            let problem = &problems[0];
            let column = u32::try_from(key.len()).unwrap() + 4;
            assert_eq!(
                summary(problem),
                ("cfg-invalid-value", 2, column),
                "{key} = {value:?}"
            );
            assert_eq!(arg(problem, "key"), &text_arg(table, key));
            let parsed = value.replace("\\n", "\n");
            assert_eq!(arg(problem, "value"), &string_arg(&parsed));
            assert_eq!(arg(problem, "expected"), &string_arg(expected));
        }
    }
}

#[test]
fn font_file_must_name_a_ttf_or_otf_file() {
    for (value, expected) in [
        ("", None),
        ("Mono.ttf", Some("Mono.ttf")),
        ("fonts/Code.OTF", Some("fonts/Code.OTF")),
        ("C:\\Fonts\\Code.TtF", Some("C:\\Fonts\\Code.TtF")),
        (
            "/usr/share/fonts/code.otf",
            Some("/usr/share/fonts/code.otf"),
        ),
        ("../shared/code.ttf", Some("../shared/code.ttf")),
    ] {
        let toml_value = value.replace('\\', "\\\\");
        let (settings, problems) = parse(&format!("[editor]\nfont_file = \"{toml_value}\"\n"));
        assert!(problems.is_empty(), "{value}: {problems:?}");
        assert_eq!(
            settings.editor.font_file,
            expected.map(PathBuf::from),
            "{value}"
        );
    }
    for value in ["code.woff", "ttf", "code.ttf.bak", ".tt", "code"] {
        let (settings, problems) = parse(&format!("[editor]\nfont_file = \"{value}\"\n"));
        assert_eq!(settings.editor.font_file, None, "{value}");
        assert_eq!(
            summary(&problems[0]),
            ("cfg-invalid-value", 2, 13),
            "{value}"
        );
        assert_eq!(arg(&problems[0], "expected"), &string_arg("font-file"));
        assert_eq!(arg(&problems[0], "value"), &string_arg(value));
    }
}

#[test]
fn bad_exclude_elements_are_dropped_and_the_others_kept() {
    let text = "[explorer]\nexclude = [\"build\", 3, \"\", \"a/b\", \"c\\\\d\", \"obj\"]\n";
    let (settings, problems) = parse(text);
    assert_eq!(settings.explorer.exclude, ["build", "obj"]);
    let found: Vec<_> = problems.iter().map(summary).collect();
    assert_eq!(
        found,
        [
            ("cfg-wrong-type", 2, 21),
            ("cfg-invalid-value", 2, 24),
            ("cfg-invalid-value", 2, 28),
            ("cfg-invalid-value", 2, 35),
        ]
    );
    assert_eq!(arg(&problems[0], "expected"), &string_arg("string"));
    assert_eq!(arg(&problems[1], "value"), &string_arg(""));
    assert_eq!(arg(&problems[2], "expected"), &string_arg("file-name"));
    assert_eq!(arg(&problems[3], "value"), &string_arg("c\\d"));
    // An empty list hides nothing.
    let (settings, problems) = parse("[explorer]\nexclude = []\n");
    assert!(problems.is_empty());
    assert!(settings.explorer.exclude.is_empty());
}

#[test]
fn an_unknown_key_keeps_its_neighbours() {
    let text = "theme = \"dark\"\n[editor]\ntab_width = 4\nfont_sise = 3\nline_numbers = false\n[edtor]\ntab_width = 2\n[ui]\nlanguage = \"fr\"\n";
    let (settings, problems) = parse(text);
    assert_eq!(settings.editor.tab_width, 4);
    assert!(!settings.editor.line_numbers);
    assert_eq!(settings.ui.language, Some(Lang::Fr));
    assert_eq!(settings.ui.theme, None);
    let found: Vec<_> = problems.iter().map(summary).collect();
    assert_eq!(
        found,
        [
            ("cfg-unknown-key", 1, 1),
            ("cfg-unknown-key", 4, 1),
            ("cfg-unknown-key", 6, 2)
        ]
    );
    assert_eq!(arg(&problems[0], "key"), &string_arg("theme"));
    assert_eq!(arg(&problems[1], "key"), &string_arg("editor.font_sise"));
    assert_eq!(arg(&problems[2], "key"), &string_arg("edtor"));
}

#[test]
fn problems_come_in_file_order_whatever_the_key_order() {
    // Keys written in reverse alphabetical order, each one bad.
    let text = "[ui]\ntheme = 1\nnative_dialogs = 2\nlanguage = 3\n";
    let (_, problems) = parse(text);
    let lines: Vec<u32> = problems.iter().map(|problem| problem.at.line).collect();
    assert_eq!(lines, [2, 3, 4]);
}

#[test]
fn dotted_keys_and_inline_tables_are_tables_too() {
    let (settings, problems) =
        parse("editor.tab_width = 2\nui = { theme = \"light\", native_dialogs = false }\n");
    assert!(problems.is_empty());
    assert_eq!(settings.editor.tab_width, 2);
    assert_eq!(settings.ui.theme, Some(ThemeKind::Light));
    assert!(!settings.ui.native_dialogs);
}

#[test]
fn another_schema_version_ignores_the_whole_file() {
    for (version, found) in [("2", "2"), ("0", "0"), ("\"1\"", "\"1\""), ("1.0", "1.0")] {
        let text = format!("schema_version = {version}\n[editor]\ntab_width = 4\nbogus = 1\n");
        let (settings, problems) = parse(&text);
        assert_eq!(settings, Settings::default(), "{version}");
        assert_eq!(problems.len(), 1, "{version}: {problems:?}");
        assert_eq!(summary(&problems[0]), ("cfg-schema-version", 1, 18));
        assert_eq!(arg(&problems[0], "found"), &string_arg(found));
    }
    let (settings, problems) = parse("schema_version = 1\n[editor]\ntab_width = 4\n");
    assert!(problems.is_empty());
    assert_eq!(settings.editor.tab_width, 4);
}

#[test]
fn a_syntax_error_gives_the_defaults_and_one_problem() {
    let text = "[editor]\ntab_width = 4\nline_numbers = \n[ui]\ntheme = \"dark\"\n";
    let (settings, problems) = parse(text);
    assert_eq!(settings, Settings::default());
    assert_eq!(problems.len(), 1, "{problems:?}");
    let problem = &problems[0];
    assert_eq!(problem.code.0, "cfg-syntax");
    assert_eq!(problem.at.line, 3, "{problem:?}");
    let DiagArg::Str(detail) = arg(problem, "detail") else {
        panic!("detail is a string");
    };
    assert!(!detail.is_empty() && !detail.contains('\n'), "{detail:?}");
    for bad in [
        "[editor",
        "tab_width = 4 4",
        "= 1",
        "a = \"unterminated",
        "[ui]\n[ui]\n",
    ] {
        let (settings, problems) = parse(bad);
        assert_eq!(settings, Settings::default(), "{bad:?}");
        assert_eq!(problems.len(), 1, "{bad:?}");
        assert_eq!(problems[0].code.0, "cfg-syntax", "{bad:?}");
    }
}

#[test]
fn positions_count_chars_after_a_byte_order_mark() {
    // A BOM is stripped; columns count chars, so `é` is one column; CRLF ends a line.
    let text = "\u{feff}# réglages\r\neditor = { \"é\" = 1, tab_width = \"x\" }\r\n[ui]\r\n\"thème\" = 1\r\ntheme = 2\r\n";
    let (_, problems) = parse(text);
    let found: Vec<_> = problems.iter().map(summary).collect();
    let stripped = &text['\u{feff}'.len_utf8()..];
    let slices: Vec<_> = problems
        .iter()
        .map(|problem| problem.range.slice(stripped).unwrap())
        .collect();
    assert_eq!(slices, ["\"é\"", "\"x\"", "\"thème\"", "2"]);
    assert_eq!(
        found,
        [
            ("cfg-unknown-key", 2, 12),
            ("cfg-wrong-type", 2, 33),
            ("cfg-unknown-key", 4, 1),
            ("cfg-wrong-type", 5, 9)
        ]
    );
    // Without a BOM, the same text gives the same positions.
    let (_, again) = parse(stripped);
    assert_eq!(again, problems);
    // A bare non-ASCII key is a syntax error, placed by chars too.
    let (_, problems) = parse("# é\n[ui]\nthème = 1\n");
    assert_eq!(summary(&problems[0]), ("cfg-syntax", 3, 3));
}

#[test]
fn bad_files_never_panic() {
    let deep = format!("a = {}1{}", "[".repeat(10_000), "]".repeat(10_000));
    let inline = format!("a = {}", "{ b = ".repeat(10_000));
    let long_value = format!("[ui]\ntheme = \"{}\"\n", "x".repeat(500_000));
    let inputs = [
        deep.as_str(),
        inline.as_str(),
        long_value.as_str(),
        "\0\u{1}\u{7f}",
        "[[ui]]\ntheme = 1\n",
        "ui = [1, 2]\n",
        "[editor]\nfont_size = 1e400\ntab_width = 99999999999999999999\n",
        "[editor]\nfont_size = -0.0\n",
        "schema_version = 99999999999999999999\n",
        "\u{feff}\u{feff}",
    ];
    for input in inputs {
        let (_, problems) = parse(input);
        for problem in &problems {
            assert!(CONFIG_CODES.contains(&problem.code));
            if let Some((_, DiagArg::Str(value))) =
                problem.args.iter().find(|(name, _)| *name == "value")
            {
                assert!(value.chars().count() <= 65, "arguments are cut");
            }
        }
    }
    let (_, problems) = parse(&long_value);
    assert_eq!(
        arg(&problems[0], "value"),
        &string_arg(&format!("{}…", "x".repeat(64)))
    );
}

// -------------------------------------------------------------------------------------------
// read_config_text
// -------------------------------------------------------------------------------------------

/// Creates a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (CLAUDE.md test rule).
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    dir
}

/// Writes `bytes` to `settings.toml` in a fresh temp folder and returns its path.
fn temp_file(bytes: &[u8]) -> PathBuf {
    let path = temp_dir("config-read").join("settings.toml");
    std::fs::write(&path, bytes).unwrap();
    path
}

/// The reading problem of `path`.
fn read_error(path: &Path) -> ConfigProblem {
    read_config_text(path).unwrap_err()
}

#[test]
fn a_missing_file_reads_as_none() {
    let dir = temp_dir("config-missing");
    assert_eq!(read_config_text(&dir.join("settings.toml")), Ok(None));
    assert_eq!(
        read_config_text(&dir.join("absent").join("settings.toml")),
        Ok(None)
    );
}

#[test]
fn a_file_up_to_the_limit_reads_whole() {
    let path = temp_file(b"[ui]\ntheme = \"dark\"\n");
    assert_eq!(
        read_config_text(&path),
        Ok(Some("[ui]\ntheme = \"dark\"\n".to_owned()))
    );
    let mut full = b"# ".to_vec();
    full.resize(MAX_CONFIG_BYTES, b'x');
    let path = temp_file(&full);
    assert_eq!(
        read_config_text(&path).unwrap().unwrap().len(),
        MAX_CONFIG_BYTES
    );
}

#[test]
fn a_file_over_the_limit_is_too_large() {
    let path = temp_file(&vec![b'#'; MAX_CONFIG_BYTES + 1]);
    let problem = read_error(&path);
    assert_eq!(problem.code.0, "cfg-too-large");
    assert_eq!(problem.args, [("bytes", DiagArg::Int(1024 * 1024 + 1))]);
    assert_eq!(problem.path, path);
    assert_eq!(problem.at, LineCol { line: 1, column: 1 });
    assert_eq!(problem.range, TextRange::default());
}

#[test]
fn a_file_that_is_not_utf8_is_reported_with_its_offset() {
    let path = temp_file(b"[ui]\ntheme = \"\xff\"\n");
    let problem = read_error(&path);
    assert_eq!(problem.code.0, "cfg-not-utf8");
    assert_eq!(problem.args, [("offset", DiagArg::Int(14))]);
}

#[test]
fn a_byte_order_mark_is_stripped() {
    let path = temp_file("\u{feff}[ui]\n".as_bytes());
    assert_eq!(read_config_text(&path), Ok(Some("[ui]\n".to_owned())));
}

#[test]
fn a_folder_is_unreadable() {
    let dir = temp_dir("config-folder");
    let problem = read_error(&dir);
    assert_eq!(problem.code.0, "cfg-unreadable");
    let DiagArg::Str(error) = arg(&problem, "error") else {
        panic!("error is a string");
    };
    assert!(!error.is_empty() && error.is_ascii(), "{error:?}");
}

// -------------------------------------------------------------------------------------------
// Diagnostics and messages
// -------------------------------------------------------------------------------------------

#[test]
fn a_problem_becomes_a_non_blocking_warning() {
    let (_, problems) = parse("[editor]\ntab_width = 99\n");
    let diagnostic = problems[0].to_diagnostic(FileId(7));
    assert_eq!(diagnostic.code, problems[0].code);
    assert_eq!(diagnostic.severity, Severity::Warning);
    assert!(!diagnostic.blocking);
    assert_eq!(diagnostic.span.file, FileId(7));
    assert_eq!(diagnostic.span.range, problems[0].range);
    assert_eq!(diagnostic.args, problems[0].args);
    assert!(diagnostic.related.is_empty() && diagnostic.suggestion.is_none());
}

/// One problem of each settings kind, produced by the parser and the reader.
fn sample_problems() -> Vec<ConfigProblem> {
    let mut problems = Vec::new();
    for text in [
        "[ui\n",
        "schema_version = 3\n",
        "[ui]\ncolour = 1\n",
        "[ui]\nnative_dialogs = 1\n",
        "[editor]\ntab_width = 0\n",
        "[render]\nrenderer = \"Glow\"\n",
        "[editor]\nfont_file = \"x.woff\"\n",
        "[explorer]\nexclude = [\"a/b\"]\n",
    ] {
        problems.extend(parse(text).1);
    }
    problems.push(read_error(&temp_file(&vec![b'#'; MAX_CONFIG_BYTES + 1])));
    problems.push(read_error(&temp_file(b"\xc3")));
    problems.push(read_error(&temp_dir("config-sample")));
    problems
}

#[test]
fn every_config_code_has_an_english_and_a_french_message() {
    for (lang, resource) in [(Lang::En, CONFIG_EN), (Lang::Fr, CONFIG_FR)] {
        let localizer = Localizer::new(lang, &[resource], &[resource]).unwrap();
        for code in CONFIG_CODES {
            let id = code.fluent_id();
            assert!(
                localizer.message(&id, None, &[]).is_some(),
                "{}: {id}",
                lang.code()
            );
            assert!(
                localizer.message(&id, Some("help"), &[]).is_some(),
                "{}: {id}.help",
                lang.code()
            );
        }
    }
}

#[test]
fn settings_problems_render_in_both_languages() {
    let problems = sample_problems();
    let codes: Vec<&str> = problems.iter().map(|problem| problem.code.0).collect();
    assert_eq!(
        codes,
        [
            "cfg-syntax",
            "cfg-schema-version",
            "cfg-unknown-key",
            "cfg-wrong-type",
            "cfg-out-of-range",
            "cfg-invalid-value",
            "cfg-invalid-value",
            "cfg-invalid-value",
            "cfg-too-large",
            "cfg-not-utf8",
            "cfg-unreadable",
        ]
    );
    for (lang, primary) in [(Lang::En, CONFIG_EN), (Lang::Fr, CONFIG_FR)] {
        let localizer = Localizer::new(lang, &[primary], &[CONFIG_EN]).unwrap();
        for problem in &problems {
            let rendered = localizer.render(&problem.to_diagnostic(FileId(0)));
            let help = rendered.help.unwrap();
            for part in [&rendered.message, &help] {
                assert!(!part.contains("{$"), "{}: {part}", lang.code());
                assert!(!part.trim().is_empty());
            }
            assert_ne!(rendered.message, problem.code.0);
        }
    }
    let french = Localizer::new(Lang::Fr, &[CONFIG_FR], &[CONFIG_EN]).unwrap();
    let rendered = french.render(&problems[4].to_diagnostic(FileId(0)));
    assert_eq!(
        rendered.message,
        "editor.tab_width doit être compris entre 1 et 16 ; cette valeur est ignorée"
    );
    let english = Localizer::new(Lang::En, &[CONFIG_EN], &[CONFIG_EN]).unwrap();
    let rendered = english.render(&problems[3].to_diagnostic(FileId(0)));
    assert_eq!(
        rendered.message,
        "ui.native_dialogs must be true or false; this value is ignored"
    );
    let rendered = english.render(&problems[5].to_diagnostic(FileId(0)));
    assert_eq!(
        rendered.help.unwrap(),
        "The accepted values are: auto, glow, wgpu. Write one of them in lowercase, between double quotes."
    );
}
