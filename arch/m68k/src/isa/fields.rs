//! Where each operand of a form goes in the instruction words (m2-assembler.md §4.9, frozen).
//!
//! M1's [`Encoding`](super::Encoding) gives the fixed bits, the size field and the EA fields of a
//! form. The other operand bits (register numbers, quick values, displacements, immediates,
//! masks) are described here by one explicit table, keyed by the entry and the form index, with
//! one [`Place`] per operand in source order. The encoder (`asm::encode`) and the disassembler
//! (`disasm::decode`) both read it, so they cannot disagree.
//!
//! WP0 scaffold: the frozen signatures of §4.9 with a `todo!()` body; WP-C implements the table
//! on day 1 of wave 2, with `tests/isa/fields.rs`.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-C implements m2 §4.9"
)]

use super::Instruction;

/// Where one operand of a form is encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    /// The form's EA field(s) (`Encoding::ea`); extension words follow in operand order.
    Ea,
    /// A register number in bits 11–9 of the opword.
    Reg9,
    /// A register number in bits 2–0 of the opword.
    Reg0,
    /// A value 1–8 in bits 11–9, 8 stored as 0 (ADDQ, SUBQ, shift counts).
    Quick9,
    /// A signed value in bits 7–0 of the opword (MOVEQ).
    Data8,
    /// A trap vector in bits 3–0 of the opword (TRAP).
    Vector,
    /// A branch displacement in the low byte of the opword.
    Disp8,
    /// A branch displacement in one extension word.
    Disp16,
    /// An immediate extension sized by the operation size (`.B`: the low byte of one word).
    Imm,
    /// One immediate extension word (STOP, LINK, the SR forms).
    ImmWord,
    /// One extension word whose low byte holds the value (the CCR forms, bit numbers).
    ImmByte,
    /// The MOVEM mask word, bit-reversed when the EA is `-(An)`.
    RegMask,
    /// No bits: the operand is implied by the opword (CCR, SR, USP).
    Implied,
}

/// Returns the places of the operands of form `form` of `entry`, one per operand in source
/// order.
pub fn places(entry: &Instruction, form: usize) -> &'static [Place] {
    todo!()
}
