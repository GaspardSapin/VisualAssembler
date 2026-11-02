//! The fields of one source line (m2-assembler.md §4.2, frozen; §4.3): label, operation and
//! operand field, read from the M1 lexer's tokens, and [`Problem`], the line-relative problem
//! that the front end and the encoder report to the driver.
//!
//! [`parse_line`] reads the fields only; the user of the operand field parses its operands
//! (`operand::parse_operand`, `expr::parse_expr`). The lexer is the only tokenizer: nothing here
//! re-scans the text of the line.
//!
//! **Lexer errors are not repeated.** An `Invalid` token is reported by the lexer's
//! `SyntaxProvider::line_diagnostics`, so a field holding one gives no [`Problem`] here: a broken
//! label field gives no label, a broken operation field no operation, and an invalid size suffix
//! reads as no suffix. The operand parsers do return a problem for an `Invalid` token (they must
//! fail), but it repeats the lexer diagnostic exactly (its `lex-*` code, the token's range and
//! the `text` argument), so the driver can recognize and drop it.
//!
//! **Macro parameters** (`\1`, `\@`) in the label or operation field give one
//! `asm-macro-param-outside` problem each, and that field reads as absent. A line that comes from
//! a macro expansion has no parameter left; the driver ignores the problems of a raw macro body
//! line, which it parses only to find `MACRO`, `ENDM` and `MEXIT`. Parameters in the operand
//! field are reported by the operand parsers.

use std::cmp::Ordering;
use std::sync::OnceLock;

use va_core::diag::{DiagArg, DiagCode};
use va_core::source::TextRange;
use va_core::syntax::{KeywordKind, SyntaxProvider as _, Token, TokenKind};

use super::codes::{ASM_LABEL_LIKE_KEYWORD, ASM_MACRO_PARAM_OUTSIDE};
use super::operand::Spanned;
use crate::isa::{SPELLINGS, Size, spelling_info};
use crate::syntax::M68kSyntax;

/// The operation field of a line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpField {
    /// An instruction spelling.
    Mnemonic {
        /// The spelling, uppercase, as listed in `isa::SPELLINGS`.
        spelling: &'static str,
        /// Its size suffix, if any (none when the lexer rejected the suffix).
        size: Option<Size>,
    },
    /// A directive.
    Directive {
        /// The directive name, uppercase (`"DC"`, `"="` …).
        name: &'static str,
        /// Its size suffix, if any (none when the lexer rejected the suffix).
        size: Option<Size>,
    },
    /// A macro call (or an unknown operation the driver resolves).
    Macro {
        /// The name as written (macro names are case-sensitive).
        name: String,
        /// The suffix letter after the name's dot, as written (`"w"` for `swap2.w`), if any;
        /// none when the lexer rejected the suffix.
        suffix: Option<String>,
    },
}

/// The fields of one line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// The label, if any (without its colon): a column-1 label, or an indented `name:`.
    pub label: Option<Spanned<String>>,
    /// Whether the label is a local label (`1$`, `\name`).
    pub local_label: bool,
    /// The operation field, if any; its range covers the stem and the suffix.
    pub op: Option<Spanned<OpField>>,
    /// The operand field (empty when the line has none), line-relative: its tokens run from
    /// the whitespace after the operation to the next whitespace, comment or suspect comment.
    /// An empty field sits at the end of the last field read.
    pub operand_field: TextRange,
    /// The problems found while reading the fields.
    pub problems: Vec<Problem>,
}

/// One problem of a line, with line-relative ranges; the driver turns it into a diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// The diagnostic code (`asm::codes`).
    pub code: DiagCode,
    /// The range the problem covers, line-relative.
    pub range: TextRange,
    /// The Fluent arguments of the message.
    pub args: Vec<(&'static str, DiagArg)>,
    /// A quick fix: the line-relative range to replace and its replacement.
    pub fix: Option<(TextRange, String)>,
}

impl Problem {
    /// Returns a problem of `code` over `range`, without argument or fix.
    pub(crate) fn new(code: DiagCode, range: TextRange) -> Self {
        Self {
            code,
            range,
            args: Vec::new(),
            fix: None,
        }
    }

    /// Appends the Fluent argument `name`.
    pub(crate) fn with_arg(mut self, name: &'static str, value: DiagArg) -> Self {
        self.args.push((name, value));
        self
    }

    /// Appends the `text` argument: the text of `line` that the problem covers, truncated
    /// (m2 §7.2, m1 §3.4).
    pub(crate) fn with_text(self, line: &str) -> Self {
        let text = text_arg(slice(line, self.range));
        self.with_arg("text", text)
    }

    /// Sets the quick fix.
    pub(crate) fn with_fix(mut self, range: TextRange, replacement: String) -> Self {
        self.fix = Some((range, replacement));
        self
    }
}

/// Longest `text` argument, in chars, before `…` is appended (m2 §7.2).
const TEXT_ARG_CHARS: usize = 32;

/// Returns `text` as a `text` argument: at most 32 chars, then `…`.
pub(crate) fn text_arg(text: &str) -> DiagArg {
    match text.char_indices().nth(TEXT_ARG_CHARS) {
        Some((cut, _)) => {
            let mut truncated = String::with_capacity(cut + '…'.len_utf8());
            truncated.push_str(text.get(..cut).unwrap_or(text));
            truncated.push('…');
            DiagArg::Str(truncated)
        }
        None => DiagArg::Str(text.to_owned()),
    }
}

/// Returns the text of `line` that `range` covers ("" when it is out of bounds).
pub(crate) fn slice(line: &str, range: TextRange) -> &str {
    range.slice(line).unwrap_or("")
}

/// Returns the text of `token` in `line`.
pub(crate) fn token_text<'line>(line: &'line str, token: &Token) -> &'line str {
    slice(line, token.range)
}

/// Tells whether `token` is the punctuation char `wanted` (`,`, `#`, `(`, `)` …).
pub(crate) fn is_punct(line: &str, token: &Token, wanted: char) -> bool {
    token.kind == TokenKind::Punctuation && token_text(line, token).starts_with(wanted)
}

/// Tells whether `token` ends an operand field: whitespace, a comment or a suspect comment.
pub(crate) fn ends_field(token: &Token) -> bool {
    matches!(
        token.kind,
        TokenKind::Whitespace | TokenKind::Comment | TokenKind::SuspectComment(_)
    )
}

/// Returns the number of tokens at the start of `tokens` that belong to one operand: up to the
/// first `,` outside parentheses or the end of the operand field.
pub(crate) fn operand_extent(line: &str, tokens: &[Token]) -> usize {
    let mut depth: u32 = 0;
    for (index, token) in tokens.iter().enumerate() {
        if ends_field(token) {
            return index;
        }
        if is_punct(line, token, '(') {
            depth += 1;
        } else if is_punct(line, token, ')') {
            depth = depth.saturating_sub(1);
        } else if depth == 0 && is_punct(line, token, ',') {
            return index;
        }
    }
    tokens.len()
}

/// Returns the range from the start of the first token to the end of the last one, or the
/// empty range at `fallback` when `tokens` is empty.
pub(crate) fn span(tokens: &[Token], fallback: u32) -> TextRange {
    match (tokens.first(), tokens.last()) {
        (Some(first), Some(last)) => TextRange::new(first.range.start, last.range.end),
        _ => TextRange::new(fallback, fallback),
    }
}

/// Returns the offset where a problem about the empty start of `tokens` sits: the start of its
/// first token, else the end of the line.
pub(crate) fn start_of(line: &str, tokens: &[Token]) -> u32 {
    tokens.first().map_or_else(
        || u32::try_from(line.len()).unwrap_or(u32::MAX),
        |token| token.range.start,
    )
}

/// Returns the problem of the first `Invalid` token (the lexer's own diagnostic, repeated) or
/// macro parameter (`asm-macro-param-outside`) of `tokens`, if any.
pub(crate) fn token_problem(line: &str, tokens: &[Token]) -> Option<Problem> {
    tokens.iter().find_map(|token| match token.kind {
        TokenKind::Invalid(code) => Some(Problem::new(code, token.range).with_text(line)),
        TokenKind::MacroParam => Some(param_problem(line, token)),
        _ => None,
    })
}

/// Returns the `asm-macro-param-outside` problem of the macro parameter `token`.
fn param_problem(line: &str, token: &Token) -> Problem {
    Problem::new(ASM_MACRO_PARAM_OUTSIDE, token.range).with_text(line)
}

/// Compares an uppercase table entry with `name`, folding the ASCII case of `name`.
fn cmp_folded(entry: &str, name: &str) -> Ordering {
    entry
        .bytes()
        .cmp(name.bytes().map(|byte| byte.to_ascii_uppercase()))
}

/// Returns the spelling of `isa::SPELLINGS` that `stem` names (any case).
fn mnemonic_spelling(stem: &str) -> Option<&'static str> {
    SPELLINGS
        .binary_search_by(|entry| cmp_folded(entry, stem))
        .ok()
        .and_then(|index| SPELLINGS.get(index).copied())
}

/// Returns the directive name (uppercase, from the lexer's keyword list) that `stem` names (any
/// case).
fn directive_name(stem: &str) -> Option<&'static str> {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    let names = NAMES.get_or_init(|| {
        let mut names: Vec<&'static str> = M68kSyntax
            .keywords()
            .iter()
            .filter(|keyword| keyword.kind == KeywordKind::Directive)
            .map(|keyword| keyword.name)
            .collect();
        names.sort_unstable();
        names
    });
    names
        .binary_search_by(|entry| cmp_folded(entry, stem))
        .ok()
        .and_then(|index| names.get(index).copied())
}

/// Tells whether `name` (any case) is a mnemonic or directive spelling, alone or followed by a
/// size suffix (`nop`, `even`, `MOVE.L`).
fn is_keyword_spelling(name: &str) -> bool {
    let stem = match name.split_once('.') {
        Some((stem, letter)) if Size::from_letter(letter).is_some() => stem,
        Some(_) => return false,
        None => name,
    };
    spelling_info(stem).is_some() || directive_name(stem).is_some()
}

/// Splits `tokens` at `at` into the word starting there (up to whitespace or a comment) and
/// returns its end index.
fn word_end(tokens: &[Token], at: usize) -> usize {
    let mut end = at;
    while let Some(token) = tokens.get(end) {
        if ends_field(token) {
            break;
        }
        end += 1;
    }
    end
}

/// Returns the index of the first token from `at` that is not whitespace.
fn skip_whitespace(tokens: &[Token], at: usize) -> usize {
    let mut index = at;
    while tokens
        .get(index)
        .is_some_and(|token| token.kind == TokenKind::Whitespace)
    {
        index += 1;
    }
    index
}

/// Reads a label word (column 1, or an indented `name:`): returns the label, whether it is
/// local and whether it has a colon; macro parameters add their problems to `problems`.
fn read_label(
    line: &str,
    word: &[Token],
    problems: &mut Vec<Problem>,
) -> (Option<Spanned<String>>, bool, bool) {
    let colon = word.last().is_some_and(|token| is_punct(line, token, ':'));
    if word
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Invalid(_)))
    {
        return (None, false, colon);
    }
    let params: Vec<&Token> = word
        .iter()
        .filter(|token| token.kind == TokenKind::MacroParam)
        .collect();
    if !params.is_empty() {
        problems.extend(params.into_iter().map(|token| param_problem(line, token)));
        return (None, false, colon);
    }
    let Some(name) = word
        .iter()
        .find(|token| matches!(token.kind, TokenKind::Label | TokenKind::LocalLabel))
    else {
        return (None, false, colon);
    };
    let label = Spanned {
        item: token_text(line, name).to_owned(),
        range: name.range,
    };
    (Some(label), name.kind == TokenKind::LocalLabel, colon)
}

/// Reads the size suffix tokens `suffix` of an operation: the size of a `SizeSuffix` token,
/// else none (a rejected suffix or a macro parameter suffix, whose problem goes to `problems`).
fn read_size(line: &str, suffix: &[Token], problems: &mut Vec<Problem>) -> Option<Size> {
    problems.extend(
        suffix
            .iter()
            .filter(|token| token.kind == TokenKind::MacroParam)
            .map(|token| param_problem(line, token)),
    );
    match suffix {
        [token] if token.kind == TokenKind::SizeSuffix => {
            Size::from_letter(token_text(line, token).get(1..).unwrap_or(""))
        }
        _ => None,
    }
}

/// Reads the operation word `word`: a stem (one `Mnemonic`, `Directive` or `MacroName` token,
/// or the `MacroName`/`MacroParam` pieces of a stem holding parameters), then its suffix tokens.
fn read_operation(line: &str, word: &[Token], problems: &mut Vec<Problem>) -> Option<OpField> {
    let pieces = word
        .iter()
        .take_while(|token| matches!(token.kind, TokenKind::MacroName | TokenKind::MacroParam))
        .count()
        .max(1);
    let (stem, suffix) = word.split_at(pieces.min(word.len()));
    let first = stem.first()?;
    if stem.iter().any(|token| token.kind == TokenKind::MacroParam) {
        problems.extend(
            word.iter()
                .filter(|token| token.kind == TokenKind::MacroParam)
                .map(|token| param_problem(line, token)),
        );
        return None;
    }
    let text = token_text(line, first);
    match first.kind {
        TokenKind::Mnemonic => {
            let spelling = mnemonic_spelling(text)?;
            let size = read_size(line, suffix, problems);
            Some(OpField::Mnemonic { spelling, size })
        }
        TokenKind::Directive => {
            let name = directive_name(text)?;
            let size = read_size(line, suffix, problems);
            Some(OpField::Directive { name, size })
        }
        TokenKind::MacroName => {
            let letter = read_size(line, suffix, problems).and_then(|_| {
                suffix
                    .first()
                    .map(|token| token_text(line, token).get(1..).unwrap_or("").to_owned())
            });
            Some(OpField::Macro {
                name: text.to_owned(),
                suffix: letter,
            })
        }
        _ => None,
    }
}

/// Reads the fields of `line` from its lexer `tokens` (line-relative ranges that cover it).
///
/// The only problems it gives are `asm-macro-param-outside` (label and operation fields) and
/// the warning `asm-label-like-keyword`, with its fix that inserts a tab in column 1: a column-1
/// label without colon, spelled like a mnemonic or a directive (any case, optionally with a
/// size suffix), with nothing but a comment after it (m2 §4.3). Everything else is the driver's
/// (unknown operations, `asm-label-required`, …) or the lexer's.
pub fn parse_line(line: &str, tokens: &[Token]) -> Line {
    let mut problems = Vec::new();
    let mut label = None;
    let mut local_label = false;
    let mut label_colon = false;
    let mut column_one_label = false;
    let mut field_end: u32 = 0;
    let mut at = 0;

    let starts_with_label = tokens
        .first()
        .is_some_and(|token| !ends_field(token) && token.range.start == 0);
    if starts_with_label {
        let end = word_end(tokens, 0);
        let word = tokens.get(..end).unwrap_or(&[]);
        (label, local_label, label_colon) = read_label(line, word, &mut problems);
        column_one_label = label.is_some();
        field_end = span(word, 0).end;
        at = end;
    }
    at = skip_whitespace(tokens, at);

    let mut word_stop = word_end(tokens, at);
    let indented_label = !starts_with_label
        && word_stop > at
        && tokens.get(at).is_some_and(|token| {
            matches!(
                token.kind,
                TokenKind::Label | TokenKind::LocalLabel | TokenKind::MacroParam
            )
        })
        && tokens
            .get(word_stop - 1)
            .is_some_and(|token| is_punct(line, token, ':'));
    if indented_label {
        let word = tokens.get(at..word_stop).unwrap_or(&[]);
        (label, local_label, label_colon) = read_label(line, word, &mut problems);
        field_end = span(word, field_end).end;
        at = skip_whitespace(tokens, word_stop);
        word_stop = word_end(tokens, at);
    }

    let mut op = None;
    let mut operand_field = TextRange::new(field_end, field_end);
    if word_stop > at {
        let word = tokens.get(at..word_stop).unwrap_or(&[]);
        let range = span(word, field_end);
        op = read_operation(line, word, &mut problems).map(|item| Spanned { item, range });
        field_end = range.end;
        operand_field = TextRange::new(field_end, field_end);
        if tokens
            .get(word_stop)
            .is_some_and(|token| token.kind == TokenKind::Whitespace)
        {
            let first = word_stop + 1;
            let last = word_end(tokens, first);
            if last > first {
                operand_field = span(tokens.get(first..last).unwrap_or(&[]), field_end);
            }
        }
    }

    if column_one_label
        && !label_colon
        && !local_label
        && word_stop == at
        && let Some(name) = &label
        && is_keyword_spelling(&name.item)
    {
        problems.push(
            Problem::new(ASM_LABEL_LIKE_KEYWORD, name.range)
                .with_arg("name", DiagArg::Str(name.item.clone()))
                .with_fix(TextRange::new(0, 0), "\t".to_owned()),
        );
    }

    Line {
        label,
        local_label,
        op,
        operand_field,
        problems,
    }
}
