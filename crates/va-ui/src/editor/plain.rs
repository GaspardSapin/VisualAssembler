//! The lexer of a build without any CPU (m4-editor-shell.md §5.2, §6.7): whitespace runs and one
//! `Symbol` token per other run, so the editor still shows and edits text.

use va_core::source::TextRange;
use va_core::syntax::{KeywordInfo, LineState, SyntaxProvider, Token, TokenKind};

/// The lexer used when the build has no architecture: whitespace runs, and one `Symbol` token
/// per other run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PlainSyntax;

/// Tells whether `c` belongs to a `Whitespace` run (spaces and tabs, as in `va-core`).
fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// Appends the token `start..end` of `kind`; lines are at most 16 MiB, so offsets fit `u32`.
fn push(out: &mut Vec<Token>, kind: TokenKind, start: usize, end: usize) {
    let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) else {
        return;
    };
    out.push(Token {
        kind,
        range: TextRange::new(start, end),
    });
}

impl SyntaxProvider for PlainSyntax {
    fn lex_line(&self, line: &str, state: LineState, out: &mut Vec<Token>) -> LineState {
        let mut run: Option<(usize, bool)> = None;
        for (index, c) in line.char_indices() {
            let blank = is_blank(c);
            match run {
                Some((_, run_blank)) if run_blank == blank => {}
                Some((start, run_blank)) => {
                    push(out, kind_of(run_blank), start, index);
                    run = Some((index, blank));
                }
                None => run = Some((index, blank)),
            }
        }
        if let Some((start, run_blank)) = run {
            push(out, kind_of(run_blank), start, line.len());
        }
        state
    }

    fn keywords(&self) -> &[KeywordInfo] {
        &[]
    }
}

/// The token kind of a run.
fn kind_of(blank: bool) -> TokenKind {
    if blank {
        TokenKind::Whitespace
    } else {
        TokenKind::Symbol
    }
}
