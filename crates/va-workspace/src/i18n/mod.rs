//! Language selection and Fluent rendering, shared by the GUI and the CLI
//! (m1-core-isa-lexer.md §6.2, m4-editor-shell.md §4.6, architecture §5).
//!
//! - [`resolve_lang`] picks the [`Lang`] of the CLI from `--lang`, `VA_LANG` and the locale
//!   variables (the M1 rule), and reports a rejected `VA_LANG` in its [`LangChoice`].
//! - [`resolve_ui_lang`] picks the GUI's [`Lang`]: `--lang`, `VA_LANG`, the `ui.language`
//!   setting, then `auto`, which ends with [`system_ui_language`] on Windows.
//! - [`Localizer`] loads Fluent resources (usually `Architecture::locale_resources`, through
//!   [`Localizer::for_architecture`], or the GUI's [`ui_resources`] with a CPU's, through
//!   [`Localizer::for_ui`]) and formats messages and diagnostics, falling back to English.
//!
//! Libraries never produce prose (architecture §5): they emit ids and arguments, and this module
//! turns them into text.

mod lang;
mod localizer;
mod system;
mod ui;

pub use lang::{Lang, LangChoice, LangSource, resolve_lang};
pub use localizer::{Localizer, LocalizerError, RenderedDiagnostic};
pub use system::system_ui_language;
pub use ui::{resolve_ui_lang, ui_resources};
