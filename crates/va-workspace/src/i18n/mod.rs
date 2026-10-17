//! Language selection and Fluent rendering, shared by the GUI and the CLI
//! (m1-core-isa-lexer.md §6.2, architecture §5).
//!
//! **WP0 stub (m1 §1), owned by WP-E in wave 2.** Empty on purpose: WP-E adds `lang.rs`
//! (`Lang`, `LangSource`, `LangChoice`, `resolve_lang`) and `localizer.rs` (`Localizer`,
//! `LocalizerError`, `RenderedDiagnostic`) with the frozen signatures of §6.2, built on
//! `fluent-bundle` and `unic-langid`.
