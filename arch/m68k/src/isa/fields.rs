//! Where each operand of a form goes in the instruction words (m2-assembler.md §4.9, frozen).
//!
//! M1's [`Encoding`](super::Encoding) gives the fixed bits, the size field and the EA fields of a
//! form. The other operand bits (register numbers, quick values, displacements, immediates,
//! masks) are described here by one explicit table, keyed by the entry and the form index, with
//! one [`Place`] per operand in source order. The encoder (`asm::encode`) and the disassembler
//! (`disasm::decode`) both read it, so they cannot disagree.
//!
//! **Word order.** The opword comes first. Then the extension words of the operands follow in
//! operand order, with one exception: the MOVEM mask word ([`Place::RegMask`]) always comes
//! right after the opword, also in `MOVEM <ea>,<list>`, where the mask is the second operand
//! but its word precedes the extension words of the `<ea>` (as the 68000 reads it).
//!
//! **Extension words per place.** [`Place::Ea`]: those of the operand's addressing mode
//! (`d16(An)`, `d8(An,Xn)`, `xxx.W`, `d16(PC)`, `d8(PC,Xn)`: one word; `xxx.L`: two; `#imm`: one
//! for `.B`/`.W`, two for `.L`). [`Place::Reg9`] and [`Place::Reg0`]: none, except for a
//! `d16(An)` operand (MOVEP), whose displacement takes one word. [`Place::Disp16`],
//! [`Place::ImmWord`], [`Place::ImmByte`], [`Place::RegMask`]: one word. [`Place::Imm`]: one word
//! for `.B`/`.W`, two for `.L`. The other places live in the opword only.
//!
//! The table has one row per entry of `INSTRUCTIONS`, in the same order; a compile-time check
//! proves that the rows and the entries agree in name, number of forms and operand counts.

use super::Instruction;

/// Where one operand of a form is encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    /// The form's EA field(s) (`Encoding::ea`); extension words follow in operand order.
    Ea,
    /// A register number in bits 11–9 of the opword. A `d16(An)` operand (MOVEP) also takes its
    /// displacement in one extension word.
    Reg9,
    /// A register number in bits 2–0 of the opword. A `d16(An)` operand (MOVEP) also takes its
    /// displacement in one extension word.
    Reg0,
    /// A value 1–8 in bits 11–9, 8 stored as 0 (ADDQ, SUBQ, shift counts).
    Quick9,
    /// A signed value in bits 7–0 of the opword (MOVEQ).
    Data8,
    /// A trap vector in bits 3–0 of the opword (TRAP).
    Vector,
    /// A branch displacement in the low byte of the opword (target − opword address − 2).
    Disp8,
    /// A branch displacement in one extension word (target − opword address − 2).
    Disp16,
    /// An immediate extension sized by the operation size (`.B`: the low byte of one word).
    Imm,
    /// One immediate extension word (STOP, LINK, the SR forms).
    ImmWord,
    /// One extension word whose low byte holds the value (the CCR forms, bit numbers).
    ImmByte,
    /// The MOVEM mask word, bit-reversed when the EA is `-(An)`. Its word comes right after the
    /// opword, before the extension words of the other operand.
    RegMask,
    /// No bits: the operand is implied by the opword (CCR, SR, USP).
    Implied,
}

use Place::{
    Data8, Disp8, Disp16, Ea, Imm, ImmByte, ImmWord, Implied, Quick9, Reg0, Reg9, RegMask, Vector,
};

/// The forms of an entry, one slice of places per form.
type Row = &'static [&'static [Place]];

/// No operand (NOP, RTS, …).
const NO_OPERAND: Row = &[&[]];
/// One effective address (CLR, TST, JMP, Scc, …).
const ONE_EA: Row = &[&[Ea]];
/// `<ea>,Rn` with the register in bits 11–9 (ADDA, CHK, LEA, MULU, …).
const EA_TO_REG9: Row = &[&[Ea, Reg9]];
/// `#<data>,<ea>` (ADDI, SUBI, CMPI).
const IMMEDIATE: Row = &[&[Imm, Ea]];
/// ABCD, SBCD, ADDX, SUBX: source register in bits 2–0, destination in bits 11–9.
const REGISTER_PAIR: Row = &[&[Reg0, Reg9], &[Reg0, Reg9]];
/// ADD, SUB: `.B <ea>,Dn`, `.W/.L <ea>,Dn`, `Dn,<ea>`.
const ADD_SUB: Row = &[&[Ea, Reg9], &[Ea, Reg9], &[Reg9, Ea]];
/// ADDQ, SUBQ: to `<ea>` and to `An`.
const QUICK: Row = &[&[Quick9, Ea], &[Quick9, Reg0]];
/// AND, OR: `<ea>,Dn` and `Dn,<ea>`.
const AND_OR: Row = &[&[Ea, Reg9], &[Reg9, Ea]];
/// ANDI, EORI, ORI: to `<ea>`, to CCR, to SR.
const IMMEDIATE_LOGIC: Row = &[&[Imm, Ea], &[ImmByte, Implied], &[ImmWord, Implied]];
/// Shifts and rotates: `#<1..8>,Dn`, `Dn,Dn` (count register in bits 11–9), `<ea>`.
const SHIFT: Row = &[&[Quick9, Reg0], &[Reg9, Reg0], &[Ea]];
/// Bit operations: dynamic on `Dn` and on `<ea>`, static on `Dn` and on `<ea>`.
const BIT: Row = &[&[Reg9, Reg0], &[Reg9, Ea], &[ImmByte, Reg0], &[ImmByte, Ea]];
/// BRA, BSR, Bcc: `.S` then `.W`.
const BRANCH: Row = &[&[Disp8], &[Disp16]];
/// One register in bits 2–0 (EXT, SWAP, UNLK).
const ONE_REG0: Row = &[&[Reg0]];

/// One row per entry of `INSTRUCTIONS`, in its order: the entry name and its forms.
const TABLE: [(&str, Row); 74] = [
    ("ABCD", REGISTER_PAIR),
    ("ADD", ADD_SUB),
    ("ADDA", EA_TO_REG9),
    ("ADDI", IMMEDIATE),
    ("ADDQ", QUICK),
    ("ADDX", REGISTER_PAIR),
    ("AND", AND_OR),
    ("ANDI", IMMEDIATE_LOGIC),
    ("ASL", SHIFT),
    ("ASR", SHIFT),
    ("Bcc", BRANCH),
    ("BCHG", BIT),
    ("BCLR", BIT),
    ("BRA", BRANCH),
    ("BSET", BIT),
    ("BSR", BRANCH),
    ("BTST", BIT),
    ("CHK", EA_TO_REG9),
    ("CLR", ONE_EA),
    ("CMP", &[&[Ea, Reg9], &[Ea, Reg9]]),
    ("CMPA", EA_TO_REG9),
    ("CMPI", IMMEDIATE),
    // CMPM (Ay)+,(Ax)+: Ay in bits 2–0, Ax in bits 11–9.
    ("CMPM", &[&[Reg0, Reg9]]),
    ("DBcc", &[&[Reg0, Disp16]]),
    ("DIVS", EA_TO_REG9),
    ("DIVU", EA_TO_REG9),
    ("EOR", &[&[Reg9, Ea]]),
    ("EORI", IMMEDIATE_LOGIC),
    // EXG Rx,Ry: Rx in bits 11–9, Ry in bits 2–0 (the data register of the Dn,An form is Rx).
    ("EXG", &[&[Reg9, Reg0], &[Reg9, Reg0], &[Reg9, Reg0]]),
    ("EXT", ONE_REG0),
    ("ILLEGAL", NO_OPERAND),
    ("JMP", ONE_EA),
    ("JSR", ONE_EA),
    ("LEA", EA_TO_REG9),
    ("LINK", &[&[Reg0, ImmWord]]),
    ("LSL", SHIFT),
    ("LSR", SHIFT),
    (
        "MOVE",
        &[
            &[Ea, Ea],
            &[Ea, Ea],
            &[Ea, Implied],
            &[Ea, Implied],
            &[Implied, Ea],
            &[Implied, Reg0],
            &[Reg0, Implied],
        ],
    ),
    ("MOVEA", EA_TO_REG9),
    ("MOVEM", &[&[RegMask, Ea], &[Ea, RegMask]]),
    // MOVEP: the address register (with its d16 word) in bits 2–0, the data register in 11–9.
    ("MOVEP", &[&[Reg0, Reg9], &[Reg9, Reg0]]),
    ("MOVEQ", &[&[Data8, Reg9]]),
    ("MULS", EA_TO_REG9),
    ("MULU", EA_TO_REG9),
    ("NBCD", ONE_EA),
    ("NEG", ONE_EA),
    ("NEGX", ONE_EA),
    ("NOP", NO_OPERAND),
    ("NOT", ONE_EA),
    ("OR", AND_OR),
    ("ORI", IMMEDIATE_LOGIC),
    ("PEA", ONE_EA),
    ("RESET", NO_OPERAND),
    ("ROL", SHIFT),
    ("ROR", SHIFT),
    ("ROXL", SHIFT),
    ("ROXR", SHIFT),
    ("RTE", NO_OPERAND),
    ("RTR", NO_OPERAND),
    ("RTS", NO_OPERAND),
    ("SBCD", REGISTER_PAIR),
    ("Scc", ONE_EA),
    ("STOP", &[&[ImmWord]]),
    ("SUB", ADD_SUB),
    ("SUBA", EA_TO_REG9),
    ("SUBI", IMMEDIATE),
    ("SUBQ", QUICK),
    ("SUBX", REGISTER_PAIR),
    ("SWAP", ONE_REG0),
    ("TAS", ONE_EA),
    ("TRAP", &[&[Vector]]),
    ("TRAPV", NO_OPERAND),
    ("TST", ONE_EA),
    ("UNLK", ONE_REG0),
];

/// Proves at compile time that [`TABLE`] has one row per entry of the instruction table, in its
/// order, with one slice per form and one place per operand.
const fn rows_match_the_instruction_table() -> bool {
    if TABLE.len() != super::TABLE.len() {
        return false;
    }
    let mut e = 0;
    while e < TABLE.len() {
        let (name, row) = TABLE[e];
        let entry = &super::TABLE[e];
        if !super::str_eq(name, entry.name) || row.len() != entry.forms.len() {
            return false;
        }
        let mut f = 0;
        while f < row.len() {
            if row[f].len() != entry.forms[f].operands.len() {
                return false;
            }
            f += 1;
        }
        e += 1;
    }
    true
}

const _: () = assert!(
    rows_match_the_instruction_table(),
    "isa::fields::TABLE disagrees with the instruction table"
);

/// Returns the places of the operands of form `form` of `entry`, one per operand in source
/// order.
///
/// The entry is found by name (ASCII case-insensitive), so a copy of a table entry works too.
/// An unknown entry name or a form index out of range gives an empty slice.
pub fn places(entry: &Instruction, form: usize) -> &'static [Place] {
    let found = TABLE.binary_search_by(|(name, _)| {
        name.bytes()
            .map(|byte| byte.to_ascii_uppercase())
            .cmp(entry.name.bytes().map(|byte| byte.to_ascii_uppercase()))
    });
    match found {
        Ok(row) => TABLE
            .get(row)
            .and_then(|(_, forms)| forms.get(form))
            .copied()
            .unwrap_or(&[]),
        Err(_) => &[],
    }
}
