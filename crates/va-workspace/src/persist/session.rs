//! `session.toml` (m4-editor-shell.md §4.7, frozen 2026-10-24): the session types, their
//! defaults, [`parse_session`] and [`session_to_toml`].
//!
//! The parser never fails and reports nothing: a missing file, a syntax error or another
//! `schema_version` give [`Session::default`], and each bad value takes its default on its own.
//! The writer emits a fixed layout (`schema_version`, then `window`, `ui` and `workspace` as
//! inline tables, then one `[[tab]]` per tab) that the parser reads back unchanged.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use toml::{Table, Value};

use super::backup::BackupId;
use crate::config::ThemeKind;

/// The key of the format version.
const SCHEMA_VERSION_KEY: &str = "schema_version";
/// The only `schema_version` this build reads and writes.
const SCHEMA_VERSION: i64 = 1;

/// Range of `window.width`, logical pixels.
const WIDTH_RANGE: (f64, f64) = (400.0, 10_000.0);
/// Range of `window.height`, logical pixels.
const HEIGHT_RANGE: (f64, f64) = (300.0, 10_000.0);
/// Range of `ui.zoom` (m4 §5.5 `ZOOM_MIN` to `ZOOM_MAX`).
const ZOOM_RANGE: (f64, f64) = (0.5, 3.0);
/// Range of `ui.explorer_width`, logical pixels.
const EXPLORER_WIDTH_RANGE: (f64, f64) = (120.0, 2000.0);
/// The value of `line`, `column` and `top_line` when absent or invalid (1-based).
const FIRST: u32 = 1;

/// The content of `session.toml` (§4.7): the window, the UI state, the project root and the
/// open tabs.
#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    /// The `window` table: the window geometry, read before the window opens.
    pub window: SessionWindow,
    /// The `ui` table: zoom, theme override and explorer.
    pub ui: SessionUi,
    /// The `workspace` table: the project root.
    pub workspace: SessionWorkspace,
    /// The `[[tab]]` tables, in tab order.
    pub tabs: Vec<SessionTab>,
}

impl Default for Session {
    /// Every table at its default, no tab.
    fn default() -> Self {
        Self {
            window: SessionWindow::default(),
            ui: SessionUi::default(),
            workspace: SessionWorkspace::default(),
            tabs: Vec::new(),
        }
    }
}

/// The `window` table of `session.toml`, in logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionWindow {
    /// `width`, 400 to 10000.
    pub width: f32,
    /// `height`, 300 to 10000.
    pub height: f32,
    /// `maximized`.
    pub maximized: bool,
}

impl Default for SessionWindow {
    /// 1280 × 800 (M0's window size), not maximized.
    fn default() -> Self {
        Self {
            width: 1280.0,
            height: 800.0,
            maximized: false,
        }
    }
}

/// The `ui` table of `session.toml`.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionUi {
    /// `zoom`, 0.5 to 3.0 (m4 §5.5 `ZOOM_MIN` to `ZOOM_MAX`).
    pub zoom: f32,
    /// `theme`: the palette toggle's override of `ui.theme`; `None` is no override.
    pub theme: Option<ThemeKind>,
    /// `explorer_visible`.
    pub explorer_visible: bool,
    /// `explorer_width` in logical pixels, 120 to 2000.
    pub explorer_width: f32,
}

impl Default for SessionUi {
    /// Zoom 1.0, no theme override, explorer visible and 240 pixels wide.
    fn default() -> Self {
        Self {
            zoom: 1.0,
            theme: None,
            explorer_visible: true,
            explorer_width: 240.0,
        }
    }
}

/// The `workspace` table of `session.toml`.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionWorkspace {
    /// `root`: the absolute project root; `None` when no folder is open.
    pub root: Option<PathBuf>,
}

impl Default for SessionWorkspace {
    /// No project root.
    fn default() -> Self {
        Self { root: None }
    }
}

/// One `[[tab]]` table of `session.toml`: what the tab shows and where its caret was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionTab {
    /// `path` or `backup`: the file or the untitled buffer.
    pub source: TabSource,
    /// `line` of the caret, 1-based.
    pub line: u32,
    /// `column` of the caret in chars, 1-based.
    pub column: u32,
    /// `top_line`: the first visible line, 1-based.
    pub top_line: u32,
    /// `active`: the selected tab (only the first one stays active).
    pub active: bool,
}

/// What a session tab shows.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TabSource {
    /// `path = "…"`: a file, by its absolute path.
    File(PathBuf),
    /// `backup = "<id>"`: an untitled buffer, by its backup.
    Backup(BackupId),
}

/// Parses `session.toml` (`text` is `None` when the file is missing); never fails: bad parts
/// are dropped.
///
/// A missing file, a syntax error or a `schema_version` other than the integer 1 gives
/// [`Session::default`]; a missing `schema_version` means 1 (as in every configuration file,
/// m4 §4.2). Otherwise each value of the wrong type or out of range takes its default (a float
/// key also takes a TOML integer, as in `settings.toml`); a `theme` other than `light`/`dark`
/// and a `root` that is not absolute are `None`; a `[[tab]]` is dropped unless it has exactly
/// one of an absolute `path` or a well-formed `backup` id; only the first `active = true` tab
/// stays active; unknown keys are ignored.
pub fn parse_session(text: Option<&str>) -> Session {
    let Some(text) = text else {
        return Session::default();
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Ok(document) = text.parse::<Table>() else {
        return Session::default();
    };
    match document.get(SCHEMA_VERSION_KEY) {
        None => {}
        Some(Value::Integer(version)) if *version == SCHEMA_VERSION => {}
        Some(_) => return Session::default(),
    }
    let mut session = Session::default();
    if let Some(Value::Table(window)) = document.get("window") {
        let target = &mut session.window;
        set_float(&mut target.width, window.get("width"), WIDTH_RANGE);
        set_float(&mut target.height, window.get("height"), HEIGHT_RANGE);
        set_bool(&mut target.maximized, window.get("maximized"));
    }
    if let Some(Value::Table(ui)) = document.get("ui") {
        let target = &mut session.ui;
        set_float(&mut target.zoom, ui.get("zoom"), ZOOM_RANGE);
        target.theme = match ui.get("theme") {
            Some(Value::String(id)) => ThemeKind::from_id(id),
            _ => None,
        };
        set_bool(&mut target.explorer_visible, ui.get("explorer_visible"));
        set_float(
            &mut target.explorer_width,
            ui.get("explorer_width"),
            EXPLORER_WIDTH_RANGE,
        );
    }
    if let Some(Value::Table(workspace)) = document.get("workspace") {
        session.workspace.root = match workspace.get("root") {
            Some(Value::String(root)) if Path::new(root).is_absolute() => Some(PathBuf::from(root)),
            _ => None,
        };
    }
    if let Some(Value::Array(tabs)) = document.get("tab") {
        let mut active_seen = false;
        for value in tabs {
            let Value::Table(table) = value else {
                continue;
            };
            let Some(mut tab) = parse_tab(table) else {
                continue;
            };
            if tab.active {
                tab.active = !active_seen;
                active_seen = true;
            }
            session.tabs.push(tab);
        }
    }
    session
}

/// One `[[tab]]` table, or `None` when it must be dropped.
fn parse_tab(table: &Table) -> Option<SessionTab> {
    let source = match (table.get("path"), table.get("backup")) {
        (Some(Value::String(path)), None) if Path::new(path).is_absolute() => {
            TabSource::File(PathBuf::from(path))
        }
        (None, Some(Value::String(id))) => TabSource::Backup(BackupId::parse(id)?),
        _ => return None,
    };
    let mut tab = SessionTab {
        source,
        line: FIRST,
        column: FIRST,
        top_line: FIRST,
        active: false,
    };
    set_position(&mut tab.line, table.get("line"));
    set_position(&mut tab.column, table.get("column"));
    set_position(&mut tab.top_line, table.get("top_line"));
    set_bool(&mut tab.active, table.get("active"));
    Some(tab)
}

/// Sets `target` from a float (or integer) value inside the inclusive `range`; anything else
/// keeps the default.
fn set_float(target: &mut f32, value: Option<&Value>, (min, max): (f64, f64)) {
    let number = match value {
        Some(Value::Float(number)) => *number,
        // A float key also takes a TOML integer (`width = 1280`); the cast is exact in range
        // (an integer beyond ±2^53 is far out of every range anyway).
        Some(Value::Integer(number)) => *number as f64,
        _ => return,
    };
    if (min..=max).contains(&number) {
        // The range bounds are exact f32 values, so the narrowed value stays in range.
        *target = number as f32;
    }
}

/// Sets `target` from a boolean value; anything else keeps the default.
fn set_bool(target: &mut bool, value: Option<&Value>) {
    if let Some(Value::Boolean(flag)) = value {
        *target = *flag;
    }
}

/// Sets a 1-based position from an integer of 1 to `u32::MAX`; anything else keeps 1.
fn set_position(target: &mut u32, value: Option<&Value>) {
    if let Some(Value::Integer(number)) = value
        && let Ok(position) = u32::try_from(*number)
        && position >= FIRST
    {
        *target = position;
    }
}

/// Serializes `session` as `session.toml` text.
///
/// The text is `schema_version = 1`, then `window`, `ui` and `workspace` as inline tables, then
/// one `[[tab]]` per tab, LF line endings. `theme` and `root` are omitted when `None`, and a
/// path that is not valid Unicode is omitted (a tab with such a path is left out). Paths are
/// written verbatim, as TOML basic strings.
pub fn session_to_toml(session: &Session) -> String {
    let mut out = String::new();
    let window = &session.window;
    let ui = &session.ui;
    // `write!` into a `String` never fails.
    let _ = writeln!(out, "{SCHEMA_VERSION_KEY} = {SCHEMA_VERSION}");
    let _ = writeln!(
        out,
        "window = {{ width = {}, height = {}, maximized = {} }}",
        toml_float(window.width),
        toml_float(window.height),
        window.maximized,
    );
    let _ = write!(out, "ui = {{ zoom = {}", toml_float(ui.zoom));
    if let Some(theme) = ui.theme {
        let _ = write!(out, ", theme = {}", toml_string(theme.id()));
    }
    let _ = writeln!(
        out,
        ", explorer_visible = {}, explorer_width = {} }}",
        ui.explorer_visible,
        toml_float(ui.explorer_width),
    );
    match session.workspace.root.as_deref().and_then(Path::to_str) {
        Some(root) => {
            let _ = writeln!(out, "workspace = {{ root = {} }}", toml_string(root));
        }
        None => out.push_str("workspace = {}\n"),
    }
    for tab in &session.tabs {
        let source = match &tab.source {
            TabSource::File(path) => match path.to_str() {
                Some(path) => format!("path = {}", toml_string(path)),
                None => continue,
            },
            TabSource::Backup(id) => format!("backup = {}", toml_string(&id.0)),
        };
        let _ = write!(
            out,
            "\n[[tab]]\n{source}\nline = {}\ncolumn = {}\ntop_line = {}\nactive = {}\n",
            tab.line, tab.column, tab.top_line, tab.active,
        );
    }
    out
}

/// `text` as a one-line TOML basic string: `"` and `\` escaped, control characters as escape
/// sequences, everything else verbatim.
pub(super) fn toml_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04X}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `value` as a TOML float that parses back to the same `f32`: the shortest decimal form with
/// a `.0` when it has no point, `nan`/`inf`/`-inf` for the special values (which the parser
/// then reads as out of range).
fn toml_float(value: f32) -> String {
    if value.is_nan() {
        return "nan".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "inf" } else { "-inf" }.to_owned();
    }
    let mut text = value.to_string();
    if !text.contains('.') {
        text.push_str(".0");
    }
    // The shortest f32 form read as f64 (as TOML does) and narrowed back is the same value in
    // practice; the exact f64 form is the fallback should rounding twice ever differ.
    let same = text.parse::<f64>().is_ok_and(|wide| wide as f32 == value);
    if same {
        text
    } else {
        format!("{:?}", f64::from(value))
    }
}
