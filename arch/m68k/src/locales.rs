//! The embedded Fluent resources of the 68000 (m1-core-isa-lexer.md §6.1, m2-assembler.md §7.1):
//! instruction documentation (`isa.ftl`) and the messages of every diagnostic code
//! (`diagnostics.ftl`: the lexer's `diag-lex-*`, then the assembler's `diag-asm-*`), in English
//! and French.
//!
//! The `.ftl` sources live in `arch/m68k/locales/<lang>/` and are compiled into the crate with
//! `include_str!`, so the apps need no file at run time. This module never parses or renders
//! them: `va-workspace`'s localizer does (m1 §6.2). The arguments each message takes are listed
//! on its code: `syntax::DIAG_CODES` (`$text`, and `$cause` for operands in a comment) and
//! `asm::codes` (one list per constant). `tests/locales.rs` checks that both languages document
//! every id the CPU can reference and formats each diagnostic with those arguments (m1 §6.3,
//! m2 §15.2 W1).

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
