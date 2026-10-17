//! Diagnostic codes of the A68k-dialect lexer (m1-core-isa-lexer.md §5.4, §5.7).
//!
//! The first ten codes mark [`TokenKind::Invalid`](va_core::syntax::TokenKind::Invalid) tokens;
//! each becomes a blocking `Severity::Error` diagnostic with the `text` argument. The eleventh,
//! [`LEX_OPERANDS_IN_COMMENT`], marks
//! [`TokenKind::SuspectComment`](va_core::syntax::TokenKind::SuspectComment) tokens and becomes a
//! non-blocking error with the `text` and `cause` arguments (§5.7). Each code is documented as
//! the Fluent message `diag-<code>` in `arch/m68k/locales/{en,fr}/diagnostics.ftl`.

use va_core::diag::DiagCode;

/// A column-1 label field that does not start with a label (`9x`, `-x`, `é:`).
pub const LEX_BAD_LABEL: DiagCode = DiagCode("lex-bad-label");

/// Text glued after a label or its colon in the label field (`lab:nop`, `1$x`).
pub const LEX_LABEL_JUNK: DiagCode = DiagCode("lex-label-junk");

/// An operation field that is not a mnemonic, directive or macro name (`123`, `*`, `move+`).
pub const LEX_BAD_OPERATION: DiagCode = DiagCode("lex-bad-operation");

/// An operation suffix other than `.B`, `.W`, `.L`, `.S` or a macro parameter (`move.x`).
pub const LEX_BAD_SIZE_SUFFIX: DiagCode = DiagCode("lex-bad-size-suffix");

/// A number glued to identifier characters (`10h`, `0x10`, `$12g`) or a prefix without digits
/// (`$`, `%2`, `@8`).
pub const LEX_BAD_NUMBER: DiagCode = DiagCode("lex-bad-number");

/// A quote without its closing delimiter; the token runs to the end of the line.
pub const LEX_UNTERMINATED_STRING: DiagCode = DiagCode("lex-unterminated-string");

/// A backslash that starts neither a macro parameter (`\1`, `\@`) nor a local label (`\name`).
pub const LEX_BAD_BACKSLASH: DiagCode = DiagCode("lex-bad-backslash");

/// ASCII characters that no operand rule accepts (`^ [ ] { } < > = ? :`, backquote, control
/// characters, a lone `.`), and a `\r` or `\n` inside a line.
pub const LEX_UNEXPECTED_CHAR: DiagCode = DiagCode("lex-unexpected-char");

/// Non-ASCII characters outside comments and strings.
pub const LEX_NON_ASCII: DiagCode = DiagCode("lex-non-ascii");

/// A line longer than `va_core::source::MAX_LINE_BYTES`, kept as one invalid token.
pub const LEX_LINE_TOO_LONG: DiagCode = DiagCode("lex-line-too-long");

/// Operand-like text after the operand field, which a68k ignores as a comment (§5.7); the only
/// non-blocking code.
pub const LEX_OPERANDS_IN_COMMENT: DiagCode = DiagCode("lex-operands-in-comment");

/// Every code the lexer can produce, in the order of m1 §5.4 followed by
/// [`LEX_OPERANDS_IN_COMMENT`]. `Architecture::diagnostic_codes` of the 68000 returns it.
pub static DIAG_CODES: [DiagCode; 11] = [
    LEX_BAD_LABEL,
    LEX_LABEL_JUNK,
    LEX_BAD_OPERATION,
    LEX_BAD_SIZE_SUFFIX,
    LEX_BAD_NUMBER,
    LEX_UNTERMINATED_STRING,
    LEX_BAD_BACKSLASH,
    LEX_UNEXPECTED_CHAR,
    LEX_NON_ASCII,
    LEX_LINE_TOO_LONG,
    LEX_OPERANDS_IN_COMMENT,
];
