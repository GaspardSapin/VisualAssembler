//! Operands (m2-assembler.md §4.2, frozen; §4.3): effective addresses, register lists, the
//! special registers and byte strings, parsed from lexer tokens in the strict A68k syntax.
//!
//! WP0 scaffold: the frozen signatures of §4.2 with a `todo!()` body; WP-B implements them.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-B implements m2 §4.2-§4.3"
)]

use va_core::source::TextRange;
use va_core::syntax::Token;

use super::expr::Expr;
use super::parser::Problem;
use crate::isa::Size;

/// An index register of `d8(An,Xn)` or `d8(PC,Xn)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Xn {
    /// The register: 0–7 are D0–D7, 8–15 are A0–A7.
    pub reg: u8,
    /// Whether the index is `.L` (`.W` by default).
    pub long: bool,
}

/// An effective address as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ea {
    /// `Dn`
    Dn(u8),
    /// `An` (and `SP` for A7)
    An(u8),
    /// `(An)`
    Ind(u8),
    /// `(An)+`
    PostInc(u8),
    /// `-(An)`
    PreDec(u8),
    /// `e(An)`
    Disp(Expr, u8),
    /// `e(An,Xn)`
    Index(Expr, u8, Xn),
    /// `e(PC)`, `e` being the target address
    PcDisp(Expr),
    /// `e(PC,Xn)`, `e` being the target address
    PcIndex(Expr, Xn),
    /// `e`, `(e).W` or `(e).L`
    Abs(Expr, Option<Size>),
    /// `#e`
    Imm(Expr),
}

/// One operand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    /// An effective address.
    Ea(Ea),
    /// A register list: bit n is Dn, bit 8 + n is An.
    RegList(u16),
    /// `CCR`
    Ccr,
    /// `SR`
    Sr,
    /// `USP`
    Usp,
    /// The bytes of a quoted string (a `DC` operand).
    Bytes(Vec<u8>),
}

/// A value with the line-relative range it was read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spanned<T> {
    /// The value.
    pub item: T,
    /// Where it is written, line-relative.
    pub range: TextRange,
}

/// Parses one operand at the start of `tokens` (line-relative ranges into `line`); returns it
/// with the number of tokens it used.
///
/// # Errors
///
/// The [`Problem`] of an operand that does not parse (with its quick fix when §4.3 gives one).
pub fn parse_operand(line: &str, tokens: &[Token]) -> Result<(Spanned<Operand>, usize), Problem> {
    todo!()
}
