//! The 68000 register file as `va_core::regs::RegisterSpec` descriptions (m1-core-isa-lexer.md
//! §3.6), in register-panel order: D0–D7, A0–A7, USP, SSP, PC, SR.
//!
//! [`RegId`] values are stable across releases (M3 snapshots and M4 layouts store them):
//! D0–D7 are 0–7, A0–A7 are 8–15, then USP 16, SSP 17, PC 18 and SR 19.
//!
//! A7 is the active stack pointer (alias `SP`); USP and SSP are the two stack pointers it stands
//! for in user and supervisor mode. SR carries the condition flags X N Z V C in its low byte (the
//! CCR) and the system byte T S I2–I0 above them.

use va_core::regs::{FlagField, RegId, RegisterGroup, RegisterSpec};

/// Builds a 32-bit register description with no alias and no field.
const fn plain(id: u16, name: &'static str, group: RegisterGroup) -> RegisterSpec {
    RegisterSpec {
        id: RegId(id),
        name,
        aliases: &[],
        bits: 32,
        group,
        fields: &[],
    }
}

/// Builds a one-bit field of the status register.
const fn bit(name: &'static str, bit: u8) -> FlagField {
    FlagField {
        name,
        bit,
        width: 1,
    }
}

/// The fields of SR, highest bit first: trace, supervisor, interrupt mask (3 bits), then the
/// condition flags X N Z V C of the CCR.
static SR_FIELDS: [FlagField; 8] = [
    bit("T", 15),
    bit("S", 13),
    FlagField {
        name: "I",
        bit: 8,
        width: 3,
    },
    bit("X", 4),
    bit("N", 3),
    bit("Z", 2),
    bit("V", 1),
    bit("C", 0),
];

/// Every register of the 68000, in panel order (returned by `M68k::registers`).
pub(crate) static REGISTERS: [RegisterSpec; 20] = [
    plain(0, "D0", RegisterGroup::General),
    plain(1, "D1", RegisterGroup::General),
    plain(2, "D2", RegisterGroup::General),
    plain(3, "D3", RegisterGroup::General),
    plain(4, "D4", RegisterGroup::General),
    plain(5, "D5", RegisterGroup::General),
    plain(6, "D6", RegisterGroup::General),
    plain(7, "D7", RegisterGroup::General),
    plain(8, "A0", RegisterGroup::Address),
    plain(9, "A1", RegisterGroup::Address),
    plain(10, "A2", RegisterGroup::Address),
    plain(11, "A3", RegisterGroup::Address),
    plain(12, "A4", RegisterGroup::Address),
    plain(13, "A5", RegisterGroup::Address),
    plain(14, "A6", RegisterGroup::Address),
    RegisterSpec {
        id: RegId(15),
        name: "A7",
        aliases: &["SP"],
        bits: 32,
        group: RegisterGroup::Address,
        fields: &[],
    },
    plain(16, "USP", RegisterGroup::Control),
    plain(17, "SSP", RegisterGroup::Control),
    plain(18, "PC", RegisterGroup::Control),
    RegisterSpec {
        id: RegId(19),
        name: "SR",
        aliases: &[],
        bits: 16,
        group: RegisterGroup::Status,
        fields: &SR_FIELDS,
    },
];
