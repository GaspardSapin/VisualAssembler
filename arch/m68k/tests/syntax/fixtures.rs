//! Golden token files of m1-core-isa-lexer.md §5.5: each `fixtures/lex/<name>.asm` lexes to
//! its `<name>.tokens`, one token per line as `<line>:<start>..<end> <kind>[ <code>] <text as a
//! JSON string>`, whitespace tokens included. `VA_BLESS=1 cargo test -p va-arch-m68k --test
//! syntax` rewrites the `.tokens` files; the diff is reviewed in git.
#![allow(clippy::unwrap_used)]

use std::fmt::Write as _;

use va_core::source::lines;
use va_core::syntax::TokenKind;

use crate::{FIXTURES, fixture_dir, fixture_source, lex};

/// Returns `text` as a JSON string literal (RFC 8259): quotes, backslashes and control chars
/// escaped, other chars kept as UTF-8.
fn json_string(text: &str) -> String {
    let mut json = String::with_capacity(text.len() + 2);
    json.push('"');
    for char in text.chars() {
        match char {
            '"' => json.push_str("\\\""),
            '\\' => json.push_str("\\\\"),
            '\n' => json.push_str("\\n"),
            '\r' => json.push_str("\\r"),
            '\t' => json.push_str("\\t"),
            '\u{8}' => json.push_str("\\b"),
            '\u{c}' => json.push_str("\\f"),
            control if u32::from(control) < 0x20 => {
                write!(json, "\\u{:04x}", u32::from(control)).unwrap();
            }
            other => json.push(other),
        }
    }
    json.push('"');
    json
}

/// Renders the tokens of a whole source text in the §5.5 format, lines numbered from 1.
fn render(source: &str) -> String {
    let mut rendered = String::new();
    for (index, (_, line)) in lines(source).enumerate() {
        for token in lex(line) {
            let code = match token.kind {
                TokenKind::Invalid(code) | TokenKind::SuspectComment(code) => {
                    format!(" {}", code.0)
                }
                _ => String::new(),
            };
            writeln!(
                rendered,
                "{}:{}..{} {}{} {}",
                index + 1,
                token.range.start,
                token.range.end,
                token.kind.name(),
                code,
                json_string(token.range.slice(line).unwrap())
            )
            .unwrap();
        }
    }
    rendered
}

#[test]
fn json_string_escapes() {
    assert_eq!(json_string("a\"b\\c"), r#""a\"b\\c""#);
    assert_eq!(
        json_string("\t\r\n\u{0}\u{1f}é"),
        "\"\\t\\r\\n\\u0000\\u001fé\""
    );
}

#[test]
fn fixtures_match_their_tokens() {
    let bless = std::env::var_os("VA_BLESS").is_some_and(|value| value == "1");
    let mut mismatches = Vec::new();
    for name in FIXTURES {
        let source = fixture_source(name);
        assert!(
            !source.contains('\r'),
            "{name}.asm must be LF-only (m1 §2.1)"
        );
        let rendered = render(&source);
        let path = fixture_dir().join(format!("{name}.tokens"));
        if bless {
            std::fs::write(&path, &rendered).unwrap();
            continue;
        }
        let expected = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error} (run with VA_BLESS=1)", path.display()));
        if rendered != expected {
            let first = rendered
                .lines()
                .zip(expected.lines())
                .position(|(got, want)| got != want)
                .unwrap_or_else(|| rendered.lines().count().min(expected.lines().count()));
            mismatches.push(format!(
                "{name}.tokens differs at token line {}: got {:?}, expected {:?}",
                first + 1,
                rendered.lines().nth(first),
                expected.lines().nth(first)
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{}\nreview, then rerun with VA_BLESS=1",
        mismatches.join("\n")
    );
}
