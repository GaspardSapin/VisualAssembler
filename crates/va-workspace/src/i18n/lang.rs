//! Language selection (m1-core-isa-lexer.md §6.2, architecture §5): the shipped languages and
//! the M1 precedence `--lang` > `VA_LANG` > `auto`.
//!
//! M1 `auto` reads only `LC_ALL`, `LC_MESSAGES` and `LANG`. The Windows UI language and the
//! `ui.language` setting arrive in M4.

/// The environment variable that overrides `auto` (m1 §6.2).
const VA_LANG: &str = "VA_LANG";

/// The `VA_LANG` value that asks for the `auto` resolution explicitly.
const AUTO: &str = "auto";

/// The locale variables read by `auto`, highest priority first; the first non-empty one decides.
const LOCALE_VARIABLES: [&str; 3] = ["LC_ALL", "LC_MESSAGES", "LANG"];

/// A language of the interface, the diagnostics and the instruction documentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    /// English: the reference language, complete by rule, and the fallback of every lookup.
    En,
    /// French.
    Fr,
}

impl Lang {
    /// Every shipped language, English first.
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Fr];

    /// Returns the code of the language, `"en"` or `"fr"`: the value of `--lang` and `VA_LANG`,
    /// and the argument of `Architecture::locale_resources`.
    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fr => "fr",
        }
    }

    /// Parses a language code: exactly `"en"` or `"fr"`, lowercase, without region or
    /// whitespace. Anything else, `"auto"` included, gives `None`.
    pub fn parse(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|lang| lang.code() == code)
    }
}

/// Where the language of a [`LangChoice`] comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LangSource {
    /// The `--lang` flag.
    Flag,
    /// `VA_LANG` set to `en` or `fr`.
    VaLang,
    /// `auto`: the first non-empty of `LC_ALL`, `LC_MESSAGES`, `LANG` names a shipped language
    /// (its value starts with `fr` or `en`, e.g. `fr_FR.UTF-8`).
    Environment,
    /// `auto` found no usable locale: no locale variable is set and non-empty, or the first
    /// non-empty one names another language (e.g. `de_DE.UTF-8`, `C`, `POSIX`), so English
    /// applies.
    Default,
}

/// The language to use, why, and the `VA_LANG` value that was rejected on the way, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LangChoice {
    /// The language to use.
    pub lang: Lang,
    /// Where it comes from.
    pub source: LangSource,
    /// The value of `VA_LANG` when it was read and is not `en`, `fr` or `auto` (an empty value
    /// included). The CLI warns about it (m1 §7.1); the choice then continues with `auto`.
    pub ignored_va_lang: Option<String>,
}

/// Chooses the language: `flag` (`--lang`), then `VA_LANG` (`en`, `fr` or `auto`; another value
/// is ignored and returned in [`LangChoice::ignored_va_lang`]), then `auto` (m1 §6.2).
///
/// `auto` takes the first non-empty of `LC_ALL`, `LC_MESSAGES`, `LANG`: a value starting with
/// `fr` gives French, any other value or none gives English (see [`LangSource`]).
///
/// `var` reads one environment variable (`None` when unset); a test passes a fake. Variables
/// are read lazily, in precedence order: with a flag, nothing is read, so a bad `VA_LANG` is not
/// reported either.
pub fn resolve_lang(flag: Option<Lang>, var: &dyn Fn(&str) -> Option<String>) -> LangChoice {
    if let Some(lang) = flag {
        return LangChoice {
            lang,
            source: LangSource::Flag,
            ignored_va_lang: None,
        };
    }
    let mut ignored_va_lang = None;
    if let Some(value) = var(VA_LANG) {
        if let Some(lang) = Lang::parse(&value) {
            return LangChoice {
                lang,
                source: LangSource::VaLang,
                ignored_va_lang: None,
            };
        }
        if value != AUTO {
            ignored_va_lang = Some(value);
        }
    }
    let (lang, source) = auto_lang(var);
    LangChoice {
        lang,
        source,
        ignored_va_lang,
    }
}

/// The M1 `auto` resolution from the locale variables.
fn auto_lang(var: &dyn Fn(&str) -> Option<String>) -> (Lang, LangSource) {
    let locale = LOCALE_VARIABLES
        .into_iter()
        .find_map(|name| var(name).filter(|value| !value.is_empty()));
    match locale {
        Some(value) if value.starts_with(Lang::Fr.code()) => (Lang::Fr, LangSource::Environment),
        Some(value) if value.starts_with(Lang::En.code()) => (Lang::En, LangSource::Environment),
        _ => (Lang::En, LangSource::Default),
    }
}
