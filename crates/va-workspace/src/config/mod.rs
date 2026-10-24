//! The configuration files and their problems (m4-editor-shell.md §4.2–§4.5): `settings.toml`,
//! `keybindings.toml` and `.visualasm/project.toml`.
//!
//! Every file is capped at [`MAX_CONFIG_BYTES`] and parsed with spans, then checked key by key:
//! a bad key never discards its neighbours, and every problem becomes a [`ConfigProblem`] with
//! its position, a non-blocking warning.
//!
//! WP0 scaffold: the frozen signatures of §4.2, §4.4 and §4.5, and the `Settings` shape of §4.3,
//! with `todo!()` bodies. WP-B implements them and may move them into `problems.rs`,
//! `settings.rs`, `keybindings.rs` and `project.rs`, re-exported here so the paths
//! `va_workspace::config::*` that the other work packages compile against stay valid.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-B implements m4 §4.2-§4.5"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use va_core::diag::{DiagArg, DiagCode, Diagnostic};
use va_core::source::{FileId, LineCol, TextRange};

// -------------------------------------------------------------------------------------------
// Problems (§4.2, frozen)
// -------------------------------------------------------------------------------------------

/// Largest configuration file read, in bytes (1 MiB).
pub const MAX_CONFIG_BYTES: usize = 1024 * 1024;

/// One problem found in a configuration file: a non-blocking warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigProblem {
    /// The file.
    pub path: PathBuf,
    /// One of [`CONFIG_CODES`].
    pub code: DiagCode,
    /// Byte range of the offending text in the file.
    pub range: TextRange,
    /// Its display position (1-based line and column).
    pub at: LineCol,
    /// Fluent arguments of the message `diag-<code>`.
    pub args: Vec<(&'static str, DiagArg)>,
}

impl ConfigProblem {
    /// The problem as a `Severity::Warning`, non-blocking diagnostic of `file` (M5 Problems
    /// panel).
    pub fn to_diagnostic(&self, file: FileId) -> Diagnostic {
        todo!()
    }
}

/// Every code a configuration problem can carry (§4.2 table, in its order).
pub static CONFIG_CODES: [DiagCode; 12] = [
    DiagCode("cfg-unreadable"),
    DiagCode("cfg-too-large"),
    DiagCode("cfg-not-utf8"),
    DiagCode("cfg-syntax"),
    DiagCode("cfg-schema-version"),
    DiagCode("cfg-unknown-key"),
    DiagCode("cfg-wrong-type"),
    DiagCode("cfg-out-of-range"),
    DiagCode("cfg-invalid-value"),
    DiagCode("cfg-unknown-command"),
    DiagCode("cfg-bad-key"),
    DiagCode("cfg-bad-path"),
];

/// Reads a configuration file as UTF-8 text, capped at [`MAX_CONFIG_BYTES`]; `Ok(None)` when
/// the file is missing.
///
/// # Errors
/// A `cfg-unreadable`, `cfg-too-large` or `cfg-not-utf8` problem.
pub fn read_config_text(path: &Path) -> Result<Option<String>, ConfigProblem> {
    todo!()
}

// -------------------------------------------------------------------------------------------
// settings.toml (§4.3)
// -------------------------------------------------------------------------------------------

/// The settings of `settings.toml`; `Default` is the §4.3 table.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    /// The `[ui]` table.
    pub ui: UiSettings,
    /// The `[editor]` table.
    pub editor: EditorSettings,
    /// The `[files]` table.
    pub files: FileSettings,
    /// The `[explorer]` table.
    pub explorer: ExplorerSettings,
    /// The `[render]` table.
    pub render: RenderSettings,
}

/// The `ui.*` keys of §4.3 (`language`, `theme`, `native_dialogs`); fields arrive with WP-B.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiSettings {}

/// The `editor.*` keys of §4.3 (font, tabs, blink, gutter, current line, diagnostics delay);
/// fields arrive with WP-B.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditorSettings {}

/// The `files.*` keys of §4.3 (hot exit, backup delay, autosave, line ending); fields arrive
/// with WP-B.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileSettings {}

/// The `explorer.*` keys of §4.3 (`exclude`); fields arrive with WP-B.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExplorerSettings {}

/// The `render.*` keys of §4.3 (`renderer`); fields arrive with WP-B.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderSettings {}

/// Parses `settings.toml` (`text` is `None` when the file is missing): the defaults overridden
/// by every valid key, and one problem per invalid one.
pub fn parse_settings(path: &Path, text: Option<&str>) -> (Settings, Vec<ConfigProblem>) {
    todo!()
}

// -------------------------------------------------------------------------------------------
// keybindings.toml (§4.4, frozen)
// -------------------------------------------------------------------------------------------

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
pub fn parse_chord(text: &str) -> Option<KeyChord> {
    todo!()
}

/// Resolves the bindings: `defaults`, then the removals (`-id`), then the user bindings of
/// `keybindings.toml` in file order (`text` is `None` when the file is missing). `is_command`
/// tells whether a command id exists.
pub fn parse_keybindings(
    path: &Path,
    text: Option<&str>,
    defaults: &[Binding],
    is_command: &dyn Fn(&str) -> bool,
) -> (Vec<Binding>, Vec<ConfigProblem>) {
    todo!()
}

// -------------------------------------------------------------------------------------------
// .visualasm/project.toml (§4.5, frozen)
// -------------------------------------------------------------------------------------------

/// A project: a folder opened with "Open folder", and its optional `project.toml`.
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    /// The project root.
    pub root: PathBuf,
    /// Id of a registered architecture; `None` means the app's default.
    pub arch: Option<String>,
    /// The I/O profile (M3 meaning; stored as is in M4).
    pub io_profile: Option<String>,
    /// The entry file, relative to the root without `..`.
    pub entry: Option<PathBuf>,
    /// Include folders, relative to the root without `..` (used from M5).
    pub include_dirs: Vec<PathBuf>,
    /// Tables reserved for later milestones (`assembler`, `output`, `run`, `debug`), kept raw.
    pub reserved: BTreeMap<String, toml::Table>,
}

/// Parses the `project.toml` of `root` (`text` is `None` when the file is missing).
pub fn parse_project(root: &Path, text: Option<&str>) -> (Project, Vec<ConfigProblem>) {
    todo!()
}

/// Path of the project file, relative to the project root.
pub const PROJECT_FILE: &str = ".visualasm/project.toml";
