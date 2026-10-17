//! Arithmetic entries: ADD…, SUB…, CMP…, MULS/MULU, DIVS/DIVU, NEG/NEGX, CLR, EXT, TST,
//! ABCD/SBCD/NBCD, CHK (m1-core-isa-lexer.md §4.1).
//!
//! Cycle counts: tables 8-4 (standard), 8-5 (immediate), 8-6 (single operand), 8-9 (CHK),
//! 8-11 (multi-precision) and 8-12 (miscellaneous) of the User's Manual. On the 68000 an
//! address register is never a byte operand, so the forms that accept `An` are split by size.

use super::encoding::{EaField, Encoding, SizeField};
use super::model::{ARITH, CHECK, CLEARED, COMPARE, DECIMAL, DIVIDE, EXTENDED, LOGIC, UNAFFECTED};
use super::model::{
    Flags, Form, Operand, QuickRange, SZ_B, SZ_BW, SZ_BWL, SZ_L, SZ_W, SZ_WL, Size,
};
use super::modes::{Mode, ModeSet};
use super::timing::{Outcome, Timing, TimingRow, Unit};

/// `Dn`.
const DN: ModeSet = ModeSet::only(Mode::Dn);
/// The sources whose long base cost is 8 instead of 6 (footnote ** of table 8-4).
const REG_IMM: ModeSet = ModeSet::of(&[Mode::Dn, Mode::An, Mode::Imm]);
/// Memory alterable.
const MA: ModeSet = ModeSet::MEMORY_ALTERABLE;

const EA_DATA_TO_DN: &[Operand] = &[Operand::Ea(ModeSet::DATA), Operand::Dn];
const EA_ALL_TO_DN: &[Operand] = &[Operand::Ea(ModeSet::ALL), Operand::Dn];
const DN_TO_MEMORY: &[Operand] = &[Operand::Dn, Operand::Ea(MA)];
const EA_ALL_TO_AN: &[Operand] = &[Operand::Ea(ModeSet::ALL), Operand::An];
const IMM_TO_DATA: &[Operand] = &[Operand::Imm, Operand::Ea(ModeSet::DATA_ALTERABLE)];
const QUICK_TO_DATA: &[Operand] = &[
    Operand::Quick(QuickRange::OneToEight),
    Operand::Ea(ModeSet::DATA_ALTERABLE),
];
const QUICK_TO_AN: &[Operand] = &[Operand::Quick(QuickRange::OneToEight), Operand::An];
const DN_DN: &[Operand] = &[Operand::Dn, Operand::Dn];
const PREDEC_PREDEC: &[Operand] = &[Operand::PreDecAn, Operand::PreDecAn];
const DATA_ALTERABLE: &[Operand] = &[Operand::Ea(ModeSet::DATA_ALTERABLE)];

/// ADD/SUB `<ea>,Dn` at `.B`.
const EA_TO_DN_B: &[TimingRow] = &[TimingRow::new(SZ_B, 4, 1, 0).on(0, ModeSet::DATA).ea()];
/// ADD/SUB `<ea>,Dn` at `.W`/`.L` (footnote ** for long register and immediate sources).
const EA_TO_DN_WL: &[TimingRow] = &[
    TimingRow::new(SZ_W, 4, 1, 0).on(0, ModeSet::ALL).ea(),
    TimingRow::new(SZ_L, 8, 1, 0).on(0, REG_IMM).ea(),
    TimingRow::new(SZ_L, 6, 1, 0)
        .on(0, ModeSet::ALL.minus(REG_IMM))
        .ea(),
];
/// `Dn,<ea>` to memory.
const DN_TO_MEMORY_ROWS: &[TimingRow] = &[
    TimingRow::new(SZ_BW, 8, 1, 1).on(1, MA).ea(),
    TimingRow::new(SZ_L, 12, 1, 2).on(1, MA).ea(),
];
/// ADDA/SUBA (footnote ** for long register and immediate sources).
const ADDRESS_ROWS: &[TimingRow] = &[
    TimingRow::new(SZ_W, 8, 1, 0).on(0, ModeSet::ALL).ea(),
    TimingRow::new(SZ_L, 8, 1, 0).on(0, REG_IMM).ea(),
    TimingRow::new(SZ_L, 6, 1, 0)
        .on(0, ModeSet::ALL.minus(REG_IMM))
        .ea(),
];
/// ADDI/SUBI (table 8-5).
const IMMEDIATE_ROWS: &[TimingRow] = &[
    TimingRow::new(SZ_BW, 8, 2, 0).on(1, DN),
    TimingRow::new(SZ_L, 16, 3, 0).on(1, DN),
    TimingRow::new(SZ_BW, 12, 2, 1).on(1, MA).ea(),
    TimingRow::new(SZ_L, 20, 3, 2).on(1, MA).ea(),
];
/// ADDQ/SUBQ to a data register or memory (table 8-5).
const QUICK_DATA_ROWS: &[TimingRow] = &[
    TimingRow::new(SZ_BW, 4, 1, 0).on(1, DN),
    TimingRow::new(SZ_L, 8, 1, 0).on(1, DN),
    TimingRow::new(SZ_BW, 8, 1, 1).on(1, MA).ea(),
    TimingRow::new(SZ_L, 12, 1, 2).on(1, MA).ea(),
];
/// ADDQ/SUBQ to an address register (table 8-5).
const QUICK_ADDRESS_ROWS: &[TimingRow] = &[TimingRow::new(SZ_WL, 8, 1, 0)];
/// ADDX/SUBX between data registers (table 8-11).
const EXTENDED_REGISTER_ROWS: &[TimingRow] = &[
    TimingRow::new(SZ_BW, 4, 1, 0),
    TimingRow::new(SZ_L, 8, 1, 0),
];
/// ADDX/SUBX between memory operands (table 8-11).
const EXTENDED_MEMORY_ROWS: &[TimingRow] = &[
    TimingRow::new(SZ_BW, 18, 3, 1),
    TimingRow::new(SZ_L, 30, 5, 2),
];
/// NEG, NEGX, CLR (table 8-6).
const SINGLE_ROWS: &[TimingRow] = &[
    TimingRow::new(SZ_BW, 4, 1, 0).on(0, DN),
    TimingRow::new(SZ_L, 6, 1, 0).on(0, DN),
    TimingRow::new(SZ_BW, 8, 1, 1).on(0, MA).ea(),
    TimingRow::new(SZ_L, 12, 1, 2).on(0, MA).ea(),
];
/// ABCD/SBCD between data registers (table 8-11).
const DECIMAL_REGISTER_ROWS: &[TimingRow] = &[TimingRow::new(SZ_B, 6, 1, 0)];
/// ABCD/SBCD between memory bytes (table 8-11).
const DECIMAL_MEMORY_ROWS: &[TimingRow] = &[TimingRow::new(SZ_B, 18, 3, 1)];

const fn add_sub(base: u16) -> [Form; 3] {
    [
        // `.B <ea>,Dn`: an address register is not a byte source.
        Form::new(
            EA_DATA_TO_DN,
            SZ_B,
            None,
            ARITH,
            Encoding::new(base, 0xF100, SizeField::Bits7_6, EaField::Low),
            Timing::Rows(EA_TO_DN_B),
        ),
        Form::new(
            EA_ALL_TO_DN,
            SZ_WL,
            Some(Size::Word),
            ARITH,
            Encoding::new(base, 0xF100, SizeField::Bits7_6, EaField::Low),
            Timing::Rows(EA_TO_DN_WL),
        ),
        Form::new(
            DN_TO_MEMORY,
            SZ_BWL,
            Some(Size::Word),
            ARITH,
            Encoding::new(base | 0x0100, 0xF100, SizeField::Bits7_6, EaField::Low),
            Timing::Rows(DN_TO_MEMORY_ROWS),
        ),
    ]
}

const fn add_sub_address(opword: u16) -> [Form; 1] {
    [Form::new(
        EA_ALL_TO_AN,
        SZ_WL,
        Some(Size::Word),
        UNAFFECTED,
        Encoding::new(opword, 0xF0C0, SizeField::Bit8, EaField::Low),
        Timing::Rows(ADDRESS_ROWS),
    )]
}

const fn add_sub_immediate(opword: u16) -> [Form; 1] {
    [Form::new(
        IMM_TO_DATA,
        SZ_BWL,
        Some(Size::Word),
        ARITH,
        Encoding::new(opword, 0xFF00, SizeField::Bits7_6, EaField::Low),
        Timing::Rows(IMMEDIATE_ROWS),
    )]
}

const fn add_sub_quick(opword: u16) -> [Form; 2] {
    [
        Form::new(
            QUICK_TO_DATA,
            SZ_BWL,
            Some(Size::Word),
            ARITH,
            Encoding::new(opword, 0xF100, SizeField::Bits7_6, EaField::Low),
            Timing::Rows(QUICK_DATA_ROWS),
        ),
        // To an address register: word or long only, and the flags are not changed.
        Form::new(
            QUICK_TO_AN,
            SZ_WL,
            Some(Size::Word),
            UNAFFECTED,
            Encoding::new(opword | 0x0008, 0xF138, SizeField::Bits7_6, EaField::None),
            Timing::Rows(QUICK_ADDRESS_ROWS),
        ),
    ]
}

const fn add_sub_extended(opword: u16) -> [Form; 2] {
    [
        Form::new(
            DN_DN,
            SZ_BWL,
            Some(Size::Word),
            EXTENDED,
            Encoding::new(opword, 0xF138, SizeField::Bits7_6, EaField::None),
            Timing::Rows(EXTENDED_REGISTER_ROWS),
        ),
        Form::new(
            PREDEC_PREDEC,
            SZ_BWL,
            Some(Size::Word),
            EXTENDED,
            Encoding::new(opword | 0x0008, 0xF138, SizeField::Bits7_6, EaField::None),
            Timing::Rows(EXTENDED_MEMORY_ROWS),
        ),
    ]
}

const fn single(opword: u16, flags: Flags) -> [Form; 1] {
    [Form::new(
        DATA_ALTERABLE,
        SZ_BWL,
        Some(Size::Word),
        flags,
        Encoding::new(opword, 0xFF00, SizeField::Bits7_6, EaField::Low),
        Timing::Rows(SINGLE_ROWS),
    )]
}

const fn decimal(opword: u16) -> [Form; 2] {
    [
        Form::new(
            DN_DN,
            SZ_B,
            Some(Size::Byte),
            DECIMAL,
            Encoding::new(opword, 0xF1F8, SizeField::None, EaField::None),
            Timing::Rows(DECIMAL_REGISTER_ROWS),
        ),
        Form::new(
            PREDEC_PREDEC,
            SZ_B,
            Some(Size::Byte),
            DECIMAL,
            Encoding::new(opword | 0x0008, 0xF1F8, SizeField::None, EaField::None),
            Timing::Rows(DECIMAL_MEMORY_ROWS),
        ),
    ]
}

const fn word_ea_to_dn(opword: u16, flags: Flags, rows: &'static [TimingRow]) -> [Form; 1] {
    [Form::new(
        EA_DATA_TO_DN,
        SZ_W,
        Some(Size::Word),
        flags,
        Encoding::new(opword, 0xF1C0, SizeField::None, EaField::Low),
        Timing::Rows(rows),
    )]
}

/// ABCD: decimal add with extend, between data registers or predecremented memory bytes.
pub(super) const ABCD: &[Form] = &decimal(0xC100);
/// ADD: `<ea>,Dn` or `Dn,<ea>`.
pub(super) const ADD: &[Form] = &add_sub(0xD000);
/// ADDA: add to an address register.
pub(super) const ADDA: &[Form] = &add_sub_address(0xD0C0);
/// ADDI: add an immediate.
pub(super) const ADDI: &[Form] = &add_sub_immediate(0x0600);
/// ADDQ: add 1 to 8.
pub(super) const ADDQ: &[Form] = &add_sub_quick(0x5000);
/// ADDX: add with extend.
pub(super) const ADDX: &[Form] = &add_sub_extended(0xD100);

/// CHK <ea>,Dn: traps when Dn is below 0 or above the bound (word only on the 68000).
pub(super) const CHK: &[Form] = &[Form::new(
    EA_DATA_TO_DN,
    SZ_W,
    Some(Size::Word),
    CHECK,
    Encoding::new(0x4180, 0xF1C0, SizeField::None, EaField::Low),
    Timing::Rows(&[
        TimingRow::new(SZ_W, 10, 1, 0)
            .on(0, ModeSet::DATA)
            .ea()
            .when(Outcome::NoTrap),
        TimingRow::new(SZ_W, 40, 4, 3)
            .on(0, ModeSet::DATA)
            .ea()
            .when(Outcome::Trap),
    ]),
)
.with_outcomes(&[Outcome::NoTrap, Outcome::Trap])];

/// CLR: clear an operand.
pub(super) const CLR: &[Form] = &single(0x4200, CLEARED);

/// CMP <ea>,Dn: split by size because an address register is not a byte source.
pub(super) const CMP: &[Form] = &[
    Form::new(
        EA_DATA_TO_DN,
        SZ_B,
        None,
        COMPARE,
        Encoding::new(0xB000, 0xF100, SizeField::Bits7_6, EaField::Low),
        Timing::Rows(&[TimingRow::new(SZ_B, 4, 1, 0).on(0, ModeSet::DATA).ea()]),
    ),
    Form::new(
        EA_ALL_TO_DN,
        SZ_WL,
        Some(Size::Word),
        COMPARE,
        Encoding::new(0xB000, 0xF100, SizeField::Bits7_6, EaField::Low),
        Timing::Rows(&[
            TimingRow::new(SZ_W, 4, 1, 0).on(0, ModeSet::ALL).ea(),
            TimingRow::new(SZ_L, 6, 1, 0).on(0, ModeSet::ALL).ea(),
        ]),
    ),
];

/// CMPA <ea>,An.
pub(super) const CMPA: &[Form] = &[Form::new(
    EA_ALL_TO_AN,
    SZ_WL,
    Some(Size::Word),
    COMPARE,
    Encoding::new(0xB0C0, 0xF0C0, SizeField::Bit8, EaField::Low),
    Timing::Rows(&[TimingRow::new(SZ_WL, 6, 1, 0).on(0, ModeSet::ALL).ea()]),
)];

/// CMPI #<data>,<ea>: data alterable destinations only on the 68000.
pub(super) const CMPI: &[Form] = &[Form::new(
    IMM_TO_DATA,
    SZ_BWL,
    Some(Size::Word),
    COMPARE,
    Encoding::new(0x0C00, 0xFF00, SizeField::Bits7_6, EaField::Low),
    Timing::Rows(&[
        TimingRow::new(SZ_BW, 8, 2, 0).on(1, DN),
        TimingRow::new(SZ_L, 14, 3, 0).on(1, DN),
        TimingRow::new(SZ_BW, 8, 2, 0).on(1, MA).ea(),
        TimingRow::new(SZ_L, 12, 3, 0).on(1, MA).ea(),
    ]),
)];

/// CMPM (An)+,(An)+.
pub(super) const CMPM: &[Form] = &[Form::new(
    &[Operand::PostIncAn, Operand::PostIncAn],
    SZ_BWL,
    Some(Size::Word),
    COMPARE,
    Encoding::new(0xB108, 0xF138, SizeField::Bits7_6, EaField::None),
    Timing::Rows(&[
        TimingRow::new(SZ_BW, 12, 3, 0),
        TimingRow::new(SZ_L, 20, 5, 0),
    ]),
)];

/// DIVS <ea>,Dn: the manual gives the worst case.
pub(super) const DIVS: &[Form] = &word_ea_to_dn(
    0x81C0,
    DIVIDE,
    &[TimingRow::new(SZ_W, 158, 1, 0)
        .on(0, ModeSet::DATA)
        .ea()
        .upper_bound()],
);
/// DIVU <ea>,Dn: the manual gives the worst case.
pub(super) const DIVU: &[Form] = &word_ea_to_dn(
    0x80C0,
    DIVIDE,
    &[TimingRow::new(SZ_W, 140, 1, 0)
        .on(0, ModeSet::DATA)
        .ea()
        .upper_bound()],
);

/// EXT Dn: sign-extend a byte to a word (`.W`) or a word to a long (`.L`).
pub(super) const EXT: &[Form] = &[Form::new(
    &[Operand::Dn],
    SZ_WL,
    Some(Size::Word),
    LOGIC,
    Encoding::new(0x4800, 0xFE38, SizeField::Ext8_6, EaField::None),
    Timing::Rows(&[TimingRow::new(SZ_WL, 4, 1, 0)]),
)];

/// MULS <ea>,Dn: 38 + 2n clocks, n the number of `01`/`10` pairs.
pub(super) const MULS: &[Form] = &word_ea_to_dn(
    0xC1C0,
    LOGIC,
    &[TimingRow::new(SZ_W, 38, 1, 0)
        .on(0, ModeSet::DATA)
        .ea()
        .per_unit(2, 0, 0, Unit::BitPairs)],
);
/// MULU <ea>,Dn: 38 + 2n clocks, n the number of 1 bits in the source.
pub(super) const MULU: &[Form] = &word_ea_to_dn(
    0xC0C0,
    LOGIC,
    &[TimingRow::new(SZ_W, 38, 1, 0)
        .on(0, ModeSet::DATA)
        .ea()
        .per_unit(2, 0, 0, Unit::OneBits)],
);

/// NBCD <ea>: decimal negate with extend.
pub(super) const NBCD: &[Form] = &[Form::new(
    DATA_ALTERABLE,
    SZ_B,
    Some(Size::Byte),
    DECIMAL,
    Encoding::new(0x4800, 0xFFC0, SizeField::None, EaField::Low),
    Timing::Rows(&[
        TimingRow::new(SZ_B, 6, 1, 0).on(0, DN),
        TimingRow::new(SZ_B, 8, 1, 1).on(0, MA).ea(),
    ]),
)];
/// NEG: negate.
pub(super) const NEG: &[Form] = &single(0x4400, ARITH);
/// NEGX: negate with extend.
pub(super) const NEGX: &[Form] = &single(0x4000, EXTENDED);

/// SBCD: decimal subtract with extend.
pub(super) const SBCD: &[Form] = &decimal(0x8100);
/// SUB: `<ea>,Dn` or `Dn,<ea>`.
pub(super) const SUB: &[Form] = &add_sub(0x9000);
/// SUBA: subtract from an address register.
pub(super) const SUBA: &[Form] = &add_sub_address(0x90C0);
/// SUBI: subtract an immediate.
pub(super) const SUBI: &[Form] = &add_sub_immediate(0x0400);
/// SUBQ: subtract 1 to 8.
pub(super) const SUBQ: &[Form] = &add_sub_quick(0x5100);
/// SUBX: subtract with extend.
pub(super) const SUBX: &[Form] = &add_sub_extended(0x9100);

/// TST <ea>: data alterable operands only on the 68000.
pub(super) const TST: &[Form] = &[Form::new(
    DATA_ALTERABLE,
    SZ_BWL,
    Some(Size::Word),
    LOGIC,
    Encoding::new(0x4A00, 0xFF00, SizeField::Bits7_6, EaField::Low),
    Timing::Rows(&[TimingRow::new(SZ_BWL, 4, 1, 0)
        .on(0, ModeSet::DATA_ALTERABLE)
        .ea()]),
)];
