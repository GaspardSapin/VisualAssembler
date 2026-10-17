//! Tests for `src/i18n/lang.rs`: `Lang` codes, and the `resolve_lang` precedence (`--lang`,
//! `VA_LANG`, then `LC_ALL`/`LC_MESSAGES`/`LANG`) through a fake environment.
#![allow(clippy::unwrap_used)]

use std::cell::RefCell;

use va_workspace::i18n::{Lang, LangChoice, LangSource, resolve_lang};

/// The variables of a fake environment, as (name, value) pairs.
type Vars = [(&'static str, &'static str)];

/// A fake environment holding `pairs` (name, value); other variables are unset.
fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let pairs: Vec<(String, String)> = pairs
        .iter()
        .map(|&(name, value)| (name.to_owned(), value.to_owned()))
        .collect();
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    }
}

/// Resolves the language with `flag` in the fake environment `pairs`.
fn resolve(flag: Option<Lang>, pairs: &[(&str, &str)]) -> LangChoice {
    resolve_lang(flag, &env(pairs))
}

/// The expected choice, with no ignored `VA_LANG`.
fn choice(lang: Lang, source: LangSource) -> LangChoice {
    LangChoice {
        lang,
        source,
        ignored_va_lang: None,
    }
}

#[test]
fn codes_round_trip() {
    assert_eq!(Lang::ALL, [Lang::En, Lang::Fr]);
    assert_eq!(Lang::En.code(), "en");
    assert_eq!(Lang::Fr.code(), "fr");
    for lang in Lang::ALL {
        assert_eq!(Lang::parse(lang.code()), Some(lang));
    }
}

#[test]
fn parse_accepts_only_exact_lowercase_codes() {
    for code in [
        "", "auto", "EN", "Fr", "fr-FR", "fr_FR", " fr", "fr ", "de", "english",
    ] {
        assert_eq!(Lang::parse(code), None, "{code:?}");
    }
}

#[test]
fn flag_wins_and_reads_no_variable() {
    let reads = RefCell::new(Vec::new());
    let var = |name: &str| {
        reads.borrow_mut().push(name.to_owned());
        Some("en".to_owned())
    };
    assert_eq!(
        resolve_lang(Some(Lang::Fr), &var),
        choice(Lang::Fr, LangSource::Flag)
    );
    assert!(reads.borrow().is_empty());
    // A bad VA_LANG is not read, hence not reported, when the flag decides.
    assert_eq!(
        resolve(
            Some(Lang::En),
            &[("VA_LANG", "bogus"), ("LANG", "fr_FR.UTF-8")]
        ),
        choice(Lang::En, LangSource::Flag)
    );
}

#[test]
fn va_lang_beats_the_locale_variables() {
    assert_eq!(
        resolve(None, &[("VA_LANG", "fr"), ("LC_ALL", "en_US.UTF-8")]),
        choice(Lang::Fr, LangSource::VaLang)
    );
    assert_eq!(
        resolve(None, &[("VA_LANG", "en"), ("LANG", "fr_FR.UTF-8")]),
        choice(Lang::En, LangSource::VaLang)
    );
}

#[test]
fn va_lang_auto_resolves_from_the_locale() {
    assert_eq!(
        resolve(None, &[("VA_LANG", "auto"), ("LANG", "fr_FR.UTF-8")]),
        choice(Lang::Fr, LangSource::Environment)
    );
    assert_eq!(
        resolve(None, &[("VA_LANG", "auto")]),
        choice(Lang::En, LangSource::Default)
    );
}

#[test]
fn invalid_va_lang_is_returned_and_auto_applies() {
    for value in ["bogus", "FR", "Auto", "fr_FR.UTF-8", " en", ""] {
        assert_eq!(
            resolve(None, &[("VA_LANG", value), ("LANG", "fr_FR.UTF-8")]),
            LangChoice {
                lang: Lang::Fr,
                source: LangSource::Environment,
                ignored_va_lang: Some(value.to_owned()),
            },
            "{value:?}"
        );
    }
    assert_eq!(
        resolve(None, &[("VA_LANG", "de")]),
        LangChoice {
            lang: Lang::En,
            source: LangSource::Default,
            ignored_va_lang: Some("de".to_owned()),
        }
    );
}

#[test]
fn first_non_empty_locale_variable_decides() {
    let cases: [(&Vars, Lang, LangSource); 6] = [
        (
            &[("LC_ALL", "fr"), ("LC_MESSAGES", "en"), ("LANG", "en")],
            Lang::Fr,
            LangSource::Environment,
        ),
        (
            &[("LC_MESSAGES", "fr_FR.UTF-8"), ("LANG", "en_US.UTF-8")],
            Lang::Fr,
            LangSource::Environment,
        ),
        (
            &[("LC_ALL", "en_US.UTF-8"), ("LANG", "fr_FR.UTF-8")],
            Lang::En,
            LangSource::Environment,
        ),
        (
            &[("LC_ALL", ""), ("LC_MESSAGES", ""), ("LANG", "fr_FR.UTF-8")],
            Lang::Fr,
            LangSource::Environment,
        ),
        // The first non-empty variable decides even when it names no shipped language.
        (
            &[("LC_ALL", "C"), ("LANG", "fr_FR.UTF-8")],
            Lang::En,
            LangSource::Default,
        ),
        (
            &[("LC_ALL", ""), ("LC_MESSAGES", ""), ("LANG", "")],
            Lang::En,
            LangSource::Default,
        ),
    ];
    for (pairs, lang, source) in cases {
        assert_eq!(resolve(None, pairs), choice(lang, source), "{pairs:?}");
    }
}

#[test]
fn locale_values_map_to_languages() {
    for (value, lang, source) in [
        ("fr", Lang::Fr, LangSource::Environment),
        ("fr_FR.UTF-8", Lang::Fr, LangSource::Environment),
        ("fr_CA", Lang::Fr, LangSource::Environment),
        ("fr_BE@euro", Lang::Fr, LangSource::Environment),
        ("en", Lang::En, LangSource::Environment),
        ("en_US.UTF-8", Lang::En, LangSource::Environment),
        ("en_GB", Lang::En, LangSource::Environment),
        ("de_DE.UTF-8", Lang::En, LangSource::Default),
        ("C", Lang::En, LangSource::Default),
        ("C.UTF-8", Lang::En, LangSource::Default),
        ("POSIX", Lang::En, LangSource::Default),
    ] {
        assert_eq!(
            resolve(None, &[("LANG", value)]),
            choice(lang, source),
            "{value:?}"
        );
    }
}

#[test]
fn auto_reads_only_the_three_locale_variables_in_order() {
    assert_eq!(
        resolve(None, &[("LANGUAGE", "fr"), ("LC_CTYPE", "fr_FR.UTF-8")]),
        choice(Lang::En, LangSource::Default)
    );
    let reads = RefCell::new(Vec::new());
    let var = |name: &str| {
        reads.borrow_mut().push(name.to_owned());
        None
    };
    assert_eq!(
        resolve_lang(None, &var),
        choice(Lang::En, LangSource::Default)
    );
    assert_eq!(
        *reads.borrow(),
        ["VA_LANG", "LC_ALL", "LC_MESSAGES", "LANG"]
    );
}

#[test]
fn va_lang_reads_stop_at_the_deciding_variable() {
    let reads = RefCell::new(Vec::new());
    let var = |name: &str| {
        reads.borrow_mut().push(name.to_owned());
        (name == "VA_LANG").then(|| "fr".to_owned())
    };
    assert_eq!(
        resolve_lang(None, &var),
        choice(Lang::Fr, LangSource::VaLang)
    );
    assert_eq!(*reads.borrow(), ["VA_LANG"]);
}
