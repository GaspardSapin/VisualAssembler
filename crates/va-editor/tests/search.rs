//! Tests of `va_editor::search`: literal, case and whole-word queries, regex lines, empty matches,
//! the match and size limits, cancellation, `find_from` wrapping, templates and `replace_all`
//! (m4-editor-shell.md §3.4, §10.1).
#![allow(clippy::unwrap_used)]

use std::sync::atomic::AtomicBool;

use va_editor::search::{
    MAX_MATCHES, Matcher, Query, QueryError, REGEX_NEST_LIMIT, expand_replacement, find_all,
    find_from, replace_all,
};
use va_editor::text::Text;

/// A query of `pattern` with the given flags.
fn query(pattern: &str, regex: bool, case_sensitive: bool, whole_word: bool) -> Query {
    Query {
        pattern: pattern.to_owned(),
        regex,
        case_sensitive,
        whole_word,
    }
}

/// The compiled query.
fn matcher(pattern: &str, regex: bool, case_sensitive: bool, whole_word: bool) -> Matcher {
    Matcher::new(&query(pattern, regex, case_sensitive, whole_word)).unwrap()
}

/// Every match of `m` in `text`, as strings.
fn found<'a>(text: &'a str, m: &Matcher) -> Vec<&'a str> {
    let matches = find_all(&Text::new(text), m, &AtomicBool::new(false)).unwrap();
    assert!(!matches.truncated);
    matches
        .ranges
        .iter()
        .map(|&(start, end)| &text[start..end])
        .collect()
}

/// The ranges of every match of `m` in `text`.
fn ranges(text: &str, m: &Matcher) -> Vec<(usize, usize)> {
    find_all(&Text::new(text), m, &AtomicBool::new(false))
        .unwrap()
        .ranges
}

#[test]
fn matchers_are_clone_send_and_sync() {
    fn assert_traits<T: Clone + Send + Sync>() {}
    assert_traits::<Matcher>();
}

#[test]
fn literal_queries_are_escaped() {
    let m = matcher("d0.w", false, true, false);
    assert_eq!(ranges("move.l d0.w,d1\nadd d0xw", &m), [(7, 11)]);
    let m = matcher("(a0)+", false, true, false);
    assert_eq!(found("move.l (a0)+,d0", &m), ["(a0)+"]);
    assert_eq!(
        Matcher::new(&query("", false, true, false)).unwrap_err(),
        QueryError::Empty
    );
    assert_eq!(
        Matcher::new(&query("", true, true, false)).unwrap_err(),
        QueryError::Empty
    );
}

#[test]
fn case_insensitive_queries_fold_unicode_case() {
    let m = matcher("MOVE", false, false, false);
    assert_eq!(found("move Move MOVE", &m), ["move", "Move", "MOVE"]);
    let m = matcher("MOVE", false, true, false);
    assert_eq!(found("move Move MOVE", &m), ["MOVE"]);
    let m = matcher("É", false, false, false);
    assert_eq!(found("été", &m), ["é", "é"]);
}

#[test]
fn whole_words_include_dots_underscores_and_at_signs() {
    let m = matcher("move", false, true, true);
    assert_eq!(
        found("move move.l _move move_2 @move (move)", &m),
        ["move", "move"]
    );
    let m = matcher("d0", false, true, true);
    assert_eq!(ranges("move.l d0,d10", &m), [(7, 9)]);
    let m = matcher("loop", false, true, true);
    assert_eq!(found("loop:\n.loop\nloop", &m), ["loop", "loop"]);
    // A rejected match does not hide the next one.
    let m = matcher("a", false, true, true);
    assert_eq!(ranges("aa a", &m), [(3, 4)]);
}

#[test]
fn regex_queries_match_line_by_line() {
    let m = matcher(r"d[0-7]", true, true, false);
    assert_eq!(found("move.l d0,d1\nadd.w d7,d8", &m), ["d0", "d1", "d7"]);
    // `^` and `$` are line edges; nothing spans a line break.
    let m = matcher("^rts$", true, true, false);
    assert_eq!(ranges("rts\r\n  rts\nrts", &m), [(0, 3), (11, 14)]);
    let m = matcher(r"a\s+b", true, true, false);
    assert!(found("a\nb", &m).is_empty());
    let m = matcher("x.y", true, true, false);
    assert!(found("x\ry", &m).is_empty());
    let m = matcher(r"(?i)\bLoop\b", true, true, false);
    assert_eq!(found("loop LOOP", &m), ["loop", "LOOP"]);
}

#[test]
fn empty_matches_are_skipped() {
    let m = matcher("a*", true, true, false);
    assert_eq!(found("baaab\n\naa", &m), ["aaa", "aa"]);
    let m = matcher("^", true, true, false);
    assert!(found("a\nb", &m).is_empty());
}

#[test]
fn invalid_regexes_report_the_crate_message() {
    match Matcher::new(&query("(unclosed", true, true, false)) {
        Err(QueryError::Invalid { detail }) => assert!(detail.contains("unclosed"), "{detail}"),
        other => panic!("{other:?}"),
    }
    match Matcher::new(&query(r"\p{Greek}", true, true, false)) {
        Err(QueryError::Invalid { detail }) => assert!(!detail.is_empty()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn size_and_nest_limits_give_too_big() {
    assert_eq!(
        Matcher::new(&query("a{100000}", true, true, false)).unwrap_err(),
        QueryError::TooBig
    );
    assert_eq!(
        Matcher::new(&query(r"(\w{500}){500}", true, true, false)).unwrap_err(),
        QueryError::TooBig
    );
    let depth = usize::try_from(REGEX_NEST_LIMIT).unwrap() + 10;
    let nested = "(".repeat(depth) + "a" + &")".repeat(depth);
    assert_eq!(
        Matcher::new(&query(&nested, true, true, false)).unwrap_err(),
        QueryError::TooBig
    );
    let deep = "(".repeat(400) + "a" + &")".repeat(400);
    assert_eq!(
        Matcher::new(&query(&deep, true, true, false)).unwrap_err(),
        QueryError::TooBig
    );
    // Within the limits it compiles.
    let shallow = "(".repeat(20) + "a" + &")".repeat(20);
    assert!(Matcher::new(&query(&shallow, true, true, false)).is_ok());
}

#[test]
fn find_all_stops_at_max_matches() {
    let m = matcher("x", false, true, false);
    let exact = Text::new(&"x\n".repeat(MAX_MATCHES));
    let matches = find_all(&exact, &m, &AtomicBool::new(false)).unwrap();
    assert_eq!(matches.ranges.len(), MAX_MATCHES);
    assert!(!matches.truncated);
    let over = Text::new(&"xx".repeat(MAX_MATCHES));
    let matches = find_all(&over, &m, &AtomicBool::new(false)).unwrap();
    assert_eq!(matches.ranges.len(), MAX_MATCHES);
    assert!(matches.truncated);
    assert_eq!(matches.ranges.last(), Some(&(MAX_MATCHES - 1, MAX_MATCHES)));
}

#[test]
fn a_set_cancel_flag_returns_none() {
    let m = matcher("x", false, true, false);
    let text = Text::new(&"x\n".repeat(5_000));
    let cancel = AtomicBool::new(true);
    assert_eq!(find_all(&text, &m, &cancel), None);
    assert!(replace_all(&text, &m, "y", &cancel).is_none());
}

#[test]
fn find_from_moves_forward_and_wraps_once() {
    let source = "ab ab\nxx\nab";
    let text = Text::new(source);
    let m = matcher("ab", false, true, false);
    assert_eq!(find_from(&text, &m, 0, false), Some((0, 2)));
    assert_eq!(find_from(&text, &m, 1, false), Some((3, 5)));
    assert_eq!(find_from(&text, &m, 5, false), Some((9, 11)));
    assert_eq!(find_from(&text, &m, 10, false), Some((0, 2)));
    assert_eq!(find_from(&text, &m, 99, false), Some((0, 2)));
    // A single match wraps back to itself.
    let once = Text::new("xx ab xx");
    assert_eq!(find_from(&once, &m, 5, false), Some((3, 5)));
    assert_eq!(find_from(&once, &m, 4, false), Some((3, 5)));
    let none = matcher("zz", false, true, false);
    assert_eq!(find_from(&text, &none, 3, false), None);
}

#[test]
fn find_from_moves_backwards_and_wraps_once() {
    let text = Text::new("ab ab\nxx\nab");
    let m = matcher("ab", false, true, false);
    assert_eq!(find_from(&text, &m, 11, true), Some((9, 11)));
    assert_eq!(find_from(&text, &m, 9, true), Some((3, 5)));
    assert_eq!(find_from(&text, &m, 4, true), Some((0, 2)));
    assert_eq!(find_from(&text, &m, 1, true), Some((9, 11)));
    assert_eq!(find_from(&text, &m, 0, true), Some((9, 11)));
    let once = Text::new("xx ab xx");
    assert_eq!(find_from(&once, &m, 3, true), Some((3, 5)));
    assert_eq!(find_from(&once, &m, 4, true), Some((3, 5)));
}

#[test]
fn templates_expand_groups_names_and_dollars() {
    let m = matcher(r"(?<op>move)\.(\w)", true, true, false);
    let line = "  move.l d0,d1";
    assert_eq!(expand_replacement(&m, line, (2, 8), "$2:${op}"), "l:move");
    assert_eq!(expand_replacement(&m, line, (2, 8), "$$1"), "$1");
    assert_eq!(expand_replacement(&m, line, (2, 8), "${1}x"), "movex");
    // Out-of-range or misaligned offsets give the template as is.
    assert_eq!(expand_replacement(&m, line, (99, 100), "$1"), "$1");
    // Literal mode inserts the template as is.
    let literal = matcher("move", false, true, false);
    assert_eq!(expand_replacement(&literal, line, (2, 6), "$1 $$"), "$1 $$");
}

#[test]
fn replace_all_counts_and_keeps_terminators() {
    let source = "move.l d0,d1\r\nmove.w d2,d3\rnop\nmove.b d4,d5";
    let text = Text::new(source);
    let m = matcher(r"move\.(\w) (d\d)", true, true, false);
    let (replaced, count) = replace_all(&text, &m, "MOVE.$1 $2", &AtomicBool::new(false)).unwrap();
    assert_eq!(count, 3);
    assert_eq!(
        replaced.to_string(),
        "MOVE.l d0,d1\r\nMOVE.w d2,d3\rnop\nMOVE.b d4,d5"
    );
    let literal = matcher("d", false, true, true);
    let (unchanged, count) = replace_all(&text, &literal, "x", &AtomicBool::new(false)).unwrap();
    assert_eq!(count, 0);
    assert_eq!(unchanged, text);
    let literal = matcher("D", false, false, false);
    let (replaced, count) = replace_all(&text, &literal, "$0", &AtomicBool::new(false)).unwrap();
    assert_eq!(count, 6);
    assert!(replaced.to_string().starts_with("move.l $00,$01\r\n"));
    // Whole-word filtering applies to regex replacements too: `\w` stops at `.`, but `.` is a
    // word char for the whole-word test, so `a` and `b` are rejected.
    let words = matcher(r"\w+", true, true, true);
    let (replaced, count) =
        replace_all(&Text::new("a.b c"), &words, "W", &AtomicBool::new(false)).unwrap();
    assert_eq!((replaced.to_string().as_str(), count), ("a.b W", 1));
}

#[test]
fn replace_all_stops_at_max_matches() {
    let m = matcher("x", false, true, false);
    let text = Text::new(&"x".repeat(MAX_MATCHES + 5));
    let (replaced, count) = replace_all(&text, &m, "y", &AtomicBool::new(false)).unwrap();
    assert_eq!(count, MAX_MATCHES);
    let replaced = replaced.to_string();
    assert_eq!(replaced.matches('y').count(), MAX_MATCHES);
    assert!(replaced.ends_with("yxxxxx"));
}
