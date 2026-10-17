//! Tests for `src/locales.rs` and `locales/{en,fr}/{isa,diagnostics}.ftl` (m1-core-isa-lexer.md
//! §6.1, §6.3, exit criterion E5): both languages parse, document every id the 68000 can
//! reference (read from the real tables through `M68K`) with the required attributes, have the
//! same id and attribute sets, hold no unreferenced id, format without resolver error, and keep
//! the style rules of §6.1.
#![allow(clippy::unwrap_used)]

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use va_arch_m68k::M68K;
use va_arch_m68k::syntax::LEX_OPERANDS_IN_COMMENT;
use va_core::arch::Architecture;
use va_core::isa::FlagEffect;

/// The languages the 68000 documents.
const LANGS: [&str; 2] = ["en", "fr"];

/// The resources of one language, in the order `locale_resources` returns them.
const RESOURCE_NAMES: [&str; 2] = ["isa.ftl", "diagnostics.ftl"];

/// The values of `$cause` for `diag-lex-operands-in-comment` (m1 §5.7).
const CAUSES: [&str; 3] = ["after-comma", "inside-expression", "operand-list"];

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
    /// A diagnostic message: `.help` and `.note` are allowed, `.help` is required for the
    /// operands-in-comment code, which alone also receives `$cause`.
    Diagnostic {
        /// Whether `.help` is required and `$cause` is passed.
        with_cause: bool,
    },
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
        let with_cause = *code == LEX_OPERANDS_IN_COMMENT;
        reference(
            &mut ids,
            code.fluent_id(),
            Expect::Diagnostic { with_cause },
        );
    }
    ids
}

/// Returns why `attributes` do not meet `expect`, if they do not.
fn attribute_problem(expect: Expect, attributes: &BTreeSet<String>) -> Option<String> {
    let names: BTreeSet<&str> = attributes.iter().map(String::as_str).collect();
    match expect {
        Expect::Entry { flags } => {
            let mut wanted = BTreeSet::from(["details"]);
            if flags {
                wanted.insert("flags");
            }
            (names != wanted).then(|| format!("attributes {names:?}, expected exactly {wanted:?}"))
        }
        Expect::Phrase => {
            (!names.is_empty()).then(|| format!("attributes {names:?}, expected none"))
        }
        Expect::Diagnostic { with_cause } => {
            if !names.iter().all(|name| ["help", "note"].contains(name)) {
                Some(format!(
                    "attributes {names:?}, only help and note are allowed"
                ))
            } else if with_cause && !names.contains("help") {
                Some("the .help attribute is required".to_owned())
            } else {
                None
            }
        }
    }
}

/// The argument sets each message is formatted with (m1 §6.3 step 5): none for documentation,
/// `$text` for diagnostics, and every `$cause` for the operands-in-comment message.
fn sample_args(expect: Expect) -> Vec<Option<FluentArgs<'static>>> {
    match expect {
        Expect::Entry { .. } | Expect::Phrase => vec![None],
        Expect::Diagnostic { with_cause: false } => {
            let mut args = FluentArgs::new();
            args.set("text", "d1");
            vec![Some(args)]
        }
        Expect::Diagnostic { with_cause: true } => CAUSES
            .iter()
            .map(|cause| {
                let mut args = FluentArgs::new();
                args.set("text", "2, 3");
                args.set("cause", *cause);
                Some(args)
            })
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
    let message = catalog.bundle.get_message(id).unwrap();
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
/// variant key, and mixed ASCII and typographic apostrophes.
fn style_problems(name: &str, source: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if source.contains('\t') || source.contains('\r') {
        problems.push(format!("{name}: tabs or carriage returns"));
    }
    if source.contains('\'') && source.contains('\u{2019}') {
        problems.push(format!("{name}: mixes ASCII and typographic apostrophes"));
    }
    let mut in_select = false;
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
            let valid = in_select
                && key.starts_with('[')
                && key.find("] ").is_some_and(|close| {
                    key[1..close]
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                });
            if !valid {
                problems.push(format!(
                    "{at}: continuation line starts with * or [: {text:?}"
                ));
            }
        }
        if text.trim_end().ends_with("->") {
            in_select = true;
        } else if text.trim_end() == "}" {
            in_select = false;
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
    assert!(count("isa-m68k-outcome-") > 0, "outcomes");
    assert!(count("isa-m68k-unit-") > 0, "units");
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
fn operands_in_comment_help_depends_on_the_cause() {
    let id = LEX_OPERANDS_IN_COMMENT.fluent_id();
    for lang in LANGS {
        let catalog = load(lang);
        let helps: BTreeSet<String> = sample_args(Expect::Diagnostic { with_cause: true })
            .iter()
            .map(|args| format(&catalog, &id, Some("help"), args.as_ref()))
            .collect();
        assert_eq!(
            helps.len(),
            CAUSES.len(),
            "{lang}: one help text per cause: {helps:?}"
        );
        let message = format(
            &catalog,
            &id,
            None,
            sample_args(Expect::Diagnostic { with_cause: true })[0].as_ref(),
        );
        assert!(
            message.contains("2, 3"),
            "{lang}: the message quotes $text: {message}"
        );
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
fn french_is_not_a_copy_of_english() {
    let en = load("en");
    let fr = load("fr");
    let mut args = FluentArgs::new();
    args.set("text", "x");
    args.set("cause", "operand-list");
    for id in en
        .messages
        .keys()
        .filter(|id| fr.messages.contains_key(*id))
    {
        let args = id.starts_with("diag-").then_some(&args);
        let english = format(&en, id, None, args);
        let french = format(&fr, id, None, args);
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
