//! The A68k-dialect lexer shared by the highlighter (M4) and the assembler (M2)
//! (m1-core-isa-lexer.md §5).
//!
//! **WP0 stub (m1 §1), owned by WP-C in wave 2.** It only provides [`lex_line`] and
//! [`M68kSyntax`] implementing `va_core::syntax::SyntaxProvider`, with the frozen signatures
//! and a placeholder behaviour that keeps the coverage contract of m1 §3.4: a non-empty line is
//! one `Comment` token, an empty line gives none, and there is no keyword. WP-C replaces it with
//! the lexer of §5 (`lexer.rs`, `literals.rs`, `keywords.rs`, `codes.rs` with `DIAG_CODES`,
//! `spacing.rs`).

use va_core::source::TextRange;
use va_core::syntax::{KeywordInfo, LineState, SyntaxProvider, Token, TokenKind};

/// The A68k-dialect lexer, seen through `va_core::syntax::SyntaxProvider`. It always returns
/// `LineState::INITIAL` (the dialect is line-oriented).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct M68kSyntax;

/// Lexes one line (content without terminator) and appends its tokens to `out`; the single
/// lexer implementation, which M2's parser calls directly.
///
/// Stub: a non-empty line is one `Comment` token over the whole line.
pub fn lex_line(line: &str, out: &mut Vec<Token>) {
    if !line.is_empty() {
        let end = u32::try_from(line.len()).unwrap_or(u32::MAX);
        out.push(Token {
            kind: TokenKind::Comment,
            range: TextRange::new(0, end),
        });
    }
}

impl SyntaxProvider for M68kSyntax {
    fn lex_line(&self, line: &str, _state: LineState, out: &mut Vec<Token>) -> LineState {
        crate::syntax::lex_line(line, out);
        LineState::INITIAL
    }

    /// Stub: no keyword until WP-C lands the 126 mnemonics, 49 directives, 21 registers and 4
    /// size suffixes (m1 §5.3).
    fn keywords(&self) -> &[KeywordInfo] {
        &[]
    }
}
