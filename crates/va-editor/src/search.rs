//! Find and replace, line by line, with a linear-time regex engine (m4-editor-shell.md §3.4).
//!
//! Each line's content (no terminator) is searched on its own: no match spans a line break,
//! `^` and `$` are line edges, and empty matches are skipped. A literal query is escaped; a
//! case-insensitive one uses Unicode case folding; a whole-word one keeps a match only when the
//! chars around it are not [`is_word_char`]. The `regex` crate runs in linear time and every
//! compiled pattern is bounded by [`REGEX_SIZE_LIMIT`] and [`REGEX_NEST_LIMIT`], so no input can
//! hang a search. These functions run on the `va-search` worker, never on the UI thread.

use std::borrow::Cow;
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicBool, Ordering};

use regex::{Regex, RegexBuilder};
use ropey::RopeBuilder;

use crate::edit::is_word_char;
use crate::text::Text;

/// A search request.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Query {
    /// The pattern: a literal text, or a regex when `regex`.
    pub pattern: String,
    /// Whether `pattern` is a regular expression.
    pub regex: bool,
    /// Whether case matters.
    pub case_sensitive: bool,
    /// Whether a match must be a whole word.
    pub whole_word: bool,
}

/// Why [`Matcher::new`] rejected a query.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum QueryError {
    /// The pattern is empty.
    Empty,
    /// The regex does not parse.
    Invalid {
        /// The regex crate's message, shown raw.
        detail: String,
    },
    /// The compiled regex exceeds a size or nesting limit.
    TooBig,
}

/// A compiled query: `Clone + Send + Sync`.
#[derive(Clone, Debug)]
pub struct Matcher {
    /// The compiled pattern.
    regex: Regex,
    /// Whether the query was literal (its replacement template is inserted as is).
    literal: bool,
    /// Whether a match must be a whole word.
    whole_word: bool,
}

/// The `regex` crate's own default nest limit: a pattern that fails at
/// [`REGEX_NEST_LIMIT`] is compiled again with it to tell a nesting failure (another message)
/// from a syntax error (the same message).
const NEST_PROBE_LIMIT: u32 = 250;

/// `find_all` and `replace_all` check their cancel flag every this many lines.
const CANCEL_EVERY_LINES: usize = 1_024;

impl Matcher {
    /// Compiles `query` within [`REGEX_SIZE_LIMIT`] and [`REGEX_NEST_LIMIT`].
    ///
    /// # Errors
    /// A [`QueryError`] for an empty, invalid or too big pattern.
    pub fn new(query: &Query) -> Result<Self, QueryError> {
        if query.pattern.is_empty() {
            return Err(QueryError::Empty);
        }
        let pattern = if query.regex {
            Cow::Borrowed(query.pattern.as_str())
        } else {
            Cow::Owned(regex::escape(&query.pattern))
        };
        let build = |nest_limit: u32| {
            RegexBuilder::new(&pattern)
                .case_insensitive(!query.case_sensitive)
                .size_limit(REGEX_SIZE_LIMIT)
                .dfa_size_limit(2 * REGEX_SIZE_LIMIT)
                .nest_limit(nest_limit)
                .build()
        };
        match build(REGEX_NEST_LIMIT) {
            Ok(regex) => Ok(Self {
                regex,
                literal: !query.regex,
                whole_word: query.whole_word,
            }),
            Err(regex::Error::CompiledTooBig(_)) => Err(QueryError::TooBig),
            Err(regex::Error::Syntax(detail)) => match build(NEST_PROBE_LIMIT) {
                Err(regex::Error::Syntax(again)) if again == detail => {
                    Err(QueryError::Invalid { detail })
                }
                _ => Err(QueryError::TooBig),
            },
            Err(other) => Err(QueryError::Invalid {
                detail: other.to_string(),
            }),
        }
    }

    /// The kept matches of one line's content, line-relative, in order.
    fn line_matches<'a>(&'a self, line: &'a str) -> impl Iterator<Item = (usize, usize)> + 'a {
        self.regex
            .find_iter(line)
            .map(|found| (found.start(), found.end()))
            .filter(move |&(start, end)| self.keeps(line, start, end))
    }

    /// Tells whether the match `start..end` of `line` counts: non-empty and, for a whole-word
    /// query, not touching word chars.
    fn keeps(&self, line: &str, start: usize, end: usize) -> bool {
        start < end && (!self.whole_word || is_whole_word(line, start, end))
    }
}

/// Tells whether neither the char before `start` nor the char at `end` of `text` is a word
/// char ([`is_word_char`]).
pub(crate) fn is_whole_word(text: &str, start: usize, end: usize) -> bool {
    let before = text.get(..start).and_then(|head| head.chars().next_back());
    let after = text.get(end..).and_then(|tail| tail.chars().next());
    !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char)
}

/// Most matches collected by one search.
pub const MAX_MATCHES: usize = 100_000;

/// Size limit of a compiled regex, in bytes (the DFA gets twice as much).
pub const REGEX_SIZE_LIMIT: usize = 1 << 20;

/// Nesting limit of a regex.
pub const REGEX_NEST_LIMIT: u32 = 64;

/// The matches of a search.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Matches {
    /// Byte ranges of the matches, in order.
    pub ranges: Vec<(usize, usize)>,
    /// Whether the search stopped at [`MAX_MATCHES`].
    pub truncated: bool,
}

/// Every match of `m` in `text` (at most [`MAX_MATCHES`]); `None` when `cancel` was set
/// (checked every 1,024 lines, the first line included).
pub fn find_all(text: &Text, m: &Matcher, cancel: &AtomicBool) -> Option<Matches> {
    let mut matches = Matches::default();
    let stopped = text.scan_lines(0, |line| {
        if line.index % CANCEL_EVERY_LINES == 0 && cancel.load(Ordering::Relaxed) {
            return ControlFlow::Break(true);
        }
        for (start, end) in m.line_matches(line.content) {
            if matches.ranges.len() == MAX_MATCHES {
                matches.truncated = true;
                return ControlFlow::Break(false);
            }
            matches.ranges.push((line.start + start, line.start + end));
        }
        ControlFlow::Continue(())
    });
    match stopped {
        Some(true) => None,
        _ => Some(matches),
    }
}

/// The next match from `from` (the first one starting at or after it), or when `backwards` the
/// previous one (the last one ending at or before it), wrapping once around the document.
pub fn find_from(text: &Text, m: &Matcher, from: usize, backwards: bool) -> Option<(usize, usize)> {
    let from = text.floor_boundary(from);
    let first = text.line_of(from);
    let relative = from - text.line_start(first);
    let line_count = text.line_count();
    let mut buf = String::new();
    let mut search = |line: usize, keep: &dyn Fn(usize, usize) -> bool| {
        let start = text.line_start(line);
        let content = text.line_content(line, &mut buf);
        let mut kept = m
            .line_matches(content)
            .filter(|&(match_start, match_end)| keep(match_start, match_end));
        let found = if backwards { kept.last() } else { kept.next() };
        found.map(|(match_start, match_end)| (start + match_start, start + match_end))
    };
    if backwards {
        search(first, &|_, end| end <= relative)
            .or_else(|| (0..first).rev().find_map(|line| search(line, &|_, _| true)))
            .or_else(|| {
                (first + 1..line_count)
                    .rev()
                    .find_map(|line| search(line, &|_, _| true))
            })
            .or_else(|| search(first, &|_, end| end > relative))
    } else {
        search(first, &|start, _| start >= relative)
            .or_else(|| (first + 1..line_count).find_map(|line| search(line, &|_, _| true)))
            .or_else(|| (0..first).find_map(|line| search(line, &|_, _| true)))
            .or_else(|| search(first, &|start, _| start < relative))
    }
}

/// The replacement of the match `found` (line-relative offsets) of `line` by `template`:
/// `$1`, `${name}` and `$$` expanded in regex mode (the `regex` crate's rules); the template as
/// is in literal mode, or when `found` is not a match start of `line`.
pub fn expand_replacement(
    m: &Matcher,
    line: &str,
    found: (usize, usize),
    template: &str,
) -> String {
    let (start, _) = found;
    if m.literal || start > line.len() || !line.is_char_boundary(start) {
        return template.to_owned();
    }
    match m.regex.captures_at(line, start) {
        Some(captures) => {
            let mut out = String::new();
            captures.expand(template, &mut out);
            out
        }
        None => template.to_owned(),
    }
}

/// The text with every match replaced (at most [`MAX_MATCHES`], later matches left as they
/// are), and the number of replacements; `None` when `cancel` was set (checked as in
/// [`find_all`]). Line terminators are kept verbatim. With no match, the text is an O(1) clone.
pub fn replace_all(
    text: &Text,
    m: &Matcher,
    template: &str,
    cancel: &AtomicBool,
) -> Option<(Text, usize)> {
    let mut builder = RopeBuilder::new();
    let mut count = 0;
    let mut out = String::new();
    let cancelled = text.scan_lines(0, |line| {
        if line.index % CANCEL_EVERY_LINES == 0 && cancel.load(Ordering::Relaxed) {
            return ControlFlow::Break(());
        }
        let content = line.content;
        out.clear();
        let mut copied = 0;
        let mut replaced = 0;
        if m.literal {
            for (start, end) in m.line_matches(content) {
                if count == MAX_MATCHES {
                    break;
                }
                out.push_str(&content[copied..start]);
                out.push_str(template);
                copied = end;
                count += 1;
                replaced += 1;
            }
        } else {
            for captures in m.regex.captures_iter(content) {
                let Some(whole) = captures.get(0) else {
                    continue;
                };
                if !m.keeps(content, whole.start(), whole.end()) {
                    continue;
                }
                if count == MAX_MATCHES {
                    break;
                }
                out.push_str(&content[copied..whole.start()]);
                captures.expand(template, &mut out);
                copied = whole.end();
                count += 1;
                replaced += 1;
            }
        }
        if replaced == 0 {
            builder.append(content);
        } else {
            out.push_str(&content[copied..]);
            builder.append(&out);
        }
        builder.append(line.terminator);
        ControlFlow::Continue(())
    });
    if cancelled.is_some() {
        return None;
    }
    if count == 0 {
        return Some((text.clone(), 0));
    }
    Some((Text::from_rope(builder.finish()), count))
}
