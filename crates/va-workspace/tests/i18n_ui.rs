//! Tests for `src/i18n/ui.rs` and `src/i18n/system.rs` (m4-editor-shell.md §4.6, §10.1, exit
//! criterion E6): the GUI precedence of `resolve_ui_lang` with a fake environment and a fake
//! system language, `system_ui_language`, the embedded root resources (they parse, keep their
//! namespaces, define no term, and `en`/`fr` have the same ids and attributes), the coverage of
//! `CONFIG_CODES`, and `Localizer::for_ui` with a fake CPU.
//!
//! The root `.ftl` files are filled by other work packages (`ui.ftl` and `commands.ftl` by
//! WP-F, `config.ftl` by WP-B). Every check here holds on header-only files and tightens as
//! they fill: the `CONFIG_CODES` coverage applies as soon as `config.ftl` defines a message,
//! and the root/CPU collision check runs on every root id that exists.
#![allow(clippy::unwrap_used)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use va_core::arch::Architecture;
use va_core::diag::DiagCode;
use va_core::isa::{EaTiming, InstructionInfo, InstructionSet, LookupError};
use va_core::regs::RegisterSpec;
use va_core::syntax::{KeywordInfo, LineState, SyntaxProvider, Token};
use va_workspace::config::CONFIG_CODES;
use va_workspace::i18n::{
    Lang, LangChoice, LangSource, Localizer, LocalizerError, resolve_lang, resolve_ui_lang,
    system_ui_language, ui_resources,
};

// -------------------------------------------------------------------------------------------
// resolve_ui_lang
// -------------------------------------------------------------------------------------------

/// A fake environment holding `pairs` (name, value), recording every name it is asked for;
/// other variables are unset.
struct Env {
    /// The set variables.
    pairs: Vec<(String, String)>,
    /// The names read so far, in order.
    reads: RefCell<Vec<String>>,
}

impl Env {
    /// An environment holding `pairs`.
    fn new(pairs: &[(&str, &str)]) -> Self {
        Self {
            pairs: pairs
                .iter()
                .map(|&(name, value)| (name.to_owned(), value.to_owned()))
                .collect(),
            reads: RefCell::new(Vec::new()),
        }
    }

    /// Reads `name`, recording the read.
    fn var(&self, name: &str) -> Option<String> {
        self.reads.borrow_mut().push(name.to_owned());
        self.pairs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    }

    /// The names read so far.
    fn reads(&self) -> Vec<String> {
        self.reads.borrow().clone()
    }
}

/// A fake system UI language, counting its calls.
struct System {
    /// What it answers.
    tag: Option<&'static str>,
    /// Number of calls so far.
    calls: RefCell<usize>,
}

impl System {
    /// A system whose UI language is `tag`.
    fn new(tag: Option<&'static str>) -> Self {
        Self {
            tag,
            calls: RefCell::new(0),
        }
    }

    /// The UI language, counting the call.
    fn get(&self) -> Option<String> {
        *self.calls.borrow_mut() += 1;
        self.tag.map(str::to_owned)
    }

    /// Number of calls so far.
    fn calls(&self) -> usize {
        *self.calls.borrow()
    }
}

/// Resolves the GUI language with `flag` and `setting` over `env` and `system`.
fn resolve(flag: Option<Lang>, setting: Option<Lang>, env: &Env, system: &System) -> LangChoice {
    resolve_ui_lang(flag, setting, &|name| env.var(name), &|| system.get())
}

/// The variables of a fake environment, as (name, value) pairs.
type Vars = [(&'static str, &'static str)];

/// The expected choice, with no ignored `VA_LANG`.
fn choice(lang: Lang, source: LangSource) -> LangChoice {
    LangChoice {
        lang,
        source,
        ignored_va_lang: None,
    }
}

/// The expected choice, with `ignored` as the rejected `VA_LANG`.
fn choice_ignoring(lang: Lang, source: LangSource, ignored: &str) -> LangChoice {
    LangChoice {
        lang,
        source,
        ignored_va_lang: Some(ignored.to_owned()),
    }
}

/// Every level set to English except the one under test, so a French result proves the level.
const ENGLISH_EVERYWHERE: [(&str, &str); 3] = [
    ("LC_ALL", "en_US.UTF-8"),
    ("LC_MESSAGES", "en_US.UTF-8"),
    ("LANG", "en_US.UTF-8"),
];

#[test]
fn flag_wins_and_reads_nothing() {
    for lang in Lang::ALL {
        let env = Env::new(&[("VA_LANG", "bogus"), ("LANG", "de_DE.UTF-8")]);
        let system = System::new(Some("de-DE"));
        assert_eq!(
            resolve(Some(lang), Some(Lang::En), &env, &system),
            choice(lang, LangSource::Flag),
            "a bad VA_LANG is not reported when --lang decides"
        );
        assert!(env.reads().is_empty(), "read {:?}", env.reads());
        assert_eq!(system.calls(), 0);
    }
}

#[test]
fn va_lang_beats_the_setting_and_auto() {
    let system = System::new(Some("en-US"));
    let mut pairs = ENGLISH_EVERYWHERE.to_vec();
    pairs.push(("VA_LANG", "fr"));
    let env = Env::new(&pairs);
    assert_eq!(
        resolve(None, Some(Lang::En), &env, &system),
        choice(Lang::Fr, LangSource::VaLang)
    );
    assert_eq!(env.reads(), ["VA_LANG"]);
    assert_eq!(system.calls(), 0);

    let env = Env::new(&[("VA_LANG", "en"), ("LANG", "fr_FR.UTF-8")]);
    let system = System::new(Some("fr-FR"));
    assert_eq!(
        resolve(None, Some(Lang::Fr), &env, &system),
        choice(Lang::En, LangSource::VaLang)
    );
    assert_eq!(system.calls(), 0);
}

#[test]
fn va_lang_auto_selects_auto_and_skips_the_setting() {
    let env = Env::new(&[("VA_LANG", "auto"), ("LANG", "en_GB.UTF-8")]);
    let system = System::new(Some("fr-FR"));
    assert_eq!(
        resolve(None, Some(Lang::Fr), &env, &system),
        choice(Lang::En, LangSource::Environment)
    );
    assert_eq!(system.calls(), 0);

    let env = Env::new(&[("VA_LANG", "auto")]);
    let system = System::new(Some("fr-CA"));
    assert_eq!(
        resolve(None, Some(Lang::En), &env, &system),
        choice(Lang::Fr, LangSource::System)
    );
}

#[test]
fn invalid_va_lang_is_reported_and_the_setting_applies() {
    for value in ["", "FR", " fr", "fr_FR", "de", "Auto"] {
        let mut pairs = ENGLISH_EVERYWHERE.to_vec();
        pairs.push(("VA_LANG", value));
        let env = Env::new(&pairs);
        let system = System::new(Some("en-US"));
        assert_eq!(
            resolve(None, Some(Lang::Fr), &env, &system),
            choice_ignoring(Lang::Fr, LangSource::Setting, value),
            "VA_LANG={value:?}"
        );
        assert_eq!(env.reads(), ["VA_LANG"], "VA_LANG={value:?}");
        assert_eq!(system.calls(), 0);
    }
}

#[test]
fn invalid_va_lang_without_setting_continues_with_auto() {
    let env = Env::new(&[("VA_LANG", "de")]);
    let system = System::new(Some("fr-FR"));
    assert_eq!(
        resolve(None, None, &env, &system),
        choice_ignoring(Lang::Fr, LangSource::System, "de")
    );
}

#[test]
fn setting_beats_the_locale_variables_and_the_system() {
    let env = Env::new(&ENGLISH_EVERYWHERE);
    let system = System::new(Some("en-US"));
    assert_eq!(
        resolve(None, Some(Lang::Fr), &env, &system),
        choice(Lang::Fr, LangSource::Setting)
    );
    assert_eq!(env.reads(), ["VA_LANG"], "no locale variable is read");
    assert_eq!(system.calls(), 0);

    let env = Env::new(&[("LC_ALL", "fr_FR.UTF-8")]);
    let system = System::new(Some("fr-FR"));
    assert_eq!(
        resolve(None, Some(Lang::En), &env, &system),
        choice(Lang::En, LangSource::Setting)
    );
}

#[test]
fn auto_takes_the_first_non_empty_locale_variable() {
    let cases: [(&Vars, Lang, LangSource); 6] = [
        (
            &[("LC_ALL", "fr_FR.UTF-8")],
            Lang::Fr,
            LangSource::Environment,
        ),
        (
            &[("LC_ALL", ""), ("LC_MESSAGES", "fr_BE"), ("LANG", "en_US")],
            Lang::Fr,
            LangSource::Environment,
        ),
        (
            &[("LC_ALL", "en_US.UTF-8"), ("LANG", "fr_FR.UTF-8")],
            Lang::En,
            LangSource::Environment,
        ),
        (
            &[("LC_MESSAGES", ""), ("LANG", "fr")],
            Lang::Fr,
            LangSource::Environment,
        ),
        // A variable naming another language decides, as in M1: English, not the system.
        (&[("LANG", "C")], Lang::En, LangSource::Default),
        (
            &[("LC_ALL", "de_DE.UTF-8"), ("LANG", "fr_FR")],
            Lang::En,
            LangSource::Default,
        ),
    ];
    for (pairs, lang, source) in cases {
        let env = Env::new(pairs);
        let system = System::new(Some("fr-FR"));
        assert_eq!(
            resolve(None, None, &env, &system),
            choice(lang, source),
            "{pairs:?}"
        );
        assert_eq!(system.calls(), 0, "{pairs:?}: the system is not asked");
    }
}

#[test]
fn auto_reads_the_variables_in_precedence_order_and_no_further() {
    let env = Env::new(&[("LC_MESSAGES", "fr_FR")]);
    let system = System::new(None);
    resolve(None, None, &env, &system);
    assert_eq!(env.reads(), ["VA_LANG", "LC_ALL", "LC_MESSAGES"]);
}

#[test]
fn auto_asks_the_system_when_no_locale_variable_is_set() {
    let cases: [(Option<&'static str>, Lang, LangSource); 7] = [
        (Some("fr-FR"), Lang::Fr, LangSource::System),
        (Some("fr"), Lang::Fr, LangSource::System),
        (Some("en-US"), Lang::En, LangSource::System),
        (Some("de-DE"), Lang::En, LangSource::Default),
        (Some("FR-fr"), Lang::En, LangSource::Default),
        (Some(""), Lang::En, LangSource::Default),
        (None, Lang::En, LangSource::Default),
    ];
    for (tag, lang, source) in cases {
        for pairs in [
            &[][..],
            &[("LC_ALL", ""), ("LC_MESSAGES", ""), ("LANG", "")][..],
        ] {
            let env = Env::new(pairs);
            let system = System::new(tag);
            assert_eq!(
                resolve(None, None, &env, &system),
                choice(lang, source),
                "system {tag:?}, variables {pairs:?}"
            );
            assert_eq!(system.calls(), 1);
        }
    }
}

#[test]
fn the_cli_rule_is_unchanged() {
    // Same inputs: the GUI asks the system, the CLI (M1) never does and has no setting level.
    let gui = resolve(None, Some(Lang::Fr), &Env::new(&[]), &System::new(None));
    assert_eq!(gui, choice(Lang::Fr, LangSource::Setting));
    let gui = resolve(None, None, &Env::new(&[]), &System::new(Some("fr-FR")));
    assert_eq!(gui, choice(Lang::Fr, LangSource::System));
    let env = Env::new(&[]);
    assert_eq!(
        resolve_lang(None, &|name| env.var(name)),
        choice(Lang::En, LangSource::Default)
    );
    let env = Env::new(&[("VA_LANG", "auto"), ("LANG", "fr_FR")]);
    assert_eq!(
        resolve_lang(None, &|name| env.var(name)),
        choice(Lang::Fr, LangSource::Environment)
    );
}

#[test]
fn system_ui_language_is_windows_only() {
    let tag = system_ui_language();
    if cfg!(windows) {
        if let Some(tag) = &tag {
            assert!(!tag.is_empty());
        }
        println!("note: system UI language here: {tag:?}");
    } else {
        assert_eq!(tag, None);
    }
}

// -------------------------------------------------------------------------------------------
// The root resources
// -------------------------------------------------------------------------------------------

/// The root resource files, in `ui_resources` order, with the id prefix each one owns
/// (architecture §5, m4 §4.6).
const ROOT_FILES: [(&str, &str); 3] = [
    ("ui.ftl", "ui-"),
    ("commands.ftl", "cmd-"),
    ("config.ftl", "diag-cfg-"),
];

/// Index of `config.ftl` in [`ROOT_FILES`].
const CONFIG_FILE: usize = 2;

/// One message of a root resource.
#[derive(Debug)]
struct Message {
    /// Index of its file in [`ROOT_FILES`].
    file: usize,
    /// Its attribute names.
    attributes: BTreeSet<String>,
    /// The variables its value and attributes reference (`{ $name }`, selectors included).
    variables: BTreeSet<String>,
}

/// One language of the root resources, parsed.
struct Catalog {
    /// The language.
    lang: Lang,
    /// Every root resource of the language in one bundle.
    bundle: FluentBundle<FluentResource>,
    /// Message id (from the column-0 scan) → its message.
    messages: BTreeMap<String, Message>,
}

impl Catalog {
    /// The ids defined in file `file`.
    fn ids_of(&self, file: usize) -> BTreeSet<&str> {
        self.messages
            .iter()
            .filter(|(_, message)| message.file == file)
            .map(|(id, _)| id.as_str())
            .collect()
    }
}

/// `"<lang>/<file>"`, for messages.
fn file_name(lang: Lang, file: usize) -> String {
    format!("{}/{}", lang.code(), ROOT_FILES[file].0)
}

/// The 1-based line of byte `offset` in `source`.
fn line_of(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())].matches('\n').count() + 1
}

/// Column-0 characters that continue an open entry: the close of a placeable, a variant, the
/// default variant, an attribute and a block placeable may all start at column 0 in Fluent.
const CONTINUATIONS: [char; 5] = ['}', '[', '*', '.', '{'];

/// Splits `source` into its messages by the column-0 rule of Fluent: a message starts at a
/// column-0 `identifier =` line and runs over the blank, indented or [`CONTINUATIONS`] lines
/// that follow; a column-0 comment ends it. Asserts that every other column-0 line starts a
/// message, so a term (`-name = …`) fails here. Returns (id, entry text) pairs; [`load`]
/// checks their count against the parser.
fn entries<'a>(name: &str, source: &'a str) -> Vec<(String, Vec<&'a str>)> {
    let mut found: Vec<(String, Vec<&'a str>)> = Vec::new();
    let mut open = false;
    for (index, line) in source.lines().enumerate() {
        let continues = line.trim().is_empty()
            || line.starts_with(' ')
            || (open && line.starts_with(CONTINUATIONS));
        if continues {
            if open && let Some((_, lines)) = found.last_mut() {
                lines.push(line);
            }
            continue;
        }
        if line.starts_with('#') {
            open = false;
            continue;
        }
        let end = line
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(line.len());
        let (id, rest) = line.split_at(end);
        assert!(
            id.starts_with(|c: char| c.is_ascii_alphabetic()) && rest.trim_start().starts_with('='),
            "{name}:{}: a column-0 line must be `identifier = …`, a comment or blank (the root \
             resources define no term): {line:?}",
            index + 1
        );
        found.push((id.to_owned(), vec![line]));
        open = true;
    }
    found
}

/// The variables an entry references: `$name` inside a placeable, right after `{`, `(` or `,`
/// (a variable, a selector, a function argument), skipping string literals. Text such as
/// "costs $5" or a variant's text is not a variable.
fn variables(lines: &[&str]) -> BTreeSet<String> {
    let chars: Vec<char> = lines.join("\n").chars().collect();
    let mut found = BTreeSet::new();
    let mut depth = 0usize;
    let mut previous = ' ';
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            '"' if depth > 0 => {
                index += 1;
                while index < chars.len() && chars[index] != '"' {
                    if chars[index] == '\\' {
                        index += 1;
                    }
                    index += 1;
                }
            }
            '$' if depth > 0 && matches!(previous, '{' | '(' | ',') => {
                let start = index + 1;
                let mut end = start;
                while end < chars.len()
                    && (chars[end].is_ascii_alphanumeric() || matches!(chars[end], '-' | '_'))
                {
                    end += 1;
                }
                if end > start {
                    found.insert(chars[start..end].iter().collect());
                }
                index = end;
                previous = 'a';
                continue;
            }
            _ => {}
        }
        if !c.is_whitespace() {
            previous = c;
        }
        index += 1;
    }
    found
}

/// Counts the messages and the terms of a parsed resource. `fluent-syntax` is not a direct
/// dependency, so the AST variant is read from its `Debug` name (`Message(…)`, `Term(…)`).
fn count_messages_and_terms(resource: &FluentResource) -> (usize, usize) {
    let names: Vec<String> = resource
        .entries()
        .map(|entry| format!("{entry:?}"))
        .collect();
    let messages = names
        .iter()
        .filter(|name| name.starts_with("Message("))
        .count();
    let terms = names
        .iter()
        .filter(|name| name.starts_with("Term("))
        .count();
    (messages, terms)
}

/// Parses the root resources of `lang` into one bundle and lists their messages.
fn load(lang: Lang) -> Catalog {
    let sources = ui_resources(lang);
    assert_eq!(
        sources.len(),
        ROOT_FILES.len(),
        "ui_resources({lang:?}) returns ui, commands and config"
    );
    let mut bundle = FluentBundle::new(vec![lang.code().parse().unwrap()]);
    bundle.set_use_isolating(false);
    let mut messages = BTreeMap::new();
    for (file, source) in sources.iter().enumerate() {
        let name = file_name(lang, file);
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
        let (message_count, term_count) = count_messages_and_terms(&resource);
        assert_eq!(
            term_count, 0,
            "{name}: the root resources define no Fluent term"
        );
        if let Err(errors) = bundle.add_resource(resource) {
            panic!("{name}: cannot be added to the bundle (duplicate id?): {errors:?}");
        }
        let scanned = entries(&name, source);
        assert_eq!(
            scanned.len(),
            message_count,
            "{name}: the column-0 scan disagrees with the parser"
        );
        for (id, lines) in scanned {
            let message = bundle
                .get_message(&id)
                .unwrap_or_else(|| panic!("{name}: column-0 id {id} is not a message"));
            let attributes = message
                .attributes()
                .map(|attribute| attribute.id().to_owned())
                .collect();
            let entry = Message {
                file,
                attributes,
                variables: variables(&lines),
            };
            assert!(
                messages.insert(id.clone(), entry).is_none(),
                "{name}: {id} is defined twice"
            );
        }
    }
    Catalog {
        lang,
        bundle,
        messages,
    }
}

/// A synthetic resource for the scanners: multi-line values, attributes, selectors (one written
/// at column 0), a function call, a string literal holding `$`, `$` in plain text, blank lines
/// and comments.
const SCANNED: &str = r#"# A comment.
ui-one = One { $count ->
    [one] item
   *[other] items
}

    .help = Help with { $path } and { "{$not}" }.
## A group comment.
ui-two =
    First line
    { $line }, then text with $plain, and { NUMBER($n, minimumFractionDigits: 2) }.
ui-three = { $mode ->
[a] Mode a
*[b] Mode b
}
.title = Title
ui-four = Four
"#;

/// A resource with a term.
const WITH_TERM: &str = "-brand = VisualAssembler\nui-x = { -brand }\n";

#[test]
fn the_scanners_split_entries_and_find_variables() {
    let found = entries("scanned.ftl", SCANNED);
    let ids: Vec<&str> = found.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["ui-one", "ui-two", "ui-three", "ui-four"]);
    let vars: Vec<BTreeSet<String>> = found.iter().map(|(_, lines)| variables(lines)).collect();
    let names =
        |list: &[&str]| -> BTreeSet<String> { list.iter().map(|&n| n.to_owned()).collect() };
    assert_eq!(vars[0], names(&["count", "path"]));
    assert_eq!(vars[1], names(&["line", "n"]));
    assert_eq!(vars[2], names(&["mode"]));
    assert_eq!(vars[3], names(&[]));
    let resource = FluentResource::try_new(SCANNED.to_owned()).unwrap();
    assert_eq!(count_messages_and_terms(&resource), (4, 0));
    let resource = FluentResource::try_new(WITH_TERM.to_owned()).unwrap();
    assert_eq!(count_messages_and_terms(&resource), (1, 1));
}

#[test]
#[should_panic(expected = "define no term")]
fn the_scanner_rejects_a_term() {
    entries("term.ftl", WITH_TERM);
}

#[test]
fn ui_resources_embed_the_root_locale_files_in_order() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../locales");
    for lang in Lang::ALL {
        let embedded = ui_resources(lang);
        assert_eq!(embedded.len(), ROOT_FILES.len());
        for (file, (name, _)) in ROOT_FILES.iter().enumerate() {
            let path = root.join(lang.code()).join(name);
            let on_disk = fs::read_to_string(&path).unwrap();
            assert_eq!(
                embedded[file],
                on_disk,
                "ui_resources({lang:?})[{file}] is not {}",
                path.display()
            );
        }
    }
}

#[test]
fn every_root_resource_parses_and_keeps_its_namespace() {
    for lang in Lang::ALL {
        let catalog = load(lang);
        for (id, message) in &catalog.messages {
            let (file, prefix) = ROOT_FILES[message.file];
            assert!(
                id.starts_with(prefix) && id.len() > prefix.len(),
                "{}/{file}: {id} is outside the namespace {prefix}* of the file",
                lang.code()
            );
        }
        let counts: Vec<String> = (0..ROOT_FILES.len())
            .map(|file| format!("{} {}", file_name(lang, file), catalog.ids_of(file).len()))
            .collect();
        println!("note: messages per file: {}", counts.join(", "));
    }
}

#[test]
fn english_and_french_have_the_same_ids_and_attributes() {
    let en = load(Lang::En);
    let fr = load(Lang::Fr);
    for file in 0..ROOT_FILES.len() {
        assert_eq!(
            en.ids_of(file),
            fr.ids_of(file),
            "{} and {} define different ids",
            file_name(Lang::En, file),
            file_name(Lang::Fr, file)
        );
    }
    for (id, english) in &en.messages {
        let french = &fr.messages[id];
        assert_eq!(
            english.attributes, french.attributes,
            "{id}: attribute sets differ between en and fr"
        );
        // The code passes the arguments of the English message: French may not need more.
        // config.ftl promises the same variables (its header), the other files a subset.
        if english.file == CONFIG_FILE {
            assert_eq!(
                english.variables, french.variables,
                "{id}: variables differ between en and fr"
            );
        } else {
            assert!(
                french.variables.is_subset(&english.variables),
                "{id}: fr uses variables {:?} that en does not have {:?}",
                french.variables,
                english.variables
            );
        }
    }
}

/// The arguments of each configuration problem (m4 §4.2 table), in `CONFIG_CODES` order; the
/// `bool` tells whether the argument is a number.
const CONFIG_ARGS: [(&str, &[(&str, bool)]); 12] = [
    ("cfg-unreadable", &[("error", false)]),
    ("cfg-too-large", &[("bytes", true)]),
    ("cfg-not-utf8", &[("offset", true)]),
    ("cfg-syntax", &[("detail", false)]),
    ("cfg-schema-version", &[("found", false)]),
    ("cfg-unknown-key", &[("key", false)]),
    ("cfg-wrong-type", &[("key", false), ("expected", false)]),
    (
        "cfg-out-of-range",
        &[("key", false), ("min", true), ("max", true)],
    ),
    (
        "cfg-invalid-value",
        &[("key", false), ("value", false), ("expected", false)],
    ),
    ("cfg-unknown-command", &[("command", false)]),
    ("cfg-bad-key", &[("key", false)]),
    ("cfg-bad-path", &[("key", false), ("value", false)]),
];

#[test]
fn config_codes_are_root_codes_listed_with_their_arguments() {
    let listed: Vec<&str> = CONFIG_ARGS.iter().map(|(code, _)| *code).collect();
    let codes: Vec<&str> = CONFIG_CODES.iter().map(|code| code.0).collect();
    assert_eq!(listed, codes, "CONFIG_ARGS follows CONFIG_CODES (m4 §4.2)");
    for code in CONFIG_CODES {
        // A CPU's codes never start with `cfg-` (architecture §5): these ids are the root's.
        assert!(code.0.starts_with("cfg-"), "{}", code.0);
        assert!(code.fluent_id().starts_with(ROOT_FILES[CONFIG_FILE].1));
    }
}

#[test]
fn config_ftl_covers_every_config_code_once_it_is_filled() {
    let en = load(Lang::En);
    let fr = load(Lang::Fr);
    if en.ids_of(CONFIG_FILE).is_empty() && fr.ids_of(CONFIG_FILE).is_empty() {
        println!("note: config.ftl is header-only (WP-B): the CONFIG_CODES coverage waits for it");
        return;
    }
    let expected: BTreeSet<String> = CONFIG_CODES.iter().map(|code| code.fluent_id()).collect();
    for catalog in [&en, &fr] {
        let defined: BTreeSet<String> = catalog
            .ids_of(CONFIG_FILE)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let missing: Vec<&String> = expected.difference(&defined).collect();
        let orphans: Vec<&String> = defined.difference(&expected).collect();
        assert!(
            missing.is_empty(),
            "{}: no message for {missing:?}",
            file_name(catalog.lang, CONFIG_FILE)
        );
        assert!(
            orphans.is_empty(),
            "{}: messages of no CONFIG_CODES entry {orphans:?}",
            file_name(catalog.lang, CONFIG_FILE)
        );
    }
}

/// Sample values for the arguments `names` of a configuration problem.
fn config_args(names: &[(&'static str, bool)]) -> FluentArgs<'static> {
    let mut args = FluentArgs::new();
    for &(name, number) in names {
        if number {
            args.set(name, FluentValue::from(42));
        } else {
            args.set(name, FluentValue::from("sample"));
        }
    }
    args
}

#[test]
fn config_messages_use_only_their_arguments() {
    for lang in Lang::ALL {
        let catalog = load(lang);
        for (code, names) in CONFIG_ARGS {
            let id = DiagCode(code).fluent_id();
            let Some(entry) = catalog.messages.get(&id) else {
                continue;
            };
            let allowed: BTreeSet<String> =
                names.iter().map(|(name, _)| (*name).to_owned()).collect();
            assert!(
                entry.variables.is_subset(&allowed),
                "{}: {id} uses {:?}, its arguments are {allowed:?}",
                lang.code(),
                entry.variables
            );
            let message = catalog.bundle.get_message(&id).unwrap();
            let args = config_args(names);
            let patterns = message
                .value()
                .into_iter()
                .map(|pattern| ("value".to_owned(), pattern))
                .chain(
                    message
                        .attributes()
                        .map(|attribute| (format!(".{}", attribute.id()), attribute.value())),
                );
            for (part, pattern) in patterns {
                let mut errors = Vec::new();
                let text = catalog
                    .bundle
                    .format_pattern(pattern, Some(&args), &mut errors);
                assert!(
                    errors.is_empty(),
                    "{}: {id} {part} does not format: {errors:?}",
                    lang.code()
                );
                assert!(
                    !text.trim().is_empty(),
                    "{}: {id} {part} is empty",
                    lang.code()
                );
            }
        }
    }
}

// -------------------------------------------------------------------------------------------
// Localizer::for_ui
// -------------------------------------------------------------------------------------------

/// A lexer that produces no token.
struct NoSyntax;

impl SyntaxProvider for NoSyntax {
    fn lex_line(&self, _line: &str, state: LineState, _out: &mut Vec<Token>) -> LineState {
        state
    }

    fn keywords(&self) -> &[KeywordInfo] {
        &[]
    }
}

/// An instruction set with no instruction.
struct NoIsa;

impl InstructionSet for NoIsa {
    fn spellings(&self) -> &[&'static str] {
        &[]
    }

    fn describe(
        &self,
        _spelling: &str,
        _size: Option<&str>,
    ) -> Result<InstructionInfo, LookupError> {
        Err(LookupError::Unknown)
    }

    fn ea_timing(&self) -> &[EaTiming] {
        &[]
    }
}

/// A CPU whose resources are `en` and `fr`, by language code.
struct FakeCpu {
    /// Resources of `"en"`.
    en: &'static [&'static str],
    /// Resources of `"fr"`.
    fr: &'static [&'static str],
}

impl Architecture for FakeCpu {
    fn id(&self) -> &'static str {
        "fake"
    }

    fn syntax(&self) -> &dyn SyntaxProvider {
        &NoSyntax
    }

    fn instruction_set(&self) -> &dyn InstructionSet {
        &NoIsa
    }

    fn registers(&self) -> &'static [RegisterSpec] {
        &[]
    }

    fn diagnostic_codes(&self) -> &'static [DiagCode] {
        &[]
    }

    fn locale_resources(&self, lang: &str) -> &'static [&'static str] {
        match lang {
            "en" => self.en,
            "fr" => self.fr,
            _ => &[],
        }
    }
}

/// English resources of the fake CPU: instruction docs, then diagnostics.
const CPU_EN: [&str; 2] = [
    "isa-fake-nop = Does nothing.\n    .details = Takes one cycle.\n",
    "diag-fake-bad = Bad { $what }.\n    .help = Remove { $what }.\ndiag-fake-only-en = English only.\n",
];

/// French resources of the fake CPU: partial on purpose (no `.details`, no `.help`, no
/// `diag-fake-only-en`).
const CPU_FR: [&str; 2] = [
    "isa-fake-nop = Ne fait rien.\n",
    "diag-fake-bad = { $what } incorrect.\n",
];

/// A text argument.
fn text(value: &str) -> va_core::diag::DiagArg {
    va_core::diag::DiagArg::Str(value.to_owned())
}

#[test]
fn for_ui_builds_without_a_cpu_in_both_languages() {
    for lang in Lang::ALL {
        let localizer = Localizer::for_ui(lang, None).unwrap();
        assert_eq!(localizer.lang(), lang);
        assert_eq!(localizer.message("isa-fake-nop", None, &[]), None);
    }
}

#[test]
fn for_ui_joins_the_cpu_resources_with_english_fallback() {
    let cpu = FakeCpu {
        en: &CPU_EN,
        fr: &CPU_FR,
    };
    let fr = Localizer::for_ui(Lang::Fr, Some(&cpu)).unwrap();
    assert_eq!(fr.lang(), Lang::Fr);
    assert_eq!(
        fr.message("isa-fake-nop", None, &[]),
        Some("Ne fait rien.".to_owned())
    );
    assert_eq!(
        fr.message("isa-fake-nop", Some("details"), &[]),
        Some("Takes one cycle.".to_owned()),
        "a missing French attribute falls back to the English one"
    );
    assert_eq!(
        fr.message("diag-fake-bad", None, &[("what", text("x"))]),
        Some("x incorrect.".to_owned())
    );
    assert_eq!(
        fr.message("diag-fake-bad", Some("help"), &[("what", text("x"))]),
        Some("Remove x.".to_owned())
    );
    assert_eq!(
        fr.message("diag-fake-only-en", None, &[]),
        Some("English only.".to_owned())
    );

    let en = Localizer::for_ui(Lang::En, Some(&cpu)).unwrap();
    assert_eq!(
        en.message("isa-fake-nop", None, &[]),
        Some("Does nothing.".to_owned())
    );
}

#[test]
fn for_ui_rejects_an_id_the_cpu_defines_twice() {
    let cpu = FakeCpu {
        en: &["diag-fake-twice = One.\n", "diag-fake-twice = Two.\n"],
        fr: &["diag-fake-twice = Un.\n"],
    };
    for lang in Lang::ALL {
        assert_eq!(
            Localizer::for_ui(lang, Some(&cpu)).unwrap_err(),
            LocalizerError::Duplicate {
                lang: Lang::En,
                id: "diag-fake-twice".to_owned(),
            },
            "{lang:?}: the English list holds the duplicate"
        );
    }
}

#[test]
fn for_ui_counts_resource_indices_from_the_root_resources() {
    let cpu = FakeCpu {
        en: &CPU_EN,
        fr: &["isa-fake-nop = Ne fait rien.\n", "diag-fake-bad = {\n"],
    };
    match Localizer::for_ui(Lang::Fr, Some(&cpu)).unwrap_err() {
        LocalizerError::Parse { lang, resource, .. } => {
            assert_eq!(lang, Lang::Fr);
            assert_eq!(resource, ui_resources(Lang::Fr).len() + 1);
        }
        other => panic!("expected a parse error, got {other:?}"),
    }
}

#[test]
fn for_ui_rejects_a_cpu_id_that_is_a_root_id() {
    let mut checked = 0;
    for lang in Lang::ALL {
        let catalog = load(lang);
        for file in 0..ROOT_FILES.len() {
            let Some(&id) = catalog.ids_of(file).iter().next() else {
                continue;
            };
            let resource: &'static str = Box::leak(format!("{id} = Clash.\n").into_boxed_str());
            let resources: &'static [&'static str] = Box::leak(Box::new([resource]));
            let cpu = FakeCpu {
                en: resources,
                fr: resources,
            };
            assert_eq!(
                Localizer::for_ui(lang, Some(&cpu)).unwrap_err(),
                LocalizerError::Duplicate {
                    lang,
                    id: id.to_owned(),
                },
                "{}: a CPU defining {id}",
                file_name(lang, file)
            );
            checked += 1;
        }
    }
    println!("note: root/CPU collision checked on {checked} root files with ids");
}
