//! Tests for `src/syntax/` (m1-core-isa-lexer.md §5, §9.1, exit criterion E3): token kinds and
//! codes (`kinds`), field and token rules with the §5.7 table (`fields`), coverage and
//! statelessness properties (`coverage`), pathological and byte-level input (`robustness`) and
//! the golden fixtures of `fixtures/lex/` (`fixtures`). This file holds the shared helpers.
#![allow(clippy::unwrap_used)]

mod coverage;
mod fields;
mod fixtures;
mod kinds;
mod robustness;

use std::path::PathBuf;

use va_arch_m68k::syntax::{self, M68kSyntax};
use va_core::diag::{DiagCode, Diagnostic};
use va_core::source::FileId;
use va_core::syntax::{LineState, SyntaxProvider, Token, TokenKind, check_line_tokens};

/// Short name of `TokenKind::Whitespace`.
pub const WS: TokenKind = TokenKind::Whitespace;
/// Short name of `TokenKind::Comment`.
pub const COMMENT: TokenKind = TokenKind::Comment;
/// Short name of `TokenKind::SuspectComment(lex-operands-in-comment)`.
pub const SUSPECT: TokenKind = TokenKind::SuspectComment(syntax::LEX_OPERANDS_IN_COMMENT);
/// Short name of `TokenKind::Label`.
pub const LABEL: TokenKind = TokenKind::Label;
/// Short name of `TokenKind::LocalLabel`.
pub const LOCAL: TokenKind = TokenKind::LocalLabel;
/// Short name of `TokenKind::Mnemonic`.
pub const MNEMONIC: TokenKind = TokenKind::Mnemonic;
/// Short name of `TokenKind::Directive`.
pub const DIRECTIVE: TokenKind = TokenKind::Directive;
/// Short name of `TokenKind::MacroName`.
pub const MACRO: TokenKind = TokenKind::MacroName;
/// Short name of `TokenKind::SizeSuffix`.
pub const SIZE: TokenKind = TokenKind::SizeSuffix;
/// Short name of `TokenKind::Register`.
pub const REG: TokenKind = TokenKind::Register;
/// Short name of `TokenKind::Symbol`.
pub const SYM: TokenKind = TokenKind::Symbol;
/// Short name of `TokenKind::Number`.
pub const NUM: TokenKind = TokenKind::Number;
/// Short name of `TokenKind::String`.
pub const STR: TokenKind = TokenKind::String;
/// Short name of `TokenKind::Operator`.
pub const OP: TokenKind = TokenKind::Operator;
/// Short name of `TokenKind::Punctuation`.
pub const PUNCT: TokenKind = TokenKind::Punctuation;
/// Short name of `TokenKind::MacroParam`.
pub const PARAM: TokenKind = TokenKind::MacroParam;

/// `TokenKind::Invalid` with `code`.
pub const fn invalid(code: DiagCode) -> TokenKind {
    TokenKind::Invalid(code)
}

/// The six golden fixtures of m1 §5.5.
pub const FIXTURES: [&str; 6] = [
    "basic", "labels", "literals", "comments", "macros", "errors",
];

/// Lexes `line` with `syntax::lex_line`, checks the coverage contract and that the
/// `SyntaxProvider` path gives the same tokens and `LineState::INITIAL`, and returns the tokens.
pub fn lex(line: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    syntax::lex_line(line, &mut tokens);
    if let Err(error) = check_line_tokens(line, &tokens) {
        panic!("{line:?}: coverage contract broken: {error:?} in {tokens:?}");
    }
    let mut through_trait = vec![Token {
        kind: TokenKind::Comment,
        range: va_core::source::TextRange::new(0, 0),
    }];
    let state = M68kSyntax.lex_line(line, LineState(7), &mut through_trait);
    assert_eq!(state, LineState::INITIAL, "{line:?}");
    assert_eq!(
        &through_trait[1..],
        &tokens[..],
        "{line:?}: tokens were not appended"
    );
    tokens
}

/// Lexes `line` and returns each token as (kind, text).
pub fn pieces(line: &str) -> Vec<(TokenKind, &str)> {
    lex(line)
        .into_iter()
        .map(|token| (token.kind, token.range.slice(line).unwrap()))
        .collect()
}

/// Lexes `line` and returns the kinds of its tokens.
pub fn kinds_of(line: &str) -> Vec<TokenKind> {
    lex(line).into_iter().map(|token| token.kind).collect()
}

/// Lexes `line` and returns its diagnostics through `M68kSyntax::line_diagnostics`, with the
/// line at file offset `line_start` of file 3.
pub fn diagnostics(line: &str, line_start: u32) -> Vec<Diagnostic> {
    let tokens = lex(line);
    let mut out = Vec::new();
    M68kSyntax.line_diagnostics(FileId(3), line_start, line, &tokens, &mut out);
    out
}

/// Returns the folder of the lexer fixtures, `arch/m68k/fixtures/lex/`.
pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("lex")
}

/// Reads the source of the fixture `name` (`<name>.asm`).
pub fn fixture_source(name: &str) -> String {
    let path = fixture_dir().join(format!("{name}.asm"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Returns `text` mixing ASCII upper and lower case (`MoVe`), for case-insensitivity checks.
pub fn mixed_case(text: &str) -> String {
    text.chars()
        .enumerate()
        .map(|(index, char)| {
            if index % 2 == 0 {
                char.to_ascii_uppercase()
            } else {
                char.to_ascii_lowercase()
            }
        })
        .collect()
}
