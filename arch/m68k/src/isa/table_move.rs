//! Data-movement entries: MOVE (with its CCR, SR and USP forms), MOVEA, MOVEM, MOVEP, MOVEQ,
//! EXG, SWAP, LEA, PEA, LINK, UNLK (m1-core-isa-lexer.md §4.1).
//!
//! Cycle counts: tables 8-2 and 8-3 (MOVE matrices), 8-10 (LEA, PEA, MOVEM), 8-12
//! (miscellaneous) and 8-13 (MOVEP) of the User's Manual.

use super::encoding::{EaField, Encoding, SizeField};
use super::model::{Form, Operand, QuickRange, SZ_B, SZ_L, SZ_W, SZ_WL, Size, SizeSet};
use super::model::{LOGIC, STATUS_LOAD, UNAFFECTED};
use super::modes::{Mode, ModeSet};
use super::timing::{Timing, TimingRow, Unit};

/// `Dn`.
const DN: ModeSet = ModeSet::only(Mode::Dn);
/// `(An)`.
const IND: ModeSet = ModeSet::only(Mode::Ind);
/// The control modes with one extension word: `d16(An)`, `xxx.W`, `d16(PC)`.
const ONE_WORD: ModeSet = ModeSet::of(&[Mode::Disp, Mode::AbsW, Mode::PcDisp]);
/// The indexed control modes: `d8(An,Xn)`, `d8(PC,Xn)`.
const INDEXED: ModeSet = ModeSet::of(&[Mode::Index, Mode::PcIndex]);
/// `xxx.L`.
const ABS_L: ModeSet = ModeSet::only(Mode::AbsL);
/// MOVEM register-to-memory destinations: control alterable and `-(An)`.
const MOVEM_TO_MEMORY: ModeSet = ModeSet::CONTROL_ALTERABLE.with(Mode::PreDec);
/// MOVEM memory-to-register sources: control and `(An)+`.
const MOVEM_FROM_MEMORY: ModeSet = ModeSet::CONTROL.with(Mode::PostInc);

/// One-row timing for a form whose cost depends on nothing.
const fn flat(sizes: SizeSet, clocks: u16, reads: u8, writes: u8) -> TimingRow {
    TimingRow::new(sizes, clocks, reads, writes)
}

/// MOVE: the general form is split by size because an address register is not a byte source.
/// The CCR, SR and USP variants are forms of the same entry.
pub(super) const MOVE: &[Form] = &[
    Form::new(
        &[
            Operand::Ea(ModeSet::DATA),
            Operand::Ea(ModeSet::DATA_ALTERABLE),
        ],
        SZ_B,
        None,
        LOGIC,
        Encoding::new(0x0000, 0xC000, SizeField::Move13_12, EaField::Both),
        Timing::MoveMatrix,
    ),
    Form::new(
        &[
            Operand::Ea(ModeSet::ALL),
            Operand::Ea(ModeSet::DATA_ALTERABLE),
        ],
        SZ_WL,
        Some(Size::Word),
        LOGIC,
        Encoding::new(0x0000, 0xC000, SizeField::Move13_12, EaField::Both),
        Timing::MoveMatrix,
    ),
    // MOVE <ea>,CCR: the low byte of the source word becomes the CCR.
    Form::new(
        &[Operand::Ea(ModeSet::DATA), Operand::Ccr],
        SZ_W,
        Some(Size::Word),
        STATUS_LOAD,
        Encoding::new(0x44C0, 0xFFC0, SizeField::None, EaField::Low),
        Timing::Rows(&[TimingRow::new(SZ_W, 12, 2, 0).on(0, ModeSet::DATA).ea()]),
    ),
    // MOVE <ea>,SR (supervisor).
    Form::new(
        &[Operand::Ea(ModeSet::DATA), Operand::Sr],
        SZ_W,
        Some(Size::Word),
        STATUS_LOAD,
        Encoding::new(0x46C0, 0xFFC0, SizeField::None, EaField::Low),
        Timing::Rows(&[TimingRow::new(SZ_W, 12, 2, 0).on(0, ModeSet::DATA).ea()]),
    )
    .supervisor(),
    // MOVE SR,<ea>: not privileged on the 68000.
    Form::new(
        &[Operand::Sr, Operand::Ea(ModeSet::DATA_ALTERABLE)],
        SZ_W,
        Some(Size::Word),
        UNAFFECTED,
        Encoding::new(0x40C0, 0xFFC0, SizeField::None, EaField::Low),
        Timing::Rows(&[
            TimingRow::new(SZ_W, 6, 1, 0).on(1, DN),
            TimingRow::new(SZ_W, 8, 1, 1)
                .on(1, ModeSet::MEMORY_ALTERABLE)
                .ea(),
        ]),
    ),
    // MOVE USP,An (supervisor).
    Form::new(
        &[Operand::Usp, Operand::An],
        SZ_L,
        Some(Size::Long),
        UNAFFECTED,
        Encoding::new(0x4E68, 0xFFF8, SizeField::None, EaField::None),
        Timing::Rows(&[flat(SZ_L, 4, 1, 0)]),
    )
    .supervisor(),
    // MOVE An,USP (supervisor).
    Form::new(
        &[Operand::An, Operand::Usp],
        SZ_L,
        Some(Size::Long),
        UNAFFECTED,
        Encoding::new(0x4E60, 0xFFF8, SizeField::None, EaField::None),
        Timing::Rows(&[flat(SZ_L, 4, 1, 0)]),
    )
    .supervisor(),
];

/// MOVEA <ea>,An: destination mode field 001; word sources are sign-extended.
pub(super) const MOVEA: &[Form] = &[Form::new(
    &[Operand::Ea(ModeSet::ALL), Operand::An],
    SZ_WL,
    Some(Size::Word),
    UNAFFECTED,
    Encoding::new(0x0040, 0xC1C0, SizeField::Move13_12, EaField::Low),
    Timing::MoveMatrix,
)];

/// MOVEM: complete per-mode times (table 8-10) plus a cost per register moved.
pub(super) const MOVEM: &[Form] = &[
    Form::new(
        &[Operand::RegList, Operand::Ea(MOVEM_TO_MEMORY)],
        SZ_WL,
        Some(Size::Word),
        UNAFFECTED,
        Encoding::new(0x4880, 0xFF80, SizeField::Bit6, EaField::Low),
        Timing::Rows(&[
            TimingRow::new(SZ_W, 8, 2, 0)
                .on(1, ModeSet::of(&[Mode::Ind, Mode::PreDec]))
                .per_unit(4, 0, 1, Unit::RegisterCount),
            TimingRow::new(SZ_W, 12, 3, 0)
                .on(1, ModeSet::of(&[Mode::Disp, Mode::AbsW]))
                .per_unit(4, 0, 1, Unit::RegisterCount),
            TimingRow::new(SZ_W, 14, 3, 0)
                .on(1, ModeSet::only(Mode::Index))
                .per_unit(4, 0, 1, Unit::RegisterCount),
            TimingRow::new(SZ_W, 16, 4, 0)
                .on(1, ABS_L)
                .per_unit(4, 0, 1, Unit::RegisterCount),
            TimingRow::new(SZ_L, 8, 2, 0)
                .on(1, ModeSet::of(&[Mode::Ind, Mode::PreDec]))
                .per_unit(8, 0, 2, Unit::RegisterCount),
            TimingRow::new(SZ_L, 12, 3, 0)
                .on(1, ModeSet::of(&[Mode::Disp, Mode::AbsW]))
                .per_unit(8, 0, 2, Unit::RegisterCount),
            TimingRow::new(SZ_L, 14, 3, 0)
                .on(1, ModeSet::only(Mode::Index))
                .per_unit(8, 0, 2, Unit::RegisterCount),
            TimingRow::new(SZ_L, 16, 4, 0)
                .on(1, ABS_L)
                .per_unit(8, 0, 2, Unit::RegisterCount),
        ]),
    ),
    Form::new(
        &[Operand::Ea(MOVEM_FROM_MEMORY), Operand::RegList],
        SZ_WL,
        Some(Size::Word),
        UNAFFECTED,
        Encoding::new(0x4C80, 0xFF80, SizeField::Bit6, EaField::Low),
        Timing::Rows(&[
            TimingRow::new(SZ_W, 12, 3, 0)
                .on(0, ModeSet::of(&[Mode::Ind, Mode::PostInc]))
                .per_unit(4, 1, 0, Unit::RegisterCount),
            TimingRow::new(SZ_W, 16, 4, 0)
                .on(0, ONE_WORD)
                .per_unit(4, 1, 0, Unit::RegisterCount),
            TimingRow::new(SZ_W, 18, 4, 0)
                .on(0, INDEXED)
                .per_unit(4, 1, 0, Unit::RegisterCount),
            TimingRow::new(SZ_W, 20, 5, 0)
                .on(0, ABS_L)
                .per_unit(4, 1, 0, Unit::RegisterCount),
            TimingRow::new(SZ_L, 12, 3, 0)
                .on(0, ModeSet::of(&[Mode::Ind, Mode::PostInc]))
                .per_unit(8, 2, 0, Unit::RegisterCount),
            TimingRow::new(SZ_L, 16, 4, 0)
                .on(0, ONE_WORD)
                .per_unit(8, 2, 0, Unit::RegisterCount),
            TimingRow::new(SZ_L, 18, 4, 0)
                .on(0, INDEXED)
                .per_unit(8, 2, 0, Unit::RegisterCount),
            TimingRow::new(SZ_L, 20, 5, 0)
                .on(0, ABS_L)
                .per_unit(8, 2, 0, Unit::RegisterCount),
        ]),
    ),
];

/// MOVEP: alternate bytes between a data register and memory (bit 7 is the direction).
pub(super) const MOVEP: &[Form] = &[
    Form::new(
        &[Operand::DispAn, Operand::Dn],
        SZ_WL,
        Some(Size::Word),
        UNAFFECTED,
        Encoding::new(0x0108, 0xF1B8, SizeField::Bit6, EaField::None),
        Timing::Rows(&[flat(SZ_W, 16, 4, 0), flat(SZ_L, 24, 6, 0)]),
    ),
    Form::new(
        &[Operand::Dn, Operand::DispAn],
        SZ_WL,
        Some(Size::Word),
        UNAFFECTED,
        Encoding::new(0x0188, 0xF1B8, SizeField::Bit6, EaField::None),
        Timing::Rows(&[flat(SZ_W, 16, 2, 2), flat(SZ_L, 24, 2, 4)]),
    ),
];

/// MOVEQ #<-128..127>,Dn: the data byte is in the opword, sign-extended to 32 bits.
pub(super) const MOVEQ: &[Form] = &[Form::new(
    &[Operand::Quick(QuickRange::Signed8), Operand::Dn],
    SZ_L,
    Some(Size::Long),
    LOGIC,
    Encoding::new(0x7000, 0xF100, SizeField::None, EaField::None),
    Timing::Rows(&[flat(SZ_L, 4, 1, 0)]),
)];

/// EXG: three register pairings (opmode 01000, 01001, 10001).
pub(super) const EXG: &[Form] = &[
    Form::new(
        &[Operand::Dn, Operand::Dn],
        SZ_L,
        Some(Size::Long),
        UNAFFECTED,
        Encoding::new(0xC140, 0xF1F8, SizeField::None, EaField::None),
        Timing::Rows(&[flat(SZ_L, 6, 1, 0)]),
    ),
    Form::new(
        &[Operand::An, Operand::An],
        SZ_L,
        Some(Size::Long),
        UNAFFECTED,
        Encoding::new(0xC148, 0xF1F8, SizeField::None, EaField::None),
        Timing::Rows(&[flat(SZ_L, 6, 1, 0)]),
    ),
    Form::new(
        &[Operand::Dn, Operand::An],
        SZ_L,
        Some(Size::Long),
        UNAFFECTED,
        Encoding::new(0xC188, 0xF1F8, SizeField::None, EaField::None),
        Timing::Rows(&[flat(SZ_L, 6, 1, 0)]),
    ),
];

/// SWAP Dn: exchanges the register halves.
pub(super) const SWAP: &[Form] = &[Form::new(
    &[Operand::Dn],
    SZ_W,
    Some(Size::Word),
    LOGIC,
    Encoding::new(0x4840, 0xFFF8, SizeField::None, EaField::None),
    Timing::Rows(&[flat(SZ_W, 4, 1, 0)]),
)];

/// LEA <ea>,An: complete per-mode times (table 8-10).
pub(super) const LEA: &[Form] = &[Form::new(
    &[Operand::Ea(ModeSet::CONTROL), Operand::An],
    SZ_L,
    Some(Size::Long),
    UNAFFECTED,
    Encoding::new(0x41C0, 0xF1C0, SizeField::None, EaField::Low),
    Timing::Rows(&[
        TimingRow::new(SZ_L, 4, 1, 0).on(0, IND),
        TimingRow::new(SZ_L, 8, 2, 0).on(0, ONE_WORD),
        TimingRow::new(SZ_L, 12, 2, 0).on(0, INDEXED),
        TimingRow::new(SZ_L, 12, 3, 0).on(0, ABS_L),
    ]),
)];

/// PEA <ea>: complete per-mode times (table 8-10).
pub(super) const PEA: &[Form] = &[Form::new(
    &[Operand::Ea(ModeSet::CONTROL)],
    SZ_L,
    Some(Size::Long),
    UNAFFECTED,
    Encoding::new(0x4840, 0xFFC0, SizeField::None, EaField::Low),
    Timing::Rows(&[
        TimingRow::new(SZ_L, 12, 1, 2).on(0, IND),
        TimingRow::new(SZ_L, 16, 2, 2).on(0, ONE_WORD),
        TimingRow::new(SZ_L, 20, 2, 2).on(0, INDEXED),
        TimingRow::new(SZ_L, 20, 3, 2).on(0, ABS_L),
    ]),
)];

/// LINK An,#<displacement>.
pub(super) const LINK: &[Form] = &[Form::new(
    &[Operand::An, Operand::Imm],
    SZ_W,
    Some(Size::Word),
    UNAFFECTED,
    Encoding::new(0x4E50, 0xFFF8, SizeField::None, EaField::None),
    Timing::Rows(&[flat(SZ_W, 16, 2, 2)]),
)];

/// UNLK An.
pub(super) const UNLK: &[Form] = &[Form::new(
    &[Operand::An],
    SizeSet::NONE,
    None,
    UNAFFECTED,
    Encoding::new(0x4E58, 0xFFF8, SizeField::None, EaField::None),
    Timing::Rows(&[flat(SizeSet::NONE, 12, 3, 0)]),
)];
