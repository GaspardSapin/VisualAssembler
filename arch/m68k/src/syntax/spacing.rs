//! Operand-like text after a space (m1-core-isa-lexer.md §5.7, user decision of 2026-10-17).
//!
//! The dialect stays strict A68k: the operand field ends at the first unquoted whitespace and
//! the rest of the line is a comment. This module only decides whether that comment text T
//! looks like the continuation of the operands, in which case the lexer emits it as a
//! `SuspectComment(lex-operands-in-comment)` token instead of a `Comment`:
//!
//! - **R1 dangling operand**: the last operand token is `,`, `#`, `(` or an `Operator`, or the
//!   operands open more `(` than they close;
//! - **R2 leading joiner**: T starts with `,` or with `+ / & ! | << >>` (not `-` or `*`, which
//!   often start free text);
//! - **R3 operand list**: the first word of T, lexed with the operand rules, has no `Invalid`
//!   token, holds a `,` outside parentheses and starts like an operand (`Register`, `Number`,
//!   `LocalLabel`, `String`, `#`, `(` or `-(`).
//!
//! It also builds the data of the non-blocking diagnostic: the `cause` argument and, for R1 and
//! R2, the quick-fix [`Suggestion`] that removes the spaces touching `,`, `#`, `(`, `)` or an
//! operator, offered only when it changes the text. In the trailing text, a quote opens a
//! string only when the line closes it, both for the `;` search and for the fix.

use va_core::diag::{DiagArg, Diagnostic, Suggestion};
use va_core::source::{FileId, SourceSpan, TextRange};
use va_core::syntax::{Token, TokenKind};

use super::literals::{is_ws, skip_ws};

/// Why operand-like text was flagged, the `cause` argument of `lex-operands-in-comment`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Cause {
    /// The text continues a list after a comma (R1 or R2 on `,`).
    AfterComma,
    /// The text continues an expression or an addressing mode (R1 or R2, other tokens).
    InsideExpression,
    /// The first word of the text is itself an operand list (R3); no fix is offered.
    OperandList,
}

impl Cause {
    /// Returns the Fluent selector value of the cause.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::AfterComma => "after-comma",
            Self::InsideExpression => "inside-expression",
            Self::OperandList => "operand-list",
        }
    }
}

/// Returns the first byte of `token`, or 0 when its range is out of `bytes`.
fn first_byte(bytes: &[u8], token: &Token) -> u8 {
    bytes.get(token.range.start as usize).copied().unwrap_or(0)
}

/// Tells whether `token` is the punctuation char `wanted`.
fn is_punct(bytes: &[u8], token: &Token, wanted: u8) -> bool {
    token.kind == TokenKind::Punctuation && first_byte(bytes, token) == wanted
}

/// Applies R1 to the operand-field tokens `operands`, then R2 to the text starting at `text`;
/// returns the cause of the first rule that fires.
pub(crate) fn dangling_or_joiner(bytes: &[u8], operands: &[Token], text: usize) -> Option<Cause> {
    let last = operands.last()?;
    if is_punct(bytes, last, b',') {
        return Some(Cause::AfterComma);
    }
    if is_punct(bytes, last, b'#')
        || is_punct(bytes, last, b'(')
        || last.kind == TokenKind::Operator
    {
        return Some(Cause::InsideExpression);
    }
    let mut open: i64 = 0;
    for token in operands {
        if is_punct(bytes, token, b'(') {
            open += 1;
        } else if is_punct(bytes, token, b')') {
            open -= 1;
        }
    }
    if open > 0 {
        return Some(Cause::InsideExpression);
    }
    match bytes.get(text).copied() {
        Some(b',') => Some(Cause::AfterComma),
        Some(b'+' | b'/' | b'&' | b'!' | b'|') => Some(Cause::InsideExpression),
        Some(angle @ (b'<' | b'>')) if bytes.get(text + 1).copied() == Some(angle) => {
            Some(Cause::InsideExpression)
        }
        _ => None,
    }
}

/// Applies R3 to `word`, the tokens of the first word of the text lexed with the operand rules.
pub(crate) fn is_operand_list(bytes: &[u8], word: &[Token]) -> bool {
    let Some(first) = word.first() else {
        return false;
    };
    if word
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Invalid(_)))
    {
        return false;
    }
    let starts_like_operand = match first.kind {
        TokenKind::Register | TokenKind::Number | TokenKind::LocalLabel | TokenKind::String => true,
        TokenKind::Punctuation => matches!(first_byte(bytes, first), b'#' | b'('),
        TokenKind::Operator => {
            first_byte(bytes, first) == b'-'
                && word.get(1).is_some_and(|next| is_punct(bytes, next, b'('))
        }
        _ => false,
    };
    if !starts_like_operand {
        return false;
    }
    let mut depth: i64 = 0;
    for token in word {
        if is_punct(bytes, token, b'(') {
            depth += 1;
        } else if is_punct(bytes, token, b')') {
            depth -= 1;
        } else if depth <= 0 && is_punct(bytes, token, b',') {
            return true;
        }
    }
    false
}

/// Returns the offset of the delimiter that closes the quote at `at` (the next byte equal to
/// `bytes[at]` before `end`), or `None` when the line never closes it.
///
/// In the trailing text, a quote opens a string only when it is closed later on the line
/// (m1 §12.8 N-7), so the apostrophe of free text (`don't`) does not hide a later `;` comment.
/// Scans stay linear: a failed search for a delimiter proves that no later byte is that
/// delimiter, so it is never repeated for the same text.
fn closing_quote(bytes: &[u8], at: usize, end: usize) -> Option<usize> {
    let quote = *bytes.get(at)?;
    let after = at + 1;
    bytes
        .get(after..end)?
        .iter()
        .position(|&byte| byte == quote)
        .map(|offset| after + offset)
}

/// Returns the offset of the first `;` outside quotes in `from..end`, or `end`. Only a closed
/// quote counts (see [`closing_quote`]); a doubled delimiter closes a literal and reopens it,
/// which keeps the state right.
pub(crate) fn unquoted_semicolon(bytes: &[u8], from: usize, end: usize) -> usize {
    let end = end.min(bytes.len());
    let mut at = from;
    while at < end {
        match bytes[at] {
            b';' => return at,
            b'\'' | b'"' => {
                if let Some(closing) = closing_quote(bytes, at, end) {
                    at = closing;
                }
            }
            _ => {}
        }
        at += 1;
    }
    end
}

/// Tells whether a whitespace run next to `byte` is removed by the quick fix: `byte` is `,`,
/// `#`, `(`, `)` or an operator char (`<` and `>` stand for `<<` and `>>`). `#` is in the set
/// so that the fix of a dangling `#` (rule R1) is not a no-op (m1 §12.1 R12-1).
fn joins(byte: Option<u8>) -> bool {
    matches!(
        byte,
        Some(
            b',' | b'#'
                | b'('
                | b')'
                | b'+'
                | b'-'
                | b'*'
                | b'/'
                | b'~'
                | b'&'
                | b'!'
                | b'|'
                | b'<'
                | b'>'
        )
    )
}

/// Returns the quick-fix replacement of `line[from..to]` (from the whitespace that separates the
/// operands from the text to the end of the suspect token): the same text with every unquoted
/// whitespace run removed when the char just before or after it joins (see [`joins`]). The
/// char before `from` (the last operand char) counts for the first run. Quotes follow the same
/// rule as [`unquoted_semicolon`]: only a quote closed before `to` protects its whitespace.
pub(crate) fn fix_replacement(line: &str, from: usize, to: usize) -> String {
    let bytes = line.as_bytes();
    let to = to.min(bytes.len());
    let mut replacement = String::with_capacity(to.saturating_sub(from));
    let mut copied = from;
    let mut at = from;
    while at < to {
        let byte = bytes[at];
        if byte == b'\'' || byte == b'"' {
            at = closing_quote(bytes, at, to).unwrap_or(at) + 1;
        } else if is_ws(byte) {
            let run_end = skip_ws(bytes, at, to);
            let before = at
                .checked_sub(1)
                .and_then(|index| bytes.get(index))
                .copied();
            let after = if run_end < to {
                bytes.get(run_end).copied()
            } else {
                None
            };
            if joins(before) || joins(after) {
                replacement.push_str(line.get(copied..at).unwrap_or(""));
                copied = run_end;
            }
            at = run_end;
        } else {
            at += 1;
        }
    }
    replacement.push_str(line.get(copied..to).unwrap_or(""));
    replacement
}

/// Completes the diagnostic of the `SuspectComment` token `tokens[index]` of `line` (whose file
/// offset is `line_start`): appends the `cause` argument and, for R1 and R2, the quick-fix
/// suggestion when its replacement differs from the text it replaces (m1 §12.8 N-1). The
/// operands are the tokens between the previous whitespace and the whitespace just before the
/// suspect token, as the lexer produces them; when the tokens do not have that shape, the cause
/// is `operand-list` and there is no fix.
pub(crate) fn complete_diagnostic(
    diagnostic: &mut Diagnostic,
    file: FileId,
    line_start: u32,
    line: &str,
    tokens: &[Token],
    index: usize,
) {
    let bytes = line.as_bytes();
    let Some(&token) = tokens.get(index) else {
        return;
    };
    let fixable = index.checked_sub(1).and_then(|separator_index| {
        let separator = tokens.get(separator_index)?;
        if separator.kind != TokenKind::Whitespace {
            return None;
        }
        let before = tokens.get(..separator_index)?;
        let operands_start = before
            .iter()
            .rposition(|candidate| candidate.kind == TokenKind::Whitespace)
            .map_or(0, |whitespace| whitespace + 1);
        let operands = before.get(operands_start..)?;
        let cause = dangling_or_joiner(bytes, operands, token.range.start as usize)?;
        Some((cause, separator.range.start))
    });
    let cause = fixable.map_or(Cause::OperandList, |(cause, _)| cause);
    diagnostic
        .args
        .push(("cause", DiagArg::Str(cause.id().to_owned())));
    if let Some((_, separator_start)) = fixable {
        let range = TextRange::new(separator_start, token.range.end);
        let replacement = fix_replacement(line, range.start as usize, range.end as usize);
        // A fix that would change nothing is not offered; the diagnostic stays (m1 §12.8 N-1).
        if range.slice(line) != Some(replacement.as_str()) {
            diagnostic.suggestion = Some(Suggestion {
                span: SourceSpan {
                    file,
                    range: range.shifted(line_start),
                },
                replacement,
            });
        }
    }
}
