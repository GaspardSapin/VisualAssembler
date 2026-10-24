//! Form selection, instruction length and encoding (m2-assembler.md §4.2, frozen; §4.10).
//!
//! [`select`] picks the form of a spelling that its operands and size fit (the spelling's
//! `also_accepts` entries first, then its own, in table order); [`length`] gives its byte length
//! from the operand shapes only; [`encode`] writes its words with final values, through
//! `isa::fields::places` and the EA fields of M1's `Encoding`.
//!
//! WP0 scaffold: the frozen signatures of §4.2 with `todo!()` bodies; WP-C implements them.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-C implements m2 §4.2, §4.10"
)]

use super::expr::Scope;
use super::operand::{Operand, Spanned};
use super::parser::Problem;
use crate::isa::{Instruction, Size};

/// The form chosen for an instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Choice {
    /// The table entry.
    pub entry: &'static Instruction,
    /// The index of the form in `entry.forms`.
    pub form: usize,
    /// The condition code (0–15) of a Bcc, DBcc or Scc spelling.
    pub condition: Option<u8>,
    /// The operation size; `None` for an unsized form.
    pub size: Option<Size>,
}

/// Selects the form of `spelling` (uppercase, without suffix) that `size` and `ops` fit.
///
/// # Errors
///
/// The closest failure (§4.10): `asm-operand-count`, `asm-size-not-allowed`,
/// `asm-mode-not-allowed`, else `asm-no-matching-form`.
pub fn select(
    spelling: &str,
    size: Option<Size>,
    ops: &[Spanned<Operand>],
) -> Result<Choice, Problem> {
    todo!()
}

/// Returns the length in bytes of `choice` with `ops`; it depends on the operand shapes only.
pub fn length(choice: &Choice, ops: &[Spanned<Operand>]) -> u32 {
    todo!()
}

/// Encodes `choice` with `ops` at `address`, evaluating the operands in `scope`, and appends
/// its words to `out`.
///
/// # Errors
///
/// The [`Problem`] of a value that cannot be encoded (`asm-value-out-of-range`,
/// `asm-branch-out-of-range`, …).
#[expect(
    clippy::ptr_arg,
    reason = "WP0 stub: the frozen §4.2 signature appends to `out`, which the todo!() body does not"
)]
pub fn encode(
    choice: &Choice,
    ops: &[Spanned<Operand>],
    scope: &dyn Scope,
    address: u64,
    out: &mut Vec<u16>,
) -> Result<(), Problem> {
    todo!()
}
