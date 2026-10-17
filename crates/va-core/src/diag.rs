//! Diagnostics: stable kebab-case codes with typed Fluent arguments, never prose
//! (m1-core-isa-lexer.md §3.3, architecture §3 and §5).
//!
//! A [`Diagnostic`] separates how a problem is shown ([`Severity`]) from whether it stops the
//! output ([`Diagnostic::blocking`]): M2 produces a program image iff no diagnostic is blocking.
//! The text is rendered by the `va-workspace` localizer from the Fluent message
//! [`DiagCode::fluent_id`].

use crate::source::SourceSpan;

/// A stable diagnostic code, kebab-case (`[a-z0-9]+(-[a-z0-9]+)*`), e.g. `lex-bad-number`.
///
/// The code doubles as the Fluent message id `diag-<code>`. CPU crates list every code they can
/// emit through `Architecture::diagnostic_codes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DiagCode(pub &'static str);

impl DiagCode {
    /// Tells whether the code is kebab-case: one or more groups of `a-z`/`0-9`, joined by single
    /// hyphens, with no leading or trailing hyphen.
    pub fn is_valid(self) -> bool {
        !self.0.is_empty()
            && self.0.split('-').all(|group| {
                !group.is_empty()
                    && group
                        .bytes()
                        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            })
    }

    /// Returns the Fluent message id of the code: `diag-` followed by the code.
    pub fn fluent_id(self) -> String {
        format!("diag-{}", self.0)
    }
}

/// How a diagnostic is shown (underline colour, Problems panel group, CLI label).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Severity {
    /// A problem the user must fix; blocking by default.
    Error,
    /// A probable mistake that does not stop the output.
    Warning,
    /// Extra information.
    Note,
}

impl Severity {
    /// Returns the identifier of the severity: `"error"`, `"warning"` or `"note"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

/// A typed Fluent argument of a diagnostic.
///
/// There is no hexadecimal or address variant: number notation is CPU-specific (`$1F` versus
/// `0x1F`), so the producer passes an already formatted [`DiagArg::Str`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DiagArg {
    /// An integer, which Fluent can use for plural selection.
    Int(i64),
    /// A text, such as an excerpt of the source or a formatted number.
    Str(String),
}

/// One problem found in a source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// What the problem is; selects the Fluent message.
    pub code: DiagCode,
    /// How the problem is shown.
    pub severity: Severity,
    /// Where the problem is (file bytes).
    pub span: SourceSpan,
    /// Whether the problem prevents the program image (m1 §5.7). Warnings and notes are never
    /// blocking; an error may be non-blocking.
    pub blocking: bool,
    /// Fluent variables of the message, e.g. `("text", DiagArg::Str(…))`.
    pub args: Vec<(&'static str, DiagArg)>,
    /// Other places involved, each with its own message.
    pub related: Vec<Related>,
    /// A machine-applicable fix, explained by the message's `.help` attribute.
    pub suggestion: Option<Suggestion>,
}

/// Another place involved in a diagnostic (e.g. the first definition of a duplicated label).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Related {
    /// Where the related place is.
    pub span: SourceSpan,
    /// Selects the Fluent message of the related place.
    pub code: DiagCode,
    /// Fluent variables of that message.
    pub args: Vec<(&'static str, DiagArg)>,
}

/// A fix: replace the bytes of `span` with `replacement`. The parent diagnostic's `.help`
/// attribute explains it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    /// The bytes to replace.
    pub span: SourceSpan,
    /// The text that replaces them.
    pub replacement: String,
}

impl Diagnostic {
    /// Builds a diagnostic with no argument, related place or suggestion; it is blocking iff
    /// `severity` is [`Severity::Error`].
    pub fn new(code: DiagCode, severity: Severity, span: SourceSpan) -> Self {
        Self {
            code,
            severity,
            span,
            blocking: severity == Severity::Error,
            args: Vec::new(),
            related: Vec::new(),
            suggestion: None,
        }
    }

    /// Appends the Fluent variable `name` with `value`.
    #[must_use]
    pub fn with_arg(mut self, name: &'static str, value: DiagArg) -> Self {
        self.args.push((name, value));
        self
    }

    /// Marks the diagnostic as non-blocking: it is still shown with its severity, but it never
    /// prevents the program image.
    #[must_use]
    pub fn non_blocking(mut self) -> Self {
        self.blocking = false;
        self
    }
}
