//! The optimizations of a68k (m2-assembler.md §4.2, frozen; §4.10): `addq`, `subq`, `moveq`,
//! `lea`, `addq-lea`, `subq-lea`, `zero-displacement`, `pc-relative` and `movem-move`, each
//! applied only to the generic spelling and only with `AsmOptions::optimize`. Branch shortening
//! (`short-branch`) is the relaxation of `asm::layout`.
//!
//! WP0 scaffold: the frozen signature of §4.2 with a `todo!()` body; WP-C implements it.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-C implements m2 §4.2, §4.10"
)]

use super::encode::Choice;
use super::expr::Scope;
use super::operand::{Operand, Spanned};

/// Rewrites `choice` with `ops` into a shorter or equivalent instruction, if a §4.10 rule
/// applies; returns the new choice, its operands and the rule id (e.g. `"addq"`).
///
/// `generic` tells whether the instruction was written with the generic spelling (ADD, SUB,
/// MOVE, …) rather than an explicit ADDI, ADDA, MOVEA, ADDQ, …, which is never rewritten.
pub fn optimize(
    choice: &Choice,
    ops: &[Spanned<Operand>],
    scope: &dyn Scope,
    generic: bool,
) -> Option<(Choice, Vec<Spanned<Operand>>, &'static str)> {
    todo!()
}
