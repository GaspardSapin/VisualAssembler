//! The configuration files and their problems (m4-editor-shell.md §4.2–§4.5): `settings.toml`,
//! `keybindings.toml` and `.visualasm/project.toml`.
//!
//! Every file is capped at [`MAX_CONFIG_BYTES`] and parsed with spans, then checked key by key:
//! a bad key never discards its neighbours, and every problem becomes a [`ConfigProblem`] with
//! its position, a non-blocking warning.
//!
//! WP0 scaffold: the frozen signatures of §4.2–§4.5 with `todo!()` bodies. The §4.3 types, their
//! `Default` impls, the enum ids and [`DEFAULT_SETTINGS_TOML`] are complete (WP0 follow-up, m4
//! §1). WP-B implements the rest and may move the items into `problems.rs`, `settings.rs`,
//! `keybindings.rs` and `project.rs`, re-exported here so the paths `va_workspace::config::*`
//! that the other work packages compile against stay valid.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-B implements m4 §4.2-§4.5"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use va_core::diag::{DiagArg, DiagCode, Diagnostic};
use va_core::source::{FileId, LineCol, TextRange};

use crate::i18n::Lang;

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
// settings.toml (§4.3, frozen)
// -------------------------------------------------------------------------------------------

/// The settings of `settings.toml`, one field per table; `Default` is the §4.3 Default column.
#[derive(Clone, Debug, PartialEq)]
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

impl Default for Settings {
    /// Every table at its default.
    fn default() -> Self {
        Self {
            ui: UiSettings::default(),
            editor: EditorSettings::default(),
            files: FileSettings::default(),
            explorer: ExplorerSettings::default(),
            render: RenderSettings::default(),
        }
    }
}

/// The `[ui]` table.
#[derive(Clone, Debug, PartialEq)]
pub struct UiSettings {
    /// `ui.language`: the UI and diagnostics language; `None` is `"auto"`. Applies at the next
    /// start.
    pub language: Option<Lang>,
    /// `ui.theme`: the base theme; `None` is `"auto"` (follows the system).
    pub theme: Option<ThemeKind>,
    /// `ui.native_dialogs`: Windows native open and save dialogs; `false` (and always on Linux)
    /// uses the internal path prompt.
    pub native_dialogs: bool,
}

impl Default for UiSettings {
    /// `language = "auto"`, `theme = "auto"`, `native_dialogs = true`.
    fn default() -> Self {
        Self {
            language: None,
            theme: None,
            native_dialogs: true,
        }
    }
}

/// The `[editor]` table.
#[derive(Clone, Debug, PartialEq)]
pub struct EditorSettings {
    /// `editor.font_size`: code font size in points before zoom, 8.0 to 40.0.
    pub font_size: f32,
    /// `editor.font_file`: a `.ttf` or `.otf` replacing the embedded font, absolute or relative
    /// to the config folder, stored as written; `None` is `""`.
    pub font_file: Option<PathBuf>,
    /// `editor.tab_width`: width of a tab stop in columns, 1 to 16.
    pub tab_width: u8,
    /// `editor.insert_spaces`: the Tab key inserts spaces; `false` inserts a tab character.
    pub insert_spaces: bool,
    /// `editor.cursor_blink`: blinking caret.
    pub cursor_blink: bool,
    /// `editor.line_numbers`: line numbers in the gutter.
    pub line_numbers: bool,
    /// `editor.highlight_current_line`: background of the caret's line.
    pub highlight_current_line: bool,
    /// `editor.diagnostics_delay_ms`: delay before diagnostics refresh, 50 to 5000; parsed now,
    /// used from M5.
    pub diagnostics_delay_ms: u32,
}

impl Default for EditorSettings {
    /// `font_size = 14.0`, `font_file = ""`, `tab_width = 8`, `insert_spaces = false`,
    /// `cursor_blink = true`, `line_numbers = true`, `highlight_current_line = true`,
    /// `diagnostics_delay_ms = 300`.
    fn default() -> Self {
        Self {
            font_size: 14.0,
            font_file: None,
            tab_width: 8,
            insert_spaces: false,
            cursor_blink: true,
            line_numbers: true,
            highlight_current_line: true,
            diagnostics_delay_ms: 300,
        }
    }
}

/// The `[files]` table.
#[derive(Clone, Debug, PartialEq)]
pub struct FileSettings {
    /// `files.hot_exit`: closing the window keeps unsaved changes and restores them at the next
    /// start.
    pub hot_exit: bool,
    /// `files.backup_delay_ms`: delay after the last edit before a backup, 250 to 60000.
    pub backup_delay_ms: u32,
    /// `files.autosave`: when the student's files are saved automatically.
    pub autosave: AutosaveMode,
    /// `files.autosave_delay_ms`: delay of the `after-delay` autosave, 250 to 60000.
    pub autosave_delay_ms: u32,
    /// `files.eol`: line ending of new files; `None` is `"auto"` (CRLF on Windows, LF
    /// elsewhere).
    pub eol: Option<EolKind>,
}

impl Default for FileSettings {
    /// `hot_exit = true`, `backup_delay_ms = 1500`, `autosave = "off"`,
    /// `autosave_delay_ms = 1000`, `eol = "auto"`.
    fn default() -> Self {
        Self {
            hot_exit: true,
            backup_delay_ms: 1500,
            autosave: AutosaveMode::Off,
            autosave_delay_ms: 1000,
            eol: None,
        }
    }
}

/// The `[explorer]` table.
#[derive(Clone, Debug, PartialEq)]
pub struct ExplorerSettings {
    /// `explorer.exclude`: names hidden in the explorer and in quick open (non-empty, without
    /// `/` or `\`; case-insensitive on Windows).
    pub exclude: Vec<String>,
}

impl Default for ExplorerSettings {
    /// `exclude = [".git"]`.
    fn default() -> Self {
        Self {
            exclude: vec![".git".to_owned()],
        }
    }
}

/// The `[render]` table.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderSettings {
    /// `render.renderer`: the renderer; `None` is `"auto"`. Applies at the next start, below
    /// `--renderer` and `VA_RENDERER`.
    pub renderer: Option<RendererKind>,
}

impl Default for RenderSettings {
    /// `renderer = "auto"`.
    fn default() -> Self {
        Self { renderer: None }
    }
}

/// A colour theme (`ui.theme`, and the session's override).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeKind {
    /// `"light"`.
    Light,
    /// `"dark"`.
    Dark,
}

impl ThemeKind {
    /// The id of the theme in TOML files: `"light"` or `"dark"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Parses an exact lowercase id; anything else, `"auto"` included, gives `None`.
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }
}

/// A line ending of new files (`files.eol`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EolKind {
    /// `"lf"`: `\n`.
    Lf,
    /// `"crlf"`: `\r\n`.
    CrLf,
}

impl EolKind {
    /// The id of the line ending in TOML files: `"lf"` or `"crlf"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Lf => "lf",
            Self::CrLf => "crlf",
        }
    }

    /// Parses an exact lowercase id; anything else, `"auto"` included, gives `None`.
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "lf" => Some(Self::Lf),
            "crlf" => Some(Self::CrLf),
            _ => None,
        }
    }
}

/// A renderer of the window (`render.renderer`, m0 §19).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RendererKind {
    /// `"glow"`: OpenGL.
    Glow,
    /// `"wgpu"`.
    Wgpu,
}

impl RendererKind {
    /// The id of the renderer in TOML files: `"glow"` or `"wgpu"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Glow => "glow",
            Self::Wgpu => "wgpu",
        }
    }

    /// Parses an exact lowercase id; anything else, `"auto"` included, gives `None`.
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "glow" => Some(Self::Glow),
            "wgpu" => Some(Self::Wgpu),
            _ => None,
        }
    }
}

/// When the student's files are saved automatically (`files.autosave`, m4 §4.8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AutosaveMode {
    /// `"off"`: only Save, Save As and Save All write a file (the default).
    #[default]
    Off,
    /// `"after-delay"`: dirty file buffers are saved `files.autosave_delay_ms` after the last
    /// edit.
    AfterDelay,
    /// `"on-focus-change"`: dirty file buffers are saved when the window loses focus or the tab
    /// changes.
    OnFocusChange,
}

impl AutosaveMode {
    /// The id of the mode in TOML files: `"off"`, `"after-delay"` or `"on-focus-change"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::AfterDelay => "after-delay",
            Self::OnFocusChange => "on-focus-change",
        }
    }

    /// Parses an exact lowercase id; anything else gives `None`.
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "off" => Some(Self::Off),
            "after-delay" => Some(Self::AfterDelay),
            "on-focus-change" => Some(Self::OnFocusChange),
            _ => None,
        }
    }
}

/// Parses `settings.toml` (`text` is `None` when the file is missing): the defaults overridden
/// by every valid key, and one problem per invalid one.
pub fn parse_settings(path: &Path, text: Option<&str>) -> (Settings, Vec<ConfigProblem>) {
    todo!()
}

/// The commented default `settings.toml` (ASCII, LF): `schema_version = 1`, then every key of
/// §4.3 commented out with its default value. The app writes it once, when the file is missing.
pub const DEFAULT_SETTINGS_TOML: &str = include_str!("default_settings.toml");

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
