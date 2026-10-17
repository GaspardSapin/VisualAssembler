//! Character classes and literal scanners of the A68k dialect (m1-core-isa-lexer.md §5.2):
//! identifiers, numbers, quoted strings, backslash sequences and the runs of rejected
//! characters.
//!
//! Scanners work on the bytes of one line and take an `end` bound (the end of the line, or of
//! the part before a stray `\r`/`\n`). Every offset they return lies on a char boundary: the
//! bytes they stop at are ASCII, and non-ASCII chars are always consumed whole. None of them
//! allocates or panics.

use va_core::syntax::TokenKind;

use super::codes::{LEX_BAD_BACKSLASH, LEX_BAD_NUMBER};

/// Class bit: space or tab, the only whitespace of the dialect.
const WS: u8 = 1;
/// Class bit: ID_START, `A–Z a–z _ .`.
const ID_START: u8 = 2;
/// Class bit: ID_CONT, ID_START plus `0–9 @`.
const ID_CONT: u8 = 4;
/// Class bit: a decimal digit.
const DIGIT: u8 = 8;
/// Class bit: a hexadecimal digit, either case.
const HEX: u8 = 16;

/// The class bits of every byte value; non-ASCII bytes have none.
static CLASSES: [u8; 256] = classes();

/// Builds [`CLASSES`] at compile time.
const fn classes() -> [u8; 256] {
    let mut table = [0_u8; 256];
    let mut index = 0;
    while index < table.len() {
        // `index` is below 256, so the cast keeps every bit.
        let byte = index as u8;
        let mut bits = 0;
        if byte == b' ' || byte == b'\t' {
            bits |= WS;
        }
        if byte.is_ascii_alphabetic() || byte == b'_' || byte == b'.' {
            bits |= ID_START | ID_CONT;
        }
        if byte.is_ascii_digit() {
            bits |= ID_CONT | DIGIT | HEX;
        }
        if byte == b'@' {
            bits |= ID_CONT;
        }
        if matches!(byte, b'a'..=b'f' | b'A'..=b'F') {
            bits |= HEX;
        }
        table[index] = bits;
        index += 1;
    }
    table
}

/// Tells whether `byte` has the class bit `bit`.
#[inline]
fn has(byte: u8, bit: u8) -> bool {
    CLASSES[usize::from(byte)] & bit != 0
}

/// Tells whether `byte` is a space or a tab.
#[inline]
pub(crate) fn is_ws(byte: u8) -> bool {
    has(byte, WS)
}

/// Tells whether `byte` is ID_START (`A–Z a–z _ .`).
#[inline]
pub(crate) fn is_id_start(byte: u8) -> bool {
    has(byte, ID_START)
}

/// Tells whether `byte` is ID_CONT (`A–Z a–z _ . 0–9 @`).
#[inline]
pub(crate) fn is_id_cont(byte: u8) -> bool {
    has(byte, ID_CONT)
}

/// Tells whether `byte` is a decimal digit.
#[inline]
pub(crate) fn is_digit(byte: u8) -> bool {
    has(byte, DIGIT)
}

/// Tells whether `byte` ends a line part: a `\r` or `\n` inside a line (API misuse, §5.2).
#[inline]
pub(crate) fn is_terminator(byte: u8) -> bool {
    byte == b'\r' || byte == b'\n'
}

/// Tells whether `byte` is a size letter `B W L S`, either case.
#[inline]
pub(crate) fn is_size_letter(byte: u8) -> bool {
    matches!(byte, b'B' | b'W' | b'L' | b'S' | b'b' | b'w' | b'l' | b's')
}

/// Returns the first offset from `from` that is not ID_CONT (or `end`).
#[inline]
pub(crate) fn skip_id_cont(bytes: &[u8], from: usize, end: usize) -> usize {
    let mut at = from;
    while at < end && is_id_cont(bytes[at]) {
        at += 1;
    }
    at
}

/// Returns the first offset from `from` that is not a space or tab (or `end`).
#[inline]
pub(crate) fn skip_ws(bytes: &[u8], from: usize, end: usize) -> usize {
    let mut at = from;
    while at < end && is_ws(bytes[at]) {
        at += 1;
    }
    at
}

/// Returns the end of the word starting at `from`: the first space, tab or `;` (or `end`).
/// Quotes are not special; the label and operation fields have no strings.
#[inline]
pub(crate) fn word_end(bytes: &[u8], from: usize, end: usize) -> usize {
    let mut at = from;
    while at < end && !is_ws(bytes[at]) && bytes[at] != b';' {
        at += 1;
    }
    at
}

/// Returns the first offset from `from` that is ASCII (or `end`); `from..` is a run of
/// non-ASCII chars, consumed whole.
#[inline]
pub(crate) fn skip_non_ascii(bytes: &[u8], from: usize, end: usize) -> usize {
    let mut at = from;
    while at < end && !bytes[at].is_ascii() {
        at += 1;
    }
    at
}

/// Tells whether a macro parameter `\0`–`\9` or `\@` starts at `at`.
#[inline]
pub(crate) fn is_macro_param_at(bytes: &[u8], at: usize, end: usize) -> bool {
    at + 1 < end && bytes[at] == b'\\' && (is_digit(bytes[at + 1]) || bytes[at + 1] == b'@')
}

/// Tells whether a size suffix `.B .W .L .S` (either case) starts at `at` and is not glued to a
/// further ID_CONT char (`.w` is a suffix, `.wx` is not).
#[inline]
pub(crate) fn is_size_suffix_at(bytes: &[u8], at: usize, end: usize) -> bool {
    at + 1 < end
        && bytes[at] == b'.'
        && is_size_letter(bytes[at + 1])
        && (at + 2 == end || !is_id_cont(bytes[at + 2]))
}

/// Returns the byte length of the UTF-8 char whose first byte is `lead`.
#[inline]
fn char_len(lead: u8) -> usize {
    match lead {
        0xF0..=0xFF => 4,
        0xE0..=0xEF => 3,
        0xC0..=0xDF => 2,
        _ => 1,
    }
}

/// Returns the end of the quoted literal whose delimiter (`'` or `"`) is at `start`: one past
/// the closing delimiter, a doubled delimiter standing for itself. `None` when the literal is
/// not closed before `end`. There are no backslash escapes (A68k: `'a\nb'` is 4 bytes).
pub(crate) fn string_end(bytes: &[u8], start: usize, end: usize) -> Option<usize> {
    let quote = bytes[start];
    let mut at = start + 1;
    loop {
        let closing = at + bytes.get(at..end)?.iter().position(|&byte| byte == quote)?;
        if closing + 1 < end && bytes[closing + 1] == quote {
            at = closing + 2;
        } else {
            return Some(closing + 1);
        }
    }
}

/// Lexes the number-like run at `start` (a digit, `$`, `%` or `@`): returns its kind and end.
///
/// `$`HEX+, `%`BIN+, `@`OCT+ and DIGIT+ are a `Number`; DIGIT+ `$` is a `LocalLabel` (`1$`). A
/// prefix with no digit (`$`, `%2`, `@8`) or digits glued to ID_CONT chars (`10h`, `0x10`,
/// `$12g`, `1.5`) is `Invalid(lex-bad-number)` over the whole run. A size suffix right after
/// the digits (`$1000.w`) is not glued: it stays for the next token.
pub(crate) fn number(bytes: &[u8], start: usize, end: usize) -> (TokenKind, usize) {
    let (digits_start, radix): (usize, fn(u8) -> bool) = match bytes[start] {
        b'$' => (start + 1, |byte| has(byte, HEX)),
        b'%' => (start + 1, |byte| matches!(byte, b'0' | b'1')),
        b'@' => (start + 1, |byte| matches!(byte, b'0'..=b'7')),
        _ => (start, is_digit),
    };
    let mut at = digits_start;
    while at < end && radix(bytes[at]) {
        at += 1;
    }
    if at == digits_start {
        return (
            TokenKind::Invalid(LEX_BAD_NUMBER),
            skip_id_cont(bytes, digits_start, end),
        );
    }
    if digits_start == start && at < end && bytes[at] == b'$' {
        return (TokenKind::LocalLabel, at + 1);
    }
    if at < end && is_id_cont(bytes[at]) && !is_size_suffix_at(bytes, at, end) {
        return (
            TokenKind::Invalid(LEX_BAD_NUMBER),
            skip_id_cont(bytes, at, end),
        );
    }
    (TokenKind::Number, at)
}

/// Lexes the backslash sequence at `start`: returns its kind and end.
///
/// `\0`–`\9` and `\@` are a `MacroParam`, `\` ID_START ID_CONT* a `LocalLabel`. Anything else is
/// `Invalid(lex-bad-backslash)` over the backslash and the next char, unless that char is a
/// space, a tab or `;`, which keep their meaning (field end, comment).
pub(crate) fn backslash(bytes: &[u8], start: usize, end: usize) -> (TokenKind, usize) {
    let next = start + 1;
    if next >= end {
        return (TokenKind::Invalid(LEX_BAD_BACKSLASH), next);
    }
    let byte = bytes[next];
    if is_digit(byte) || byte == b'@' {
        (TokenKind::MacroParam, next + 1)
    } else if is_id_start(byte) {
        (TokenKind::LocalLabel, skip_id_cont(bytes, next + 1, end))
    } else if is_ws(byte) || byte == b';' {
        (TokenKind::Invalid(LEX_BAD_BACKSLASH), next)
    } else {
        (
            TokenKind::Invalid(LEX_BAD_BACKSLASH),
            (next + char_len(byte)).min(end),
        )
    }
}

/// Tells whether the ASCII char at `at` belongs to a run of rejected characters
/// (`lex-unexpected-char`): `^ [ ] { } < > = ? :`, backquote, control characters other than
/// tab, DEL, and a lone `.` (not followed by ID_CONT). `<<` and `>>` are operators, so a `<` or
/// `>` followed by the same char is not part of a run.
#[inline]
pub(crate) fn is_rejected_at(bytes: &[u8], at: usize, end: usize) -> bool {
    let byte = bytes[at];
    let next = if at + 1 < end { bytes[at + 1] } else { 0 };
    match byte {
        b'^' | b'[' | b']' | b'{' | b'}' | b'=' | b'?' | b':' | b'`' | 0x7F => true,
        b'<' | b'>' => next != byte,
        b'.' => at + 1 >= end || !is_id_cont(next),
        0x00..=0x1F => byte != b'\t',
        _ => false,
    }
}

/// Returns the end of the run of rejected characters starting at `start` (whose first char the
/// caller already rejected).
#[inline]
pub(crate) fn rejected_run_end(bytes: &[u8], start: usize, end: usize) -> usize {
    let mut at = start + 1;
    while at < end && is_rejected_at(bytes, at, end) {
        at += 1;
    }
    at
}
