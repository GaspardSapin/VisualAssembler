//! Language selection and Fluent rendering, shared by the GUI and the CLI
//! (m1-core-isa-lexer.md §6.2, architecture §5).
//!
//! - [`resolve_lang`] picks the [`Lang`] from `--lang`, `VA_LANG` and the locale variables, and
//!   reports a rejected `VA_LANG` in its [`LangChoice`].
//! - [`Localizer`] loads Fluent resources (usually `Architecture::locale_resources`, through
//!   [`Localizer::for_architecture`]) and formats messages and diagnostics, falling back to
//!   English.
//!
//! Libraries never produce prose (architecture §5): they emit ids and arguments, and this module
//! turns them into text.

mod lang;
mod localizer;

pub use lang::{Lang, LangChoice, LangSource, resolve_lang};
pub use localizer::{Localizer, LocalizerError, RenderedDiagnostic};
