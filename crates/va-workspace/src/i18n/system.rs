//! The operating system's UI language, the last step of the GUI's `auto` (m4-editor-shell.md
//! §4.6, architecture §5): on Windows through `sys-locale`; elsewhere `auto` stops at the locale
//! variables, so there is none.

/// Returns the user's UI language as a BCP 47 tag (e.g. `"fr-FR"`): on Windows the first entry
/// of `sys_locale::get_locales()` (the most preferred one), `None` when the list is empty or
/// that entry is empty; on every other system `None`.
///
/// The result is meant for the `system` argument of
/// [`resolve_ui_lang`](super::resolve_ui_lang); the CLI never calls it (m4 §13).
#[cfg(windows)]
pub fn system_ui_language() -> Option<String> {
    sys_locale::get_locales()
        .next()
        .filter(|tag| !tag.is_empty())
}

/// Returns the user's UI language: `None` outside Windows, where the GUI's `auto` reads only
/// the locale variables (m4-editor-shell.md §4.6).
#[cfg(not(windows))]
pub fn system_ui_language() -> Option<String> {
    None
}
