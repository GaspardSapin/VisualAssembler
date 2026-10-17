//! Cycle counts (m1-core-isa-lexer.md §4.4), typed from the MC68000 User's Manual §8
//! (tables 8-1 to 8-14).
//!
//! A cost is written `clocks(reads/writes)`, as in the manual. A form's [`TimingRow`]s
//! partition the space (size × addressing mode of one varying operand × outcome); a row gives
//! the base cost and whether the EA calculation time of that operand ([`EA_TIME`]) is added
//! (the manual's `+`). A footnote that changes the base for some modes is a separate row.
//! MOVE and MOVEA use the two full matrices [`MOVE_BW`] and [`MOVE_L`].

use va_core::isa::{Bound, Cycles};

use super::model::{Form, Size, SizeSet};
use super::modes::{Mode, ModeSet};

/// What happened when an instruction ran, for the instructions whose cost depends on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// The cost does not depend on an outcome.
    Always,
    /// The branch was taken (Bcc).
    Taken,
    /// The branch was not taken (Bcc).
    NotTaken,
    /// The condition was true (Scc; DBcc, which then falls through).
    True,
    /// The condition was false (Scc).
    False,
    /// DBcc: the condition was false and the counter did not reach −1 (the loop branches).
    NotExpired,
    /// DBcc: the condition was false and the counter reached −1 (the loop ends).
    Expired,
    /// The instruction trapped (CHK, TRAPV).
    Trap,
    /// The instruction did not trap (CHK, TRAPV).
    NoTrap,
}

impl Outcome {
    /// Returns the kebab-case id of the outcome (documented as `isa-m68k-outcome-<id>`).
    pub const fn id(self) -> &'static str {
        match self {
            Outcome::Always => "always",
            Outcome::Taken => "taken",
            Outcome::NotTaken => "not-taken",
            Outcome::True => "true",
            Outcome::False => "false",
            Outcome::NotExpired => "not-expired",
            Outcome::Expired => "expired",
            Outcome::Trap => "trap",
            Outcome::NoTrap => "no-trap",
        }
    }
}

/// What a count-dependent cost counts: the `n` in `6+2n`, `8+4n`, `38+2n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unit {
    /// Shift or rotation count.
    ShiftCount,
    /// Number of registers moved by MOVEM.
    RegisterCount,
    /// Number of 1 bits in the MULU source.
    OneBits,
    /// Number of `01` or `10` bit pairs in the MULS source extended with a 0 below its LSB.
    BitPairs,
}

impl Unit {
    /// Returns the kebab-case id of the unit (documented as `isa-m68k-unit-<id>`).
    pub const fn id(self) -> &'static str {
        match self {
            Unit::ShiftCount => "shift-count",
            Unit::RegisterCount => "register-count",
            Unit::OneBits => "one-bits",
            Unit::BitPairs => "bit-pairs",
        }
    }
}

/// One timing row of a form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TimingRow {
    /// The sizes the row applies to ([`SizeSet::NONE`] for an unsized form).
    pub sizes: SizeSet,
    /// Index of the varying operand.
    pub operand: u8,
    /// The modes of that operand the row applies to; [`ModeSet::NONE`] when the cost does not
    /// depend on a mode.
    pub modes: ModeSet,
    /// The outcome the row applies to.
    pub outcome: Outcome,
    /// The base cost.
    pub base: Cycles,
    /// Whether the EA calculation time of the varying operand is added.
    pub plus_ea: bool,
    /// A cost added once per counted unit.
    pub per: Option<(Cycles, Unit)>,
    /// Whether the total is exact or an upper bound.
    pub bound: Bound,
}

/// Returns `clocks(reads/writes)`.
pub(super) const fn cost(clocks: u16, reads: u8, writes: u8) -> Cycles {
    Cycles {
        clocks,
        reads,
        writes,
    }
}

impl TimingRow {
    /// Returns an exact, mode-independent row for any outcome, without EA time.
    pub(super) const fn new(sizes: SizeSet, clocks: u16, reads: u8, writes: u8) -> TimingRow {
        TimingRow {
            sizes,
            operand: 0,
            modes: ModeSet::NONE,
            outcome: Outcome::Always,
            base: cost(clocks, reads, writes),
            plus_ea: false,
            per: None,
            bound: Bound::Exact,
        }
    }

    /// Returns this row restricted to `modes` of operand `operand`.
    pub(super) const fn on(self, operand: u8, modes: ModeSet) -> TimingRow {
        TimingRow {
            operand,
            modes,
            ..self
        }
    }

    /// Returns this row with the EA calculation time of its operand added.
    pub(super) const fn ea(self) -> TimingRow {
        TimingRow {
            plus_ea: true,
            ..self
        }
    }

    /// Returns this row restricted to `outcome`.
    pub(super) const fn when(self, outcome: Outcome) -> TimingRow {
        TimingRow { outcome, ..self }
    }

    /// Returns this row with `clocks(reads/writes)` added per `unit`.
    pub(super) const fn per_unit(
        self,
        clocks: u16,
        reads: u8,
        writes: u8,
        unit: Unit,
    ) -> TimingRow {
        TimingRow {
            per: Some((cost(clocks, reads, writes), unit)),
            ..self
        }
    }

    /// Returns this row with its total marked as an upper bound.
    pub(super) const fn upper_bound(self) -> TimingRow {
        TimingRow {
            bound: Bound::Max,
            ..self
        }
    }

    /// Returns whether the row applies to `size` (`None` for an unsized form), to `mode` of
    /// its varying operand (ignored by a mode-independent row) and to `outcome`.
    pub fn applies(&self, size: Option<Size>, mode: Option<Mode>, outcome: Outcome) -> bool {
        let size_holds = match size {
            None => self.sizes.is_empty(),
            Some(size) => self.sizes.contains(size),
        };
        let mode_holds = self.modes.is_empty() || mode.is_some_and(|m| self.modes.contains(m));
        size_holds && mode_holds && self.outcome == outcome
    }
}

/// How a form's cycle counts are given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Timing {
    /// Rows partitioning (size × mode of the varying operand × outcome).
    Rows(&'static [TimingRow]),
    /// MOVE and MOVEA: [`MOVE_BW`] or [`MOVE_L`], by source mode and destination column.
    MoveMatrix,
}

/// EA calculation times, table 8-1: `[mode][0 = .B/.W, 1 = .L]`, rows in [`Mode`] order.
pub static EA_TIME: [[Cycles; 2]; 12] = [
    [cost(0, 0, 0), cost(0, 0, 0)],   // Dn
    [cost(0, 0, 0), cost(0, 0, 0)],   // An
    [cost(4, 1, 0), cost(8, 2, 0)],   // (An)
    [cost(4, 1, 0), cost(8, 2, 0)],   // (An)+
    [cost(6, 1, 0), cost(10, 2, 0)],  // -(An)
    [cost(8, 2, 0), cost(12, 3, 0)],  // d16(An)
    [cost(10, 2, 0), cost(14, 3, 0)], // d8(An,Xn)
    [cost(8, 2, 0), cost(12, 3, 0)],  // xxx.W
    [cost(12, 3, 0), cost(16, 4, 0)], // xxx.L
    [cost(8, 2, 0), cost(12, 3, 0)],  // d16(PC)
    [cost(10, 2, 0), cost(14, 3, 0)], // d8(PC,Xn)
    [cost(4, 1, 0), cost(8, 2, 0)],   // #imm
];

/// MOVE.B and MOVE.W, table 8-2: `[source mode][destination column]`, source rows in [`Mode`]
/// order, destination columns `Dn An (An) (An)+ -(An) d16(An) d8(An,Xn) xxx.W xxx.L`.
#[rustfmt::skip]
pub static MOVE_BW: [[Cycles; 9]; 12] = [
    // Dn
    [cost(4, 1, 0), cost(4, 1, 0), cost(8, 1, 1), cost(8, 1, 1), cost(8, 1, 1),
     cost(12, 2, 1), cost(14, 2, 1), cost(12, 2, 1), cost(16, 3, 1)],
    // An
    [cost(4, 1, 0), cost(4, 1, 0), cost(8, 1, 1), cost(8, 1, 1), cost(8, 1, 1),
     cost(12, 2, 1), cost(14, 2, 1), cost(12, 2, 1), cost(16, 3, 1)],
    // (An)
    [cost(8, 2, 0), cost(8, 2, 0), cost(12, 2, 1), cost(12, 2, 1), cost(12, 2, 1),
     cost(16, 3, 1), cost(18, 3, 1), cost(16, 3, 1), cost(20, 4, 1)],
    // (An)+
    [cost(8, 2, 0), cost(8, 2, 0), cost(12, 2, 1), cost(12, 2, 1), cost(12, 2, 1),
     cost(16, 3, 1), cost(18, 3, 1), cost(16, 3, 1), cost(20, 4, 1)],
    // -(An)
    [cost(10, 2, 0), cost(10, 2, 0), cost(14, 2, 1), cost(14, 2, 1), cost(14, 2, 1),
     cost(18, 3, 1), cost(20, 3, 1), cost(18, 3, 1), cost(22, 4, 1)],
    // d16(An)
    [cost(12, 3, 0), cost(12, 3, 0), cost(16, 3, 1), cost(16, 3, 1), cost(16, 3, 1),
     cost(20, 4, 1), cost(22, 4, 1), cost(20, 4, 1), cost(24, 5, 1)],
    // d8(An,Xn)
    [cost(14, 3, 0), cost(14, 3, 0), cost(18, 3, 1), cost(18, 3, 1), cost(18, 3, 1),
     cost(22, 4, 1), cost(24, 4, 1), cost(22, 4, 1), cost(26, 5, 1)],
    // xxx.W
    [cost(12, 3, 0), cost(12, 3, 0), cost(16, 3, 1), cost(16, 3, 1), cost(16, 3, 1),
     cost(20, 4, 1), cost(22, 4, 1), cost(20, 4, 1), cost(24, 5, 1)],
    // xxx.L
    [cost(16, 4, 0), cost(16, 4, 0), cost(20, 4, 1), cost(20, 4, 1), cost(20, 4, 1),
     cost(24, 5, 1), cost(26, 5, 1), cost(24, 5, 1), cost(28, 6, 1)],
    // d16(PC)
    [cost(12, 3, 0), cost(12, 3, 0), cost(16, 3, 1), cost(16, 3, 1), cost(16, 3, 1),
     cost(20, 4, 1), cost(22, 4, 1), cost(20, 4, 1), cost(24, 5, 1)],
    // d8(PC,Xn)
    [cost(14, 3, 0), cost(14, 3, 0), cost(18, 3, 1), cost(18, 3, 1), cost(18, 3, 1),
     cost(22, 4, 1), cost(24, 4, 1), cost(22, 4, 1), cost(26, 5, 1)],
    // #imm
    [cost(8, 2, 0), cost(8, 2, 0), cost(12, 2, 1), cost(12, 2, 1), cost(12, 2, 1),
     cost(16, 3, 1), cost(18, 3, 1), cost(16, 3, 1), cost(20, 4, 1)],
];

/// MOVE.L, table 8-3: `[source mode][destination column]`, same layout as [`MOVE_BW`].
#[rustfmt::skip]
pub static MOVE_L: [[Cycles; 9]; 12] = [
    // Dn
    [cost(4, 1, 0), cost(4, 1, 0), cost(12, 1, 2), cost(12, 1, 2), cost(12, 1, 2),
     cost(16, 2, 2), cost(18, 2, 2), cost(16, 2, 2), cost(20, 3, 2)],
    // An
    [cost(4, 1, 0), cost(4, 1, 0), cost(12, 1, 2), cost(12, 1, 2), cost(12, 1, 2),
     cost(16, 2, 2), cost(18, 2, 2), cost(16, 2, 2), cost(20, 3, 2)],
    // (An)
    [cost(12, 3, 0), cost(12, 3, 0), cost(20, 3, 2), cost(20, 3, 2), cost(20, 3, 2),
     cost(24, 4, 2), cost(26, 4, 2), cost(24, 4, 2), cost(28, 5, 2)],
    // (An)+
    [cost(12, 3, 0), cost(12, 3, 0), cost(20, 3, 2), cost(20, 3, 2), cost(20, 3, 2),
     cost(24, 4, 2), cost(26, 4, 2), cost(24, 4, 2), cost(28, 5, 2)],
    // -(An)
    [cost(14, 3, 0), cost(14, 3, 0), cost(22, 3, 2), cost(22, 3, 2), cost(22, 3, 2),
     cost(26, 4, 2), cost(28, 4, 2), cost(26, 4, 2), cost(30, 5, 2)],
    // d16(An)
    [cost(16, 4, 0), cost(16, 4, 0), cost(24, 4, 2), cost(24, 4, 2), cost(24, 4, 2),
     cost(28, 5, 2), cost(30, 5, 2), cost(28, 5, 2), cost(32, 6, 2)],
    // d8(An,Xn)
    [cost(18, 4, 0), cost(18, 4, 0), cost(26, 4, 2), cost(26, 4, 2), cost(26, 4, 2),
     cost(30, 5, 2), cost(32, 5, 2), cost(30, 5, 2), cost(34, 6, 2)],
    // xxx.W
    [cost(16, 4, 0), cost(16, 4, 0), cost(24, 4, 2), cost(24, 4, 2), cost(24, 4, 2),
     cost(28, 5, 2), cost(30, 5, 2), cost(28, 5, 2), cost(32, 6, 2)],
    // xxx.L
    [cost(20, 5, 0), cost(20, 5, 0), cost(28, 5, 2), cost(28, 5, 2), cost(28, 5, 2),
     cost(32, 6, 2), cost(34, 6, 2), cost(32, 6, 2), cost(36, 7, 2)],
    // d16(PC)
    [cost(16, 4, 0), cost(16, 4, 0), cost(24, 4, 2), cost(24, 4, 2), cost(24, 4, 2),
     cost(28, 5, 2), cost(30, 5, 2), cost(28, 5, 2), cost(32, 6, 2)],
    // d8(PC,Xn)
    [cost(18, 4, 0), cost(18, 4, 0), cost(26, 4, 2), cost(26, 4, 2), cost(26, 4, 2),
     cost(30, 5, 2), cost(32, 5, 2), cost(30, 5, 2), cost(34, 6, 2)],
    // #imm
    [cost(12, 3, 0), cost(12, 3, 0), cost(20, 3, 2), cost(20, 3, 2), cost(20, 3, 2),
     cost(24, 4, 2), cost(26, 4, 2), cost(24, 4, 2), cost(28, 5, 2)],
];

/// Returns the EA calculation time of `mode` for an operation of `size` (table 8-1).
pub fn ea_time(mode: Mode, size: Size) -> Cycles {
    let column = usize::from(size == Size::Long);
    EA_TIME[mode.index()][column]
}

/// Returns the column of a MOVE destination mode in [`MOVE_BW`] and [`MOVE_L`], or `None` for a
/// mode that is never a destination.
pub const fn move_column(mode: Mode) -> Option<usize> {
    match mode {
        Mode::PcDisp | Mode::PcIndex | Mode::Imm => None,
        _ => Some(mode.index()),
    }
}

fn add(a: Cycles, b: Cycles) -> Option<Cycles> {
    Some(Cycles {
        clocks: a.clocks.checked_add(b.clocks)?,
        reads: a.reads.checked_add(b.reads)?,
        writes: a.writes.checked_add(b.writes)?,
    })
}

fn times(cycles: Cycles, n: u32) -> Option<Cycles> {
    let n16 = u16::try_from(n).ok()?;
    let n8 = u8::try_from(n).ok()?;
    Some(Cycles {
        clocks: cycles.clocks.checked_mul(n16)?,
        reads: cycles.reads.checked_mul(n8)?,
        writes: cycles.writes.checked_mul(n8)?,
    })
}

/// Returns the total cost of `form` at `size`, with `modes` as the modes of its two operands,
/// for `outcome` and the count `n` of a count-dependent cost (ignored otherwise).
///
/// The EA calculation time and the per-unit cost are included. `size` is ignored for an
/// unsized form. Returns `None` when the form does not accept `size`, when no row matches, or
/// when the total overflows.
pub fn cycles(
    form: &Form,
    size: Size,
    modes: [Option<Mode>; 2],
    outcome: Outcome,
    n: u32,
) -> Option<(Cycles, Bound)> {
    let size = if form.sizes.is_empty() {
        None
    } else if form.sizes.contains(size) {
        Some(size)
    } else {
        return None;
    };
    match form.timing {
        Timing::MoveMatrix => {
            let [Some(source), Some(destination)] = modes else {
                return None;
            };
            let accepts = |index: usize, mode: Mode| {
                form.operands
                    .get(index)
                    .is_some_and(|operand| operand.modes().contains(mode))
            };
            if outcome != Outcome::Always || !accepts(0, source) || !accepts(1, destination) {
                return None;
            }
            let matrix = if size == Some(Size::Long) {
                &MOVE_L
            } else {
                &MOVE_BW
            };
            let column = move_column(destination)?;
            Some((matrix[source.index()][column], Bound::Exact))
        }
        Timing::Rows(rows) => {
            let row = rows.iter().find(|row| {
                let mode = modes.get(usize::from(row.operand)).copied().flatten();
                row.applies(size, mode, outcome)
            })?;
            let mut total = row.base;
            if row.plus_ea {
                let mode = modes.get(usize::from(row.operand)).copied().flatten()?;
                total = add(total, ea_time(mode, size.unwrap_or(Size::Word)))?;
            }
            if let Some((unit_cost, _)) = row.per {
                total = add(total, times(unit_cost, n)?)?;
            }
            Some((total, row.bound))
        }
    }
}
