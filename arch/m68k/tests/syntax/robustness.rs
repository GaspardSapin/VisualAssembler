//! Robustness of the lexer (m1-core-isa-lexer.md §5.2, §5.6, §9.1): 64 KiB pathological lines
//! in linear time (the CI guard against quadratic behaviour), the 64 KiB limit, `\r`, `\n` and
//! NUL inside a line, 4-byte UTF-8 chars, and every keyword in mixed case.
#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use va_arch_m68k::syntax::{
    LEX_BAD_LABEL, LEX_BAD_OPERATION, LEX_LABEL_JUNK, LEX_LINE_TOO_LONG, LEX_NON_ASCII,
    LEX_UNEXPECTED_CHAR, LEX_UNTERMINATED_STRING, M68kSyntax,
};
use va_core::source::{MAX_LINE_BYTES, TextRange};
use va_core::syntax::{KeywordKind, SyntaxProvider, Token, TokenKind};

use crate::{
    COMMENT, DIRECTIVE, LABEL, MNEMONIC, NUM, PUNCT, REG, SIZE, STR, WS, invalid, lex, mixed_case,
    pieces,
};

/// Repeats `pattern` up to `MAX_LINE_BYTES` bytes, cut on a char boundary.
fn repeated(prefix: &str, pattern: &str) -> String {
    let mut line = String::with_capacity(MAX_LINE_BYTES);
    line.push_str(prefix);
    while line.len() + pattern.len() <= MAX_LINE_BYTES {
        line.push_str(pattern);
    }
    for char in pattern.chars() {
        if line.len() + char.len_utf8() > MAX_LINE_BYTES {
            break;
        }
        line.push(char);
    }
    line
}

/// The 40 pathological 64 KiB lines of the §5.6 CI guard: one char repeated, and the patterns
/// that make a naive lexer rescan (strings, suffixes, backslashes, §5.7 trailing text).
fn pathological_lines() -> Vec<String> {
    let mut lines = Vec::new();
    for pattern in [
        "'", "\"", "\\", "$", "%", "@", "(", ")", ";", "a", ".", "é", "😀", ",", " ", "\t", "1",
        "*", ":", "#", "<", "-", "\u{0}",
    ] {
        lines.push(repeated("", pattern));
    }
    for (prefix, pattern) in [
        ("\tdc.b\t", "1, "),
        ("\tdc.b\t1,", " 2"),
        ("\tmove.l\td0,", " d1,"),
        ("\tdc.b\t", "''"),
        ("\tdc.b\t", "'a"),
        ("\tdc.b\t", "1$"),
        ("\tdc.b\t", "\\@"),
        ("\tdc.b\t", "$1."),
        ("\tdc.b\t", "d0.w"),
        ("", "a:"),
        ("\t", "x\\@"),
        ("\tdc.b\t1 ", ",("),
        ("\tdc.b\t1,2 ", "3,"),
        ("\tinclude\t", "a\"b"),
        ("\tdc.b\t", "(("),
        ("\tdc.b\t", "<<<"),
        ("\tdc.b\t", "é1"),
    ] {
        lines.push(repeated(prefix, pattern));
    }
    lines
}

#[test]
fn pathological_lines_are_linear() {
    let lines = pathological_lines();
    assert_eq!(lines.len(), 40);
    let started = Instant::now();
    for line in &lines {
        assert!(line.len() <= MAX_LINE_BYTES && line.len() > MAX_LINE_BYTES - 8);
        lex(line);
    }
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(30),
        "40 pathological lines took {elapsed:?}"
    );
}

#[test]
fn a_line_over_the_limit_is_one_invalid_token() {
    let mut line = "\tdc.b\t1".to_owned();
    line.push_str(&" ".repeat(MAX_LINE_BYTES - line.len()));
    let tokens = lex(&line);
    assert!(
        tokens
            .iter()
            .all(|token| token.kind != invalid(LEX_LINE_TOO_LONG))
    );
    line.push('x');
    let end = u32::try_from(line.len()).unwrap();
    assert_eq!(
        lex(&line),
        [Token {
            kind: invalid(LEX_LINE_TOO_LONG),
            range: TextRange::new(0, end)
        }]
    );
    let long = "é".repeat(MAX_LINE_BYTES);
    assert_eq!(lex(&long).len(), 1);
}

#[test]
fn carriage_return_and_line_feed_inside_a_line() {
    let unexpected = invalid(LEX_UNEXPECTED_CHAR);
    assert_eq!(
        pieces("a\rb"),
        [(LABEL, "a"), (unexpected, "\r"), (LABEL, "b")]
    );
    assert_eq!(
        pieces("\tnop\r\n\tnop"),
        [
            (WS, "\t"),
            (MNEMONIC, "nop"),
            (unexpected, "\r\n"),
            (WS, "\t"),
            (MNEMONIC, "nop")
        ]
    );
    assert_eq!(
        pieces("; c\rx"),
        [(COMMENT, "; c"), (unexpected, "\r"), (LABEL, "x")]
    );
    assert_eq!(
        pieces("\tdc.b\t'a\nb'"),
        [
            (WS, "\t"),
            (DIRECTIVE, "dc"),
            (SIZE, ".b"),
            (WS, "\t"),
            (invalid(LEX_UNTERMINATED_STRING), "'a"),
            (unexpected, "\n"),
            (LABEL, "b"),
            (invalid(LEX_LABEL_JUNK), "'"),
        ]
    );
    assert_eq!(pieces("\n"), [(unexpected, "\n")]);
    assert_eq!(
        pieces("\r\r\nx\n"),
        [(unexpected, "\r\r\n"), (LABEL, "x"), (unexpected, "\n")]
    );
}

#[test]
fn nul_inside_a_line() {
    assert_eq!(
        pieces("\tdc.b\t1\u{0}"),
        [
            (WS, "\t"),
            (DIRECTIVE, "dc"),
            (SIZE, ".b"),
            (WS, "\t"),
            (NUM, "1"),
            (invalid(LEX_UNEXPECTED_CHAR), "\u{0}"),
        ]
    );
    assert_eq!(pieces("; a\u{0}b"), [(COMMENT, "; a\u{0}b")]);
    assert_eq!(
        pieces("lab\u{0}"),
        [(LABEL, "lab"), (invalid(LEX_LABEL_JUNK), "\u{0}")]
    );
    assert_eq!(pieces("\u{0}"), [(invalid(LEX_BAD_LABEL), "\u{0}")]);
    assert_eq!(
        pieces("\t\u{0}"),
        [(WS, "\t"), (invalid(LEX_BAD_OPERATION), "\u{0}")]
    );
}

#[test]
fn four_byte_chars_in_comments_strings_and_elsewhere() {
    assert_eq!(pieces("; 😀😀"), [(COMMENT, "; 😀😀")]);
    assert_eq!(pieces("* 😀"), [(COMMENT, "* 😀")]);
    assert_eq!(
        pieces("\tdc.b\t'😀',\"😀\"\t😀"),
        [
            (WS, "\t"),
            (DIRECTIVE, "dc"),
            (SIZE, ".b"),
            (WS, "\t"),
            (STR, "'😀'"),
            (PUNCT, ","),
            (STR, "\"😀\""),
            (WS, "\t"),
            (COMMENT, "😀"),
        ]
    );
    assert_eq!(
        pieces("\tdc.b\t😀"),
        [
            (WS, "\t"),
            (DIRECTIVE, "dc"),
            (SIZE, ".b"),
            (WS, "\t"),
            (invalid(LEX_NON_ASCII), "😀")
        ]
    );
    assert_eq!(pieces("😀:"), [(invalid(LEX_BAD_LABEL), "😀:")]);
    assert_eq!(
        pieces("\t😀"),
        [(WS, "\t"), (invalid(LEX_BAD_OPERATION), "😀")]
    );
}

#[test]
fn keyword_list_has_the_spec_counts() {
    let keywords = M68kSyntax.keywords();
    let count = |kind| {
        keywords
            .iter()
            .filter(|keyword| keyword.kind == kind)
            .count()
    };
    assert_eq!(count(KeywordKind::Mnemonic), 126);
    assert_eq!(count(KeywordKind::Directive), 49);
    assert_eq!(count(KeywordKind::Register), 21);
    assert_eq!(count(KeywordKind::SizeSuffix), 4);
    assert_eq!(keywords.len(), 200);
    for keyword in keywords {
        assert_eq!(keyword.name, keyword.name.to_ascii_uppercase());
        assert_eq!(
            keyword.doc_id.is_some(),
            keyword.kind == KeywordKind::Mnemonic,
            "{keyword:?}"
        );
    }
}

#[test]
fn every_keyword_lexes_as_its_kind_in_mixed_case() {
    for keyword in M68kSyntax.keywords() {
        for spelling in [
            keyword.name.to_owned(),
            keyword.name.to_ascii_lowercase(),
            mixed_case(keyword.name),
            mixed_case(&keyword.name.to_ascii_lowercase()),
        ] {
            let (line, wanted, expected) = match keyword.kind {
                KeywordKind::Mnemonic => (format!("\t{spelling}"), 1, MNEMONIC),
                KeywordKind::Directive => (format!("\t{spelling}"), 1, DIRECTIVE),
                KeywordKind::Register => (format!("\tmove.l\t{spelling},d0"), 4, REG),
                KeywordKind::SizeSuffix => (format!("\tbra{spelling}\tx"), 2, SIZE),
            };
            let tokens = lex(&line);
            let token: &Token = &tokens[wanted];
            assert_eq!(token.kind, expected, "{line:?}");
            assert_eq!(
                token.range.slice(&line),
                Some(spelling.as_str()),
                "{line:?}"
            );
        }
    }
    assert!(matches!(lex("\tmove")[1].kind, TokenKind::Mnemonic));
}
