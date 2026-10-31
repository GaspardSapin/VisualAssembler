//! UI localization (m4-editor-shell.md §4.6, architecture §5): the embedded root resources of
//! the GUI, the GUI localizer that joins them with a CPU's resources, and the GUI's language
//! precedence.
//!
//! The root resources `locales/<lang>/{ui,commands,config}.ftl` own the `ui-*`, `cmd-*` and
//! `diag-cfg-*` ids and define no Fluent term; a CPU owns `diag-<its codes>` and `isa-<arch>-*`.
//! [`Localizer::for_ui`] loads both sets into one bundle per language, where a shared id is a
//! [`LocalizerError::Duplicate`] (a build defect). User overrides in
//! `<config>/locales/<lang>/*.ftl` are deferred (not loaded in M4).

use va_core::arch::Architecture;

use super::lang::{AUTO, Lang, LangChoice, LangSource, VA_LANG, locale_lang, locale_variable};
use super::localizer::{Localizer, LocalizerError};

/// The English root resources, in load order: shell text, command titles, configuration
/// problems.
static EN: [&str; 3] = [
    include_str!("../../../../locales/en/ui.ftl"),
    include_str!("../../../../locales/en/commands.ftl"),
    include_str!("../../../../locales/en/config.ftl"),
];

/// The French root resources, in the same order as [`EN`].
static FR: [&str; 3] = [
    include_str!("../../../../locales/fr/ui.ftl"),
    include_str!("../../../../locales/fr/commands.ftl"),
    include_str!("../../../../locales/fr/config.ftl"),
];

/// Returns the root resources of `lang`, embedded with `include_str!`, in this order:
/// `ui.ftl` (`ui-*`), `commands.ftl` (`cmd-*`), `config.ftl` (`diag-cfg-*`).
pub fn ui_resources(lang: Lang) -> &'static [&'static str] {
    match lang {
        Lang::En => &EN,
        Lang::Fr => &FR,
    }
}

impl Localizer {
    /// Builds the GUI localizer for `lang`: [`ui_resources`] then, when `arch` is given,
    /// `arch.locale_resources(lang.code())` as the primary list, and the same two sets in
    /// English as the fallback (per-pattern English fallback, isolation marks off, as
    /// [`Localizer::new`]).
    ///
    /// Each list is one bundle, so a CPU id that is also a root id, or an id defined twice by
    /// the CPU, is an error. Resource indices in errors count the root resources first (0 to 2),
    /// then the CPU's.
    ///
    /// # Errors
    ///
    /// The first resource that does not parse ([`LocalizerError::Parse`]), or the first
    /// duplicated id ([`LocalizerError::Duplicate`]), the `lang` list before the English one.
    pub fn for_ui(lang: Lang, arch: Option<&dyn Architecture>) -> Result<Self, LocalizerError> {
        Self::new(lang, &ui_list(lang, arch), &ui_list(Lang::En, arch))
    }
}

/// The resources [`Localizer::for_ui`] loads for `lang`: the root ones, then the CPU's.
fn ui_list(lang: Lang, arch: Option<&dyn Architecture>) -> Vec<&'static str> {
    let mut resources = ui_resources(lang).to_vec();
    if let Some(arch) = arch {
        resources.extend_from_slice(arch.locale_resources(lang.code()));
    }
    resources
}

/// Chooses the GUI language (m4-editor-shell.md §4.6): `flag` (`--lang`), then `VA_LANG`,
/// then `setting` (`ui.language`), then `auto`.
///
/// - `VA_LANG` set to `en` or `fr` decides; set to `auto` it selects the `auto` resolution
///   directly (the setting is not used, as `VA_RENDERER=auto` skips `render.renderer`); any
///   other value (an empty one included) is ignored, returned in
///   [`LangChoice::ignored_va_lang`], and the choice continues with `setting`.
/// - `setting` is `None` when `ui.language` is `"auto"`, absent or invalid
///   (`UiSettings::language`).
/// - `auto` takes the first non-empty of `LC_ALL`, `LC_MESSAGES`, `LANG` as M1 does (`fr…`
///   French, `en…` English, any other value English with [`LangSource::Default`]); only when
///   none is set and non-empty it asks `system` (on Windows the user's UI language,
///   [`system_ui_language`](super::system_ui_language)), with the same prefix rule. With no
///   answer it gives English ([`LangSource::Default`]).
///
/// `var` reads one environment variable (`None` when unset) and `system` the OS UI language;
/// tests pass fakes. Both are called lazily, in precedence order: with a flag nothing is read,
/// so a bad `VA_LANG` is not reported either. [`resolve_lang`](super::resolve_lang) (the CLI)
/// keeps the M1 rule.
pub fn resolve_ui_lang(
    flag: Option<Lang>,
    setting: Option<Lang>,
    var: &dyn Fn(&str) -> Option<String>,
    system: &dyn Fn() -> Option<String>,
) -> LangChoice {
    if let Some(lang) = flag {
        return choice(lang, LangSource::Flag, None);
    }
    let mut ignored_va_lang = None;
    if let Some(value) = var(VA_LANG) {
        if let Some(lang) = Lang::parse(&value) {
            return choice(lang, LangSource::VaLang, None);
        }
        if value == AUTO {
            let (lang, source) = auto_ui_lang(var, system);
            return choice(lang, source, None);
        }
        ignored_va_lang = Some(value);
    }
    if let Some(lang) = setting {
        return choice(lang, LangSource::Setting, ignored_va_lang);
    }
    let (lang, source) = auto_ui_lang(var, system);
    choice(lang, source, ignored_va_lang)
}

/// The GUI `auto` resolution: the locale variables, then the system UI language.
fn auto_ui_lang(
    var: &dyn Fn(&str) -> Option<String>,
    system: &dyn Fn() -> Option<String>,
) -> (Lang, LangSource) {
    if let Some(value) = locale_variable(var) {
        return locale_lang(&value, LangSource::Environment);
    }
    match system() {
        Some(tag) => locale_lang(&tag, LangSource::System),
        None => (Lang::En, LangSource::Default),
    }
}

/// A [`LangChoice`] from its parts.
fn choice(lang: Lang, source: LangSource, ignored_va_lang: Option<String>) -> LangChoice {
    LangChoice {
        lang,
        source,
        ignored_va_lang,
    }
}
