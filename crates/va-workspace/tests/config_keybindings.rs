//! Tests of `keybindings.toml` (m4-editor-shell.md §4.4, §10.1): chord parsing (modifiers in any
//! case, aliases, every `KEY_NAMES` entry), the keys kept by the editor, removals, replacement,
//! file order, unknown commands, bad keys, `when`, and the messages of their problems.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use va_core::diag::DiagArg;
use va_core::source::FileId;
use va_workspace::config::{
    Binding, ConfigProblem, KEY_NAMES, KeyChord, When, is_reserved_chord, parse_chord,
    parse_keybindings,
};
use va_workspace::i18n::{Lang, Localizer};

/// The English configuration messages.
const CONFIG_EN: &str = include_str!("../../../locales/en/config.ftl");
/// The French configuration messages.
const CONFIG_FR: &str = include_str!("../../../locales/fr/config.ftl");

/// The command ids the fake registry knows.
const COMMANDS: [&str; 9] = [
    "file.save",
    "file.save-all",
    "file.open",
    "edit.select-next-occurrence",
    "edit.toggle-comment",
    "explorer.rename",
    "palette.commands",
    "view.zoom-in",
    "tab.next",
];

/// Tells whether the fake registry knows `id`.
fn is_command(id: &str) -> bool {
    COMMANDS.contains(&id)
}

/// The chord of `text`, which must parse.
fn chord(text: &str) -> KeyChord {
    parse_chord(text).unwrap_or_else(|| panic!("{text:?} is a chord"))
}

/// A binding of `key` to `command` in `when`.
fn binding(key: &str, command: &str, when: When) -> Binding {
    Binding {
        chord: chord(key),
        command: command.to_owned(),
        when,
    }
}

/// A few defaults of m4 §5.12.
fn defaults() -> Vec<Binding> {
    vec![
        binding("Ctrl+S", "file.save", When::Global),
        binding("Ctrl+O", "file.open", When::Global),
        binding("Ctrl+D", "edit.select-next-occurrence", When::Global),
        binding("Ctrl+Slash", "edit.toggle-comment", When::Global),
        binding("F2", "explorer.rename", When::Explorer),
        binding("Ctrl+Shift+P", "palette.commands", When::Global),
        binding("F1", "palette.commands", When::Global),
        binding("Ctrl+Equals", "view.zoom-in", When::Global),
        binding("Ctrl+Plus", "view.zoom-in", When::Global),
    ]
}

/// The path given to the parser (never read).
fn keybindings_path() -> PathBuf {
    PathBuf::from("config").join("keybindings.toml")
}

/// Resolves `text` over [`defaults`].
fn resolve(text: &str) -> (Vec<Binding>, Vec<ConfigProblem>) {
    parse_keybindings(&keybindings_path(), Some(text), &defaults(), &is_command)
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

// -------------------------------------------------------------------------------------------
// Chords
// -------------------------------------------------------------------------------------------

#[test]
fn a_chord_has_modifiers_then_one_key() {
    assert_eq!(
        parse_chord("Ctrl+Shift+P"),
        Some(KeyChord {
            ctrl: true,
            shift: true,
            alt: false,
            key: "P"
        })
    );
    assert_eq!(
        parse_chord("F5"),
        Some(KeyChord {
            ctrl: false,
            shift: false,
            alt: false,
            key: "F5"
        })
    );
    assert_eq!(
        parse_chord("Shift+Alt+Down"),
        Some(KeyChord {
            ctrl: false,
            shift: true,
            alt: true,
            key: "Down"
        })
    );
}

#[test]
fn modifier_and_key_names_ignore_case_and_order() {
    let expected = chord("Ctrl+Shift+Alt+PageUp");
    for text in [
        "ctrl+shift+alt+pageup",
        "CTRL+SHIFT+ALT+PAGEUP",
        "Alt+Shift+Ctrl+PageUp",
        "shift+CTRL+aLt+PaGeUp",
    ] {
        assert_eq!(parse_chord(text), Some(expected), "{text}");
    }
}

#[test]
fn aliases_give_the_canonical_names() {
    for (alias, canonical) in [
        ("ArrowUp", "Up"),
        ("arrowdown", "Down"),
        ("ARROWLEFT", "Left"),
        ("ArrowRight", "Right"),
        ("Esc", "Escape"),
        ("esc", "Escape"),
        ("Return", "Enter"),
        ("Ctrl+Alt+ArrowUp", "Up"),
    ] {
        assert_eq!(parse_chord(alias).unwrap().key, canonical, "{alias}");
    }
}

#[test]
fn every_key_name_parses_to_itself() {
    let unique: BTreeSet<&str> = KEY_NAMES.iter().copied().collect();
    assert_eq!(unique.len(), KEY_NAMES.len(), "KEY_NAMES has no duplicate");
    for name in KEY_NAMES {
        for text in [
            name.to_owned(),
            name.to_ascii_lowercase(),
            name.to_ascii_uppercase(),
            format!("Ctrl+{name}"),
            format!("ctrl+shift+alt+{}", name.to_ascii_lowercase()),
        ] {
            let parsed = parse_chord(&text).unwrap_or_else(|| panic!("{text:?}"));
            assert_eq!(parsed.key, name, "{text:?}");
            assert!(
                std::ptr::eq(parsed.key, name),
                "{text:?} is the canonical &str"
            );
        }
    }
}

#[test]
fn texts_that_are_not_one_chord_are_refused() {
    for text in [
        "",
        "+",
        "Ctrl+",
        "+P",
        "Ctrl++",
        "Ctrl+Ctrl+P",
        "Shift+shift+P",
        "Ctrl+K Ctrl+S",
        " Ctrl+P",
        "Ctrl+P ",
        "Ctrl + P",
        "Meta+P",
        "Cmd+P",
        "Control+P",
        "Ctrl+Shift",
        "P+Ctrl",
        "F21",
        "F0",
        "Ctrl+PageUp+Down",
        "Ctrl-P",
        "é",
        "+",
    ] {
        assert_eq!(parse_chord(text), None, "{text:?}");
    }
}

#[test]
fn caret_motion_editing_and_clipboard_keys_stay_with_the_editor() {
    for text in [
        "Up",
        "Shift+Down",
        "Ctrl+Left",
        "Ctrl+Shift+Right",
        "Home",
        "Shift+End",
        "Ctrl+Home",
        "PageUp",
        "Shift+PageDown",
        "Backspace",
        "Ctrl+Backspace",
        "Delete",
        "Shift+Delete",
        "Enter",
        "Shift+Enter",
        "Tab",
        "Shift+Tab",
        "Escape",
        "Ctrl+C",
        "Ctrl+X",
        "Ctrl+V",
        "Ctrl+Shift+V",
        "Ctrl+Alt+C",
        "Ctrl+Insert",
        "Shift+Insert",
    ] {
        assert!(is_reserved_chord(&chord(text)), "{text}");
    }
    for text in [
        "Alt+Up",
        "Ctrl+Alt+Down",
        "Shift+Alt+Down",
        "Alt+Home",
        "Ctrl+Tab",
        "Ctrl+Shift+Tab",
        "Ctrl+Enter",
        "Alt+Enter",
        "Ctrl+Escape",
        "Insert",
        "Ctrl+Shift+Insert",
        "C",
        "Shift+C",
        "Alt+V",
        "F2",
        "Ctrl+S",
        "Ctrl+Slash",
        "Space",
    ] {
        assert!(!is_reserved_chord(&chord(text)), "{text}");
    }
}

#[test]
fn no_default_key_of_the_command_table_is_reserved() {
    // m4 §5.12 default keys.
    for text in [
        "Ctrl+N",
        "Ctrl+O",
        "Ctrl+Shift+O",
        "Ctrl+S",
        "Ctrl+Shift+S",
        "Ctrl+W",
        "Ctrl+Z",
        "Ctrl+Y",
        "Ctrl+Shift+Z",
        "Ctrl+A",
        "Ctrl+D",
        "Ctrl+Alt+Up",
        "Ctrl+Alt+Down",
        "Ctrl+Slash",
        "Ctrl+Shift+K",
        "Shift+Alt+Down",
        "Alt+Up",
        "Alt+Down",
        "Ctrl+F",
        "Ctrl+H",
        "F3",
        "Shift+F3",
        "Ctrl+G",
        "Ctrl+P",
        "Ctrl+Shift+P",
        "F1",
        "Ctrl+Equals",
        "Ctrl+Plus",
        "Ctrl+Minus",
        "Ctrl+0",
        "Ctrl+B",
        "Ctrl+Tab",
        "Ctrl+Shift+Tab",
        "F2",
        "Ctrl+Comma",
        "Ctrl+Q",
    ] {
        assert!(!is_reserved_chord(&chord(text)), "{text}");
    }
}

#[test]
fn when_ids_round_trip() {
    for when in [When::Global, When::Editor, When::Explorer] {
        assert_eq!(When::from_id(when.id()), Some(when));
    }
    assert_eq!(When::from_id("Editor"), None);
    assert_eq!(When::from_id(""), None);
}

// -------------------------------------------------------------------------------------------
// Resolution
// -------------------------------------------------------------------------------------------

#[test]
fn a_missing_or_empty_file_gives_the_defaults() {
    let (bindings, problems) =
        parse_keybindings(&keybindings_path(), None, &defaults(), &is_command);
    assert_eq!(bindings, defaults());
    assert!(problems.is_empty());
    for text in ["", "schema_version = 1\n", "# nothing yet\n"] {
        assert_eq!(resolve(text), (defaults(), Vec::new()), "{text:?}");
    }
}

#[test]
fn a_removal_drops_that_default_binding_of_the_key() {
    let text = "[[binding]]\nkey = \"Ctrl+D\"\ncommand = \"-edit.select-next-occurrence\"\n";
    let (bindings, problems) = resolve(text);
    assert!(problems.is_empty(), "{problems:?}");
    let mut expected = defaults();
    expected.remove(2);
    assert_eq!(bindings, expected);
    // A removal matches the command, whatever the default's `when`.
    let (bindings, _) = resolve("[[binding]]\nkey = \"F2\"\ncommand = \"-explorer.rename\"\n");
    assert!(!bindings.iter().any(|b| b.command == "explorer.rename"));
    // Another command on that key removes nothing.
    let (bindings, problems) = resolve("[[binding]]\nkey = \"Ctrl+S\"\ncommand = \"-file.open\"\n");
    assert!(problems.is_empty());
    assert_eq!(bindings, defaults());
    // One key of a command with two: only that one goes.
    let (bindings, _) = resolve("[[binding]]\nkey = \"F1\"\ncommand = \"-palette.commands\"\n");
    let palette: Vec<&Binding> = bindings
        .iter()
        .filter(|b| b.command == "palette.commands")
        .collect();
    assert_eq!(
        palette,
        [&binding("Ctrl+Shift+P", "palette.commands", When::Global)]
    );
}

#[test]
fn removals_apply_to_the_defaults_before_any_user_binding() {
    // The user binding comes first in the file, the removal after it: the user binding stays.
    let text = "[[binding]]\nkey = \"Ctrl+D\"\ncommand = \"edit.select-next-occurrence\"\nwhen = \"editor\"\n\n[[binding]]\nkey = \"Ctrl+D\"\ncommand = \"-edit.select-next-occurrence\"\n";
    let (bindings, problems) = resolve(text);
    assert!(problems.is_empty());
    let on_ctrl_d: Vec<&Binding> = bindings
        .iter()
        .filter(|b| b.chord == chord("Ctrl+D"))
        .collect();
    assert_eq!(
        on_ctrl_d,
        [&binding(
            "Ctrl+D",
            "edit.select-next-occurrence",
            When::Editor
        )]
    );
}

#[test]
fn a_user_binding_replaces_the_binding_of_the_same_chord_and_when() {
    let text = "[[binding]]\nkey = \"ctrl+s\"\ncommand = \"file.save-all\"\n";
    let (bindings, problems) = resolve(text);
    assert!(problems.is_empty());
    let mut expected = defaults();
    expected.remove(0);
    expected.push(binding("Ctrl+S", "file.save-all", When::Global));
    assert_eq!(
        bindings, expected,
        "remaining defaults, then the user binding"
    );
    // Another `when` does not replace: F2 in the explorer stays.
    let (bindings, _) = resolve("[[binding]]\nkey = \"F2\"\ncommand = \"file.save\"\n");
    assert!(bindings.contains(&binding("F2", "explorer.rename", When::Explorer)));
    assert_eq!(
        bindings.last(),
        Some(&binding("F2", "file.save", When::Global))
    );
}

#[test]
fn user_bindings_apply_in_file_order() {
    let text = "[[binding]]\nkey = \"Alt+F5\"\ncommand = \"file.save\"\n[[binding]]\nkey = \"Alt+F6\"\ncommand = \"file.open\"\n[[binding]]\nkey = \"alt+f5\"\ncommand = \"tab.next\"\n";
    let (bindings, problems) = resolve(text);
    assert!(problems.is_empty());
    let user = &bindings[defaults().len()..];
    assert_eq!(
        user,
        [
            binding("Alt+F6", "file.open", When::Global),
            binding("Alt+F5", "tab.next", When::Global)
        ]
    );
}

#[test]
fn when_selects_where_a_binding_applies() {
    let text = "[[binding]]\nkey = \"Alt+E\"\ncommand = \"file.save\"\nwhen = \"editor\"\n[[binding]]\nkey = \"Alt+X\"\ncommand = \"file.save\"\nwhen = \"explorer\"\n[[binding]]\nkey = \"Alt+G\"\ncommand = \"file.save\"\nwhen = \"global\"\n";
    let (bindings, problems) = resolve(text);
    assert!(problems.is_empty());
    assert_eq!(
        &bindings[defaults().len()..],
        [
            binding("Alt+E", "file.save", When::Editor),
            binding("Alt+X", "file.save", When::Explorer),
            binding("Alt+G", "file.save", When::Global)
        ]
    );
}

#[test]
fn an_invalid_when_is_reported_and_the_binding_applies_globally() {
    let text = "[[binding]]\nkey = \"Alt+E\"\ncommand = \"file.save\"\nwhen = \"Editor\"\n[[binding]]\nkey = \"Alt+F\"\ncommand = \"file.save\"\nwhen = 2\n";
    let (bindings, problems) = resolve(text);
    let found: Vec<_> = problems.iter().map(summary).collect();
    assert_eq!(
        found,
        [("cfg-invalid-value", 4, 8), ("cfg-wrong-type", 8, 8)]
    );
    assert_eq!(arg(&problems[0], "key"), &string_arg("binding.when"));
    assert_eq!(arg(&problems[0], "value"), &string_arg("Editor"));
    assert_eq!(
        arg(&problems[0], "expected"),
        &string_arg("global, editor, explorer")
    );
    assert_eq!(
        &bindings[defaults().len()..],
        [
            binding("Alt+E", "file.save", When::Global),
            binding("Alt+F", "file.save", When::Global)
        ]
    );
}

#[test]
fn an_unknown_command_skips_the_binding() {
    let text = "[[binding]]\nkey = \"Alt+F5\"\ncommand = \"file.sav\"\n[[binding]]\nkey = \"Ctrl+D\"\ncommand = \"-edit.nothing\"\n[[binding]]\nkey = \"Ctrl+O\"\ncommand = \"-\"\n";
    let (bindings, problems) = resolve(text);
    assert_eq!(bindings, defaults());
    let found: Vec<_> = problems.iter().map(summary).collect();
    assert_eq!(
        found,
        [
            ("cfg-unknown-command", 3, 11),
            ("cfg-unknown-command", 6, 11),
            ("cfg-unknown-command", 9, 11)
        ]
    );
    assert_eq!(arg(&problems[0], "command"), &string_arg("file.sav"));
    assert_eq!(arg(&problems[1], "command"), &string_arg("-edit.nothing"));
    assert_eq!(arg(&problems[2], "command"), &string_arg("-"));
}

#[test]
fn a_bad_or_reserved_key_skips_the_binding() {
    let text = "[[binding]]\nkey = \"Ctrl+K Ctrl+S\"\ncommand = \"file.save\"\n[[binding]]\nkey = \"Up\"\ncommand = \"file.save\"\n[[binding]]\nkey = \"Ctrl+C\"\ncommand = \"-file.open\"\n[[binding]]\nkey = \"Hyper+Q\"\ncommand = \"nope\"\n";
    let (bindings, problems) = resolve(text);
    assert_eq!(bindings, defaults());
    let found: Vec<_> = problems.iter().map(summary).collect();
    assert_eq!(
        found,
        [
            ("cfg-bad-key", 2, 7),
            ("cfg-bad-key", 5, 7),
            ("cfg-bad-key", 8, 7),
            ("cfg-bad-key", 11, 7),
            ("cfg-unknown-command", 12, 11)
        ]
    );
    assert_eq!(arg(&problems[0], "key"), &string_arg("Ctrl+K Ctrl+S"));
    assert_eq!(arg(&problems[1], "key"), &string_arg("Up"));
}

#[test]
fn a_missing_or_mistyped_field_skips_the_binding() {
    let text = "[[binding]]\ncommand = \"file.save\"\n[[binding]]\nkey = \"Alt+S\"\n[[binding]]\nkey = 5\ncommand = \"file.save\"\n[[binding]]\nkey = \"Alt+T\"\ncommand = [\"file.save\"]\n";
    let (bindings, problems) = resolve(text);
    assert_eq!(bindings, defaults());
    let found: Vec<_> = problems.iter().map(summary).collect();
    assert_eq!(
        found,
        [
            ("cfg-wrong-type", 1, 1),
            ("cfg-wrong-type", 3, 1),
            ("cfg-wrong-type", 6, 7),
            ("cfg-wrong-type", 10, 11)
        ]
    );
    let keys: Vec<&DiagArg> = problems.iter().map(|p| arg(p, "key")).collect();
    assert_eq!(
        keys,
        [
            &string_arg("binding.key"),
            &string_arg("binding.command"),
            &string_arg("binding.key"),
            &string_arg("binding.command")
        ]
    );
    assert!(
        problems
            .iter()
            .all(|p| arg(p, "expected") == &string_arg("string"))
    );
}

#[test]
fn unknown_keys_are_reported_and_ignored() {
    let text =
        "theme = \"dark\"\n[[binding]]\nkey = \"Alt+S\"\ncommand = \"file.save\"\nargs = [1]\n";
    let (bindings, problems) = resolve(text);
    assert_eq!(
        bindings.last(),
        Some(&binding("Alt+S", "file.save", When::Global)),
        "the binding is kept"
    );
    let found: Vec<_> = problems.iter().map(summary).collect();
    assert_eq!(
        found,
        [("cfg-unknown-key", 1, 1), ("cfg-unknown-key", 5, 1)]
    );
    assert_eq!(arg(&problems[0], "key"), &string_arg("theme"));
    assert_eq!(arg(&problems[1], "key"), &string_arg("binding.args"));
}

#[test]
fn a_binding_list_of_the_wrong_shape_is_reported() {
    let (bindings, problems) = resolve("binding = 3\n");
    assert_eq!(bindings, defaults());
    assert_eq!(summary(&problems[0]), ("cfg-wrong-type", 1, 11));
    assert_eq!(arg(&problems[0], "expected"), &string_arg("array"));
    let (bindings, problems) =
        resolve("binding = [1, { key = \"Alt+S\", command = \"file.save\" }]\n");
    assert_eq!(summary(&problems[0]), ("cfg-wrong-type", 1, 12));
    assert_eq!(arg(&problems[0], "expected"), &string_arg("table"));
    assert_eq!(
        bindings.last(),
        Some(&binding("Alt+S", "file.save", When::Global)),
        "the other element is kept"
    );
}

#[test]
fn another_schema_version_or_a_syntax_error_gives_the_defaults() {
    let text = "schema_version = 2\n[[binding]]\nkey = \"Alt+S\"\ncommand = \"file.save\"\n";
    let (bindings, problems) = resolve(text);
    assert_eq!(bindings, defaults());
    assert_eq!(problems.len(), 1);
    assert_eq!(summary(&problems[0]), ("cfg-schema-version", 1, 18));
    let (bindings, problems) = resolve("[[binding]\nkey = \"Alt+S\"\n");
    assert_eq!(bindings, defaults());
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].code.0, "cfg-syntax");
    assert_eq!(problems[0].path, keybindings_path());
}

#[test]
fn keybinding_problems_render_in_both_languages() {
    let text = "[[binding]]\nkey = \"Up\"\ncommand = \"nope\"\nwhen = \"Editor\"\nextra = 1\n[[binding]]\ncommand = \"file.save\"\n";
    let (_, problems) = resolve(text);
    let codes: Vec<&str> = problems.iter().map(|p| p.code.0).collect();
    assert_eq!(
        codes,
        [
            "cfg-bad-key",
            "cfg-unknown-command",
            "cfg-invalid-value",
            "cfg-unknown-key",
            "cfg-wrong-type"
        ]
    );
    for (lang, primary) in [(Lang::En, CONFIG_EN), (Lang::Fr, CONFIG_FR)] {
        let localizer = Localizer::new(lang, &[primary], &[CONFIG_EN]).unwrap();
        for problem in &problems {
            let rendered = localizer.render(&problem.to_diagnostic(FileId(0)));
            let help = rendered.help.unwrap();
            for part in [&rendered.message, &help] {
                assert!(!part.contains("{$"), "{}: {part}", lang.code());
            }
        }
    }
    let english = Localizer::new(Lang::En, &[CONFIG_EN], &[CONFIG_EN]).unwrap();
    assert_eq!(
        english
            .render(&problems[0].to_diagnostic(FileId(0)))
            .message,
        "“Up” is not a key combination that can be bound; this binding is ignored"
    );
    let french = Localizer::new(Lang::Fr, &[CONFIG_FR], &[CONFIG_EN]).unwrap();
    assert_eq!(
        french.render(&problems[1].to_diagnostic(FileId(0))).message,
        "commande inconnue « nope » ; ce raccourci est ignoré"
    );
}
