//! Every `TokenKind` and every lexer diagnostic code is produced (m1-core-isa-lexer.md §9.1,
//! exit criterion E3): a table of lines with their expected kinds, then the union over that
//! table and the six fixtures, asserted complete. Also checks `DIAG_CODES` itself.
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use va_arch_m68k::syntax::{
    self, DIAG_CODES, LEX_BAD_BACKSLASH, LEX_BAD_LABEL, LEX_BAD_NUMBER, LEX_BAD_OPERATION,
    LEX_BAD_SIZE_SUFFIX, LEX_LABEL_JUNK, LEX_LINE_TOO_LONG, LEX_NON_ASCII, LEX_UNEXPECTED_CHAR,
    LEX_UNTERMINATED_STRING,
};
use va_core::diag::DiagCode;
use va_core::source::{MAX_LINE_BYTES, lines};
use va_core::syntax::TokenKind;

use crate::{
    COMMENT, DIRECTIVE, FIXTURES, LABEL, LOCAL, MACRO, MNEMONIC, NUM, OP, PARAM, PUNCT, REG, SIZE,
    STR, SUSPECT, SYM, WS, fixture_source, invalid, kinds_of, lex,
};

/// Every token kind name of `va_core::syntax::TokenKind::name`.
const KIND_NAMES: [&str; 17] = [
    "whitespace",
    "comment",
    "suspect-comment",
    "label",
    "local-label",
    "mnemonic",
    "directive",
    "macro-name",
    "size-suffix",
    "register",
    "symbol",
    "number",
    "string",
    "operator",
    "punctuation",
    "macro-param",
    "invalid",
];

/// Lines with their expected token kinds, covering every kind and every code.
fn table() -> Vec<(String, Vec<TokenKind>)> {
    let mut table: Vec<(String, Vec<TokenKind>)> = vec![
        (
            "start:\tmove.l\t#$10,d0\t; c".into(),
            vec![
                LABEL, PUNCT, WS, MNEMONIC, SIZE, WS, PUNCT, NUM, PUNCT, REG, WS, COMMENT,
            ],
        ),
        ("* whole line".into(), vec![COMMENT]),
        ("\tbra.s\t1$".into(), vec![WS, MNEMONIC, SIZE, WS, LOCAL]),
        (
            "\tdc.b\t'a',sym+1".into(),
            vec![WS, DIRECTIVE, SIZE, WS, STR, PUNCT, SYM, OP, NUM],
        ),
        ("\tmymac\t\\1".into(), vec![WS, MACRO, WS, PARAM]),
        (
            "\tdc.b\t1, 2".into(),
            vec![WS, DIRECTIVE, SIZE, WS, NUM, PUNCT, WS, SUSPECT],
        ),
        ("9x".into(), vec![invalid(LEX_BAD_LABEL)]),
        ("x:y".into(), vec![LABEL, PUNCT, invalid(LEX_LABEL_JUNK)]),
        ("\t123".into(), vec![WS, invalid(LEX_BAD_OPERATION)]),
        (
            "\tmove.q".into(),
            vec![WS, MNEMONIC, invalid(LEX_BAD_SIZE_SUFFIX)],
        ),
        (
            "\tdc.b\t$".into(),
            vec![WS, DIRECTIVE, SIZE, WS, invalid(LEX_BAD_NUMBER)],
        ),
        (
            "\tdc.b\t'a".into(),
            vec![WS, DIRECTIVE, SIZE, WS, invalid(LEX_UNTERMINATED_STRING)],
        ),
        (
            "\tdc.b\t\\%".into(),
            vec![WS, DIRECTIVE, SIZE, WS, invalid(LEX_BAD_BACKSLASH)],
        ),
        (
            "\tdc.b\t?".into(),
            vec![WS, DIRECTIVE, SIZE, WS, invalid(LEX_UNEXPECTED_CHAR)],
        ),
        (
            "\tdc.b\té".into(),
            vec![WS, DIRECTIVE, SIZE, WS, invalid(LEX_NON_ASCII)],
        ),
    ];
    table.push((
        format!("; {}", "x".repeat(MAX_LINE_BYTES)),
        vec![invalid(LEX_LINE_TOO_LONG)],
    ));
    table
}

/// Adds the kind name and, for `Invalid`/`SuspectComment`, the code of each token of `line`.
fn collect(line: &str, names: &mut BTreeSet<&'static str>, codes: &mut BTreeSet<DiagCode>) {
    for token in lex(line) {
        names.insert(token.kind.name());
        if let TokenKind::Invalid(code) | TokenKind::SuspectComment(code) = token.kind {
            codes.insert(code);
        }
    }
}

#[test]
fn table_lines_lex_to_their_kinds() {
    for (line, expected) in table() {
        assert_eq!(
            kinds_of(&line),
            expected,
            "{:?}",
            line.get(..40).unwrap_or(&line)
        );
    }
}

#[test]
fn table_and_fixtures_produce_every_kind_and_code() {
    let mut names = BTreeSet::new();
    let mut codes = BTreeSet::new();
    for (line, _) in table() {
        collect(&line, &mut names, &mut codes);
    }
    for fixture in FIXTURES {
        let source = fixture_source(fixture);
        for (_, line) in lines(&source) {
            collect(line, &mut names, &mut codes);
        }
    }
    let all_names: BTreeSet<&str> = KIND_NAMES.into_iter().collect();
    assert_eq!(names, all_names);
    let all_codes: BTreeSet<DiagCode> = DIAG_CODES.into_iter().collect();
    assert_eq!(codes, all_codes);
}

#[test]
fn errors_fixture_alone_produces_every_code() {
    let source = fixture_source("errors");
    let mut names = BTreeSet::new();
    let mut codes = BTreeSet::new();
    for (_, line) in lines(&source) {
        collect(line, &mut names, &mut codes);
    }
    let all_codes: BTreeSet<DiagCode> = DIAG_CODES.into_iter().collect();
    assert_eq!(codes, all_codes);
}

#[test]
fn diag_codes_are_the_eleven_lexer_codes() {
    let expected = [
        "lex-bad-label",
        "lex-label-junk",
        "lex-bad-operation",
        "lex-bad-size-suffix",
        "lex-bad-number",
        "lex-unterminated-string",
        "lex-bad-backslash",
        "lex-unexpected-char",
        "lex-non-ascii",
        "lex-line-too-long",
        "lex-operands-in-comment",
    ];
    let actual: Vec<&str> = DIAG_CODES.iter().map(|code| code.0).collect();
    assert_eq!(actual, expected);
    assert!(DIAG_CODES.iter().all(|code| code.is_valid()));
    assert_eq!(syntax::LEX_OPERANDS_IN_COMMENT, DIAG_CODES[10]);
}
