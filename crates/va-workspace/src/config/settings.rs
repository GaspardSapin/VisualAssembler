//! `settings.toml` (m4-editor-shell.md §4.3, frozen 2026-10-24): the settings types, their
//! defaults, the commented default file and [`parse_settings`].
//!
//! The parser walks the schema by hand over a spanned TOML table: each key is checked on its own,
//! so a bad value keeps its default and never discards its neighbours.

use std::path::{Path, PathBuf};

use toml::Spanned;
use toml::de::{DeTable, DeValue};

use super::problems::{
    ConfigProblem, Report, SCHEMA_VERSION_KEY, expected, parse_document, strip_bom,
};
use crate::i18n::Lang;

/// The string value that means "decide automatically" (`None` in the settings).
const AUTO: &str = "auto";

/// The `expected` argument of `ui.language` problems.
const LANGUAGE_VALUES: &str = "auto, en, fr";
/// The `expected` argument of `ui.theme` problems.
const THEME_VALUES: &str = "auto, light, dark";
/// The `expected` argument of `files.autosave` problems.
const AUTOSAVE_VALUES: &str = "off, after-delay, on-focus-change";
/// The `expected` argument of `files.eol` problems.
const EOL_VALUES: &str = "auto, lf, crlf";
/// The `expected` argument of `render.renderer` problems.
const RENDERER_VALUES: &str = "auto, glow, wgpu";
/// The `expected` id of an `editor.font_file` that is not a `.ttf` or `.otf` path.
const FONT_FILE: &str = "font-file";
/// The `expected` id of an `explorer.exclude` element that is not a plain name.
const FILE_NAME: &str = "file-name";

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
/// by every valid key, and one problem per invalid one, in file order.
///
/// A syntax error or a `schema_version` other than 1 gives the defaults and that one problem.
/// Otherwise an unknown key is `cfg-unknown-key`, a value of the wrong TOML type
/// `cfg-wrong-type`, a number outside its range `cfg-out-of-range` and a string outside its
/// allowed values `cfg-invalid-value`; each keeps that key's default. String values are exact
/// lowercase ids; `"auto"` gives `None` for `language`, `theme`, `eol` and `renderer`.
pub fn parse_settings(path: &Path, text: Option<&str>) -> (Settings, Vec<ConfigProblem>) {
    let mut settings = Settings::default();
    let Some(text) = text else {
        return (settings, Vec::new());
    };
    let text = strip_bom(text);
    let mut report = Report::new(path.to_path_buf(), text);
    if let Some(root) = parse_document(&mut report, text) {
        for (key, value) in &root {
            let name: &str = key.get_ref();
            let reader: fn(&mut Report<'_>, &DeTable<'_>, &mut Settings) = match name {
                SCHEMA_VERSION_KEY => continue,
                "ui" => read_ui,
                "editor" => read_editor,
                "files" => read_files,
                "explorer" => read_explorer,
                "render" => read_render,
                _ => {
                    report.unknown_key(name, key.span());
                    continue;
                }
            };
            if let Some(table) = report.table(name, value) {
                reader(&mut report, table, &mut settings);
            }
        }
    }
    (settings, report.finish())
}

/// The commented default `settings.toml` (ASCII, LF): `schema_version = 1`, then every key of
/// §4.3 commented out with its default value. The app writes it once, when the file is missing.
pub const DEFAULT_SETTINGS_TOML: &str = include_str!("default_settings.toml");

/// Calls `read` with the dotted key `<table>.<key>` and the value of each entry of `table`;
/// `read` returns `false` for a key outside the schema, which becomes `cfg-unknown-key`.
fn for_each_key<'t>(
    report: &mut Report<'t>,
    table_name: &str,
    table: &DeTable<'_>,
    mut read: impl FnMut(&mut Report<'t>, &str, &str, &Spanned<DeValue<'_>>) -> bool,
) {
    for (key, value) in table {
        let name: &str = key.get_ref();
        let dotted = format!("{table_name}.{name}");
        if !read(report, name, &dotted, value) {
            report.unknown_key(&dotted, key.span());
        }
    }
}

/// Stores `found` in `slot` when it is `Some` (the default stays otherwise).
fn set<T>(slot: &mut T, found: Option<T>) {
    if let Some(found) = found {
        *slot = found;
    }
}

/// Reads a string id where `"auto"` means `None`: an id `parse` rejects is `cfg-invalid-value`
/// and gives `None` too (the default of these keys).
fn auto_id<T>(
    report: &mut Report<'_>,
    key: &str,
    value: &Spanned<DeValue<'_>>,
    parse: fn(&str) -> Option<T>,
    allowed: &str,
) -> Option<T> {
    let text = report.string(key, value)?;
    if text == AUTO {
        return None;
    }
    let parsed = parse(text);
    if parsed.is_none() {
        report.invalid_value(key, text, allowed, value.span());
    }
    parsed
}

/// Reads an integer key in `min..=max` milliseconds.
fn millis(
    report: &mut Report<'_>,
    key: &str,
    value: &Spanned<DeValue<'_>>,
    min: u32,
    max: u32,
) -> Option<u32> {
    report
        .integer_in(key, value, i64::from(min), i64::from(max))
        .and_then(|found| u32::try_from(found).ok())
}

/// The `[ui]` table.
fn read_ui(report: &mut Report<'_>, table: &DeTable<'_>, settings: &mut Settings) {
    let ui = &mut settings.ui;
    for_each_key(report, "ui", table, |report, name, key, value| {
        match name {
            "language" => ui.language = auto_id(report, key, value, Lang::parse, LANGUAGE_VALUES),
            "theme" => ui.theme = auto_id(report, key, value, ThemeKind::from_id, THEME_VALUES),
            "native_dialogs" => set(&mut ui.native_dialogs, report.boolean(key, value)),
            _ => return false,
        }
        true
    });
}

/// The `[editor]` table.
fn read_editor(report: &mut Report<'_>, table: &DeTable<'_>, settings: &mut Settings) {
    let editor = &mut settings.editor;
    for_each_key(report, "editor", table, |report, name, key, value| {
        match name {
            "font_size" => set(
                &mut editor.font_size,
                report.number_in(key, value, 8, 40).map(|size| size as f32),
            ),
            "font_file" => set(&mut editor.font_file, font_file(report, key, value)),
            "tab_width" => set(
                &mut editor.tab_width,
                report
                    .integer_in(key, value, 1, 16)
                    .and_then(|width| u8::try_from(width).ok()),
            ),
            "insert_spaces" => set(&mut editor.insert_spaces, report.boolean(key, value)),
            "cursor_blink" => set(&mut editor.cursor_blink, report.boolean(key, value)),
            "line_numbers" => set(&mut editor.line_numbers, report.boolean(key, value)),
            "highlight_current_line" => {
                set(
                    &mut editor.highlight_current_line,
                    report.boolean(key, value),
                );
            }
            "diagnostics_delay_ms" => set(
                &mut editor.diagnostics_delay_ms,
                millis(report, key, value, 50, 5000),
            ),
            _ => return false,
        }
        true
    });
}

/// `editor.font_file`: `""` is `None`; another string must end in `.ttf` or `.otf` (ASCII case
/// ignored) and is stored as written. `None` (the outer one) keeps the default.
fn font_file(
    report: &mut Report<'_>,
    key: &str,
    value: &Spanned<DeValue<'_>>,
) -> Option<Option<PathBuf>> {
    let text = report.string(key, value)?;
    if text.is_empty() {
        return Some(None);
    }
    let bytes = text.as_bytes();
    let font = bytes.len() >= 4 && {
        let suffix = &bytes[bytes.len() - 4..];
        suffix.eq_ignore_ascii_case(b".ttf") || suffix.eq_ignore_ascii_case(b".otf")
    };
    if font {
        Some(Some(PathBuf::from(text)))
    } else {
        report.invalid_value(key, text, FONT_FILE, value.span());
        None
    }
}

/// The `[files]` table.
fn read_files(report: &mut Report<'_>, table: &DeTable<'_>, settings: &mut Settings) {
    let files = &mut settings.files;
    for_each_key(report, "files", table, |report, name, key, value| {
        match name {
            "hot_exit" => set(&mut files.hot_exit, report.boolean(key, value)),
            "backup_delay_ms" => set(
                &mut files.backup_delay_ms,
                millis(report, key, value, 250, 60_000),
            ),
            "autosave" => {
                if let Some(text) = report.string(key, value) {
                    let mode = AutosaveMode::from_id(text);
                    if mode.is_none() {
                        report.invalid_value(key, text, AUTOSAVE_VALUES, value.span());
                    }
                    set(&mut files.autosave, mode);
                }
            }
            "autosave_delay_ms" => set(
                &mut files.autosave_delay_ms,
                millis(report, key, value, 250, 60_000),
            ),
            "eol" => files.eol = auto_id(report, key, value, EolKind::from_id, EOL_VALUES),
            _ => return false,
        }
        true
    });
}

/// The `[explorer]` table.
fn read_explorer(report: &mut Report<'_>, table: &DeTable<'_>, settings: &mut Settings) {
    let explorer = &mut settings.explorer;
    for_each_key(report, "explorer", table, |report, name, key, value| {
        match name {
            "exclude" => set(&mut explorer.exclude, exclude(report, key, value)),
            _ => return false,
        }
        true
    });
}

/// `explorer.exclude`: an array of non-empty names without `/` or `\`. A bad element is dropped
/// with its problem and the others are kept; a value that is not an array keeps the default.
fn exclude(
    report: &mut Report<'_>,
    key: &str,
    value: &Spanned<DeValue<'_>>,
) -> Option<Vec<String>> {
    let array = report.array(key, value)?;
    let mut names = Vec::with_capacity(array.len());
    for element in array.iter() {
        let Some(name) = element.get_ref().as_str() else {
            report.wrong_type(key, expected::STRING, element.span());
            continue;
        };
        if name.is_empty() || name.contains(['/', '\\']) {
            report.invalid_value(key, name, FILE_NAME, element.span());
            continue;
        }
        names.push(name.to_owned());
    }
    Some(names)
}

/// The `[render]` table.
fn read_render(report: &mut Report<'_>, table: &DeTable<'_>, settings: &mut Settings) {
    let render = &mut settings.render;
    for_each_key(report, "render", table, |report, name, key, value| {
        match name {
            "renderer" => {
                render.renderer =
                    auto_id(report, key, value, RendererKind::from_id, RENDERER_VALUES);
            }
            _ => return false,
        }
        true
    });
}
