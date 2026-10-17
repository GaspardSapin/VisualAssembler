//! Logic entries: AND…, OR…, EOR… (with their CCR and SR forms), NOT, shifts and rotates, bit
//! operations, TAS, Scc (m1-core-isa-lexer.md §4.1).
//!
//! Cycle counts: tables 8-4 (standard), 8-5 (immediate), 8-6 (single operand), 8-7 (shifts),
//! 8-8 (bit operations) and 8-12 (CCR and SR forms) of the User's Manual.

use super::encoding::{EaField, Encoding, SizeField};
use super::model::{ARITH, BIT, LOGIC, ROTATE, SHIFT, STATUS_LOGIC, UNAFFECTED};
use super::model::{Flags, Form, Operand, QuickRange, SZ_B, SZ_BW, SZ_BWL, SZ_L, SZ_W, Size};
use super::modes::{Mode, ModeSet};
use super::timing::{Outcome, Timing, TimingRow, Unit};

/// `Dn`.
const DN: ModeSet = ModeSet::only(Mode::Dn);
/// The sources whose long base cost is 8 instead of 6 (footnote ** of table 8-4).
const REG_IMM: ModeSet = ModeSet::of(&[Mode::Dn, Mode::Imm]);
/// Memory alterable.
const MA: ModeSet = ModeSet::MEMORY_ALTERABLE;
/// Static bit operations on memory: every memory mode but immediate.
const MEMORY_NOT_IMM: ModeSet = ModeSet::MEMORY.without(Mode::Imm);

const EA_DATA_TO_DN: &[Operand] = &[Operand::Ea(ModeSet::DATA), Operand::Dn];
const DN_TO_MEMORY: &[Operand] = &[Operand::Dn, Operand::Ea(MA)];
const IMM_TO_DATA: &[Operand] = &[Operand::Imm, Operand::Ea(ModeSet::DATA_ALTERABLE)];
const IMM_TO_CCR: &[Operand] = &[Operand::Imm, Operand::Ccr];
const IMM_TO_SR: &[Operand] = &[Operand::Imm, Operand::Sr];
const DATA_ALTERABLE: &[Operand] = &[Operand::Ea(ModeSet::DATA_ALTERABLE)];
const MEMORY_ALTERABLE: &[Operand] = &[Operand::Ea(MA)];
const QUICK_TO_DN: &[Operand] = &[Operand::Quick(QuickRange::OneToEight), Operand::Dn];
const DN_DN: &[Operand] = &[Operand::Dn, Operand::Dn];
const IMM_DN: &[Operand] = &[Operand::Imm, Operand::Dn];
const DN_TO_MEMORY_DATA: &[Operand] = &[Operand::Dn, Operand::Ea(ModeSet::MEMORY)];
const IMM_TO_MEMORY_DATA: &[Operand] = &[Operand::Imm, Operand::Ea(MEMORY_NOT_IMM)];
const IMM_TO_MEMORY: &[Operand] = &[Operand::Imm, Operand::Ea(MA)];

/// AND/OR `<ea>,Dn` (footnote ** for long data register and immediate sources).
const EA_TO_DN_ROWS: &[TimingRow] = &[
    TimingRow::new(SZ_BW, 4, 1, 0).on(0, ModeSet::DATA).ea(),
    TimingRow::new(SZ_L, 8, 1, 0).on(0, REG_IMM).ea(),
    TimingRow::new(SZ_L, 6, 1, 0)
        .on(0, ModeSet::DATA.minus(REG_IMM))
        .ea(),
];
/// `Dn,<ea>` to memory.
const DN_TO_MEMORY_ROWS: &[TimingRow] = &[
    TimingRow::new(SZ_BW, 8, 1, 1).on(1, MA).ea(),
    TimingRow::new(SZ_L, 12, 1, 2).on(1, MA).ea(),
];
/// ORI and EORI to a data register or memory (table 8-5).
const IMMEDIATE_ROWS: &[TimingRow] = &[
    TimingRow::new(SZ_BW, 8, 2, 0).on(1, DN),
    TimingRow::new(SZ_L, 16, 3, 0).on(1, DN),
    TimingRow::new(SZ_BW, 12, 2, 1).on(1, MA).ea(),
    TimingRow::new(SZ_L, 20, 3, 2).on(1, MA).ea(),
];
/// ANDI to a data register or memory (table 8-5: the long register form takes 14 clocks).
const ANDI_ROWS: &[TimingRow] = &[
    TimingRow::new(SZ_BW, 8, 2, 0).on(1, DN),
    TimingRow::new(SZ_L, 14, 3, 0).on(1, DN),
    TimingRow::new(SZ_BW, 12, 2, 1).on(1, MA).ea(),
    TimingRow::new(SZ_L, 20, 3, 2).on(1, MA).ea(),
];
/// ANDI, EORI, ORI to CCR (table 8-12).
const TO_CCR_ROWS: &[TimingRow] = &[TimingRow::new(SZ_B, 20, 3, 0)];
/// ANDI, EORI, ORI to SR (table 8-12).
const TO_SR_ROWS: &[TimingRow] = &[TimingRow::new(SZ_W, 20, 3, 0)];
/// Register shifts and rotates: 6 + 2n (`.B`/`.W`) or 8 + 2n (`.L`) clocks (table 8-7).
const SHIFT_REGISTER_ROWS: &[TimingRow] = &[
    TimingRow::new(SZ_BW, 6, 1, 0).per_unit(2, 0, 0, Unit::ShiftCount),
    TimingRow::new(SZ_L, 8, 1, 0).per_unit(2, 0, 0, Unit::ShiftCount),
];
/// Memory shifts and rotates: one position, word only (table 8-7).
const SHIFT_MEMORY_ROWS: &[TimingRow] = &[TimingRow::new(SZ_W, 8, 1, 1).on(0, MA).ea()];

const fn and_or(opword: u16) -> [Form; 2] {
    [
        Form::new(
            EA_DATA_TO_DN,
            SZ_BWL,
            Some(Size::Word),
            LOGIC,
            Encoding::new(opword, 0xF100, SizeField::Bits7_6, EaField::Low),
            Timing::Rows(EA_TO_DN_ROWS),
        ),
        Form::new(
            DN_TO_MEMORY,
            SZ_BWL,
            Some(Size::Word),
            LOGIC,
            Encoding::new(opword | 0x0100, 0xF100, SizeField::Bits7_6, EaField::Low),
            Timing::Rows(DN_TO_MEMORY_ROWS),
        ),
    ]
}

/// ANDI, EORI, ORI: to a data alterable operand, to CCR, and to SR (supervisor).
const fn immediate_logic(opword: u16, rows: &'static [TimingRow]) -> [Form; 3] {
    [
        Form::new(
            IMM_TO_DATA,
            SZ_BWL,
            Some(Size::Word),
            LOGIC,
            Encoding::new(opword, 0xFF00, SizeField::Bits7_6, EaField::Low),
            Timing::Rows(rows),
        ),
        Form::new(
            IMM_TO_CCR,
            SZ_B,
            Some(Size::Byte),
            STATUS_LOGIC,
            Encoding::new(opword | 0x003C, 0xFFFF, SizeField::None, EaField::None),
            Timing::Rows(TO_CCR_ROWS),
        ),
        Form::new(
            IMM_TO_SR,
            SZ_W,
            Some(Size::Word),
            STATUS_LOGIC,
            Encoding::new(opword | 0x007C, 0xFFFF, SizeField::None, EaField::None),
            Timing::Rows(TO_SR_ROWS),
        )
        .supervisor(),
    ]
}

/// A shift or rotate: `kind` is the type field (00 arithmetic, 01 logical, 10 rotate through
/// X, 11 rotate), `left` the direction bit.
const fn shift(kind: u16, left: bool, flags: Flags) -> [Form; 3] {
    let direction = if left { 0x0100 } else { 0x0000 };
    [
        Form::new(
            QUICK_TO_DN,
            SZ_BWL,
            Some(Size::Word),
            flags,
            Encoding::new(
                0xE000 | direction | (kind << 3),
                0xF138,
                SizeField::Bits7_6,
                EaField::None,
            ),
            Timing::Rows(SHIFT_REGISTER_ROWS),
        ),
        Form::new(
            DN_DN,
            SZ_BWL,
            Some(Size::Word),
            flags,
            Encoding::new(
                0xE020 | direction | (kind << 3),
                0xF138,
                SizeField::Bits7_6,
                EaField::None,
            ),
            Timing::Rows(SHIFT_REGISTER_ROWS),
        ),
        Form::new(
            MEMORY_ALTERABLE,
            SZ_W,
            Some(Size::Word),
            flags,
            Encoding::new(
                0xE0C0 | (kind << 9) | direction,
                0xFFC0,
                SizeField::None,
                EaField::Low,
            ),
            Timing::Rows(SHIFT_MEMORY_ROWS),
        ),
    ]
}

/// The timing rows of one bit operation, by form.
struct BitRows {
    dynamic_register: &'static [TimingRow],
    dynamic_memory: &'static [TimingRow],
    static_register: &'static [TimingRow],
    static_memory: &'static [TimingRow],
}

/// A bit operation: `kind` is bits 7–6 (00 BTST, 01 BCHG, 10 BCLR, 11 BSET). The bit number
/// comes from a data register (dynamic) or an immediate byte (static); the operation is long
/// on a data register and byte on memory.
const fn bit_op(
    kind: u16,
    dynamic_memory: &'static [Operand],
    static_memory: &'static [Operand],
    rows: BitRows,
) -> [Form; 4] {
    [
        Form::new(
            DN_DN,
            SZ_L,
            Some(Size::Long),
            BIT,
            Encoding::new(0x0100 | (kind << 6), 0xF1F8, SizeField::None, EaField::None),
            Timing::Rows(rows.dynamic_register),
        ),
        Form::new(
            dynamic_memory,
            SZ_B,
            Some(Size::Byte),
            BIT,
            Encoding::new(0x0100 | (kind << 6), 0xF1C0, SizeField::None, EaField::Low),
            Timing::Rows(rows.dynamic_memory),
        ),
        Form::new(
            IMM_DN,
            SZ_L,
            Some(Size::Long),
            BIT,
            Encoding::new(0x0800 | (kind << 6), 0xFFF8, SizeField::None, EaField::None),
            Timing::Rows(rows.static_register),
        ),
        Form::new(
            static_memory,
            SZ_B,
            Some(Size::Byte),
            BIT,
            Encoding::new(0x0800 | (kind << 6), 0xFFC0, SizeField::None, EaField::Low),
            Timing::Rows(rows.static_memory),
        ),
    ]
}

/// BCHG, BCLR, BSET on memory: dynamic and static rows (table 8-8).
const CHANGE_DYNAMIC_MEMORY: &[TimingRow] = &[TimingRow::new(SZ_B, 8, 1, 1).on(1, MA).ea()];
const CHANGE_STATIC_MEMORY: &[TimingRow] = &[TimingRow::new(SZ_B, 12, 2, 1).on(1, MA).ea()];

/// AND: `<ea>,Dn` or `Dn,<ea>`.
pub(super) const AND: &[Form] = &and_or(0xC000);
/// ANDI: with an immediate, to an operand, CCR or SR.
pub(super) const ANDI: &[Form] = &immediate_logic(0x0200, ANDI_ROWS);
/// ASL: arithmetic shift left (V reports a sign change).
pub(super) const ASL: &[Form] = &shift(0b00, true, ARITH);
/// ASR: arithmetic shift right.
pub(super) const ASR: &[Form] = &shift(0b00, false, ARITH);

/// BCHG: test a bit and invert it (register times depend on the bit number: worst case).
pub(super) const BCHG: &[Form] = &bit_op(
    0b01,
    DN_TO_MEMORY,
    IMM_TO_MEMORY,
    BitRows {
        dynamic_register: &[TimingRow::new(SZ_L, 8, 1, 0).upper_bound()],
        dynamic_memory: CHANGE_DYNAMIC_MEMORY,
        static_register: &[TimingRow::new(SZ_L, 12, 2, 0).upper_bound()],
        static_memory: CHANGE_STATIC_MEMORY,
    },
);
/// BCLR: test a bit and clear it (register times depend on the bit number: worst case).
pub(super) const BCLR: &[Form] = &bit_op(
    0b10,
    DN_TO_MEMORY,
    IMM_TO_MEMORY,
    BitRows {
        dynamic_register: &[TimingRow::new(SZ_L, 10, 1, 0).upper_bound()],
        dynamic_memory: CHANGE_DYNAMIC_MEMORY,
        static_register: &[TimingRow::new(SZ_L, 14, 2, 0).upper_bound()],
        static_memory: CHANGE_STATIC_MEMORY,
    },
);
/// BSET: test a bit and set it (register times depend on the bit number: worst case).
pub(super) const BSET: &[Form] = &bit_op(
    0b11,
    DN_TO_MEMORY,
    IMM_TO_MEMORY,
    BitRows {
        dynamic_register: &[TimingRow::new(SZ_L, 8, 1, 0).upper_bound()],
        dynamic_memory: CHANGE_DYNAMIC_MEMORY,
        static_register: &[TimingRow::new(SZ_L, 12, 2, 0).upper_bound()],
        static_memory: CHANGE_STATIC_MEMORY,
    },
);
/// BTST: test a bit; memory operands may be PC-relative (and immediate in the dynamic form).
pub(super) const BTST: &[Form] = &bit_op(
    0b00,
    DN_TO_MEMORY_DATA,
    IMM_TO_MEMORY_DATA,
    BitRows {
        dynamic_register: &[TimingRow::new(SZ_L, 6, 1, 0)],
        dynamic_memory: &[TimingRow::new(SZ_B, 4, 1, 0).on(1, ModeSet::MEMORY).ea()],
        static_register: &[TimingRow::new(SZ_L, 10, 2, 0)],
        static_memory: &[TimingRow::new(SZ_B, 8, 2, 0).on(1, MEMORY_NOT_IMM).ea()],
    },
);

/// EOR Dn,<ea>: the source is always a data register.
pub(super) const EOR: &[Form] = &[Form::new(
    &[Operand::Dn, Operand::Ea(ModeSet::DATA_ALTERABLE)],
    SZ_BWL,
    Some(Size::Word),
    LOGIC,
    Encoding::new(0xB100, 0xF100, SizeField::Bits7_6, EaField::Low),
    Timing::Rows(&[
        TimingRow::new(SZ_BW, 4, 1, 0).on(1, DN),
        TimingRow::new(SZ_L, 8, 1, 0).on(1, DN),
        TimingRow::new(SZ_BW, 8, 1, 1).on(1, MA).ea(),
        TimingRow::new(SZ_L, 12, 1, 2).on(1, MA).ea(),
    ]),
)];
/// EORI: with an immediate, to an operand, CCR or SR.
pub(super) const EORI: &[Form] = &immediate_logic(0x0A00, IMMEDIATE_ROWS);

/// LSL: logical shift left.
pub(super) const LSL: &[Form] = &shift(0b01, true, SHIFT);
/// LSR: logical shift right.
pub(super) const LSR: &[Form] = &shift(0b01, false, SHIFT);

/// NOT: one's complement.
pub(super) const NOT: &[Form] = &[Form::new(
    DATA_ALTERABLE,
    SZ_BWL,
    Some(Size::Word),
    LOGIC,
    Encoding::new(0x4600, 0xFF00, SizeField::Bits7_6, EaField::Low),
    Timing::Rows(&[
        TimingRow::new(SZ_BW, 4, 1, 0).on(0, DN),
        TimingRow::new(SZ_L, 6, 1, 0).on(0, DN),
        TimingRow::new(SZ_BW, 8, 1, 1).on(0, MA).ea(),
        TimingRow::new(SZ_L, 12, 1, 2).on(0, MA).ea(),
    ]),
)];

/// OR: `<ea>,Dn` or `Dn,<ea>`.
pub(super) const OR: &[Form] = &and_or(0x8000);
/// ORI: with an immediate, to an operand, CCR or SR.
pub(super) const ORI: &[Form] = &immediate_logic(0x0000, IMMEDIATE_ROWS);

/// ROL: rotate left.
pub(super) const ROL: &[Form] = &shift(0b11, true, ROTATE);
/// ROR: rotate right.
pub(super) const ROR: &[Form] = &shift(0b11, false, ROTATE);
/// ROXL: rotate left through X.
pub(super) const ROXL: &[Form] = &shift(0b10, true, SHIFT);
/// ROXR: rotate right through X.
pub(super) const ROXR: &[Form] = &shift(0b10, false, SHIFT);

/// Scc <ea>: the condition field (bits 11–8, all 16 codes) is free; the byte becomes all ones
/// when the condition is true, zero otherwise.
pub(super) const SCC: &[Form] = &[Form::new(
    DATA_ALTERABLE,
    SZ_B,
    Some(Size::Byte),
    UNAFFECTED,
    Encoding::new(0x50C0, 0xF0C0, SizeField::None, EaField::Low),
    Timing::Rows(&[
        TimingRow::new(SZ_B, 6, 1, 0).on(0, DN).when(Outcome::True),
        TimingRow::new(SZ_B, 4, 1, 0).on(0, DN).when(Outcome::False),
        TimingRow::new(SZ_B, 8, 1, 1)
            .on(0, MA)
            .ea()
            .when(Outcome::True),
        TimingRow::new(SZ_B, 8, 1, 1)
            .on(0, MA)
            .ea()
            .when(Outcome::False),
    ]),
)
.with_outcomes(&[Outcome::True, Outcome::False])];

/// TAS <ea>: test a byte and set its bit 7 in one indivisible read-modify-write cycle.
pub(super) const TAS: &[Form] = &[Form::new(
    DATA_ALTERABLE,
    SZ_B,
    Some(Size::Byte),
    LOGIC,
    Encoding::new(0x4AC0, 0xFFC0, SizeField::None, EaField::Low),
    Timing::Rows(&[
        TimingRow::new(SZ_B, 4, 1, 0).on(0, DN),
        TimingRow::new(SZ_B, 10, 1, 1).on(0, MA).ea(),
    ]),
)];
