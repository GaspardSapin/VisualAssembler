//! The lexer contract shared by the highlighter (M4) and the assembler (M2)
//! (m1-core-isa-lexer.md §3.4).
//!
//! **Contract of every [`SyntaxProvider`]**: the tokens [`SyntaxProvider::lex_line`] appends are
//! non-empty, contiguous, in order, start and end on char boundaries, and cover exactly
//! `0..line.len()`; an empty line appends nothing. `lex_line` never panics, runs in O(len), is
//! deterministic, and allocates nothing besides growing `out`. [`check_line_tokens`] checks the
//! coverage part for any CPU.

use crate::diag::{DiagArg, DiagCode, Diagnostic, Severity};
use crate::source::{FileId, SourceSpan, TextRange};

/// Opaque lexer state carried from one line to the next. Line-oriented dialects (A68k) always
/// return [`LineState::INITIAL`]; a dialect with block comments would use it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct LineState(pub u32);

impl LineState {
    /// The state at the start of a file.
    pub const INITIAL: Self = Self(0);
}

/// What a token is. Exhaustive and final for M1–M6: the highlighter matches every kind.
///
/// [`TokenKind::Invalid`] and [`TokenKind::SuspectComment`] carry their diagnostic code, so the
/// highlighter, the assembler and `va lex` report the same problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// A run of spaces or tabs.
    Whitespace,
    /// A comment, to the end of the line.
    Comment,
    /// Text the dialect treats as a comment although it looks like operands (m1 §5.7); shown as
    /// a comment with an error underline.
    SuspectComment(DiagCode),
    /// A label definition or reference name.
    Label,
    /// A local label (`1$`, `\name` in the A68k dialect).
    LocalLabel,
    /// An instruction mnemonic.
    Mnemonic,
    /// An assembler directive.
    Directive,
    /// An unknown word in the operation field, taken as a macro call.
    MacroName,
    /// An operation or operand size suffix, dot included (`.W`).
    SizeSuffix,
    /// A register name.
    Register,
    /// Any other identifier in the operands.
    Symbol,
    /// A numeric literal.
    Number,
    /// A quoted literal, or an include path.
    String,
    /// An expression operator.
    Operator,
    /// Punctuation: `#`, `,`, `(`, `)`, a label colon.
    Punctuation,
    /// A macro parameter reference (`\1`, `\@`).
    MacroParam,
    /// Text the lexer rejects, with the reason.
    Invalid(DiagCode),
}

impl TokenKind {
    /// Returns the kebab-case identifier of the kind, e.g. `"local-label"` or `"invalid"` (the
    /// code of `Invalid` and `SuspectComment` is not part of it).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Whitespace => "whitespace",
            Self::Comment => "comment",
            Self::SuspectComment(_) => "suspect-comment",
            Self::Label => "label",
            Self::LocalLabel => "local-label",
            Self::Mnemonic => "mnemonic",
            Self::Directive => "directive",
            Self::MacroName => "macro-name",
            Self::SizeSuffix => "size-suffix",
            Self::Register => "register",
            Self::Symbol => "symbol",
            Self::Number => "number",
            Self::String => "string",
            Self::Operator => "operator",
            Self::Punctuation => "punctuation",
            Self::MacroParam => "macro-param",
            Self::Invalid(_) => "invalid",
        }
    }
}

/// One token of a line: its kind and its line-relative byte range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    /// What the token is.
    pub kind: TokenKind,
    /// Line-relative byte range of the token.
    pub range: TextRange,
}

/// The category of a keyword returned by [`SyntaxProvider::keywords`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeywordKind {
    /// An instruction mnemonic.
    Mnemonic,
    /// An assembler directive.
    Directive,
    /// A register name.
    Register,
    /// A size suffix.
    SizeSuffix,
}

/// A keyword of a dialect, for completion and hover.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeywordInfo {
    /// The keyword, uppercase (a size suffix includes its dot).
    pub name: &'static str,
    /// Its category.
    pub kind: KeywordKind,
    /// Fluent id of its documentation, if it has one.
    pub doc_id: Option<&'static str>,
}

/// The lexer of one CPU dialect.
pub trait SyntaxProvider: Send + Sync {
    /// Lexes one line (content without terminator) and APPENDS its tokens to `out`.
    fn lex_line(&self, line: &str, state: LineState, out: &mut Vec<Token>) -> LineState;

    /// Mnemonics, directives, registers and size suffixes (uppercase names), for completion and
    /// hover.
    fn keywords(&self) -> &[KeywordInfo];

    /// Diagnostics of one lexed line (file offsets). Default: [`token_diagnostics`].
    ///
    /// `line_start` is the file offset of the line, `tokens` the tokens `lex_line` produced for
    /// `line`; diagnostics are appended to `out`.
    fn line_diagnostics(
        &self,
        file: FileId,
        line_start: u32,
        line: &str,
        tokens: &[Token],
        out: &mut Vec<Diagnostic>,
    ) {
        token_diagnostics(file, line_start, line, tokens, out);
    }
}

/// The first way a token list breaks the coverage contract, found by [`check_line_tokens`].
/// Offsets are line-relative bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenInvariantError {
    /// No token covers the bytes starting at `at` (a token starts after the previous end).
    Gap {
        /// First uncovered byte.
        at: u32,
    },
    /// A token starts at `at`, before the end of the previous token.
    Overlap {
        /// Start of the overlapping token.
        at: u32,
    },
    /// The token at `index` holds no byte.
    Empty {
        /// Position of the token in the list.
        index: usize,
    },
    /// A token ends at `at`, inside a multi-byte char.
    NotCharBoundary {
        /// The offending offset.
        at: u32,
    },
    /// The tokens stop at `end`, before the end of the line.
    Short {
        /// End of the last token (0 when there is none).
        end: u32,
    },
    /// A token ends at `end`, past the end of the line.
    Long {
        /// End of that token.
        end: u32,
    },
}

/// Checks that `tokens` are non-empty, contiguous, in order, on char boundaries, and cover
/// exactly `0..line.len()` (no token for an empty line). Returns the first violation.
pub fn check_line_tokens(line: &str, tokens: &[Token]) -> Result<(), TokenInvariantError> {
    let mut position: u32 = 0;
    for (index, token) in tokens.iter().enumerate() {
        let TextRange { start, end } = token.range;
        if start > position {
            return Err(TokenInvariantError::Gap { at: position });
        }
        if start < position {
            return Err(TokenInvariantError::Overlap { at: start });
        }
        if end <= start {
            return Err(TokenInvariantError::Empty { index });
        }
        if end as usize > line.len() {
            return Err(TokenInvariantError::Long { end });
        }
        if !line.is_char_boundary(end as usize) {
            return Err(TokenInvariantError::NotCharBoundary { at: end });
        }
        position = end;
    }
    if (position as usize) < line.len() {
        return Err(TokenInvariantError::Short { end: position });
    }
    Ok(())
}

/// Longest `text` argument of a token diagnostic, in chars, before `…` is appended.
const TEXT_ARG_CHARS: usize = 32;

/// Returns `text` limited to [`TEXT_ARG_CHARS`] chars; a longer text is cut and gets `…`.
fn text_arg(text: &str) -> String {
    match text.char_indices().nth(TEXT_ARG_CHARS) {
        Some((cut, _)) => {
            let mut truncated = String::with_capacity(cut + '…'.len_utf8());
            truncated.push_str(text.get(..cut).unwrap_or(text));
            truncated.push('…');
            truncated
        }
        None => text.to_owned(),
    }
}

/// Appends one diagnostic per [`TokenKind::Invalid`] token (a blocking error) and per
/// [`TokenKind::SuspectComment`] token (a non-blocking error), in token order.
///
/// Each has the token's code, its span shifted by `line_start` into file offsets, and the Fluent
/// argument `text`: the token text, truncated to 32 chars plus `…`. Other tokens produce
/// nothing.
pub fn token_diagnostics(
    file: FileId,
    line_start: u32,
    line: &str,
    tokens: &[Token],
    out: &mut Vec<Diagnostic>,
) {
    for token in tokens {
        let (code, blocking) = match token.kind {
            TokenKind::Invalid(code) => (code, true),
            TokenKind::SuspectComment(code) => (code, false),
            _ => continue,
        };
        let span = SourceSpan {
            file,
            range: token.range.shifted(line_start),
        };
        let text = text_arg(token.range.slice(line).unwrap_or(""));
        let diagnostic =
            Diagnostic::new(code, Severity::Error, span).with_arg("text", DiagArg::Str(text));
        out.push(if blocking {
            diagnostic
        } else {
            diagnostic.non_blocking()
        });
    }
}
