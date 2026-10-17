//! The line lexer of the A68k dialect (m1-core-isa-lexer.md §5.1, §5.2): label field,
//! operation field, operand field (or path, or nothing, by the operation's policy, §5.3) and the
//! trailing comment, which [`spacing`] may turn into a `SuspectComment` (§5.7).
//!
//! The lexer is one forward pass over the bytes of the line. The only look-backs are bounded:
//! the indented-label check re-lexes the first word when it ends with `:`, and rule R3 of §5.7
//! lexes the first word of the trailing text once. Each byte is therefore visited a bounded
//! number of times (linear time), and nothing is allocated besides the tokens pushed to `out`
//! (R3's trial tokens are truncated away).
//!
//! **A `\r` or `\n` inside a line** (API misuse: callers pass line contents without terminator)
//! is an `Invalid(lex-unexpected-char)` token, one per run of such bytes. The text after it is
//! lexed as the start of a new line, the way `va_core::source::lines` would have split it.

use va_core::source::{MAX_LINE_BYTES, TextRange};
use va_core::syntax::{Token, TokenKind};

use super::codes::{
    LEX_BAD_LABEL, LEX_BAD_OPERATION, LEX_BAD_SIZE_SUFFIX, LEX_LABEL_JUNK, LEX_LINE_TOO_LONG,
    LEX_NON_ASCII, LEX_OPERANDS_IN_COMMENT, LEX_UNEXPECTED_CHAR, LEX_UNTERMINATED_STRING,
};
use super::keywords::{self, OperandPolicy};
use super::literals::{
    backslash, is_id_cont, is_id_start, is_macro_param_at, is_size_letter, is_size_suffix_at,
    is_terminator, is_ws, number, rejected_run_end, skip_id_cont, skip_non_ascii, skip_ws,
    string_end, word_end,
};
use super::spacing;

/// Lexes one line (content without terminator) and appends its tokens to `out` (§5.2).
pub(crate) fn lex_line(line: &str, out: &mut Vec<Token>) {
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return;
    }
    let mut lexer = Lexer {
        line,
        bytes,
        end: bytes.len(),
        out,
    };
    if bytes.len() > MAX_LINE_BYTES {
        lexer.push(TokenKind::Invalid(LEX_LINE_TOO_LONG), 0, bytes.len());
        return;
    }
    let mut start = 0;
    while start < bytes.len() {
        let mut end = start;
        while end < bytes.len() && !is_terminator(bytes[end]) {
            end += 1;
        }
        lexer.end = end;
        if end > start {
            lexer.line_part(start);
        }
        if end == bytes.len() {
            return;
        }
        let mut after = end + 1;
        while after < bytes.len() && is_terminator(bytes[after]) {
            after += 1;
        }
        lexer.push(TokenKind::Invalid(LEX_UNEXPECTED_CHAR), end, after);
        start = after;
    }
}

/// What the lexer expects after the operation field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rest {
    /// Nothing: whitespace, then a comment for any remaining text.
    Comment,
    /// A path operand (`INCLUDE`, `INCBIN`).
    Path,
    /// Operands. `checked` enables the §5.7 rule on the trailing text; it is off after an
    /// invalid operation field.
    Operands {
        /// Whether the §5.7 rule applies to the trailing text.
        checked: bool,
    },
}

/// Converts a line offset to a token offset (lines are at most 64 KiB, so it always fits).
#[inline]
fn offset(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// The state of one `lex_line` call: the line and the end of the part being lexed.
struct Lexer<'line, 'out> {
    /// The whole line.
    line: &'line str,
    /// The bytes of `line`.
    bytes: &'line [u8],
    /// End of the line part being lexed: the line end, or a stray `\r`/`\n`.
    end: usize,
    /// Where tokens are appended.
    out: &'out mut Vec<Token>,
}

impl Lexer<'_, '_> {
    /// Appends a token of `kind` over `start..end`.
    #[inline]
    fn push(&mut self, kind: TokenKind, start: usize, end: usize) {
        self.out.push(Token {
            kind,
            range: TextRange {
                start: offset(start),
                end: offset(end),
            },
        });
    }

    /// Appends a `Comment` from `start` to the end of the line part.
    fn comment(&mut self, start: usize) {
        self.push(TokenKind::Comment, start, self.end);
    }

    /// Appends the `Whitespace` run starting at `start` and returns its end.
    fn whitespace(&mut self, start: usize) -> usize {
        let end = skip_ws(self.bytes, start, self.end);
        self.push(TokenKind::Whitespace, start, end);
        end
    }

    /// Handles the end of a field at `at` (the end, a `;` or whitespace): lexes the comment or
    /// the whitespace, and returns the start of the next field, if any.
    fn next_field(&mut self, at: usize) -> Option<usize> {
        if at >= self.end {
            return None;
        }
        let start = if is_ws(self.bytes[at]) {
            self.whitespace(at)
        } else {
            at
        };
        if start >= self.end {
            None
        } else if self.bytes[start] == b';' {
            self.comment(start);
            None
        } else {
            Some(start)
        }
    }

    /// Lexes the line part `start..self.end` (§5.1).
    fn line_part(&mut self, start: usize) {
        let (bytes, end) = (self.bytes, self.end);
        let first = bytes[start];
        if first == b'*' || first == b';' {
            self.comment(start);
            return;
        }
        let indented = is_ws(first);
        let field_start = if indented {
            start
        } else {
            let label_end = word_end(bytes, start, end);
            self.label(start, label_end);
            label_end
        };
        let Some(mut word_start) = self.next_field(field_start) else {
            return;
        };
        let mut word_stop = word_end(bytes, word_start, end);
        if indented && bytes[word_stop - 1] == b':' {
            let mark = self.out.len();
            if self.label(word_start, word_stop) {
                let Some(next) = self.next_field(word_stop) else {
                    return;
                };
                word_start = next;
                word_stop = word_end(bytes, word_start, end);
            } else {
                self.out.truncate(mark);
            }
        }
        let rest = self.operation(word_start, word_stop);
        let Some(operand_start) = self.next_field(word_stop) else {
            return;
        };
        match rest {
            Rest::Comment => self.comment(operand_start),
            Rest::Path => {
                let stop = self.path(operand_start);
                self.trailing(stop, None);
            }
            Rest::Operands { checked } => {
                let first_operand = self.out.len();
                let stop = self.operands(operand_start);
                self.trailing(stop, checked.then_some(first_operand));
            }
        }
    }

    /// Lexes the label field `start..stop` (§5.2 label rows) and tells whether it is exactly a
    /// label followed by its colon run.
    ///
    /// A label is a run of pieces: `Label` (ID_START ID_CONT*), `LocalLabel` (DIGIT+ `$` or `\`
    /// ID_START ID_CONT*, first piece only) and `MacroParam` (`\0`–`\9`, `\@`); text after a
    /// macro parameter continues the name with the kind of its first piece. A run of `:` may
    /// follow, as one `Punctuation` token (a68k assembles `lab::` and `lab:::` like `lab:`,
    /// m1 §12.1 R12-2).
    /// The first byte that fits none of this starts an `Invalid` token to `stop`:
    /// `lex-bad-label` if no piece came before it, else `lex-label-junk`.
    fn label(&mut self, start: usize, stop: usize) -> bool {
        /// Where the label machine is.
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum State {
            /// Nothing lexed yet.
            Start,
            /// After a `Label` or `LocalLabel` piece.
            Text,
            /// After a `MacroParam` piece.
            Param,
            /// After the colon.
            Colon,
        }
        let bytes = self.bytes;
        let mut at = start;
        let mut state = State::Start;
        let mut name_kind = TokenKind::Label;
        while at < stop {
            let byte = bytes[at];
            if state != State::Colon && is_macro_param_at(bytes, at, stop) {
                self.push(TokenKind::MacroParam, at, at + 2);
                at += 2;
                state = State::Param;
                continue;
            }
            let piece = match state {
                State::Start if byte == b'\\' && at + 1 < stop && is_id_start(bytes[at + 1]) => {
                    name_kind = TokenKind::LocalLabel;
                    Some((TokenKind::LocalLabel, skip_id_cont(bytes, at + 2, stop)))
                }
                State::Start if byte.is_ascii_digit() => {
                    let digits = skip_digits(bytes, at, stop);
                    (digits < stop && bytes[digits] == b'$').then(|| {
                        name_kind = TokenKind::LocalLabel;
                        (TokenKind::LocalLabel, digits + 1)
                    })
                }
                State::Start if is_id_start(byte) => {
                    Some((TokenKind::Label, skip_id_cont(bytes, at + 1, stop)))
                }
                State::Param if is_id_cont(byte) => {
                    Some((name_kind, skip_id_cont(bytes, at + 1, stop)))
                }
                State::Text | State::Param if byte == b':' => {
                    let mut colons_end = at + 1;
                    while colons_end < stop && bytes[colons_end] == b':' {
                        colons_end += 1;
                    }
                    Some((TokenKind::Punctuation, colons_end))
                }
                _ => None,
            };
            let Some((kind, piece_end)) = piece else {
                break;
            };
            self.push(kind, at, piece_end);
            state = if kind == TokenKind::Punctuation {
                State::Colon
            } else {
                State::Text
            };
            at = piece_end;
        }
        if at < stop {
            let code = if state == State::Start {
                LEX_BAD_LABEL
            } else {
                LEX_LABEL_JUNK
            };
            self.push(TokenKind::Invalid(code), at, stop);
            return false;
        }
        state == State::Colon
    }

    /// Lexes the operation field `start..stop` (§5.2 op rows) and returns what follows it.
    ///
    /// `=` alone is a `Directive`. Otherwise the field must be a stem (ID_START without `.`,
    /// then ID_CONT without `.`, macro parameters allowed) optionally followed by a suffix from
    /// its first `.` to the field end; anything else is one `Invalid(lex-bad-operation)` over the
    /// whole field (a68k: "No such op-code"). A plain stem is a `Mnemonic`, a `Directive` or a
    /// `MacroName`; a stem holding macro parameters is `MacroName` and `MacroParam` pieces.
    fn operation(&mut self, start: usize, stop: usize) -> Rest {
        let bytes = self.bytes;
        if stop - start == 1 && bytes[start] == b'=' {
            self.push(TokenKind::Directive, start, stop);
            return Rest::Operands { checked: true };
        }
        let first = bytes[start];
        let starts_stem =
            (is_id_start(first) && first != b'.') || is_macro_param_at(bytes, start, stop);
        let mut stem_end = start;
        let mut has_param = false;
        if starts_stem {
            loop {
                if is_macro_param_at(bytes, stem_end, stop) {
                    has_param = true;
                    stem_end += 2;
                } else if stem_end < stop && is_id_cont(bytes[stem_end]) && bytes[stem_end] != b'.'
                {
                    stem_end += 1;
                } else {
                    break;
                }
            }
        }
        if !starts_stem || (stem_end < stop && bytes[stem_end] != b'.') {
            self.push(TokenKind::Invalid(LEX_BAD_OPERATION), start, stop);
            return Rest::Operands { checked: false };
        }
        let rest = if has_param {
            self.param_stem(start, stem_end);
            Rest::Operands { checked: true }
        } else {
            let stem = self.line.get(start..stem_end).unwrap_or("");
            if let Some(policy) = keywords::mnemonic(stem) {
                self.push(TokenKind::Mnemonic, start, stem_end);
                rest_of(policy)
            } else if let Some(policy) = keywords::directive(stem) {
                self.push(TokenKind::Directive, start, stem_end);
                rest_of(policy)
            } else {
                self.push(TokenKind::MacroName, start, stem_end);
                Rest::Operands { checked: true }
            }
        };
        if stem_end < stop {
            self.suffix(stem_end, stop);
        }
        rest
    }

    /// Appends the pieces of a stem holding macro parameters: `MacroParam` for each parameter,
    /// `MacroName` for the text between them.
    fn param_stem(&mut self, start: usize, stop: usize) {
        let mut at = start;
        while at < stop {
            if is_macro_param_at(self.bytes, at, stop) {
                self.push(TokenKind::MacroParam, at, at + 2);
                at += 2;
            } else {
                let mut text_end = at + 1;
                while text_end < stop && !is_macro_param_at(self.bytes, text_end, stop) {
                    text_end += 1;
                }
                self.push(TokenKind::MacroName, at, text_end);
                at = text_end;
            }
        }
    }

    /// Lexes the operation suffix `dot..stop` (from its `.`): `SizeSuffix` for `.B .W .L .S`,
    /// `Punctuation` + `MacroParam` for `.\0`…`.\9`, `.\@`, else `Invalid(lex-bad-size-suffix)`.
    fn suffix(&mut self, dot: usize, stop: usize) {
        let bytes = self.bytes;
        if stop - dot == 2 && is_size_letter(bytes[dot + 1]) {
            self.push(TokenKind::SizeSuffix, dot, stop);
        } else if stop - dot == 3 && is_macro_param_at(bytes, dot + 1, stop) {
            self.push(TokenKind::Punctuation, dot, dot + 1);
            self.push(TokenKind::MacroParam, dot + 1, stop);
        } else {
            self.push(TokenKind::Invalid(LEX_BAD_SIZE_SUFFIX), dot, stop);
        }
    }

    /// Lexes a path operand (`INCLUDE`, `INCBIN`) from `start` to the first unquoted whitespace
    /// or `;`, and returns where it stopped. Quoted pieces and unquoted pieces (which may hold
    /// `\`, `:` and quotes) are `String` tokens; an unterminated quote is
    /// `Invalid(lex-unterminated-string)` to the end of the line.
    fn path(&mut self, start: usize) -> usize {
        let (bytes, end) = (self.bytes, self.end);
        let mut at = start;
        while at < end && !is_ws(bytes[at]) && bytes[at] != b';' {
            if bytes[at] == b'\'' || bytes[at] == b'"' {
                let Some(closed) = string_end(bytes, at, end) else {
                    self.push(TokenKind::Invalid(LEX_UNTERMINATED_STRING), at, end);
                    return end;
                };
                self.push(TokenKind::String, at, closed);
                at = closed;
            } else {
                let piece_end = word_end(bytes, at, end);
                self.push(TokenKind::String, at, piece_end);
                at = piece_end;
            }
        }
        at
    }

    /// Lexes an operand field from `start` to the first unquoted whitespace or `;` (§5.2
    /// operand rows), and returns where it stopped (the end of the line after an unterminated
    /// string).
    fn operands(&mut self, start: usize) -> usize {
        let (bytes, end) = (self.bytes, self.end);
        let first_token = self.out.len();
        let mut at = start;
        while at < end {
            let byte = bytes[at];
            let (kind, token_end) = match byte {
                b' ' | b'\t' | b';' => break,
                b'\'' | b'"' => match string_end(bytes, at, end) {
                    Some(closed) => (TokenKind::String, closed),
                    None => (TokenKind::Invalid(LEX_UNTERMINATED_STRING), end),
                },
                b'$' | b'%' | b'@' | b'0'..=b'9' => number(bytes, at, end),
                b'\\' => backslash(bytes, at, end),
                b'+' | b'-' | b'*' | b'/' | b'~' | b'&' | b'!' | b'|' => {
                    (TokenKind::Operator, at + 1)
                }
                b'<' | b'>' if at + 1 < end && bytes[at + 1] == byte => {
                    (TokenKind::Operator, at + 2)
                }
                b'#' | b',' | b'(' | b')' => (TokenKind::Punctuation, at + 1),
                b'.' if self.suffix_follows(first_token, at) => (TokenKind::SizeSuffix, at + 2),
                _ if is_id_start(byte)
                    && (byte != b'.' || (at + 1 < end && is_id_cont(bytes[at + 1]))) =>
                {
                    at = self.identifier(at);
                    continue;
                }
                0x80..=0xFF => (
                    TokenKind::Invalid(LEX_NON_ASCII),
                    skip_non_ascii(bytes, at, end),
                ),
                _ => (
                    TokenKind::Invalid(LEX_UNEXPECTED_CHAR),
                    rejected_run_end(bytes, at, end),
                ),
            };
            self.push(kind, at, token_end);
            at = token_end;
        }
        at
    }

    /// Tells whether the `.` at `at` is an operand size suffix: `.B .W .L .S` right after a `)`
    /// or a `Number` of the same field (whose tokens start at `first_token`).
    fn suffix_follows(&self, first_token: usize, at: usize) -> bool {
        self.out.len() > first_token
            && self.out.last().is_some_and(|previous| {
                previous.kind == TokenKind::Number
                    || (previous.kind == TokenKind::Punctuation
                        && self.bytes.get(previous.range.start as usize) == Some(&b')'))
            })
            && is_size_suffix_at(self.bytes, at, self.end)
    }

    /// Lexes the operand identifier at `start` (ID_START ID_CONT*, `.` and `@` included) and
    /// returns its end: `Register` for a register name, `Register` + `SizeSuffix` for
    /// `d0.w`-style names, else `Symbol`.
    fn identifier(&mut self, start: usize) -> usize {
        let bytes = self.bytes;
        let stop = skip_id_cont(bytes, start + 1, self.end);
        let word = &bytes[start..stop];
        if keywords::is_register(word) {
            self.push(TokenKind::Register, start, stop);
        } else if (4..=5).contains(&word.len())
            && is_size_suffix_at(bytes, stop - 2, stop)
            && keywords::is_register(&bytes[start..stop - 2])
        {
            self.push(TokenKind::Register, start, stop - 2);
            self.push(TokenKind::SizeSuffix, stop - 2, stop);
        } else {
            self.push(TokenKind::Symbol, start, stop);
        }
        stop
    }

    /// Lexes what follows a path or operand field that stopped at `at` (§5.2 last row, §5.7):
    /// a `;` comment, or whitespace then the trailing text T as a `Comment`, or as a
    /// `SuspectComment` when `operands_from` (the index of the first operand token) is given
    /// and a §5.7 rule fires.
    fn trailing(&mut self, at: usize, operands_from: Option<usize>) {
        let (bytes, end) = (self.bytes, self.end);
        if at >= end {
            return;
        }
        if bytes[at] == b';' {
            self.comment(at);
            return;
        }
        let text = skip_ws(bytes, at, end);
        let suspect = text < end
            && bytes[text] != b';'
            && operands_from.is_some_and(|first| self.rule_fires(first, text));
        self.push(TokenKind::Whitespace, at, text);
        if text >= end {
            return;
        }
        if !suspect {
            self.comment(text);
            return;
        }
        let semicolon = spacing::unquoted_semicolon(bytes, text, end);
        let mut suspect_end = semicolon;
        while suspect_end > text && is_ws(bytes[suspect_end - 1]) {
            suspect_end -= 1;
        }
        self.push(
            TokenKind::SuspectComment(LEX_OPERANDS_IN_COMMENT),
            text,
            suspect_end,
        );
        if suspect_end < semicolon {
            self.push(TokenKind::Whitespace, suspect_end, semicolon);
        }
        if semicolon < end {
            self.comment(semicolon);
        }
    }

    /// Tells whether a §5.7 rule fires for the operand tokens `out[first..]` and the trailing
    /// text starting at `text`. R3 lexes the first word of the text with the operand rules,
    /// inspects its tokens and removes them again.
    fn rule_fires(&mut self, first: usize, text: usize) -> bool {
        let operands = self.out.get(first..).unwrap_or(&[]);
        if spacing::dangling_or_joiner(self.bytes, operands, text).is_some() {
            return true;
        }
        let mark = self.out.len();
        self.operands(text);
        let fires = spacing::is_operand_list(self.bytes, self.out.get(mark..).unwrap_or(&[]));
        self.out.truncate(mark);
        fires
    }
}

/// Returns the first offset from `from` that is not a decimal digit (or `stop`).
fn skip_digits(bytes: &[u8], from: usize, stop: usize) -> usize {
    let mut at = from;
    while at < stop && bytes[at].is_ascii_digit() {
        at += 1;
    }
    at
}

/// Maps the operand policy of a keyword to what the lexer expects after it.
fn rest_of(policy: OperandPolicy) -> Rest {
    match policy {
        OperandPolicy::None => Rest::Comment,
        OperandPolicy::Path => Rest::Path,
        OperandPolicy::Operands => Rest::Operands { checked: true },
    }
}
