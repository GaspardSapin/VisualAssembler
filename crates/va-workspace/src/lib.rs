//! Project model, settings/keybindings TOML, session and hot-exit backups, Fluent loading and
//! rendering (architecture §5, §6; m4-editor-shell.md §4). No UI; it knows nothing of ropey or
//! egui: text crosses its API as `&str` or as a writer closure.
//!
//! Allowed internal dependencies: `va-core`.
//!
//! Modules: [`i18n`], the Fluent localizer (m1-core-isa-lexer.md §6.2, extended by m4 §4.6);
//! [`paths`], the config and data folders and portable mode (m4 §4.1); [`config`], the
//! configuration files and their problems (m4 §4.2–§4.5); [`persist`], the I/O worker, atomic
//! writes, the session, its lock and the hot-exit backups (m4 §4.7–§4.8).

pub mod config;
pub mod i18n;
pub mod paths;
pub mod persist;
