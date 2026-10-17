//! Project model and tree, settings/keybindings TOML, session and hot-exit backups, file
//! watching, Fluent loading and rendering (architecture §5, §6). No UI.
//!
//! Allowed internal dependencies: `va-core`.
//!
//! M1 brings [`i18n`], the Fluent localizer (m1-core-isa-lexer.md §6.2); the project, settings,
//! session, backup and watch modules arrive in M4.
//!
//! **WP0 stub (m1 §1), owned by WP-E in wave 2.**

pub mod i18n;
