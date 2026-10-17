//! Program-flow entries: Bcc, BRA, BSR, DBcc, JMP, JSR, RTE, RTR, RTS, TRAP, TRAPV, NOP, RESET,
//! STOP, ILLEGAL (m1-core-isa-lexer.md §4.1).
//!
//! Cycle counts: tables 8-9 (conditional instructions), 8-10 (JMP, JSR), 8-12 (miscellaneous)
//! and 8-14 (exceptions) of the User's Manual.

use super::encoding::{EaField, Encoding, FieldRule, SizeField};
use super::model::{Form, Operand, QuickRange, SZ_S, SZ_W, Size, SizeSet};
use super::model::{STATUS_LOAD, UNAFFECTED};
use super::modes::{Mode, ModeSet};
use super::timing::{Outcome, Timing, TimingRow};

const LABEL: &[Operand] = &[Operand::Label];
const NO_OPERAND: &[Operand] = &[];
const BRANCH_OUTCOMES: &[Outcome] = &[Outcome::Taken, Outcome::NotTaken];

/// `(An)`.
const IND: ModeSet = ModeSet::only(Mode::Ind);
/// The control modes with one extension word: `d16(An)`, `xxx.W`, `d16(PC)`.
const ONE_WORD: ModeSet = ModeSet::of(&[Mode::Disp, Mode::AbsW, Mode::PcDisp]);
/// The indexed control modes: `d8(An,Xn)`, `d8(PC,Xn)`.
const INDEXED: ModeSet = ModeSet::of(&[Mode::Index, Mode::PcIndex]);
/// `xxx.L`.
const ABS_L: ModeSet = ModeSet::only(Mode::AbsL);

/// An unsized form without operand, at a fixed opword.
const fn fixed(opword: u16, rows: &'static [TimingRow]) -> Form {
    Form::new(
        NO_OPERAND,
        SizeSet::NONE,
        None,
        UNAFFECTED,
        Encoding::new(opword, 0xFFFF, SizeField::None, EaField::None),
        Timing::Rows(rows),
    )
}

/// Bcc: `.S` keeps a non-zero 8-bit displacement in the opword; `.W` has a zero low byte and a
/// 16-bit displacement in the next word. The condition field (bits 11–8, codes 2–15) is free.
pub(super) const BCC: &[Form] = &[
    Form::new(
        LABEL,
        SZ_S,
        None,
        UNAFFECTED,
        Encoding::new(0x6000, 0xF000, SizeField::None, EaField::None)
            .with_rule(FieldRule::Disp8NonZero),
        Timing::Rows(&[
            TimingRow::new(SZ_S, 10, 2, 0).when(Outcome::Taken),
            TimingRow::new(SZ_S, 8, 1, 0).when(Outcome::NotTaken),
        ]),
    )
    .with_outcomes(BRANCH_OUTCOMES),
    Form::new(
        LABEL,
        SZ_W,
        None,
        UNAFFECTED,
        Encoding::new(0x6000, 0xF0FF, SizeField::None, EaField::None),
        Timing::Rows(&[
            TimingRow::new(SZ_W, 10, 2, 0).when(Outcome::Taken),
            TimingRow::new(SZ_W, 12, 2, 0).when(Outcome::NotTaken),
        ]),
    )
    .with_outcomes(BRANCH_OUTCOMES),
];

/// BRA (condition code 0): always taken.
pub(super) const BRA: &[Form] = &[
    Form::new(
        LABEL,
        SZ_S,
        None,
        UNAFFECTED,
        Encoding::new(0x6000, 0xFF00, SizeField::None, EaField::None)
            .with_rule(FieldRule::Disp8NonZero),
        Timing::Rows(&[TimingRow::new(SZ_S, 10, 2, 0)]),
    ),
    Form::new(
        LABEL,
        SZ_W,
        None,
        UNAFFECTED,
        Encoding::new(0x6000, 0xFFFF, SizeField::None, EaField::None),
        Timing::Rows(&[TimingRow::new(SZ_W, 10, 2, 0)]),
    ),
];

/// BSR (condition code 1): pushes the return address, then branches.
pub(super) const BSR: &[Form] = &[
    Form::new(
        LABEL,
        SZ_S,
        None,
        UNAFFECTED,
        Encoding::new(0x6100, 0xFF00, SizeField::None, EaField::None)
            .with_rule(FieldRule::Disp8NonZero),
        Timing::Rows(&[TimingRow::new(SZ_S, 18, 2, 2)]),
    ),
    Form::new(
        LABEL,
        SZ_W,
        None,
        UNAFFECTED,
        Encoding::new(0x6100, 0xFFFF, SizeField::None, EaField::None),
        Timing::Rows(&[TimingRow::new(SZ_W, 18, 2, 2)]),
    ),
];

/// DBcc Dn,<label>: the condition field (bits 11–8, all 16 codes) is free. "True" falls
/// through; otherwise the counter is decremented and the loop branches until it reaches −1.
pub(super) const DBCC: &[Form] = &[Form::new(
    &[Operand::Dn, Operand::Label],
    SZ_W,
    Some(Size::Word),
    UNAFFECTED,
    Encoding::new(0x50C8, 0xF0F8, SizeField::None, EaField::None),
    Timing::Rows(&[
        TimingRow::new(SZ_W, 12, 2, 0).when(Outcome::True),
        TimingRow::new(SZ_W, 10, 2, 0).when(Outcome::NotExpired),
        TimingRow::new(SZ_W, 14, 3, 0).when(Outcome::Expired),
    ]),
)
.with_outcomes(&[Outcome::True, Outcome::NotExpired, Outcome::Expired])];

/// ILLEGAL: the reserved illegal opword; its time is the illegal-instruction exception.
pub(super) const ILLEGAL: &[Form] = &[fixed(0x4AFC, &[TimingRow::new(SizeSet::NONE, 34, 4, 3)])];

/// JMP <ea>: complete per-mode times (table 8-10), so no EA time is added.
pub(super) const JMP: &[Form] = &[Form::new(
    &[Operand::Ea(ModeSet::CONTROL)],
    SizeSet::NONE,
    None,
    UNAFFECTED,
    Encoding::new(0x4EC0, 0xFFC0, SizeField::None, EaField::Low),
    Timing::Rows(&[
        TimingRow::new(SizeSet::NONE, 8, 2, 0).on(0, IND),
        TimingRow::new(SizeSet::NONE, 10, 2, 0).on(0, ONE_WORD),
        TimingRow::new(SizeSet::NONE, 14, 3, 0).on(0, INDEXED),
        TimingRow::new(SizeSet::NONE, 12, 3, 0).on(0, ABS_L),
    ]),
)];

/// JSR <ea>: complete per-mode times (table 8-10).
pub(super) const JSR: &[Form] = &[Form::new(
    &[Operand::Ea(ModeSet::CONTROL)],
    SizeSet::NONE,
    None,
    UNAFFECTED,
    Encoding::new(0x4E80, 0xFFC0, SizeField::None, EaField::Low),
    Timing::Rows(&[
        TimingRow::new(SizeSet::NONE, 16, 2, 2).on(0, IND),
        TimingRow::new(SizeSet::NONE, 18, 2, 2).on(0, ONE_WORD),
        TimingRow::new(SizeSet::NONE, 22, 2, 2).on(0, INDEXED),
        TimingRow::new(SizeSet::NONE, 20, 3, 2).on(0, ABS_L),
    ]),
)];

/// NOP.
pub(super) const NOP: &[Form] = &[fixed(0x4E71, &[TimingRow::new(SizeSet::NONE, 4, 1, 0)])];

/// RESET (supervisor).
pub(super) const RESET: &[Form] =
    &[fixed(0x4E70, &[TimingRow::new(SizeSet::NONE, 132, 1, 0)]).supervisor()];

/// RTE (supervisor): SR and PC are restored from the stack, so every flag is reloaded.
pub(super) const RTE: &[Form] = &[Form {
    flags: STATUS_LOAD,
    ..fixed(0x4E73, &[TimingRow::new(SizeSet::NONE, 20, 5, 0)]).supervisor()
}];

/// RTR: CCR and PC are restored from the stack.
pub(super) const RTR: &[Form] = &[Form {
    flags: STATUS_LOAD,
    ..fixed(0x4E77, &[TimingRow::new(SizeSet::NONE, 20, 5, 0)])
}];

/// RTS.
pub(super) const RTS: &[Form] = &[fixed(0x4E75, &[TimingRow::new(SizeSet::NONE, 16, 4, 0)])];

/// STOP #<data> (supervisor): SR is loaded from the immediate word.
pub(super) const STOP: &[Form] = &[Form::new(
    &[Operand::Imm],
    SizeSet::NONE,
    None,
    STATUS_LOAD,
    Encoding::new(0x4E72, 0xFFFF, SizeField::None, EaField::None),
    Timing::Rows(&[TimingRow::new(SizeSet::NONE, 4, 0, 0)]),
)
.supervisor()];

/// TRAP #<0..15>: the vector number is the low nibble.
pub(super) const TRAP: &[Form] = &[Form::new(
    &[Operand::Quick(QuickRange::Vector4)],
    SizeSet::NONE,
    None,
    UNAFFECTED,
    Encoding::new(0x4E40, 0xFFF0, SizeField::None, EaField::None),
    Timing::Rows(&[TimingRow::new(SizeSet::NONE, 34, 4, 3)]),
)];

/// TRAPV: traps when V is set.
pub(super) const TRAPV: &[Form] = &[fixed(
    0x4E76,
    &[
        TimingRow::new(SizeSet::NONE, 4, 1, 0).when(Outcome::NoTrap),
        TimingRow::new(SizeSet::NONE, 34, 5, 3).when(Outcome::Trap),
    ],
)
.with_outcomes(&[Outcome::NoTrap, Outcome::Trap])];
