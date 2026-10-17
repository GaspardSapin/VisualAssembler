//! Tests for `src/diag.rs`: `DiagCode` validation and Fluent ids, `Severity` ids, and the
//! `Diagnostic` builder with its blocking rule.
#![allow(clippy::unwrap_used)]

use va_core::diag::{DiagArg, DiagCode, Diagnostic, Related, Severity, Suggestion};
use va_core::source::{FileId, SourceSpan, TextRange};

/// A span of file 3, bytes `start..end`.
fn span(start: u32, end: u32) -> SourceSpan {
    SourceSpan {
        file: FileId(3),
        range: TextRange::new(start, end),
    }
}

#[test]
fn kebab_case_codes_are_valid() {
    for code in [
        "a",
        "7",
        "lex-bad-number",
        "lex-operands-in-comment",
        "m68k",
        "a1-2b-c3",
    ] {
        assert!(DiagCode(code).is_valid(), "{code:?}");
    }
}

#[test]
fn other_codes_are_invalid() {
    for code in [
        "", "-", "-a", "a-", "a--b", "Lex-bad", "lex_bad", "lex bad", "lex.bad", "é", "diag-",
        "a-\u{0}",
    ] {
        assert!(!DiagCode(code).is_valid(), "{code:?}");
    }
}

#[test]
fn fluent_id_prefixes_diag() {
    assert_eq!(
        DiagCode("lex-bad-number").fluent_id(),
        "diag-lex-bad-number"
    );
}

#[test]
fn codes_order_by_text() {
    assert!(DiagCode("lex-a") < DiagCode("lex-b"));
}

#[test]
fn severity_ids() {
    assert_eq!(Severity::Error.id(), "error");
    assert_eq!(Severity::Warning.id(), "warning");
    assert_eq!(Severity::Note.id(), "note");
}

#[test]
fn new_errors_are_blocking() {
    let diagnostic = Diagnostic::new(DiagCode("lex-bad-number"), Severity::Error, span(4, 6));
    assert_eq!(
        diagnostic,
        Diagnostic {
            code: DiagCode("lex-bad-number"),
            severity: Severity::Error,
            span: span(4, 6),
            blocking: true,
            args: Vec::new(),
            related: Vec::new(),
            suggestion: None,
        }
    );
}

#[test]
fn new_warnings_and_notes_are_not_blocking() {
    for severity in [Severity::Warning, Severity::Note] {
        let diagnostic = Diagnostic::new(DiagCode("asm-x"), severity, span(0, 1));
        assert!(!diagnostic.blocking, "{severity:?}");
        assert_eq!(diagnostic.severity, severity);
    }
}

#[test]
fn non_blocking_keeps_the_severity() {
    let diagnostic = Diagnostic::new(
        DiagCode("lex-operands-in-comment"),
        Severity::Error,
        span(0, 1),
    )
    .non_blocking();
    assert!(!diagnostic.blocking);
    assert_eq!(diagnostic.severity, Severity::Error);
    // Idempotent, also on a diagnostic that already was non-blocking.
    let note = Diagnostic::new(DiagCode("n"), Severity::Note, span(0, 0)).non_blocking();
    assert!(!note.blocking);
}

#[test]
fn with_arg_appends_in_order() {
    let diagnostic = Diagnostic::new(DiagCode("lex-bad-number"), Severity::Error, span(1, 2))
        .with_arg("text", DiagArg::Str("$".to_owned()))
        .with_arg("bytes", DiagArg::Int(-3))
        .with_arg("text", DiagArg::Str("again".to_owned()));
    assert_eq!(
        diagnostic.args,
        vec![
            ("text", DiagArg::Str("$".to_owned())),
            ("bytes", DiagArg::Int(-3)),
            ("text", DiagArg::Str("again".to_owned())),
        ]
    );
    assert!(diagnostic.blocking);
}

#[test]
fn related_and_suggestion_are_plain_data() {
    let mut diagnostic = Diagnostic::new(DiagCode("asm-duplicate"), Severity::Error, span(10, 14));
    diagnostic.related.push(Related {
        span: span(0, 4),
        code: DiagCode("asm-first-definition"),
        args: vec![("name", DiagArg::Str("loop".to_owned()))],
    });
    diagnostic.suggestion = Some(Suggestion {
        span: span(10, 14),
        replacement: "loop2".to_owned(),
    });
    let copy = diagnostic.clone();
    assert_eq!(copy, diagnostic);
    assert_eq!(
        copy.related[0].code.fluent_id(),
        "diag-asm-first-definition"
    );
    assert_eq!(copy.suggestion.unwrap().replacement, "loop2");
}
