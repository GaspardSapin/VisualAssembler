//! Coverage, determinism and statelessness properties of the lexer (m1-core-isa-lexer.md §3.4,
//! §9.1), checked with `va_core::syntax::check_line_tokens`: every ASCII string of length ≤ 3
//! over 24 interesting chars, 100,000 deterministic pseudo-random lines, lexing in shuffled
//! order, and the returned `LineState`. Diagnostics of every line are checked too.
#![allow(clippy::unwrap_used)]

use va_arch_m68k::syntax::{LEX_OPERANDS_IN_COMMENT, M68kSyntax};
use va_core::source::FileId;
use va_core::syntax::{LineState, SyntaxProvider, Token, TokenKind};

use crate::{diagnostics, lex};

/// The 24 chars combined exhaustively: field separators, identifier chars, number prefixes,
/// quotes, comment starters, punctuation, operators and a rejected char.
const INTERESTING: [char; 24] = [
    ' ', '\t', 'a', 'd', 'w', '0', '1', '$', '%', '@', '\\', '\'', '"', ';', '*', ':', '.', ',',
    '(', ')', '#', '-', '<', '^',
];

/// Checks the properties every line must have: coverage (in `lex`), determinism, and one
/// diagnostic per `Invalid` or `SuspectComment` token, inside the line, with suggestions
/// inside the line too.
fn check_line(line: &str) {
    let tokens = lex(line);
    assert_eq!(lex(line), tokens, "{line:?}: not deterministic");
    let line_start = 1000;
    let diagnostics = diagnostics(line, line_start);
    let flagged: Vec<&Token> = tokens
        .iter()
        .filter(|token| {
            matches!(
                token.kind,
                TokenKind::Invalid(_) | TokenKind::SuspectComment(_)
            )
        })
        .collect();
    assert_eq!(diagnostics.len(), flagged.len(), "{line:?}");
    let line_end = line_start + u32::try_from(line.len()).unwrap();
    for (diagnostic, token) in diagnostics.iter().zip(flagged) {
        assert_eq!(diagnostic.span.file, FileId(3));
        assert_eq!(diagnostic.span.range, token.range.shifted(line_start));
        let suspect = matches!(token.kind, TokenKind::SuspectComment(_));
        assert_eq!(diagnostic.blocking, !suspect, "{line:?}");
        if suspect {
            assert_eq!(diagnostic.code, LEX_OPERANDS_IN_COMMENT);
            assert_eq!(diagnostic.args.len(), 2, "{line:?}");
            assert_eq!(diagnostic.args[1].0, "cause");
        }
        if let Some(suggestion) = &diagnostic.suggestion {
            let range = suggestion.span.range;
            assert!(
                line_start <= range.start && range.start <= range.end && range.end <= line_end,
                "{line:?}: {suggestion:?}"
            );
        }
    }
}

/// Calls `visit` with every string of length `length` over [`INTERESTING`].
fn for_each_string(length: u32, visit: &mut impl FnMut(&str)) {
    let base = INTERESTING.len();
    let count = base.pow(length);
    let mut text = String::new();
    for mut index in 0..count {
        text.clear();
        for _ in 0..length {
            text.push(INTERESTING[index % base]);
            index /= base;
        }
        visit(&text);
    }
}

#[test]
fn all_short_ascii_strings_are_covered() {
    let mut checked = 0;
    for length in 0..=3 {
        for_each_string(length, &mut |line| {
            check_line(line);
            checked += 1;
        });
    }
    assert_eq!(checked, 1 + 24 + 24 * 24 + 24 * 24 * 24);
}

/// A xorshift64 generator with a fixed seed, so the "random" lines are the same on every run.
struct XorShift(u64);

impl XorShift {
    /// Returns the next pseudo-random number.
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Returns a pseudo-random number below `bound`.
    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

/// Multi-byte chars mixed into the random lines (2, 3 and 4 bytes in UTF-8).
const MULTI_BYTE: [char; 4] = ['é', '日', '€', '😀'];

/// Builds the `count` pseudo-random lines: printable ASCII, tabs and multi-byte chars, with a
/// bias towards instruction-like text so that every field is reached.
fn random_lines(count: usize) -> Vec<String> {
    const WORDS: [&str; 12] = [
        "\tmove.l\t",
        "\tdc.b\t",
        " dc.b ",
        "lab",
        "\tinclude\t",
        "d0",
        ",",
        ", ",
        "(a0)",
        "'",
        "\\@",
        "\trts ",
    ];
    let mut random = XorShift(0x9E37_79B9_7F4A_7C15);
    let mut lines = Vec::with_capacity(count);
    for _ in 0..count {
        let length = random.below(48);
        let mut line = String::new();
        for _ in 0..length {
            match random.below(16) {
                0 => line.push('\t'),
                1 => line.push(MULTI_BYTE[usize::try_from(random.below(4)).unwrap()]),
                2 | 3 => line.push_str(WORDS[usize::try_from(random.below(12)).unwrap()]),
                _ => line.push(char::from(u8::try_from(0x20 + random.below(95)).unwrap())),
            }
        }
        lines.push(line);
    }
    lines
}

#[test]
fn random_lines_are_covered() {
    for line in random_lines(100_000) {
        check_line(&line);
    }
}

#[test]
fn lexing_is_stateless() {
    let lines = random_lines(2_000);
    let in_order: Vec<Vec<Token>> = lines.iter().map(|line| lex(line)).collect();
    let mut order: Vec<usize> = (0..lines.len()).collect();
    let mut random = XorShift(42);
    for index in (1..order.len()).rev() {
        let other = usize::try_from(random.below(u64::try_from(index + 1).unwrap())).unwrap();
        order.swap(index, other);
    }
    let mut out = Vec::new();
    let mut state = LineState::INITIAL;
    for &index in &order {
        out.clear();
        state = M68kSyntax.lex_line(&lines[index], state, &mut out);
        assert_eq!(out, in_order[index], "{:?}", lines[index]);
    }
}

#[test]
fn returned_state_is_always_initial() {
    for state in [LineState::INITIAL, LineState(1), LineState(u32::MAX)] {
        for line in ["", "x", "\tmove.l\td0,d1", "* c", "\tdc.b\t'open"] {
            let mut out = Vec::new();
            assert_eq!(
                M68kSyntax.lex_line(line, state, &mut out),
                LineState::INITIAL
            );
        }
    }
}

#[test]
fn empty_line_appends_nothing() {
    let mut out = Vec::new();
    va_arch_m68k::syntax::lex_line("", &mut out);
    assert!(out.is_empty());
}
