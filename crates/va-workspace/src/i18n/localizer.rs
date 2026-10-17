//! The Fluent localizer (m1-core-isa-lexer.md §6.2, architecture §5): the single renderer of
//! diagnostics and documentation, shared by the GUI and the CLI.
//!
//! Lookups go to the requested language, then to English. Bundles are
//! `fluent_bundle::concurrent::FluentBundle`, so a [`Localizer`] is `Send + Sync`, and Unicode
//! isolation marks (U+2068, U+2069) are disabled for terminals and egui.

use std::fmt;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentError, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;
use va_core::arch::Architecture;
use va_core::diag::{DiagArg, Diagnostic};

use super::lang::Lang;

/// One language's messages.
type Bundle = FluentBundle<FluentResource>;

/// Fluent numbers are `f64`, exact for integers up to this magnitude (2^53). Larger
/// [`DiagArg::Int`] values are passed as text so they print exactly.
const MAX_EXACT_INT: i64 = 1 << 53;

/// The attribute that explains a diagnostic's fix (architecture §5).
const HELP: &str = "help";

/// The attribute that adds background to a diagnostic (architecture §5).
const NOTE: &str = "note";

/// Formats Fluent messages in one language, with English as the fallback.
///
/// Built once from embedded resources (`include_str!`), then shared: it is `Send + Sync`.
pub struct Localizer {
    /// The requested language.
    lang: Lang,
    /// The messages of `lang`.
    primary: Bundle,
    /// The English messages; `None` when `primary` already holds exactly them (`lang` is
    /// English and both resource lists are equal).
    english: Option<Bundle>,
}

/// Why the resources of a [`Localizer`] do not load. Both cases are build defects of the
/// embedded resources (the CLI's `locale-invalid`, m1 §7.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalizerError {
    /// A resource is not valid Fluent syntax.
    Parse {
        /// The language of the resource list holding it (the requested language for `primary`,
        /// English for `english`).
        lang: Lang,
        /// Its index in that list.
        resource: usize,
        /// One description per syntax error, `line <n>: <problem>` (1-based line in the
        /// resource).
        errors: Vec<String>,
    },
    /// Two entries of one language's resources share an id (in one resource or across them).
    Duplicate {
        /// The language of the resource list.
        lang: Lang,
        /// The duplicated id (for a term, its name without the leading `-`).
        id: String,
    },
}

impl fmt::Display for LocalizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse {
                lang,
                resource,
                errors,
            } => write!(
                f,
                "Fluent resource {resource} of language {} does not parse: {}",
                lang.code(),
                errors.join("; ")
            ),
            Self::Duplicate { lang, id } => write!(
                f,
                "Fluent resources of language {} define `{id}` more than once",
                lang.code()
            ),
        }
    }
}

impl std::error::Error for LocalizerError {}

/// The text of one diagnostic, from the Fluent message `diag-<code>` and its attributes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedDiagnostic {
    /// The message value, or the code itself when no language defines it.
    pub message: String,
    /// The `.help` attribute (how to fix the problem; explains the suggestion), if defined.
    pub help: Option<String>,
    /// The `.note` attribute (background), if defined.
    pub note: Option<String>,
}

impl Localizer {
    /// Builds a localizer for `lang` from `primary` (the resources of `lang`) and `english`
    /// (the English resources, the fallback).
    ///
    /// Each list is loaded in order into its own bundle; ids must be unique within a list, while
    /// the two lists naturally share ids. For English, pass the same list twice: it is loaded
    /// once.
    ///
    /// # Errors
    ///
    /// The first resource that does not parse ([`LocalizerError::Parse`]), or the first
    /// duplicated id ([`LocalizerError::Duplicate`]), `primary` before `english`.
    pub fn new(
        lang: Lang,
        primary: &[&'static str],
        english: &[&'static str],
    ) -> Result<Self, LocalizerError> {
        let primary_bundle = build_bundle(lang, primary)?;
        let english_bundle = if lang == Lang::En && primary == english {
            None
        } else {
            Some(build_bundle(Lang::En, english)?)
        };
        Ok(Self {
            lang,
            primary: primary_bundle,
            english: english_bundle,
        })
    }

    /// Builds a localizer for `lang` from the CPU's embedded resources:
    /// `arch.locale_resources(lang.code())`, with `arch.locale_resources("en")` as the fallback.
    ///
    /// # Errors
    ///
    /// As [`Localizer::new`].
    pub fn for_architecture(lang: Lang, arch: &dyn Architecture) -> Result<Self, LocalizerError> {
        Self::new(
            lang,
            arch.locale_resources(lang.code()),
            arch.locale_resources(Lang::En.code()),
        )
    }

    /// The requested language (lookups that miss in it fall back to English).
    pub fn lang(&self) -> Lang {
        self.lang
    }

    /// Formats the message `id`, or its attribute `attribute` (e.g. `Some("details")`), with
    /// the Fluent variables `args`.
    ///
    /// The requested language is tried first, then English, per pattern: a translated message
    /// whose attribute is missing gets the English attribute. Returns `None` when neither
    /// language has the pattern.
    ///
    /// [`DiagArg::Int`] becomes a Fluent number (plural selection uses the language's rules;
    /// beyond ±2^53 it becomes text, so it prints exactly) and [`DiagArg::Str`] a Fluent string.
    /// A resolver error (e.g. a missing variable) never fails: Fluent's best-effort text is
    /// returned, with the unresolved placeable written as `{$name}`.
    pub fn message(
        &self,
        id: &str,
        attribute: Option<&str>,
        args: &[(&str, DiagArg)],
    ) -> Option<String> {
        self.lookup(id, attribute, &fluent_args(args))
    }

    /// Renders `diagnostic` from the message [`va_core::diag::DiagCode::fluent_id`]
    /// (`diag-<code>`) and its `.help` and `.note` attributes, with the diagnostic's arguments.
    ///
    /// Never fails: a message missing from every language renders as the code itself, with no
    /// help or note. Related places are rendered by the caller with [`Localizer::message`]
    /// (`related.code.fluent_id()`, `related.args`).
    pub fn render(&self, diagnostic: &Diagnostic) -> RenderedDiagnostic {
        let id = diagnostic.code.fluent_id();
        let args = fluent_args(&diagnostic.args);
        RenderedDiagnostic {
            message: self
                .lookup(&id, None, &args)
                .unwrap_or_else(|| diagnostic.code.0.to_owned()),
            help: self.lookup(&id, Some(HELP), &args),
            note: self.lookup(&id, Some(NOTE), &args),
        }
    }

    /// Formats one pattern from the first bundle that has it.
    fn lookup(&self, id: &str, attribute: Option<&str>, args: &FluentArgs<'_>) -> Option<String> {
        std::iter::once(&self.primary)
            .chain(self.english.as_ref())
            .find_map(|bundle| format_pattern(bundle, id, attribute, args))
    }
}

impl fmt::Debug for Localizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Localizer")
            .field("lang", &self.lang)
            .field("separate_english", &self.english.is_some())
            .finish_non_exhaustive()
    }
}

/// Loads `resources` into a new bundle for `lang`, without isolation marks.
fn build_bundle(lang: Lang, resources: &[&'static str]) -> Result<Bundle, LocalizerError> {
    let mut bundle = Bundle::new_concurrent(vec![language_id(lang)]);
    bundle.set_use_isolating(false);
    for (index, &source) in resources.iter().enumerate() {
        let resource = FluentResource::try_new(source.to_owned()).map_err(|(_, errors)| {
            LocalizerError::Parse {
                lang,
                resource: index,
                errors: errors
                    .iter()
                    .map(|error| {
                        format!("line {}: {}", line_of(source, error.pos.start), error.kind)
                    })
                    .collect(),
            }
        })?;
        bundle
            .add_resource(resource)
            .map_err(|errors| add_error(lang, index, &errors))?;
    }
    Ok(bundle)
}

/// Maps the errors of `FluentBundle::add_resource` (in practice only `Overriding`) to the first
/// duplicated id; any other error is reported as a parse error of the resource.
fn add_error(lang: Lang, resource: usize, errors: &[FluentError]) -> LocalizerError {
    let duplicate = errors.iter().find_map(|error| match error {
        FluentError::Overriding { id, .. } => Some(id.clone()),
        FluentError::ParserError(_) | FluentError::ResolverError(_) => None,
    });
    match duplicate {
        Some(id) => LocalizerError::Duplicate { lang, id },
        None => LocalizerError::Parse {
            lang,
            resource,
            errors: errors.iter().map(ToString::to_string).collect(),
        },
    }
}

/// The 1-based line of byte `offset` in `source` (clamped to its end).
fn line_of(source: &str, offset: usize) -> usize {
    source
        .bytes()
        .take(offset)
        .filter(|&byte| byte == b'\n')
        .count()
        + 1
}

/// The Fluent locale of `lang`, which selects its plural rules. Both codes are valid BCP 47
/// tags; should parsing ever fail, the undetermined locale keeps formatting working.
fn language_id(lang: Lang) -> LanguageIdentifier {
    lang.code().parse().unwrap_or_default()
}

/// Converts diagnostic arguments to Fluent variables (a repeated name keeps its last value).
fn fluent_args<'a>(args: &'a [(&'a str, DiagArg)]) -> FluentArgs<'a> {
    let mut fluent = FluentArgs::with_capacity(args.len());
    for (name, value) in args {
        fluent.set(*name, fluent_value(value));
    }
    fluent
}

/// One argument as a Fluent value: a number when `f64` holds it exactly, else text.
fn fluent_value(value: &DiagArg) -> FluentValue<'_> {
    match value {
        DiagArg::Int(number) if (-MAX_EXACT_INT..=MAX_EXACT_INT).contains(number) => {
            FluentValue::from(*number)
        }
        DiagArg::Int(number) => FluentValue::from(number.to_string()),
        DiagArg::Str(text) => FluentValue::from(text.as_str()),
    }
}

/// Formats the value of message `id` (or its attribute) from `bundle`, if it has that pattern.
fn format_pattern(
    bundle: &Bundle,
    id: &str,
    attribute: Option<&str>,
    args: &FluentArgs<'_>,
) -> Option<String> {
    let message = bundle.get_message(id)?;
    let pattern = match attribute {
        None => message.value()?,
        Some(name) => message.get_attribute(name)?.value(),
    };
    let mut errors = Vec::new();
    Some(
        bundle
            .format_pattern(pattern, Some(args), &mut errors)
            .into_owned(),
    )
}
