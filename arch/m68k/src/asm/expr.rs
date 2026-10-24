//! Expressions (m2-assembler.md §4.2, frozen; §4.4): their syntax tree, how symbols resolve
//! ([`Scope`], [`Lookup`]), parsing from lexer tokens and evaluation to a 32-bit value.
//!
//! Values are 32-bit two's complement, sign-extended to `i64`. Precedence, highest first:
//! unary `-` `+` `~`; `<<` `>>`; `&` `!`; `*` `/`; `+` `-` (observed on a68k, §4.4).
//!
//! WP0 scaffold: the frozen signatures of §4.2 with `todo!()` bodies; WP-B implements them.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-B implements m2 §4.2, §4.4"
)]

use va_core::source::TextRange;
use va_core::syntax::Token;

use super::parser::Problem;

/// An expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    /// A number.
    Num(i64),
    /// A quoted character constant (1–4 characters, right-justified).
    Char(i64),
    /// `*`, the location counter of the statement.
    Here,
    /// A symbol or local label.
    Sym(SymRef),
    /// A unary operation.
    Unary(UnOp, Box<Expr>),
    /// A binary operation.
    Binary(BinOp, Box<Expr>, Box<Expr>),
}

/// A reference to a symbol in an expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymRef {
    /// The name as written (symbols are case-sensitive).
    pub name: String,
    /// Whether it is a local label (`1$`, `\name`).
    pub local: bool,
    /// Where it is written, line-relative.
    pub range: TextRange,
}

/// A unary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnOp {
    /// `-`
    Neg,
    /// `+`
    Plus,
    /// `~`
    Not,
}

/// A binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinOp {
    /// `<<`
    Shl,
    /// `>>` (arithmetic)
    Shr,
    /// `&`
    And,
    /// `!`
    Or,
    /// `*`
    Mul,
    /// `/` (truncates toward zero)
    Div,
    /// `+`
    Add,
    /// `-`
    Sub,
}

/// What a symbol resolves to at the statement being evaluated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lookup {
    /// Defined on an earlier statement (in expansion order), with this value.
    Known(i64),
    /// Defined further down, with this value (pass 2 only).
    Later(i64),
    /// Defined further down, value not known yet (pass 1).
    Unknown,
    /// Never defined.
    Undefined,
    /// Its definition failed: uses report nothing (§4.13).
    Poisoned,
}

/// How an expression sees the symbols and the location counter.
pub trait Scope {
    /// Resolves `sym`.
    fn lookup(&self, sym: &SymRef) -> Lookup;

    /// The location counter of the statement (`*`), after its alignment.
    fn here(&self) -> i64;
}

/// The value of an expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Value {
    /// The 32-bit result, sign-extended.
    pub value: i64,
    /// Whether no `Later` or `Unknown` symbol was involved.
    pub known: bool,
}

/// Parses an expression at the start of `tokens` (line-relative ranges into `line`); returns it
/// with the number of tokens it used.
///
/// # Errors
///
/// The [`Problem`] of an expression that does not parse.
pub fn parse_expr(line: &str, tokens: &[Token]) -> Result<(Expr, usize), Problem> {
    todo!()
}

/// Evaluates `expr` in `scope`.
///
/// # Errors
///
/// The [`Problem`] of an expression that has no value (division by zero, undefined symbol, …).
pub fn eval(expr: &Expr, scope: &dyn Scope) -> Result<Value, Problem> {
    todo!()
}
