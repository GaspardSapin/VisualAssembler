//! `va lex <FILE|->`: splits a source into lines, lexes each one with the default architecture's
//! lexer and reports its diagnostics (m1-core-isa-lexer.md §7.3).
//!
//! - **Text**: stdout gets one line per non-whitespace token, `<line>:<column> <kind> <text as a
//!   JSON string>`, plus ` <code>` for `invalid` and `suspect-comment` tokens. stderr gets the
//!   diagnostics, GNU style: `<file>:<line>:<column>: error[<code>]: <message>`, then
//!   `  help: …`, `  note: …` and `  fix: <the line with the fix applied>` when present.
//! - **JSON** ([`LEX_SCHEMA`]): one line, written incrementally and never held whole. The lines
//!   and their tokens (whitespace included) are streamed in a first pass; the diagnostics come
//!   from a second, identical lexing pass, so memory stays bounded by one line whatever the
//!   number of diagnostics.
//!
//! Exit code 0 without error diagnostic, 1 with at least one, blocking or not (§5.7 makes
//! `dc.b 1, 2, 3` a non-blocking error, hence exit 1). Columns count chars and are 1-based; token
//! `start`/`end` are line-relative bytes, line `offset` and diagnostic `start`/`end` file bytes.

use std::ffi::OsStr;
use std::io::{self, BufWriter, Write};

use serde::Serialize;
use va_core::diag::{DiagCode, Diagnostic, Severity, Suggestion};
use va_core::source::{FileId, lines};
use va_core::syntax::{LineState, SyntaxProvider, Token, TokenKind};
use va_workspace::i18n::{Localizer, RenderedDiagnostic};

use crate::commands::Session;
use crate::input::{self, Input};
use crate::report::{CommandError, Outcome};

/// Schema identifier of the JSON document printed by `va lex --json`.
pub const LEX_SCHEMA: &str = "va.lex/1";

/// The file id of the single input (M1 lexes one file; M2's assembler numbers its includes).
const FILE: FileId = FileId(0);

/// Runs `va lex` on the input named by `file` (a path, or `-` for stdin).
///
/// # Errors
///
/// The input errors of [`input::read`] (exit 3, before any output), or a stdout or stderr write
/// failure (exit 3).
pub fn run(session: &Session, file: &OsStr, json: bool) -> Result<Outcome, CommandError> {
    let input = input::read(file)?;
    let errors = if json {
        let mut stdout = BufWriter::new(io::stdout().lock());
        write_json(&mut stdout, session, &input)
            .and_then(|errors| stdout.flush().map(|()| errors))
            .map_err(CommandError::Stdout)?
    } else {
        write_text(session, &input)?
    };
    Ok(if errors == 0 {
        Outcome::Clean
    } else {
        Outcome::ProblemsFound
    })
}

/// One lexed line, as handed to the writers.
struct LexedLine<'a> {
    /// 1-based line number.
    number: u32,
    /// File offset of the first byte of the line.
    start: u32,
    /// The line content, without terminator.
    text: &'a str,
    /// Its tokens (line-relative ranges), covering the whole line.
    tokens: &'a [Token],
    /// Its diagnostics (file offsets).
    diagnostics: &'a [Diagnostic],
}

/// Lexes every line of `text` in order and calls `visit` with each one.
///
/// The token and diagnostic buffers are reused from line to line, and the lexer state is carried
/// from one line to the next as the `SyntaxProvider` contract requires (A68k always returns the
/// initial state).
fn for_each_line<E>(
    syntax: &dyn SyntaxProvider,
    text: &str,
    mut visit: impl FnMut(&LexedLine<'_>) -> Result<(), E>,
) -> Result<(), E> {
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut state = LineState::INITIAL;
    for (index, (range, line)) in lines(text).enumerate() {
        tokens.clear();
        diagnostics.clear();
        state = syntax.lex_line(line, state, &mut tokens);
        syntax.line_diagnostics(FILE, range.start, line, &tokens, &mut diagnostics);
        visit(&LexedLine {
            // A 16 MiB input has far fewer than 2^32 lines.
            number: u32::try_from(index + 1).unwrap_or(u32::MAX),
            start: range.start,
            text: line,
            tokens: &tokens,
            diagnostics: &diagnostics,
        })?;
    }
    Ok(())
}

/// The code carried by an `invalid` or `suspect-comment` token.
fn token_code(kind: TokenKind) -> Option<DiagCode> {
    match kind {
        TokenKind::Invalid(code) | TokenKind::SuspectComment(code) => Some(code),
        _ => None,
    }
}

/// The 1-based char column of the line-relative byte `offset` (clamped to the line, rounded down
/// to a char boundary).
fn column_at(line: &str, offset: u32) -> u32 {
    let mut end = usize::try_from(offset)
        .unwrap_or(usize::MAX)
        .min(line.len());
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    let chars = line.get(..end).map_or(0, |prefix| prefix.chars().count());
    u32::try_from(chars + 1).unwrap_or(u32::MAX)
}

/// The line-relative byte offset of the file offset `offset` on `line` (0 if it lies before).
fn relative(line: &LexedLine<'_>, offset: u32) -> u32 {
    offset.saturating_sub(line.start)
}

/// The text of `line` with `suggestion` applied, or `None` if the suggestion does not lie on
/// char boundaries of this line.
fn fixed_line(line: &LexedLine<'_>, suggestion: &Suggestion) -> Option<String> {
    let start = usize::try_from(suggestion.span.range.start.checked_sub(line.start)?).ok()?;
    let end = usize::try_from(suggestion.span.range.end.checked_sub(line.start)?).ok()?;
    let before = line.text.get(..start)?;
    let after = line.text.get(end..)?;
    Some(format!("{before}{}{after}", suggestion.replacement))
}

/// Writes the text form: token lines on stdout, diagnostics on stderr. Returns the number of
/// error diagnostics.
///
/// Both streams are buffered; before a line's diagnostics, stdout is flushed, and stderr after
/// them, so a terminal showing both gets each diagnostic right after the tokens of its line.
fn write_text(session: &Session, input: &Input) -> Result<u64, CommandError> {
    let syntax = session.arch.syntax();
    let mut stdout = BufWriter::new(io::stdout().lock());
    let mut stderr = BufWriter::new(io::stderr().lock());
    let mut errors = 0_u64;
    for_each_line(syntax, input.text.as_str(), |line| {
        write_token_lines(&mut stdout, line).map_err(CommandError::Stdout)?;
        if line.diagnostics.is_empty() {
            return Ok(());
        }
        stdout.flush().map_err(CommandError::Stdout)?;
        for diagnostic in line.diagnostics {
            if diagnostic.severity == Severity::Error {
                errors += 1;
            }
            write_diagnostic_text(
                &mut stderr,
                &session.localizer,
                &input.name,
                line,
                diagnostic,
            )
            .map_err(CommandError::Stderr)?;
        }
        stderr.flush().map_err(CommandError::Stderr)
    })?;
    stdout.flush().map_err(CommandError::Stdout)?;
    stderr.flush().map_err(CommandError::Stderr)?;
    Ok(errors)
}

/// Writes `<line>:<column> <kind> <text>[ <code>]` for every non-whitespace token of `line`.
fn write_token_lines(out: &mut impl Write, line: &LexedLine<'_>) -> io::Result<()> {
    let mut column = 1_u32;
    for token in line.tokens {
        let text = token.range.slice(line.text).unwrap_or("");
        if token.kind != TokenKind::Whitespace {
            write!(out, "{}:{column} {} ", line.number, token.kind.name())?;
            serde_json::to_writer(&mut *out, text)?;
            if let Some(code) = token_code(token.kind) {
                write!(out, " {}", code.0)?;
            }
            out.write_all(b"\n")?;
        }
        column = column.saturating_add(u32::try_from(text.chars().count()).unwrap_or(u32::MAX));
    }
    Ok(())
}

/// Writes one diagnostic in the GNU style, with its help, note and fixed line when present.
fn write_diagnostic_text(
    out: &mut impl Write,
    localizer: &Localizer,
    file: &str,
    line: &LexedLine<'_>,
    diagnostic: &Diagnostic,
) -> io::Result<()> {
    let RenderedDiagnostic {
        message,
        help,
        note,
    } = localizer.render(diagnostic);
    let column = column_at(line.text, relative(line, diagnostic.span.range.start));
    writeln!(
        out,
        "{file}:{}:{column}: {}[{}]: {message}",
        line.number,
        diagnostic.severity.id(),
        diagnostic.code.0
    )?;
    if let Some(help) = help {
        writeln!(out, "  help: {help}")?;
    }
    if let Some(note) = note {
        writeln!(out, "  note: {note}")?;
    }
    if let Some(fixed) = diagnostic
        .suggestion
        .as_ref()
        .and_then(|suggestion| fixed_line(line, suggestion))
    {
        writeln!(out, "  fix: {fixed}")?;
    }
    Ok(())
}

/// One token of `va.lex/1`. Field order is the JSON key order.
#[derive(Serialize)]
struct TokenJson<'a> {
    /// `TokenKind::name`.
    kind: &'static str,
    /// Line-relative start byte.
    start: u32,
    /// Line-relative end byte.
    end: u32,
    /// 1-based char column of `start`.
    column: u32,
    /// The token text.
    text: &'a str,
    /// The code of an `invalid` or `suspect-comment` token; absent for every other kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'static str>,
}

/// The fix of a diagnostic in `va.lex/1` (file bytes).
#[derive(Serialize)]
struct FixJson<'a> {
    /// File offset of the first replaced byte.
    start: u32,
    /// File offset after the last replaced byte.
    end: u32,
    /// The replacement text.
    replacement: &'a str,
}

/// One diagnostic of `va.lex/1`. Field order is the JSON key order.
#[derive(Serialize)]
struct DiagnosticJson<'a> {
    /// The diagnostic code.
    code: &'static str,
    /// `Severity::id`.
    severity: &'static str,
    /// Whether it would prevent the program image.
    blocking: bool,
    /// 1-based line of `start`.
    line: u32,
    /// 1-based char column of `start`.
    column: u32,
    /// File offset of the first byte.
    start: u32,
    /// File offset after the last byte.
    end: u32,
    /// The rendered message.
    message: &'a str,
    /// The rendered `.help`, or `null`.
    help: Option<&'a str>,
    /// The rendered `.note`, or `null`.
    note: Option<&'a str>,
    /// The machine-applicable fix, or `null`.
    fix: Option<FixJson<'a>>,
}

/// The `summary` object of `va.lex/1`.
#[derive(Serialize)]
struct SummaryJson {
    /// Number of lines (an empty input has one).
    lines: u64,
    /// Number of tokens, whitespace included.
    tokens: u64,
    /// Number of error diagnostics, blocking or not.
    errors: u64,
}

/// Writes `value` as compact JSON.
fn write_value<T: Serialize + ?Sized>(out: &mut impl Write, value: &T) -> io::Result<()> {
    serde_json::to_writer(out, value).map_err(io::Error::from)
}

/// Writes the whole `va.lex/1` line and returns the number of error diagnostics.
fn write_json(out: &mut impl Write, session: &Session, input: &Input) -> io::Result<u64> {
    let syntax = session.arch.syntax();
    let text = input.text.as_str();

    out.write_all(b"{\"schema\":")?;
    write_value(out, LEX_SCHEMA)?;
    out.write_all(b",\"arch\":")?;
    write_value(out, session.arch.id())?;
    out.write_all(b",\"lang\":")?;
    write_value(out, session.localizer.lang().code())?;
    out.write_all(b",\"file\":")?;
    write_value(out, &input.name)?;

    out.write_all(b",\"lines\":[")?;
    let mut line_count = 0_u64;
    let mut token_count = 0_u64;
    for_each_line(syntax, text, |line| {
        if line_count > 0 {
            out.write_all(b",")?;
        }
        line_count += 1;
        write!(
            out,
            "{{\"line\":{},\"offset\":{},\"tokens\":[",
            line.number, line.start
        )?;
        let mut column = 1_u32;
        for (index, token) in line.tokens.iter().enumerate() {
            if index > 0 {
                out.write_all(b",")?;
            }
            let token_text = token.range.slice(line.text).unwrap_or("");
            write_value(
                out,
                &TokenJson {
                    kind: token.kind.name(),
                    start: token.range.start,
                    end: token.range.end,
                    column,
                    text: token_text,
                    code: token_code(token.kind).map(|code| code.0),
                },
            )?;
            column = column
                .saturating_add(u32::try_from(token_text.chars().count()).unwrap_or(u32::MAX));
        }
        token_count += line.tokens.len() as u64;
        out.write_all(b"]}")
    })?;

    // Second pass: the lexer is deterministic, so re-lexing gives the same diagnostics without
    // holding them all.
    out.write_all(b"],\"diagnostics\":[")?;
    let mut error_count = 0_u64;
    let mut first = true;
    for_each_line(syntax, text, |line| {
        for diagnostic in line.diagnostics {
            if !first {
                out.write_all(b",")?;
            }
            first = false;
            if diagnostic.severity == Severity::Error {
                error_count += 1;
            }
            let rendered = session.localizer.render(diagnostic);
            let range = diagnostic.span.range;
            write_value(
                out,
                &DiagnosticJson {
                    code: diagnostic.code.0,
                    severity: diagnostic.severity.id(),
                    blocking: diagnostic.blocking,
                    line: line.number,
                    column: column_at(line.text, relative(line, range.start)),
                    start: range.start,
                    end: range.end,
                    message: &rendered.message,
                    help: rendered.help.as_deref(),
                    note: rendered.note.as_deref(),
                    fix: diagnostic.suggestion.as_ref().map(|suggestion| FixJson {
                        start: suggestion.span.range.start,
                        end: suggestion.span.range.end,
                        replacement: &suggestion.replacement,
                    }),
                },
            )?;
        }
        Ok::<(), io::Error>(())
    })?;

    out.write_all(b"],\"summary\":")?;
    write_value(
        out,
        &SummaryJson {
            lines: line_count,
            tokens: token_count,
            errors: error_count,
        },
    )?;
    out.write_all(b"}\n")?;
    Ok(error_count)
}
