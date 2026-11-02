//! `keybindings.toml` (m4-editor-shell.md §4.4, frozen): key chords, the canonical key names and
//! [`parse_keybindings`].
//!
//! The file holds `[[binding]]` tables (`key`, `command`, optional `when`). Resolution is the
//! defaults, then the removals (`command = "-<id>"`), then the user bindings in file order, each
//! replacing any binding of the same chord and `when`.

use std::path::Path;

use toml::Spanned;
use toml::de::{DeTable, DeValue};
use va_core::diag::DiagArg;

use super::problems::{
    BAD_KEY, ConfigProblem, Report, SCHEMA_VERSION_KEY, UNKNOWN_COMMAND, excerpt, expected,
    parse_document, strip_bom,
};

/// The top-level array of bindings.
const BINDING: &str = "binding";

/// The `expected` argument of `binding.when` problems.
const WHEN_VALUES: &str = "global, editor, explorer";

/// The prefix of a command that removes a default binding.
const REMOVE_PREFIX: char = '-';

/// Key-name aliases accepted on input (ASCII case ignored), with their canonical name.
const KEY_ALIASES: [(&str, &str); 6] = [
    ("ArrowUp", "Up"),
    ("ArrowDown", "Down"),
    ("ArrowLeft", "Left"),
    ("ArrowRight", "Right"),
    ("Esc", "Escape"),
    ("Return", "Enter"),
];

/// One key with its modifiers; `Ctrl` is egui's `Modifiers::COMMAND`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyChord {
    /// Ctrl (Command) held.
    pub ctrl: bool,
    /// Shift held.
    pub shift: bool,
    /// Alt held.
    pub alt: bool,
    /// The canonical key name, one of [`KEY_NAMES`].
    pub key: &'static str,
}

/// Where a binding applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum When {
    /// Everywhere (the default).
    Global,
    /// While an editor has focus.
    Editor,
    /// While the explorer has focus.
    Explorer,
}

impl When {
    /// The id in `keybindings.toml`: `"global"`, `"editor"` or `"explorer"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Editor => "editor",
            Self::Explorer => "explorer",
        }
    }

    /// Parses an exact lowercase id; anything else gives `None`.
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "global" => Some(Self::Global),
            "editor" => Some(Self::Editor),
            "explorer" => Some(Self::Explorer),
            _ => None,
        }
    }
}

/// A key binding: a chord runs a command id of m4 §5.12.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Binding {
    /// The key and its modifiers.
    pub chord: KeyChord,
    /// The command id, e.g. `palette.commands`.
    pub command: String,
    /// Where the binding applies.
    pub when: When,
}

/// The canonical key names (egui 0.36 `Key::name()` spellings) accepted in `keybindings.toml`.
pub static KEY_NAMES: [&str; 83] = [
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "0",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "F13",
    "F14",
    "F15",
    "F16",
    "F17",
    "F18",
    "F19",
    "F20",
    "Up",
    "Down",
    "Left",
    "Right",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Insert",
    "Delete",
    "Backspace",
    "Enter",
    "Tab",
    "Escape",
    "Space",
    "Comma",
    "Period",
    "Slash",
    "Backslash",
    "Semicolon",
    "Quote",
    "Backtick",
    "Minus",
    "Plus",
    "Equals",
    "OpenBracket",
    "CloseBracket",
];

/// Parses a chord such as `"Ctrl+Shift+P"` (modifier and key names case-insensitive, aliases
/// accepted); `None` when it is not one.
///
/// The text is `+`-separated parts without spaces: the modifiers `Ctrl`, `Shift` and `Alt`, each
/// at most once and in any order, then exactly one key of [`KEY_NAMES`] or an alias (`ArrowUp`,
/// `ArrowDown`, `ArrowLeft`, `ArrowRight`, `Esc`, `Return`). Sequences such as `Ctrl+K Ctrl+S`
/// are not chords.
pub fn parse_chord(text: &str) -> Option<KeyChord> {
    let mut parts = text.split('+');
    let key_name = parts.next_back()?;
    let mut chord = KeyChord {
        ctrl: false,
        shift: false,
        alt: false,
        key: canonical_key(key_name)?,
    };
    for modifier in parts {
        let held = if modifier.eq_ignore_ascii_case("ctrl") {
            &mut chord.ctrl
        } else if modifier.eq_ignore_ascii_case("shift") {
            &mut chord.shift
        } else if modifier.eq_ignore_ascii_case("alt") {
            &mut chord.alt
        } else {
            return None;
        };
        if *held {
            return None;
        }
        *held = true;
    }
    Some(chord)
}

/// The canonical spelling of a key name or alias, ASCII case ignored.
fn canonical_key(name: &str) -> Option<&'static str> {
    KEY_NAMES
        .iter()
        .copied()
        .find(|canonical| canonical.eq_ignore_ascii_case(name))
        .or_else(|| {
            KEY_ALIASES
                .iter()
                .find(|(alias, _)| alias.eq_ignore_ascii_case(name))
                .map(|&(_, canonical)| canonical)
        })
}

/// Tells whether a chord is kept by the editor and the clipboard in M4, so a user binding cannot
/// take it (m4 §4.4): caret motion and basic editing keys without Alt (the arrows, `Home`, `End`,
/// `PageUp`, `PageDown`, `Backspace`, `Delete`, with any Ctrl and Shift), `Enter`, `Escape` and
/// `Tab` without Ctrl or Alt (so `Shift+Tab` too), and the clipboard keys egui-winit turns into
/// copy, cut and paste events (`Ctrl+C`, `Ctrl+X`, `Ctrl+V` with any Shift and Alt,
/// `Ctrl+Insert`, `Shift+Insert`).
///
/// Bindings such as `Alt+Up`, `Ctrl+Alt+Up`, `Ctrl+Tab` or `Ctrl+Enter` stay free.
pub fn is_reserved_chord(chord: &KeyChord) -> bool {
    let motion = matches!(
        chord.key,
        "Up" | "Down"
            | "Left"
            | "Right"
            | "Home"
            | "End"
            | "PageUp"
            | "PageDown"
            | "Backspace"
            | "Delete"
    );
    let editing = matches!(chord.key, "Enter" | "Escape" | "Tab");
    let clipboard = (chord.ctrl && matches!(chord.key, "C" | "X" | "V"))
        || (chord.key == "Insert" && !chord.alt && chord.ctrl != chord.shift);
    (motion && !chord.alt) || (editing && !chord.ctrl && !chord.alt) || clipboard
}

/// One `[[binding]]` entry that passed its checks.
enum Entry {
    /// `command = "-<id>"`: removes the default bindings of `chord` that run `command`.
    Remove {
        /// The chord whose default binding goes.
        chord: KeyChord,
        /// The command id, without the `-`.
        command: String,
    },
    /// A user binding.
    Add(Binding),
}

/// Resolves the bindings: `defaults`, then the removals (`-id`), then the user bindings of
/// `keybindings.toml` in file order (`text` is `None` when the file is missing). `is_command`
/// tells whether a command id exists.
///
/// - A removal drops every default binding of its chord that runs its command, whatever its
///   `when`; it never removes a user binding.
/// - A user binding replaces any binding of the same chord and `when`, and comes after the
///   remaining defaults.
/// - An unknown command (`cfg-unknown-command`), a key that does not parse or that
///   [`is_reserved_chord`] keeps (`cfg-bad-key`), or a missing or non-string `key` or `command`
///   (`cfg-wrong-type`) skips the binding. An invalid `when` is `cfg-invalid-value` and the
///   binding applies globally (the default). Unknown keys are reported and ignored.
/// - A syntax error or a `schema_version` other than 1 gives the defaults and that one problem.
pub fn parse_keybindings(
    path: &Path,
    text: Option<&str>,
    defaults: &[Binding],
    is_command: &dyn Fn(&str) -> bool,
) -> (Vec<Binding>, Vec<ConfigProblem>) {
    let mut bindings = defaults.to_vec();
    let Some(text) = text else {
        return (bindings, Vec::new());
    };
    let text = strip_bom(text);
    let mut report = Report::new(path.to_path_buf(), text);
    let mut entries = Vec::new();
    if let Some(root) = parse_document(&mut report, text) {
        for (key, value) in &root {
            let name: &str = key.get_ref();
            match name {
                SCHEMA_VERSION_KEY => {}
                BINDING => read_bindings(&mut report, value, is_command, &mut entries),
                _ => report.unknown_key(name, key.span()),
            }
        }
    }
    for entry in &entries {
        if let Entry::Remove { chord, command } = entry {
            bindings.retain(|binding| !(binding.chord == *chord && binding.command == *command));
        }
    }
    for entry in entries {
        if let Entry::Add(added) = entry {
            bindings
                .retain(|binding| !(binding.chord == added.chord && binding.when == added.when));
            bindings.push(added);
        }
    }
    (bindings, report.finish())
}

/// The `binding` array: each element must be a table; its valid entries are appended in order.
fn read_bindings(
    report: &mut Report<'_>,
    value: &Spanned<DeValue<'_>>,
    is_command: &dyn Fn(&str) -> bool,
    entries: &mut Vec<Entry>,
) {
    let Some(array) = report.array(BINDING, value) else {
        return;
    };
    for element in array.iter() {
        if let Some(table) = report.table(BINDING, element)
            && let Some(entry) = read_binding(report, table, element.span(), is_command)
        {
            entries.push(entry);
        }
    }
}

/// One `[[binding]]` table (`span`: where it is written), or `None` when it is skipped.
fn read_binding(
    report: &mut Report<'_>,
    table: &DeTable<'_>,
    span: std::ops::Range<usize>,
    is_command: &dyn Fn(&str) -> bool,
) -> Option<Entry> {
    let mut chord = None;
    let mut command = None;
    let mut when = When::Global;
    let mut skipped = false;
    for (key, value) in table {
        let name: &str = key.get_ref();
        let dotted = format!("{BINDING}.{name}");
        match name {
            "key" => match report.string(&dotted, value) {
                Some(text) => {
                    chord = parse_chord(text).filter(|chord| !is_reserved_chord(chord));
                    if chord.is_none() {
                        report.push(
                            BAD_KEY,
                            value.span(),
                            vec![("key", DiagArg::Str(excerpt(text)))],
                        );
                        skipped = true;
                    }
                }
                None => skipped = true,
            },
            "command" => match report.string(&dotted, value) {
                Some(text) => {
                    let id = text.strip_prefix(REMOVE_PREFIX).unwrap_or(text);
                    if is_command(id) {
                        command = Some(text.to_owned());
                    } else {
                        report.push(
                            UNKNOWN_COMMAND,
                            value.span(),
                            vec![("command", DiagArg::Str(excerpt(text)))],
                        );
                        skipped = true;
                    }
                }
                None => skipped = true,
            },
            "when" => {
                if let Some(text) = report.string(&dotted, value) {
                    match When::from_id(text) {
                        Some(found) => when = found,
                        None => report.invalid_value(&dotted, text, WHEN_VALUES, value.span()),
                    }
                }
            }
            _ => report.unknown_key(&dotted, key.span()),
        }
    }
    if !skipped {
        for (field, found) in [("key", chord.is_some()), ("command", command.is_some())] {
            if !found {
                report.wrong_type(
                    &format!("{BINDING}.{field}"),
                    expected::STRING,
                    span.clone(),
                );
                skipped = true;
            }
        }
    }
    if skipped {
        return None;
    }
    let (chord, command) = (chord?, command?);
    Some(match command.strip_prefix(REMOVE_PREFIX) {
        Some(id) => Entry::Remove {
            chord,
            command: id.to_owned(),
        },
        None => Entry::Add(Binding {
            chord,
            command,
            when,
        }),
    })
}
