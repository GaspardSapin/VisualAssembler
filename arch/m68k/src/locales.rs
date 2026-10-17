//! The embedded Fluent resources of the 68000 (m1-core-isa-lexer.md §6.1): instruction
//! documentation (`isa.ftl`) and lexer diagnostics (`diagnostics.ftl`), in English and French.
//!
//! The `.ftl` sources live in `arch/m68k/locales/<lang>/` and are compiled into the crate with
//! `include_str!`, so the apps need no file at run time. This module never parses or renders
//! them: `va-workspace`'s localizer does (m1 §6.2), and `tests/locales.rs` checks that both
//! languages document every id the CPU can reference (m1 §6.3).

/// The English resources, in load order: instruction docs, then diagnostics.
static EN: [&str; 2] = [
    include_str!("../locales/en/isa.ftl"),
    include_str!("../locales/en/diagnostics.ftl"),
];

/// The French resources, in the same order as [`EN`].
static FR: [&str; 2] = [
    include_str!("../locales/fr/isa.ftl"),
    include_str!("../locales/fr/diagnostics.ftl"),
];

/// Returns the resources of language `lang` (`"en"` or `"fr"`, exactly as `Lang::code` spells
/// them), or an empty slice for any other code.
pub(crate) fn resources(lang: &str) -> &'static [&'static str] {
    match lang {
        "en" => &EN,
        "fr" => &FR,
        _ => &[],
    }
}
