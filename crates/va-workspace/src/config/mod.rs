//! The configuration files and their problems (m4-editor-shell.md §4.2–§4.5): `settings.toml`,
//! `keybindings.toml` and `.visualasm/project.toml`.
//!
//! Every file is capped at [`MAX_CONFIG_BYTES`] and parsed with spans, then checked key by key:
//! a bad key never discards its neighbours, and every problem becomes a [`ConfigProblem`] with
//! its position, a non-blocking warning whose message is the Fluent `diag-<code>` of
//! `locales/<lang>/config.ftl`.
//!
//! The items live in private files (`problems.rs`, `settings.rs`, `keybindings.rs`,
//! `project.rs`) and are re-exported here by name, so their public paths are
//! `va_workspace::config::*` (m4 §15.1 M2).

mod keybindings;
mod problems;
mod project;
mod settings;

pub use keybindings::{
    Binding, KEY_NAMES, KeyChord, When, is_reserved_chord, parse_chord, parse_keybindings,
};
pub use problems::{CONFIG_CODES, ConfigProblem, MAX_CONFIG_BYTES, read_config_text};
pub use project::{PROJECT_FILE, Project, parse_project, project_arch_problem};
pub use settings::{
    AutosaveMode, DEFAULT_SETTINGS_TOML, EditorSettings, EolKind, ExplorerSettings, FileSettings,
    RenderSettings, RendererKind, Settings, ThemeKind, UiSettings, parse_settings,
};
