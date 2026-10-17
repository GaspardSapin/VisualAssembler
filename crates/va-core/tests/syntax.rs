//! Tests for `src/syntax.rs`: `TokenKind` ids, `LineState`, `check_line_tokens` (a valid cover
//! and every `TokenInvariantError`), `token_diagnostics` (offsets, truncation, blocking flags)
//! and the default `SyntaxProvider::line_diagnostics`.
#![allow(clippy::unwrap_used)]

use std::collections::HashSet;

use va_core::diag::{DiagArg, DiagCode, Diagnostic, Severity};
use va_core::source::{FileId, SourceSpan, TextRange};
use va_core::syntax::{
    KeywordInfo, KeywordKind, LineState, SyntaxProvider, Token, TokenInvariantError, TokenKind,
    check_line_tokens, token_diagnostics,
};

/// Code used for invalid tokens in these tests.
const BAD: DiagCode = DiagCode("lex-bad-number");

/// Code used for suspect comments in these tests.
const SUSPECT: DiagCode = DiagCode("lex-operands-in-comment");

/// Every token kind, once.
const ALL_KINDS: [TokenKind; 17] = [
    TokenKind::Whitespace,
    TokenKind::Comment,
    TokenKind::SuspectComment(SUSPECT),
    TokenKind::Label,
    TokenKind::LocalLabel,
    TokenKind::Mnemonic,
    TokenKind::Directive,
    TokenKind::MacroName,
    TokenKind::SizeSuffix,
    TokenKind::Register,
    TokenKind::Symbol,
    TokenKind::Number,
    TokenKind::String,
    TokenKind::Operator,
    TokenKind::Punctuation,
    TokenKind::MacroParam,
    TokenKind::Invalid(BAD),
];

/// Builds a token of `kind` over `start..end`.
fn token(kind: TokenKind, start: u32, end: u32) -> Token {
    Token {
        kind,
        range: TextRange::new(start, end),
    }
}

/// Builds tokens of kind `Symbol` from consecutive or arbitrary `(start, end)` pairs.
fn symbols(ranges: &[(u32, u32)]) -> Vec<Token> {
    ranges
        .iter()
        .map(|&(start, end)| token(TokenKind::Symbol, start, end))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Kinds and states
// ---------------------------------------------------------------------------------------------

#[test]
fn token_kind_names_are_unique_kebab_ids() {
    let names: Vec<&str> = ALL_KINDS.iter().map(|kind| kind.name()).collect();
    let unique: HashSet<&str> = names.iter().copied().collect();
    assert_eq!(unique.len(), ALL_KINDS.len(), "{names:?}");
    for name in &names {
        assert!(DiagCode(name).is_valid(), "{name:?} is not kebab-case");
    }
}

#[test]
fn token_kind_names_of_the_spec_examples() {
    assert_eq!(TokenKind::LocalLabel.name(), "local-label");
    assert_eq!(TokenKind::Invalid(BAD).name(), "invalid");
    assert_eq!(TokenKind::SuspectComment(SUSPECT).name(), "suspect-comment");
    assert_eq!(TokenKind::MacroName.name(), "macro-name");
    assert_eq!(TokenKind::SizeSuffix.name(), "size-suffix");
    assert_eq!(TokenKind::MacroParam.name(), "macro-param");
}

#[test]
fn the_code_is_not_part_of_the_name() {
    assert_eq!(
        TokenKind::Invalid(DiagCode("lex-non-ascii")).name(),
        TokenKind::Invalid(BAD).name()
    );
    assert_ne!(
        TokenKind::Invalid(DiagCode("lex-non-ascii")),
        TokenKind::Invalid(BAD)
    );
}

#[test]
fn initial_line_state_is_zero_and_the_default() {
    assert_eq!(LineState::INITIAL, LineState(0));
    assert_eq!(LineState::default(), LineState::INITIAL);
}

// ---------------------------------------------------------------------------------------------
// check_line_tokens
// ---------------------------------------------------------------------------------------------

#[test]
fn a_valid_cover_is_accepted() {
    let line = "\tmove.l d0,d1 ; é";
    let tokens = vec![
        token(TokenKind::Whitespace, 0, 1),
        token(TokenKind::Mnemonic, 1, 5),
        token(TokenKind::SizeSuffix, 5, 7),
        token(TokenKind::Whitespace, 7, 8),
        token(TokenKind::Register, 8, 10),
        token(TokenKind::Punctuation, 10, 11),
        token(TokenKind::Register, 11, 13),
        token(TokenKind::Whitespace, 13, 14),
        token(TokenKind::Comment, 14, 18),
    ];
    assert_eq!(check_line_tokens(line, &tokens), Ok(()));
    assert_eq!(
        check_line_tokens(line, &[token(TokenKind::Comment, 0, 18)]),
        Ok(())
    );
}

#[test]
fn an_empty_line_has_no_token() {
    assert_eq!(check_line_tokens("", &[]), Ok(()));
    assert_eq!(
        check_line_tokens("", &[token(TokenKind::Whitespace, 0, 0)]),
        Err(TokenInvariantError::Empty { index: 0 })
    );
    assert_eq!(
        check_line_tokens("", &[token(TokenKind::Whitespace, 0, 1)]),
        Err(TokenInvariantError::Long { end: 1 })
    );
}

#[test]
fn a_gap_is_reported_where_it_starts() {
    assert_eq!(
        check_line_tokens("abc", &symbols(&[(1, 3)])),
        Err(TokenInvariantError::Gap { at: 0 })
    );
    assert_eq!(
        check_line_tokens("abcd", &symbols(&[(0, 1), (2, 4)])),
        Err(TokenInvariantError::Gap { at: 1 })
    );
}

#[test]
fn an_overlap_is_reported_at_the_overlapping_start() {
    assert_eq!(
        check_line_tokens("abcd", &symbols(&[(0, 3), (2, 4)])),
        Err(TokenInvariantError::Overlap { at: 2 })
    );
    // Out of order is an overlap too.
    assert_eq!(
        check_line_tokens("abcd", &symbols(&[(0, 2), (2, 4), (0, 2)])),
        Err(TokenInvariantError::Overlap { at: 0 })
    );
}

#[test]
fn an_empty_token_is_reported_by_index() {
    assert_eq!(
        check_line_tokens("abc", &symbols(&[(0, 1), (1, 1), (1, 3)])),
        Err(TokenInvariantError::Empty { index: 1 })
    );
    let reversed = Token {
        kind: TokenKind::Symbol,
        range: TextRange { start: 0, end: 0 },
    };
    assert_eq!(
        check_line_tokens("a", &[reversed]),
        Err(TokenInvariantError::Empty { index: 0 })
    );
}

#[test]
fn a_boundary_inside_a_char_is_reported() {
    // "é" is 2 bytes: 0xC3 0xA9.
    assert_eq!(
        check_line_tokens("aéb", &symbols(&[(0, 2), (2, 4)])),
        Err(TokenInvariantError::NotCharBoundary { at: 2 })
    );
    assert_eq!(
        check_line_tokens("aéb", &symbols(&[(0, 3), (3, 4)])),
        Ok(())
    );
}

#[test]
fn tokens_that_stop_early_are_short() {
    assert_eq!(
        check_line_tokens("abc", &[]),
        Err(TokenInvariantError::Short { end: 0 })
    );
    assert_eq!(
        check_line_tokens("abc", &symbols(&[(0, 1), (1, 2)])),
        Err(TokenInvariantError::Short { end: 2 })
    );
}

#[test]
fn tokens_past_the_end_are_long() {
    assert_eq!(
        check_line_tokens("abc", &symbols(&[(0, 1), (1, 4)])),
        Err(TokenInvariantError::Long { end: 4 })
    );
    assert_eq!(
        check_line_tokens("abc", &symbols(&[(0, 3), (3, 5)])),
        Err(TokenInvariantError::Long { end: 5 })
    );
}

// ---------------------------------------------------------------------------------------------
// token_diagnostics
// ---------------------------------------------------------------------------------------------

/// Runs `token_diagnostics` for file 7 with a line starting at file offset `line_start`.
fn diagnostics_of(line_start: u32, line: &str, tokens: &[Token]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    token_diagnostics(FileId(7), line_start, line, tokens, &mut out);
    out
}

/// Returns the `text` argument of `diagnostic`.
fn text_of(diagnostic: &Diagnostic) -> &str {
    match diagnostic.args.as_slice() {
        [("text", DiagArg::Str(text))] => text,
        other => panic!("unexpected args {other:?}"),
    }
}

#[test]
fn invalid_tokens_are_blocking_errors_with_file_offsets() {
    let line = " dc.b $";
    let tokens = [
        token(TokenKind::Whitespace, 0, 1),
        token(TokenKind::Directive, 1, 3),
        token(TokenKind::SizeSuffix, 3, 5),
        token(TokenKind::Whitespace, 5, 6),
        token(TokenKind::Invalid(BAD), 6, 7),
    ];
    let diagnostics = diagnostics_of(100, line, &tokens);
    assert_eq!(
        diagnostics,
        vec![
            Diagnostic::new(
                BAD,
                Severity::Error,
                SourceSpan {
                    file: FileId(7),
                    range: TextRange::new(106, 107),
                },
            )
            .with_arg("text", DiagArg::Str("$".to_owned()))
        ]
    );
    assert!(diagnostics[0].blocking);
}

#[test]
fn suspect_comments_are_non_blocking_errors() {
    let line = "dc.b 1, 2, 3";
    let tokens = [
        token(TokenKind::Directive, 0, 2),
        token(TokenKind::SizeSuffix, 2, 4),
        token(TokenKind::Whitespace, 4, 5),
        token(TokenKind::Number, 5, 6),
        token(TokenKind::Punctuation, 6, 7),
        token(TokenKind::Whitespace, 7, 8),
        token(TokenKind::SuspectComment(SUSPECT), 8, 12),
    ];
    let diagnostics = diagnostics_of(0, line, &tokens);
    assert_eq!(diagnostics.len(), 1);
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.code, SUSPECT);
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(!diagnostic.blocking);
    assert_eq!(diagnostic.span.range, TextRange::new(8, 12));
    assert_eq!(text_of(diagnostic), "2, 3");
    assert!(diagnostic.related.is_empty());
    assert_eq!(diagnostic.suggestion, None);
}

#[test]
fn other_kinds_produce_nothing_and_order_is_kept() {
    let line = "abcdefghijklmnopq";
    let mut tokens: Vec<Token> = ALL_KINDS
        .iter()
        .zip(0..)
        .map(|(&kind, start)| token(kind, start, start + 1))
        .collect();
    tokens.push(token(TokenKind::Invalid(DiagCode("lex-non-ascii")), 16, 17));
    let diagnostics = diagnostics_of(0, line, &tokens);
    let codes: Vec<&str> = diagnostics.iter().map(|d| d.code.0).collect();
    assert_eq!(
        codes,
        vec!["lex-operands-in-comment", "lex-bad-number", "lex-non-ascii"]
    );
    assert_eq!(text_of(&diagnostics[0]), "c");
}

#[test]
fn diagnostics_are_appended() {
    let mut out = vec![Diagnostic::new(
        DiagCode("existing"),
        Severity::Note,
        SourceSpan {
            file: FileId(0),
            range: TextRange::default(),
        },
    )];
    token_diagnostics(
        FileId(1),
        0,
        "$",
        &[token(TokenKind::Invalid(BAD), 0, 1)],
        &mut out,
    );
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].code, DiagCode("existing"));
    assert_eq!(out[1].code, BAD);
}

#[test]
fn the_text_argument_is_truncated_to_32_chars() {
    let exactly = "é".repeat(32);
    let longer = format!("{exactly}x");
    for (text, expected) in [
        (exactly.clone(), exactly.clone()),
        (longer, format!("{exactly}…")),
        ("a".repeat(100), format!("{}…", "a".repeat(32))),
        (String::new(), String::new()),
    ] {
        let end = u32::try_from(text.len()).unwrap();
        let tokens: Vec<Token> = if text.is_empty() {
            Vec::new()
        } else {
            vec![token(TokenKind::Invalid(BAD), 0, end)]
        };
        let diagnostics = diagnostics_of(0, &text, &tokens);
        if text.is_empty() {
            assert!(diagnostics.is_empty());
        } else {
            assert_eq!(text_of(&diagnostics[0]), expected);
        }
    }
}

#[test]
fn a_token_outside_the_line_gives_an_empty_text_instead_of_a_panic() {
    let diagnostics = diagnostics_of(0, "ab", &[token(TokenKind::Invalid(BAD), 1, 9)]);
    assert_eq!(text_of(&diagnostics[0]), "");
}

// ---------------------------------------------------------------------------------------------
// SyntaxProvider default method
// ---------------------------------------------------------------------------------------------

/// A provider that makes every non-empty line one token: `Invalid` if it contains `$`, else a
/// `Comment`.
struct WholeLine;

/// Keywords of [`WholeLine`].
static WHOLE_LINE_KEYWORDS: [KeywordInfo; 1] = [KeywordInfo {
    name: "NOP",
    kind: KeywordKind::Mnemonic,
    doc_id: Some("isa-test-nop"),
}];

impl SyntaxProvider for WholeLine {
    fn lex_line(&self, line: &str, state: LineState, out: &mut Vec<Token>) -> LineState {
        if !line.is_empty() {
            let kind = if line.contains('$') {
                TokenKind::Invalid(BAD)
            } else {
                TokenKind::Comment
            };
            out.push(token(kind, 0, u32::try_from(line.len()).unwrap()));
        }
        state
    }

    fn keywords(&self) -> &[KeywordInfo] {
        &WHOLE_LINE_KEYWORDS
    }
}

#[test]
fn the_default_line_diagnostics_is_token_diagnostics() {
    let provider: &dyn SyntaxProvider = &WholeLine;
    let line = "move #$";
    let mut tokens = Vec::new();
    assert_eq!(
        provider.lex_line(line, LineState::INITIAL, &mut tokens),
        LineState::INITIAL
    );
    assert_eq!(check_line_tokens(line, &tokens), Ok(()));

    let mut through_trait = Vec::new();
    provider.line_diagnostics(FileId(2), 40, line, &tokens, &mut through_trait);
    let mut direct = Vec::new();
    token_diagnostics(FileId(2), 40, line, &tokens, &mut direct);
    assert_eq!(through_trait, direct);
    assert_eq!(through_trait.len(), 1);
    assert_eq!(through_trait[0].span.range, TextRange::new(40, 47));
    assert_eq!(provider.keywords()[0].name, "NOP");
}

#[test]
fn lex_line_appends_to_out() {
    let mut tokens = vec![token(TokenKind::Comment, 0, 1)];
    WholeLine.lex_line("x", LineState::INITIAL, &mut tokens);
    WholeLine.lex_line("", LineState::INITIAL, &mut tokens);
    assert_eq!(tokens.len(), 2);
}
