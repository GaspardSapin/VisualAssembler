//! Tests for `src/i18n/localizer.rs` on synthetic resources: fallback to English, missing ids
//! (`None` / the code), arguments and plurals, no isolation marks, parse and duplicate errors,
//! `for_architecture`, and `Send + Sync`.
#![allow(clippy::unwrap_used)]

use va_core::arch::Architecture;
use va_core::diag::{DiagArg, DiagCode, Diagnostic, Severity};
use va_core::isa::{EaTiming, InstructionInfo, InstructionSet, LookupError};
use va_core::regs::RegisterSpec;
use va_core::source::{FileId, SourceSpan, TextRange};
use va_core::syntax::{KeywordInfo, LineState, SyntaxProvider, Token};
use va_workspace::i18n::{Lang, Localizer, LocalizerError, RenderedDiagnostic};

/// English resource: the complete reference.
const EN: &str = "\
hello = Hello, { $name }!
farewell = Goodbye.
    .help = Wave.
only-en = English only.
    .details = English details.
attrs-only =
    .help = Only help.
items = { $count ->
    [one] one item
   *[other] { $count } items
}
number = { $n }
diag-test-problem = Problem with “{ $text }”.
    .help = Fix “{ $text }”.
    .note = Background.
diag-test-bare = Bare problem.
";

/// French resource: partial on purpose (no `only-en`, no `.help` on `farewell`, no `.help` or
/// `.note` on `diag-test-problem`).
const FR: &str = "\
hello = Bonjour, { $name } !
farewell = Au revoir.
items = { $count ->
    [one] { $count } élément
   *[other] { $count } éléments
}
diag-test-problem = Problème avec « { $text } ».
";

/// A localizer for `lang` over [`EN`] and [`FR`].
fn localizer(lang: Lang) -> Localizer {
    let primary: &[&'static str] = match lang {
        Lang::En => &[EN],
        Lang::Fr => &[FR],
    };
    Localizer::new(lang, primary, &[EN]).unwrap()
}

/// A text argument.
fn text(value: &str) -> DiagArg {
    DiagArg::Str(value.to_owned())
}

/// A diagnostic with code `code` and the argument `text`.
fn diagnostic(code: &'static str) -> Diagnostic {
    let span = SourceSpan {
        file: FileId(0),
        range: TextRange::new(4, 9),
    };
    Diagnostic::new(DiagCode(code), Severity::Error, span).with_arg("text", text("d0 d1"))
}

#[test]
fn english_formats_with_arguments() {
    let en = localizer(Lang::En);
    assert_eq!(en.lang(), Lang::En);
    assert_eq!(
        en.message("hello", None, &[("name", text("Ada"))]),
        Some("Hello, Ada!".to_owned())
    );
    assert_eq!(
        en.message("farewell", Some("help"), &[]),
        Some("Wave.".to_owned())
    );
}

#[test]
fn requested_language_comes_first() {
    let fr = localizer(Lang::Fr);
    assert_eq!(fr.lang(), Lang::Fr);
    assert_eq!(
        fr.message("hello", None, &[("name", text("Ada"))]),
        Some("Bonjour, Ada !".to_owned())
    );
    assert_eq!(
        fr.message("farewell", None, &[]),
        Some("Au revoir.".to_owned())
    );
}

#[test]
fn missing_message_falls_back_to_english() {
    let fr = localizer(Lang::Fr);
    assert_eq!(
        fr.message("only-en", None, &[]),
        Some("English only.".to_owned())
    );
    assert_eq!(
        fr.message("only-en", Some("details"), &[]),
        Some("English details.".to_owned())
    );
}

#[test]
fn missing_attribute_falls_back_to_english_per_pattern() {
    let fr = localizer(Lang::Fr);
    assert_eq!(
        fr.message("farewell", Some("help"), &[]),
        Some("Wave.".to_owned())
    );
}

#[test]
fn missing_everywhere_is_none() {
    for lang in Lang::ALL {
        let localizer = localizer(lang);
        assert_eq!(localizer.message("no-such-id", None, &[]), None);
        assert_eq!(localizer.message("hello", Some("details"), &[]), None);
        assert_eq!(localizer.message("attrs-only", None, &[]), None);
        assert_eq!(
            localizer.message("attrs-only", Some("help"), &[]),
            Some("Only help.".to_owned())
        );
        assert_eq!(localizer.message("", None, &[]), None);
    }
}

#[test]
fn plurals_follow_the_language_rules() {
    let en = localizer(Lang::En);
    let fr = localizer(Lang::Fr);
    let count = |n: i64| [("count", DiagArg::Int(n))];
    // English: only 1 is "one"; French: 0 and 1 are "one".
    assert_eq!(
        en.message("items", None, &count(0)),
        Some("0 items".to_owned())
    );
    assert_eq!(
        en.message("items", None, &count(1)),
        Some("one item".to_owned())
    );
    assert_eq!(
        en.message("items", None, &count(2)),
        Some("2 items".to_owned())
    );
    assert_eq!(
        fr.message("items", None, &count(0)),
        Some("0 élément".to_owned())
    );
    assert_eq!(
        fr.message("items", None, &count(1)),
        Some("1 élément".to_owned())
    );
    assert_eq!(
        fr.message("items", None, &count(2)),
        Some("2 éléments".to_owned())
    );
}

#[test]
fn integers_print_exactly() {
    let en = localizer(Lang::En);
    for (value, expected) in [
        (0, "0"),
        (-42, "-42"),
        (65_535, "65535"),
        (1 << 53, "9007199254740992"),
        ((1 << 53) + 1, "9007199254740993"),
        (-(1 << 53) - 1, "-9007199254740993"),
        (i64::MAX, "9223372036854775807"),
        (i64::MIN, "-9223372036854775808"),
    ] {
        assert_eq!(
            en.message("number", None, &[("n", DiagArg::Int(value))]),
            Some(expected.to_owned()),
            "{value}"
        );
    }
}

#[test]
fn text_arguments_are_verbatim_and_the_last_repeat_wins() {
    let en = localizer(Lang::En);
    assert_eq!(
        en.message("number", None, &[("n", text("$1F"))]),
        Some("$1F".to_owned())
    );
    assert_eq!(
        en.message("hello", None, &[("name", text("A")), ("name", text("B"))]),
        Some("Hello, B!".to_owned())
    );
}

#[test]
fn missing_argument_gives_best_effort_text() {
    let en = localizer(Lang::En);
    assert_eq!(
        en.message("hello", None, &[]),
        Some("Hello, {$name}!".to_owned())
    );
}

#[test]
fn no_isolation_marks() {
    let marks = ['\u{2068}', '\u{2069}'];
    for lang in Lang::ALL {
        let localizer = localizer(lang);
        let hello = localizer
            .message("hello", None, &[("name", text("Ada"))])
            .unwrap();
        let items = localizer
            .message("items", None, &[("count", DiagArg::Int(3))])
            .unwrap();
        let rendered = localizer.render(&diagnostic("test-problem"));
        let help = rendered.help.clone().unwrap();
        for output in [hello, items, rendered.message, help] {
            assert!(!output.contains(marks), "{lang:?}: {output:?}");
        }
    }
}

#[test]
fn render_uses_the_message_and_its_help_and_note() {
    assert_eq!(
        localizer(Lang::En).render(&diagnostic("test-problem")),
        RenderedDiagnostic {
            message: "Problem with “d0 d1”.".to_owned(),
            help: Some("Fix “d0 d1”.".to_owned()),
            note: Some("Background.".to_owned()),
        }
    );
}

#[test]
fn render_falls_back_to_english_per_attribute() {
    assert_eq!(
        localizer(Lang::Fr).render(&diagnostic("test-problem")),
        RenderedDiagnostic {
            message: "Problème avec « d0 d1 ».".to_owned(),
            help: Some("Fix “d0 d1”.".to_owned()),
            note: Some("Background.".to_owned()),
        }
    );
}

#[test]
fn render_without_attributes_or_message() {
    for lang in Lang::ALL {
        let localizer = localizer(lang);
        assert_eq!(
            localizer.render(&diagnostic("test-bare")),
            RenderedDiagnostic {
                message: "Bare problem.".to_owned(),
                help: None,
                note: None,
            }
        );
        assert_eq!(
            localizer.render(&diagnostic("test-unknown")),
            RenderedDiagnostic {
                message: "test-unknown".to_owned(),
                help: None,
                note: None,
            }
        );
        // An invalid code never panics either: it simply has no message.
        assert_eq!(
            localizer.render(&diagnostic("Not Kebab")).message,
            "Not Kebab"
        );
    }
}

#[test]
fn english_given_twice_and_english_overrides() {
    // English with distinct lists: the first list wins, the second is the fallback.
    let en = Localizer::new(Lang::En, &["hello = Hi."], &[EN]).unwrap();
    assert_eq!(en.message("hello", None, &[]), Some("Hi.".to_owned()));
    assert_eq!(
        en.message("only-en", None, &[]),
        Some("English only.".to_owned())
    );
    // Empty lists are valid: every lookup misses.
    let empty = Localizer::new(Lang::Fr, &[], &[]).unwrap();
    assert_eq!(empty.message("hello", None, &[]), None);
    assert_eq!(
        empty.render(&diagnostic("test-problem")).message,
        "test-problem"
    );
}

#[test]
fn resources_load_in_order_across_several_files() {
    let fr = Localizer::new(Lang::Fr, &["a = un", "b = deux"], &["a = one", "c = three"]).unwrap();
    assert_eq!(fr.message("a", None, &[]), Some("un".to_owned()));
    assert_eq!(fr.message("b", None, &[]), Some("deux".to_owned()));
    assert_eq!(fr.message("c", None, &[]), Some("three".to_owned()));
}

#[test]
fn parse_errors_name_the_language_resource_and_line() {
    let junk = "ok = Fine.\nthis line has no equals sign\n";
    match Localizer::new(Lang::En, &[EN, junk], &[EN, junk]) {
        Err(LocalizerError::Parse {
            lang: Lang::En,
            resource: 1,
            errors,
        }) => {
            assert!(!errors.is_empty());
            assert!(
                errors.iter().all(|e| e.starts_with("line 2: ")),
                "{errors:?}"
            );
        }
        other => panic!("{other:?}"),
    }
    match Localizer::new(Lang::Fr, &[FR, "bad = { $x"], &[EN]) {
        Err(LocalizerError::Parse {
            lang: Lang::Fr,
            resource: 1,
            errors,
        }) => assert!(errors[0].starts_with("line 1: "), "{errors:?}"),
        other => panic!("{other:?}"),
    }
    // A broken English list is reported as English even for a French localizer.
    match Localizer::new(Lang::Fr, &[FR], &["junk"]) {
        Err(LocalizerError::Parse {
            lang: Lang::En,
            resource: 0,
            ..
        }) => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn duplicate_ids_are_rejected_within_one_language() {
    let cases: [(Lang, &[&'static str], &[&'static str], LocalizerError); 4] = [
        (
            Lang::En,
            &["a = 1\na = 2"],
            &["a = 1\na = 2"],
            LocalizerError::Duplicate {
                lang: Lang::En,
                id: "a".to_owned(),
            },
        ),
        (
            Lang::Fr,
            &["a = un", "b = deux\na = encore"],
            &[EN],
            LocalizerError::Duplicate {
                lang: Lang::Fr,
                id: "a".to_owned(),
            },
        ),
        (
            Lang::Fr,
            &[FR],
            &[EN, "hello = Again."],
            LocalizerError::Duplicate {
                lang: Lang::En,
                id: "hello".to_owned(),
            },
        ),
        (
            Lang::En,
            &["-brand = VA\n-brand = VisualAssembler"],
            &[],
            LocalizerError::Duplicate {
                lang: Lang::En,
                id: "brand".to_owned(),
            },
        ),
    ];
    for (lang, primary, english, expected) in cases {
        assert_eq!(
            Localizer::new(lang, primary, english).unwrap_err(),
            expected,
            "{primary:?} {english:?}"
        );
    }
}

#[test]
fn errors_display_their_cause() {
    let duplicate = LocalizerError::Duplicate {
        lang: Lang::Fr,
        id: "isa-m68k-add".to_owned(),
    };
    let parse = LocalizerError::Parse {
        lang: Lang::En,
        resource: 2,
        errors: vec!["line 3: first".to_owned(), "line 9: second".to_owned()],
    };
    assert_eq!(
        duplicate.to_string(),
        "Fluent resources of language fr define `isa-m68k-add` more than once"
    );
    assert_eq!(
        parse.to_string(),
        "Fluent resource 2 of language en does not parse: line 3: first; line 9: second"
    );
    let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(parse);
    assert!(boxed.source().is_none());
}

#[test]
fn localizer_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Localizer>();
    assert_send_sync::<LocalizerError>();
    let fr = localizer(Lang::Fr);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..4)
            .map(|n| {
                let fr = &fr;
                scope.spawn(move || fr.message("items", None, &[("count", DiagArg::Int(n))]))
            })
            .collect();
        let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
        assert_eq!(
            results,
            ["0 élément", "1 élément", "2 éléments", "3 éléments"].map(|s| Some(s.to_owned()))
        );
    });
    assert!(format!("{fr:?}").contains("Fr"));
}

/// A lexer that produces nothing (only `locale_resources` matters here).
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

#[test]
fn for_architecture_loads_the_cpu_resources_by_code() {
    let cpu = FakeCpu {
        en: &[EN],
        fr: &[FR],
    };
    let fr = Localizer::for_architecture(Lang::Fr, &cpu).unwrap();
    assert_eq!(fr.lang(), Lang::Fr);
    assert_eq!(
        fr.message("hello", None, &[("name", text("Ada"))]),
        Some("Bonjour, Ada !".to_owned())
    );
    assert_eq!(
        fr.message("only-en", None, &[]),
        Some("English only.".to_owned())
    );
    let en = Localizer::for_architecture(Lang::En, &cpu).unwrap();
    assert_eq!(
        en.message("hello", None, &[("name", text("Ada"))]),
        Some("Hello, Ada!".to_owned())
    );
}

#[test]
fn for_architecture_without_french_falls_back_to_english() {
    let cpu = FakeCpu { en: &[EN], fr: &[] };
    let fr = Localizer::for_architecture(Lang::Fr, &cpu).unwrap();
    assert_eq!(
        fr.render(&diagnostic("test-problem")).message,
        "Problem with “d0 d1”."
    );
}

#[test]
fn for_architecture_reports_broken_resources() {
    let cpu = FakeCpu {
        en: &[EN],
        fr: &["junk"],
    };
    assert!(matches!(
        Localizer::for_architecture(Lang::Fr, &cpu),
        Err(LocalizerError::Parse {
            lang: Lang::Fr,
            resource: 0,
            ..
        })
    ));
    // English alone does not read the French list.
    assert!(Localizer::for_architecture(Lang::En, &cpu).is_ok());
}
