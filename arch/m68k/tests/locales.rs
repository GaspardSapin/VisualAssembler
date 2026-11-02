//! Tests for `src/locales.rs` and `locales/{en,fr}/{isa,diagnostics}.ftl` (m1-core-isa-lexer.md
//! §6.1, §6.3, exit criterion E5; m2-assembler.md §7, §15.2 W1): both languages parse, document
//! every id the 68000 can reference (read from the real tables through `M68K`) with the required
//! attributes, have the same id and attribute sets, hold no unreferenced id, format without
//! resolver error with the arguments of each diagnostic (built by hand as `DiagArg`s, as the
//! localizer receives them), and keep the style rules of §6.1.
#![allow(clippy::unwrap_used)]

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use va_arch_m68k::M68K;
use va_arch_m68k::asm::codes::*;
use va_arch_m68k::syntax::{
    LEX_BAD_BACKSLASH, LEX_BAD_LABEL, LEX_BAD_NUMBER, LEX_BAD_OPERATION, LEX_BAD_SIZE_SUFFIX,
    LEX_LABEL_JUNK, LEX_LINE_TOO_LONG, LEX_NON_ASCII, LEX_OPERANDS_IN_COMMENT, LEX_UNEXPECTED_CHAR,
    LEX_UNTERMINATED_STRING,
};
use va_core::arch::Architecture;
use va_core::asm::PathRefusal;
use va_core::diag::{DiagArg, DiagCode};
use va_core::isa::FlagEffect;

/// The languages the 68000 documents.
const LANGS: [&str; 2] = ["en", "fr"];

/// The resources of one language, in the order `locale_resources` returns them.
const RESOURCE_NAMES: [&str; 2] = ["isa.ftl", "diagnostics.ftl"];

/// The values of `$cause` for the operands-in-comment messages (m1 §5.7).
const CAUSES: [&str; 3] = ["after-comma", "inside-expression", "operand-list"];

/// One Fluent variable of a diagnostic message and the values it is formatted with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Var {
    /// A printed text (`DiagArg::Str`): its sample must appear in some rendering of the message.
    Text(&'static str, &'static str),
    /// A text that is passed but need not be printed (the lexer passes `$text` to every message).
    Quiet(&'static str, &'static str),
    /// A count (`DiagArg::Int`), printed or used for plural selection: every value is tried.
    Int(&'static str, &'static [i64]),
    /// A selector (`DiagArg::Str`): every value is tried, and each gives a different text.
    Select(&'static str, &'static [&'static str]),
}

impl Var {
    /// The Fluent variable name.
    fn name(self) -> &'static str {
        match self {
            Var::Text(name, _) | Var::Quiet(name, _) | Var::Int(name, _) | Var::Select(name, _) => {
                name
            }
        }
    }

    /// Every value the variable is formatted with, first value first.
    fn values(self) -> Vec<DiagArg> {
        match self {
            Var::Text(_, sample) | Var::Quiet(_, sample) => vec![DiagArg::Str(sample.to_owned())],
            Var::Int(_, values) => values.iter().map(|value| DiagArg::Int(*value)).collect(),
            Var::Select(_, values) => values
                .iter()
                .map(|value| DiagArg::Str((*value).to_owned()))
                .collect(),
        }
    }
}

/// The arguments and attributes of one diagnostic message (m1 §5.4, §5.7; m2 §7.2 and the
/// documentation of each constant in `src/asm/codes.rs`).
#[derive(Debug, PartialEq, Eq)]
struct Signature {
    /// The code; its message is `diag-<code>`.
    code: DiagCode,
    /// The variables the producer passes, each exactly once.
    vars: &'static [Var],
    /// The attributes the message has, exactly, in both languages.
    attributes: &'static [&'static str],
}

/// Shorthand for a [`Signature`] row.
const fn sig(
    code: DiagCode,
    vars: &'static [Var],
    attributes: &'static [&'static str],
) -> Signature {
    Signature {
        code,
        vars,
        attributes,
    }
}

const TEXT: Var = Var::Text("text", "t3xt");
const NAME: Var = Var::Text("name", "n4me");
const MNEMONIC: Var = Var::Text("mnemonic", "MN3M");
const PATH: Var = Var::Text("path", "d1r/f1le.i");
const LIMIT: Var = Var::Int("limit", &[16, 1_000_000]);
const SUGGESTED: [Var; 2] = [
    Var::Int("count", &[0, 1, 3]),
    Var::Text("suggestions", "s5gg, s6gg"),
];
const HELP: &[&str] = &["help"];
const NOTE: &[&str] = &["note"];
const NONE: &[&str] = &[];

/// Every diagnostic code of the 68000 with its arguments, in `diagnostic_codes()` order: the 11
/// lexer codes, then `ASM_DIAG_CODES` (§7.2 order, the five related notes last).
static SIGNATURES: [Signature; 61] = [
    sig(LEX_BAD_LABEL, &[TEXT], HELP),
    sig(LEX_LABEL_JUNK, &[TEXT], HELP),
    sig(LEX_BAD_OPERATION, &[TEXT], HELP),
    sig(LEX_BAD_SIZE_SUFFIX, &[TEXT], HELP),
    sig(LEX_BAD_NUMBER, &[TEXT], HELP),
    sig(LEX_UNTERMINATED_STRING, &[TEXT], HELP),
    sig(LEX_BAD_BACKSLASH, &[TEXT], HELP),
    sig(LEX_UNEXPECTED_CHAR, &[TEXT], HELP),
    sig(LEX_NON_ASCII, &[TEXT], HELP),
    sig(LEX_LINE_TOO_LONG, &[Var::Quiet("text", "t3xt")], HELP),
    sig(
        LEX_OPERANDS_IN_COMMENT,
        &[TEXT, Var::Select("cause", &CAUSES)],
        HELP,
    ),
    sig(
        ASM_UNKNOWN_OPERATION,
        &[NAME, SUGGESTED[0], SUGGESTED[1]],
        NOTE,
    ),
    sig(
        ASM_NOT_A68K_DIRECTIVE,
        &[Var::Select("name", &["ELSE", "REPT", "OPT"])],
        HELP,
    ),
    sig(
        ASM_UNSUPPORTED_DIRECTIVE,
        &[Var::Select("name", &["EQUR", "REG", "RORG", "NEAR", "FAR"])],
        HELP,
    ),
    sig(ASM_NOT_IN_S_FORMAT, &[NAME], HELP),
    sig(ASM_BAD_OPERAND, &[TEXT], HELP),
    sig(ASM_BAD_EXPRESSION, &[TEXT], HELP),
    sig(
        ASM_OPERAND_COUNT,
        &[
            MNEMONIC,
            Var::Int("expected", &[0, 1, 2]),
            Var::Int("found", &[1, 3]),
        ],
        HELP,
    ),
    sig(
        ASM_MODE_NOT_ALLOWED,
        &[
            MNEMONIC,
            Var::Text("mode", "m0de"),
            Var::Text("allowed", "m1, m2"),
        ],
        NOTE,
    ),
    sig(
        ASM_SIZE_NOT_ALLOWED,
        &[
            MNEMONIC,
            Var::Text("size", "Sz"),
            Var::Text("allowed", ".S1, .S2"),
        ],
        NOTE,
    ),
    sig(
        ASM_NO_MATCHING_FORM,
        &[MNEMONIC, Var::Text("forms", "f1,f2; f3")],
        NOTE,
    ),
    sig(ASM_REGISTER_SIZE_SUFFIX, &[TEXT], HELP),
    sig(ASM_MISSING_DISPLACEMENT, &[], HELP),
    sig(ASM_ABSOLUTE_SIZE_SYNTAX, &[TEXT], HELP),
    sig(
        ASM_VALUE_OUT_OF_RANGE,
        &[
            Var::Text("value", "v4l"),
            Var::Text("min", "m1n"),
            Var::Text("max", "m4x"),
        ],
        HELP,
    ),
    sig(
        ASM_BRANCH_OUT_OF_RANGE,
        &[
            Var::Int("distance", &[130, 40_000]),
            Var::Text("size", "Sz"),
        ],
        HELP,
    ),
    sig(ASM_SHORT_BRANCH_ZERO, &[], HELP),
    sig(
        ASM_ODD_BRANCH_TARGET,
        &[Var::Text("target", "$t4rget")],
        HELP,
    ),
    sig(
        ASM_UNDEFINED_SYMBOL,
        &[NAME, SUGGESTED[0], SUGGESTED[1]],
        &["help", "note"],
    ),
    sig(ASM_UNDEFINED_LOCAL_LABEL, &[NAME], NOTE),
    sig(ASM_DUPLICATE_SYMBOL, &[NAME], HELP),
    sig(ASM_RESERVED_NAME, &[NAME], HELP),
    sig(ASM_LABEL_LIKE_KEYWORD, &[NAME], HELP),
    sig(ASM_LABEL_REQUIRED, &[NAME], HELP),
    sig(
        ASM_VALUE_NOT_KNOWN,
        &[NAME, Var::Text("symbol", "sym8")],
        HELP,
    ),
    sig(ASM_DIVISION_BY_ZERO, &[], HELP),
    sig(ASM_NEGATIVE_COUNT, &[NAME, Var::Int("count", &[-3])], NONE),
    sig(ASM_ADDRESS_OVERFLOW, &[Var::Text("address", "$4ddr")], HELP),
    sig(ASM_OVERLAP, &[Var::Text("address", "$4ddr")], HELP),
    sig(ASM_STRING_IN_EXPRESSION, &[TEXT], HELP),
    sig(
        ASM_STRING_TOO_LONG,
        &[
            Var::Text("size", "Sz"),
            Var::Int("max", &[2, 4]),
            Var::Int("length", &[5]),
        ],
        HELP,
    ),
    sig(ASM_TEXT_AFTER_END, &[], HELP),
    sig(
        ASM_UNBALANCED_CONDITIONAL,
        &[NAME, Var::Select("kind", &["close", "open"])],
        HELP,
    ),
    sig(
        ASM_UNBALANCED_MACRO,
        &[NAME, Var::Select("kind", &["close", "open"])],
        HELP,
    ),
    sig(ASM_MACRO_PARAM_OUTSIDE, &[TEXT], HELP),
    sig(ASM_MACRO_TOO_DEEP, &[LIMIT], HELP),
    sig(ASM_EXPANSION_LIMIT, &[LIMIT], HELP),
    sig(ASM_INCLUDE_TOO_DEEP, &[LIMIT], HELP),
    sig(ASM_INCLUDE_CYCLE, &[PATH], HELP),
    sig(ASM_INCLUDE_NOT_FOUND, &[PATH], NOTE),
    sig(
        ASM_INCLUDE_REFUSED,
        &[
            PATH,
            Var::Select(
                "reason",
                &["absolute", "drive", "colon", "device", "escapes", "empty"],
            ),
        ],
        HELP,
    ),
    sig(
        ASM_INCLUDE_FAILED,
        &[
            PATH,
            Var::Select(
                "reason",
                &[
                    "too-large",
                    "not-utf8",
                    "too-many-files",
                    "unsupported",
                    "unreadable",
                ],
            ),
        ],
        HELP,
    ),
    sig(ASM_TOO_MANY_DIAGNOSTICS, &[LIMIT], HELP),
    sig(
        ASM_OPERANDS_IN_COMMENT,
        &[
            TEXT,
            Var::Select("kind", &["data", "instruction"]),
            Var::Int("bytes", &[1, 3]),
            Var::Text("kept", "k3pt"),
            Var::Select("cause", &CAUSES),
        ],
        HELP,
    ),
    sig(
        ASM_MOVEM_SINGLE,
        &[
            Var::Text("size", "Sz"),
            Var::Select("effect", &["sign", "flags"]),
        ],
        HELP,
    ),
    sig(ASM_PHASE_ERROR, &[NAME], HELP),
    sig(ASM_RELATED_FIRST_DEFINITION, &[NAME], NONE),
    sig(ASM_RELATED_SCOPE_START, &[], NONE),
    sig(ASM_RELATED_INCLUDED_FROM, &[], NONE),
    sig(ASM_RELATED_IN_MACRO, &[NAME], NONE),
    sig(ASM_RELATED_OTHER_BYTES, &[], NONE),
];

/// Returns the signature of `code`, which every diagnostic code must have.
fn signature(code: DiagCode) -> &'static Signature {
    SIGNATURES
        .iter()
        .find(|signature| signature.code == code)
        .unwrap_or_else(|| panic!("{code:?} has no row in SIGNATURES"))
}

/// What the 68000 requires of one referenced message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expect {
    /// An instruction entry: `.details`, and `.flags` iff one of its forms has a
    /// `FlagEffect::Special` flag.
    Entry {
        /// Whether `.flags` is required (and allowed).
        flags: bool,
    },
    /// A condition, outcome or unit phrase: a value and no attribute.
    Phrase,
    /// A diagnostic message: its arguments and exactly its attributes.
    Diagnostic(&'static Signature),
}

/// One language, parsed: its bundle and the attribute names of each message.
struct Catalog {
    /// Language code, as passed to `locale_resources`.
    lang: &'static str,
    /// The messages of every resource of the language.
    bundle: FluentBundle<FluentResource>,
    /// Message id (from the column-0 scan) → attribute names.
    messages: BTreeMap<String, BTreeSet<String>>,
}

/// Returns the 1-based line of byte `offset` in `source`.
fn line_of(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())].matches('\n').count() + 1
}

/// Lists the message ids of `source` from its column-0 `identifier =` lines (m1 §6.3 step 2),
/// after checking that every other column-0 line is blank or a comment.
fn column0_ids(name: &str, source: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for (index, line) in source.lines().enumerate() {
        if line.is_empty() || line.starts_with(' ') || line.starts_with('#') {
            continue;
        }
        let end = line
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(line.len());
        let (id, rest) = line.split_at(end);
        assert!(
            id.starts_with(|c: char| c.is_ascii_alphabetic()) && rest.trim_start().starts_with('='),
            "{name}:{}: a column-0 line must be `identifier = …`, a comment or blank: {line:?}",
            index + 1
        );
        ids.push(id.to_owned());
    }
    ids
}

/// Parses every resource of `lang` (m1 §6.3 steps 1 and 2) into one bundle.
fn load(lang: &'static str) -> Catalog {
    let sources = M68K.locale_resources(lang);
    assert_eq!(sources.len(), RESOURCE_NAMES.len(), "resources of {lang}");
    let mut bundle = FluentBundle::new(vec![lang.parse().unwrap()]);
    bundle.set_use_isolating(false);
    let mut scanned = Vec::new();
    for (source, name) in sources.iter().zip(RESOURCE_NAMES) {
        let name = format!("{lang}/{name}");
        let resource = match FluentResource::try_new((*source).to_owned()) {
            Ok(resource) => resource,
            Err((_, errors)) => {
                let lines: Vec<String> = errors
                    .iter()
                    .map(|error| {
                        format!(
                            "{name}:{}: {:?}",
                            line_of(source, error.pos.start),
                            error.kind
                        )
                    })
                    .collect();
                panic!("Fluent syntax errors:\n{}", lines.join("\n"));
            }
        };
        if let Err(errors) = bundle.add_resource(resource) {
            panic!("{name}: cannot be added to the bundle: {errors:?}");
        }
        scanned.extend(column0_ids(&name, source));
    }
    let mut messages = BTreeMap::new();
    for id in scanned {
        let message = bundle
            .get_message(&id)
            .unwrap_or_else(|| panic!("{lang}: column-0 id {id} is not a message of the bundle"));
        assert!(message.value().is_some(), "{lang}: {id} has no value");
        let attributes = message
            .attributes()
            .map(|attribute| attribute.id().to_owned())
            .collect();
        assert!(
            messages.insert(id.clone(), attributes).is_none(),
            "{lang}: {id} is defined twice"
        );
    }
    Catalog {
        lang,
        bundle,
        messages,
    }
}

/// Records that `id` is referenced as `expect`; the conditional spellings of one family share
/// their entry, whose `.flags` requirement is the union over them.
fn reference(ids: &mut BTreeMap<String, Expect>, id: String, expect: Expect) {
    match ids.entry(id) {
        Entry::Vacant(slot) => {
            slot.insert(expect);
        }
        Entry::Occupied(mut slot) => match (*slot.get(), expect) {
            (Expect::Entry { flags: a }, Expect::Entry { flags: b }) => {
                slot.insert(Expect::Entry { flags: a || b });
            }
            (old, new) => assert_eq!(old, new, "{} is referenced in two ways", slot.key()),
        },
    }
}

/// Collects every id the 68000 can reference (m1 §6.3 step 3): for each spelling,
/// `describe(spelling, None)` gives the entry, condition, outcome and unit ids, and
/// `diagnostic_codes()` gives `diag-<code>`.
fn referenced_ids() -> BTreeMap<String, Expect> {
    let arch: &dyn Architecture = &M68K;
    let isa = arch.instruction_set();
    let mut ids = BTreeMap::new();
    for spelling in isa.spellings() {
        let info = isa
            .describe(spelling, None)
            .unwrap_or_else(|error| panic!("describe({spelling}, None) failed: {error:?}"));
        let special = info.forms.iter().any(|form| {
            form.flags
                .iter()
                .any(|flag| flag.effect == FlagEffect::Special)
        });
        reference(
            &mut ids,
            info.doc_id.to_owned(),
            Expect::Entry { flags: special },
        );
        if let Some(condition) = info.condition {
            reference(&mut ids, condition.doc_id.to_owned(), Expect::Phrase);
        }
        for row in info.forms.iter().flat_map(|form| &form.timing) {
            if let Some(outcome) = row.outcome {
                let id = format!("isa-{}-outcome-{outcome}", arch.id());
                reference(&mut ids, id, Expect::Phrase);
            }
            if let Some(per_unit) = row.per_unit {
                let id = format!("isa-{}-unit-{}", arch.id(), per_unit.unit);
                reference(&mut ids, id, Expect::Phrase);
            }
        }
    }
    for code in arch.diagnostic_codes() {
        reference(
            &mut ids,
            code.fluent_id(),
            Expect::Diagnostic(signature(*code)),
        );
    }
    ids
}

/// Returns why `attributes` do not meet `expect`, if they do not.
fn attribute_problem(expect: Expect, attributes: &BTreeSet<String>) -> Option<String> {
    let names: BTreeSet<&str> = attributes.iter().map(String::as_str).collect();
    let wanted: BTreeSet<&str> = match expect {
        Expect::Entry { flags } => {
            let mut wanted = BTreeSet::from(["details"]);
            if flags {
                wanted.insert("flags");
            }
            wanted
        }
        Expect::Phrase => BTreeSet::new(),
        Expect::Diagnostic(signature) => signature.attributes.iter().copied().collect(),
    };
    (names != wanted).then(|| format!("attributes {names:?}, expected exactly {wanted:?}"))
}

/// Every combination of the values of `vars` (the cartesian product, first values first), as
/// the `DiagArg` lists a producer would pass.
fn combinations(vars: &[Var]) -> Vec<Vec<(&'static str, DiagArg)>> {
    let mut combinations = vec![Vec::new()];
    for var in vars {
        combinations = combinations
            .into_iter()
            .flat_map(|prefix| {
                var.values().into_iter().map(move |value| {
                    let mut args = prefix.clone();
                    args.push((var.name(), value));
                    args
                })
            })
            .collect();
    }
    combinations
}

/// Converts hand-built arguments to Fluent arguments the way the `va-workspace` localizer does:
/// `Int` becomes a Fluent number (so plural rules apply) and `Str` a Fluent string.
fn fluent_args(args: &[(&'static str, DiagArg)]) -> FluentArgs<'static> {
    let mut fluent = FluentArgs::new();
    for (name, value) in args {
        let value = match value {
            DiagArg::Int(number) => FluentValue::from(*number),
            DiagArg::Str(text) => FluentValue::from(text.clone()),
        };
        fluent.set(*name, value);
    }
    fluent
}

/// The argument sets each message is formatted with (m1 §6.3 step 5): none for documentation,
/// every combination of the sample values of its variables for a diagnostic.
fn sample_args(expect: Expect) -> Vec<Option<FluentArgs<'static>>> {
    match expect {
        Expect::Entry { .. } | Expect::Phrase => vec![None],
        Expect::Diagnostic(signature) => combinations(signature.vars)
            .iter()
            .map(|args| Some(fluent_args(args)))
            .collect(),
    }
}

/// Formats the value (attribute `None`) or an attribute of message `id`, asserting that it
/// exists and resolves without error.
fn format(
    catalog: &Catalog,
    id: &str,
    attribute: Option<&str>,
    args: Option<&FluentArgs<'_>>,
) -> String {
    let lang = catalog.lang;
    let message = catalog
        .bundle
        .get_message(id)
        .unwrap_or_else(|| panic!("{lang}: {id} is missing"));
    let pattern = match attribute {
        None => message.value().unwrap(),
        Some(name) => message
            .get_attribute(name)
            .unwrap_or_else(|| panic!("{lang}: {id} has no .{name}"))
            .value(),
    };
    let mut errors = Vec::new();
    let text = catalog
        .bundle
        .format_pattern(pattern, args, &mut errors)
        .into_owned();
    assert!(
        errors.is_empty(),
        "{lang}: {id} {attribute:?}: resolver errors {errors:?}"
    );
    assert!(
        !text.trim().is_empty(),
        "{lang}: {id} {attribute:?} is empty"
    );
    text
}

/// Renders a diagnostic message whole: its value, then each attribute in name order, one per
/// line, formatted with `args`.
fn render(catalog: &Catalog, signature: &Signature, args: &[(&'static str, DiagArg)]) -> String {
    let id = signature.code.fluent_id();
    let args = fluent_args(args);
    let mut text = format(catalog, &id, None, Some(&args));
    let mut attributes: Vec<&str> = signature.attributes.to_vec();
    attributes.sort_unstable();
    for attribute in attributes {
        text.push('\n');
        text.push_str(&format(catalog, &id, Some(attribute), Some(&args)));
    }
    text
}

/// The arguments of `signature` with every variable at its first value, except `var` at
/// `value`.
fn with_value(signature: &Signature, var: Var, value: &DiagArg) -> Vec<(&'static str, DiagArg)> {
    signature
        .vars
        .iter()
        .map(|other| {
            let chosen = if *other == var {
                value.clone()
            } else {
                other.values().swap_remove(0)
            };
            (other.name(), chosen)
        })
        .collect()
}

/// Counts the sentences of `text`: a `.`, `!` or `?` ending the text, or followed by
/// whitespace and a character that is not a lowercase letter.
fn sentences(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut count = 0;
    for (index, c) in chars.iter().enumerate() {
        if !matches!(c, '.' | '!' | '?') {
            continue;
        }
        let rest = &chars[index + 1..];
        let next = rest.iter().find(|c| !c.is_whitespace());
        let ends = match rest.first() {
            None => true,
            Some(after) => after.is_whitespace() && next.is_none_or(|c| !c.is_lowercase()),
        };
        if ends {
            count += 1;
        }
    }
    count
}

/// Returns the breaches of the §6.1 style rules in `source`: tabs or CR, braces outside
/// placeables, continuation lines starting with `.`, `*` or `[` that are not an attribute or a
/// variant key, and mixed ASCII and typographic apostrophes. Variant keys may be ASCII letters
/// of either case, digits and `-` (`[ELSE]` selects on a directive name); select expressions
/// may nest.
fn style_problems(name: &str, source: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if source.contains('\t') || source.contains('\r') {
        problems.push(format!("{name}: tabs or carriage returns"));
    }
    if source.contains('\'') && source.contains('\u{2019}') {
        problems.push(format!("{name}: mixes ASCII and typographic apostrophes"));
    }
    let mut depth = 0usize;
    for (index, line) in source.lines().enumerate() {
        let at = format!("{name}:{}", index + 1);
        for (offset, _) in line.match_indices('{') {
            let inside = line[offset + 1..].trim_start_matches(' ');
            if !(inside.starts_with('$') || inside.starts_with('"')) {
                problems.push(format!(
                    "{at}: brace outside a variable or literal placeable"
                ));
            }
        }
        let text = line.trim_start_matches(' ');
        let continuation = text.len() != line.len();
        if continuation && text.starts_with('.') {
            let attribute = text[1..].split(" = ").next().unwrap_or_default();
            let valid = text.contains(" = ")
                && attribute.starts_with(|c: char| c.is_ascii_lowercase())
                && attribute
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '-');
            if !valid {
                problems.push(format!(
                    "{at}: continuation line starts with a dot: {text:?}"
                ));
            }
        } else if continuation && (text.starts_with('[') || text.starts_with('*')) {
            let key = text.strip_prefix('*').unwrap_or(text);
            let valid = depth > 0
                && key.starts_with('[')
                && key.find("] ").is_some_and(|close| {
                    key[1..close]
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-')
                });
            if !valid {
                problems.push(format!(
                    "{at}: continuation line starts with * or [: {text:?}"
                ));
            }
        }
        if text.starts_with('}') {
            depth = depth.saturating_sub(1);
        }
        if text.trim_end().ends_with("->") {
            depth += 1;
        }
    }
    problems
}

#[test]
fn every_resource_parses_and_lists_its_ids() {
    for lang in LANGS {
        let catalog = load(lang);
        assert!(!catalog.messages.is_empty(), "{lang}: no message");
    }
}

#[test]
fn only_en_and_fr_have_resources() {
    for lang in LANGS {
        assert_eq!(M68K.locale_resources(lang).len(), 2, "{lang}");
    }
    for lang in ["", "de", "en-US", "english"] {
        assert!(M68K.locale_resources(lang).is_empty(), "{lang:?}");
    }
}

#[test]
fn referenced_ids_have_the_expected_counts() {
    let ids = referenced_ids();
    let count = |prefix: &str| ids.keys().filter(|id| id.starts_with(prefix)).count();
    let entries = ids
        .values()
        .filter(|expect| matches!(expect, Expect::Entry { .. }))
        .count();
    assert_eq!(entries, 74, "documentation entries (m1 §4.1)");
    assert_eq!(count("isa-m68k-cond-"), 16, "conditions (m1 §4.1)");
    assert_eq!(count("diag-lex-"), 11, "lexer diagnostics (m1 §5.4)");
    assert_eq!(count("diag-asm-"), 50, "assembler diagnostics (m2 §7.2)");
    assert_eq!(count("diag-asm-related-"), 5, "related notes (m2 §7.2)");
    assert!(count("isa-m68k-outcome-") > 0, "outcomes");
    assert!(count("isa-m68k-unit-") > 0, "units");
}

#[test]
fn signatures_follow_the_diagnostic_codes() {
    let codes: Vec<DiagCode> = SIGNATURES.iter().map(|signature| signature.code).collect();
    assert_eq!(codes, M68K.diagnostic_codes());
    for signature in &SIGNATURES {
        let code = signature.code.0;
        let names: BTreeSet<&str> = signature.vars.iter().map(|var| var.name()).collect();
        assert_eq!(
            names.len(),
            signature.vars.len(),
            "{code}: a variable twice"
        );
        assert!(
            signature
                .attributes
                .iter()
                .all(|attribute| ["help", "note"].contains(attribute)),
            "{code}: only .help and .note"
        );
        if code.starts_with("asm-related-") {
            assert!(
                signature.attributes.is_empty(),
                "{code}: a note has no attribute"
            );
        }
        if code.starts_with("lex-") {
            assert!(names.contains("text"), "{code}: the lexer passes $text");
            assert!(
                names.is_subset(&BTreeSet::from(["text", "cause"])),
                "{code}: the lexer passes only $text and $cause"
            );
        }
    }
}

#[test]
fn every_referenced_id_is_documented_with_its_attributes() {
    let referenced = referenced_ids();
    let mut problems = Vec::new();
    for lang in LANGS {
        let catalog = load(lang);
        for (id, expect) in &referenced {
            match catalog.messages.get(id) {
                None => problems.push(format!("{lang}: {id} is missing")),
                Some(attributes) => {
                    if let Some(problem) = attribute_problem(*expect, attributes) {
                        problems.push(format!("{lang}: {id}: {problem}"));
                    }
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn english_and_french_have_the_same_ids_and_attributes() {
    let en = load("en");
    let fr = load("fr");
    let mut problems = Vec::new();
    for id in en
        .messages
        .keys()
        .filter(|id| !fr.messages.contains_key(*id))
    {
        problems.push(format!("{id} is in en but not in fr"));
    }
    for id in fr
        .messages
        .keys()
        .filter(|id| !en.messages.contains_key(*id))
    {
        problems.push(format!("{id} is in fr but not in en"));
    }
    for (id, attributes) in &en.messages {
        if let Some(other) = fr.messages.get(id).filter(|other| *other != attributes) {
            problems.push(format!(
                "{id}: attributes {attributes:?} in en, {other:?} in fr"
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn no_id_is_unreferenced() {
    let referenced = referenced_ids();
    for lang in LANGS {
        let catalog = load(lang);
        let orphans: Vec<&String> = catalog
            .messages
            .keys()
            .filter(|id| !referenced.contains_key(*id))
            .collect();
        assert!(orphans.is_empty(), "{lang}: unreferenced ids {orphans:?}");
    }
}

#[test]
fn every_message_formats_without_resolver_error() {
    let referenced = referenced_ids();
    for lang in LANGS {
        let catalog = load(lang);
        for (id, attributes) in &catalog.messages {
            let expect = referenced.get(id).copied().unwrap_or(Expect::Phrase);
            for args in sample_args(expect) {
                format(&catalog, id, None, args.as_ref());
                for attribute in attributes {
                    format(&catalog, id, Some(attribute), args.as_ref());
                }
            }
        }
    }
}

#[test]
fn every_printed_argument_appears_in_the_text() {
    let mut problems = Vec::new();
    for lang in LANGS {
        let catalog = load(lang);
        for signature in &SIGNATURES {
            let renderings: Vec<String> = combinations(signature.vars)
                .iter()
                .map(|args| render(&catalog, signature, args))
                .collect();
            for var in signature.vars {
                if let Var::Text(name, sample) = var
                    && !renderings.iter().any(|text| text.contains(*sample))
                {
                    problems.push(format!(
                        "{lang}: {}: ${name} never appears",
                        signature.code.0
                    ));
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn each_selector_value_gives_its_own_text() {
    let mut problems = Vec::new();
    for lang in LANGS {
        let catalog = load(lang);
        for signature in &SIGNATURES {
            for var in signature.vars {
                let Var::Select(name, values) = *var else {
                    continue;
                };
                let texts: BTreeSet<String> = var
                    .values()
                    .iter()
                    .map(|value| render(&catalog, signature, &with_value(signature, *var, value)))
                    .collect();
                if texts.len() != values.len() {
                    problems.push(format!(
                        "{lang}: {}: ${name} gives {} texts for {values:?}",
                        signature.code.0,
                        texts.len()
                    ));
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn include_refusal_reasons_are_the_path_refusal_ids() {
    let refusals = [
        PathRefusal::Empty,
        PathRefusal::Absolute,
        PathRefusal::Prefix,
        PathRefusal::Colon,
        PathRefusal::DeviceName,
        PathRefusal::Escapes,
        PathRefusal::Nul,
    ];
    let ids: BTreeSet<&str> = refusals.iter().map(|refusal| refusal.id()).collect();
    let signature = signature(ASM_INCLUDE_REFUSED);
    let reason = signature
        .vars
        .iter()
        .copied()
        .find(|var| var.name() == "reason")
        .unwrap();
    let Var::Select(_, worded) = reason else {
        panic!("reason is a selector");
    };
    let mut expected: BTreeSet<&str> = worded.iter().copied().collect();
    expected.insert("nul");
    assert_eq!(ids, expected, "every refusal id is worded");
    for lang in LANGS {
        let catalog = load(lang);
        let text = |id: &str| {
            render(
                &catalog,
                signature,
                &with_value(signature, reason, &DiagArg::Str(id.to_owned())),
            )
        };
        assert_eq!(
            text("nul"),
            text("empty"),
            "{lang}: nul shares the wording of empty"
        );
    }
}

#[test]
fn operands_in_comment_help_is_the_same_for_the_lexer_and_the_assembler() {
    for lang in LANGS {
        let catalog = load(lang);
        for cause in CAUSES {
            let mut args = FluentArgs::new();
            args.set("cause", cause);
            let lexer = format(
                &catalog,
                &LEX_OPERANDS_IN_COMMENT.fluent_id(),
                Some("help"),
                Some(&args),
            );
            let assembler = format(
                &catalog,
                &ASM_OPERANDS_IN_COMMENT.fluent_id(),
                Some("help"),
                Some(&args),
            );
            assert_eq!(lexer, assembler, "{lang}: {cause}");
        }
    }
}

#[test]
fn summaries_are_one_line_and_details_have_two_to_six_sentences() {
    let mut problems = Vec::new();
    for lang in LANGS {
        let catalog = load(lang);
        let entries = catalog
            .messages
            .iter()
            .filter(|(_, attributes)| attributes.contains("details"));
        for (id, _) in entries {
            let summary = format(&catalog, id, None, None);
            if summary.contains('\n') {
                problems.push(format!("{lang}: {id}: the summary spans several lines"));
            }
            let details = format(&catalog, id, Some("details"), None);
            let count = sentences(&details);
            if !(2..=6).contains(&count) {
                problems.push(format!("{lang}: {id}: .details has {count} sentences"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn diagnostic_messages_are_one_line() {
    let mut problems = Vec::new();
    for lang in LANGS {
        let catalog = load(lang);
        for signature in &SIGNATURES {
            for args in combinations(signature.vars) {
                let text = render(&catalog, signature, &args);
                if text.lines().count() != signature.attributes.len() + 1 {
                    problems.push(format!("{lang}: {}: {text:?}", signature.code.0));
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn french_is_not_a_copy_of_english() {
    let referenced = referenced_ids();
    let en = load("en");
    let fr = load("fr");
    for id in en
        .messages
        .keys()
        .filter(|id| fr.messages.contains_key(*id))
    {
        let expect = referenced.get(id).copied().unwrap_or(Expect::Phrase);
        let args = sample_args(expect).swap_remove(0);
        let english = format(&en, id, None, args.as_ref());
        let french = format(&fr, id, None, args.as_ref());
        assert_ne!(english, french, "{id} has the same text in en and fr");
    }
}

#[test]
fn files_follow_the_style_rules() {
    let mut problems = Vec::new();
    for lang in LANGS {
        for (source, name) in M68K.locale_resources(lang).iter().zip(RESOURCE_NAMES) {
            problems.extend(style_problems(&format!("{lang}/{name}"), source));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn style_check_catches_each_rule() {
    let bad = "a = x { b }\n    .details = y\n        .W z\n        [w] v\n    }\n";
    let problems = style_problems("t", bad);
    assert_eq!(problems.len(), 3, "{problems:?}");
    assert_eq!(style_problems("t", "a = it's it\u{2019}s\n").len(), 1);
    let good = "a = { $c ->\n        [one] x\n       *[other] y\n    }\n    .help = z\n";
    assert!(style_problems("t", good).is_empty());
    let nested = "a = { $k ->\n    [ELSE] { $n ->\n        [one] x\n       *[other] y\n    } z\n   *[other] w\n}\n";
    assert!(
        style_problems("t", nested).is_empty(),
        "{:?}",
        style_problems("t", nested)
    );
    let after = "a = { $k ->\n   *[other] w\n}\n    [x] v\n";
    assert_eq!(
        style_problems("t", after).len(),
        1,
        "a key after the select closed"
    );
}

#[test]
fn sentence_count_ignores_suffixes_and_numbers() {
    assert_eq!(sentences("MOVE.W copies. It takes 4.5 clock periods."), 2);
    assert_eq!(
        sentences("Syntax: ADD.B/W/L <ea>,Dn (vector 5). N is set."),
        2
    );
    assert_eq!(sentences("one, e.g. two"), 0);
}
