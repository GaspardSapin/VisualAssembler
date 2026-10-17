//! The A68k-dialect lexer shared by the highlighter (M4) and the assembler (M2)
//! (m1-core-isa-lexer.md §5).
//!
//! [`lex_line`] is the single implementation; [`M68kSyntax`] exposes it as a
//! `va_core::syntax::SyntaxProvider` and M2's parser calls it directly. A line is
//! `[label] [ws operation [ws operands]] [ws comment]` (§5.1): the label is in column 1, the
//! operand field ends at the first unquoted whitespace and the rest of the line is a comment,
//! exactly as a68k 2.71.F3w reads it. When that comment looks like the continuation of the
//! operands (`dc.b 1, 2, 3`), it is a `SuspectComment` with the non-blocking error
//! [`LEX_OPERANDS_IN_COMMENT`] and quick-fix data (§5.7, user decision).
//!
//! Files: `lexer.rs` (fields and token rules), `literals.rs` (character classes, numbers,
//! strings, backslashes), `keywords.rs` (directives, registers, size suffixes, keyword list),
//! `codes.rs` ([`DIAG_CODES`]) and `spacing.rs` (the §5.7 rules and quick fix).

mod codes;
mod keywords;
mod lexer;
mod literals;
mod spacing;

pub use codes::{
    DIAG_CODES, LEX_BAD_BACKSLASH, LEX_BAD_LABEL, LEX_BAD_NUMBER, LEX_BAD_OPERATION,
    LEX_BAD_SIZE_SUFFIX, LEX_LABEL_JUNK, LEX_LINE_TOO_LONG, LEX_NON_ASCII, LEX_OPERANDS_IN_COMMENT,
    LEX_UNEXPECTED_CHAR, LEX_UNTERMINATED_STRING,
};

use va_core::diag::Diagnostic;
use va_core::source::FileId;
use va_core::syntax::{
    KeywordInfo, LineState, SyntaxProvider, Token, TokenKind, token_diagnostics,
};

/// The A68k-dialect lexer, seen through `va_core::syntax::SyntaxProvider`. It always returns
/// `LineState::INITIAL` (the dialect is line-oriented).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct M68kSyntax;

/// Lexes one line (content without terminator) and appends its tokens to `out`: the single
/// lexer implementation, which M2's parser calls directly.
///
/// The tokens cover `0..line.len()` exactly, without gap or overlap, on char boundaries; an
/// empty line appends nothing; a line longer than `va_core::source::MAX_LINE_BYTES` is one
/// `Invalid(lex-line-too-long)` token. The call never panics, runs in linear time, and
/// allocates nothing besides growing `out`.
pub fn lex_line(line: &str, out: &mut Vec<Token>) {
    lexer::lex_line(line, out);
}

impl SyntaxProvider for M68kSyntax {
    fn lex_line(&self, line: &str, _state: LineState, out: &mut Vec<Token>) -> LineState {
        lexer::lex_line(line, out);
        LineState::INITIAL
    }

    /// The 126 mnemonics (with their doc id), the 49 directives, the 21 registers and the 4
    /// size suffixes (§5.3).
    fn keywords(&self) -> &[KeywordInfo] {
        keywords::keywords()
    }

    /// The default token diagnostics (`va_core::syntax::token_diagnostics`), with each
    /// `lex-operands-in-comment` diagnostic completed by its `cause` argument and, for rules R1
    /// and R2, its quick-fix suggestion (§5.7).
    fn line_diagnostics(
        &self,
        file: FileId,
        line_start: u32,
        line: &str,
        tokens: &[Token],
        out: &mut Vec<Diagnostic>,
    ) {
        let first = out.len();
        token_diagnostics(file, line_start, line, tokens, out);
        let mut produced = first;
        for (index, token) in tokens.iter().enumerate() {
            match token.kind {
                TokenKind::Invalid(_) => produced += 1,
                TokenKind::SuspectComment(code) => {
                    if code == LEX_OPERANDS_IN_COMMENT
                        && let Some(diagnostic) = out.get_mut(produced)
                    {
                        spacing::complete_diagnostic(
                            diagnostic, file, line_start, line, tokens, index,
                        );
                    }
                    produced += 1;
                }
                _ => {}
            }
        }
    }
}
