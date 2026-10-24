//! The fields of one source line (m2-assembler.md §4.2, frozen; §4.3): label, operation and
//! operand field, read from the M1 lexer's tokens, and [`Problem`], the line-relative problem
//! that the front end and the encoder report to the driver.
//!
//! [`parse_line`] reads the fields only; the user of the operand field parses its operands
//! (`operand::parse_operand`, `expr::parse_expr`).
//!
//! WP0 scaffold: the frozen signatures of §4.2 with a `todo!()` body; WP-B implements them.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-B implements m2 §4.2-§4.3"
)]

use va_core::diag::{DiagArg, DiagCode};
use va_core::source::TextRange;
use va_core::syntax::Token;

use super::operand::Spanned;
use crate::isa::Size;

/// The operation field of a line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpField {
    /// An instruction spelling.
    Mnemonic {
        /// The spelling, uppercase, as listed in `isa::SPELLINGS`.
        spelling: &'static str,
        /// Its size suffix, if any.
        size: Option<Size>,
    },
    /// A directive.
    Directive {
        /// The directive name, uppercase (`"DC"`, `"="` …).
        name: &'static str,
        /// Its size suffix, if any.
        size: Option<Size>,
    },
    /// A macro call (or an unknown operation the driver resolves).
    Macro {
        /// The name as written (macro names are case-sensitive).
        name: String,
        /// The suffix after the name, as written, if any.
        suffix: Option<String>,
    },
}

/// The fields of one line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// The label, if any (without its colon).
    pub label: Option<Spanned<String>>,
    /// Whether the label is a local label (`1$`, `\name`).
    pub local_label: bool,
    /// The operation field, if any.
    pub op: Option<Spanned<OpField>>,
    /// The operand field (empty when the line has none), line-relative.
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

/// Reads the fields of `line` from its lexer `tokens` (line-relative ranges that cover it).
pub fn parse_line(line: &str, tokens: &[Token]) -> Line {
    todo!()
}
