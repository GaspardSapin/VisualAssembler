//! Effective-address modes and the Motorola addressing categories (m1-core-isa-lexer.md §4.3).
//!
//! A [`ModeSet`] is a 12-bit set over [`Mode`], one bit per mode in Motorola order. The named
//! categories follow the Programmer's Reference Manual definitions for the 68000: *data* is every
//! mode but `An`, *memory* every mode but `Dn` and `An`, *control* the modes that name a memory
//! location without a size-dependent side effect, *alterable* the modes that can be written.

/// A 68000 effective-address mode, in Motorola order (also the row order of `EA_TIME` and of
/// the MOVE tables).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Mode {
    /// Data register direct, `Dn`.
    Dn,
    /// Address register direct, `An`.
    An,
    /// Address register indirect, `(An)`.
    Ind,
    /// Address register indirect with postincrement, `(An)+`.
    PostInc,
    /// Address register indirect with predecrement, `-(An)`.
    PreDec,
    /// Address register indirect with a 16-bit displacement, `d16(An)`.
    Disp,
    /// Address register indirect with an 8-bit displacement and an index, `d8(An,Xn)`.
    Index,
    /// Absolute short address, `xxx.W`.
    AbsW,
    /// Absolute long address, `xxx.L`.
    AbsL,
    /// Program counter with a 16-bit displacement, `d16(PC)`.
    PcDisp,
    /// Program counter with an 8-bit displacement and an index, `d8(PC,Xn)`.
    PcIndex,
    /// Immediate data, `#imm`.
    Imm,
}

impl Mode {
    /// Every mode, in Motorola order.
    pub const ALL: [Mode; 12] = [
        Mode::Dn,
        Mode::An,
        Mode::Ind,
        Mode::PostInc,
        Mode::PreDec,
        Mode::Disp,
        Mode::Index,
        Mode::AbsW,
        Mode::AbsL,
        Mode::PcDisp,
        Mode::PcIndex,
        Mode::Imm,
    ];

    /// Returns the position of the mode in Motorola order: 0 for `Dn` up to 11 for `#imm`.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Returns the frozen notation of the mode (used in JSON and by `va isa`):
    /// `Dn An (An) (An)+ -(An) d16(An) d8(An,Xn) xxx.W xxx.L d16(PC) d8(PC,Xn) #imm`.
    pub const fn notation(self) -> &'static str {
        match self {
            Mode::Dn => "Dn",
            Mode::An => "An",
            Mode::Ind => "(An)",
            Mode::PostInc => "(An)+",
            Mode::PreDec => "-(An)",
            Mode::Disp => "d16(An)",
            Mode::Index => "d8(An,Xn)",
            Mode::AbsW => "xxx.W",
            Mode::AbsL => "xxx.L",
            Mode::PcDisp => "d16(PC)",
            Mode::PcIndex => "d8(PC,Xn)",
            Mode::Imm => "#imm",
        }
    }

    /// Returns the 3-bit mode field of the mode and, for mode 7, the register field value that
    /// selects it (`None` when the register field holds a register number).
    pub const fn fields(self) -> (u8, Option<u8>) {
        match self {
            Mode::Dn => (0, None),
            Mode::An => (1, None),
            Mode::Ind => (2, None),
            Mode::PostInc => (3, None),
            Mode::PreDec => (4, None),
            Mode::Disp => (5, None),
            Mode::Index => (6, None),
            Mode::AbsW => (7, Some(0)),
            Mode::AbsL => (7, Some(1)),
            Mode::PcDisp => (7, Some(2)),
            Mode::PcIndex => (7, Some(3)),
            Mode::Imm => (7, Some(4)),
        }
    }

    /// Decodes a 3-bit mode field and a 3-bit register field (higher bits are ignored).
    ///
    /// Returns `None` for mode 7 with a register field of 5, 6 or 7, which name no 68000 mode.
    pub const fn from_fields(mode: u8, register: u8) -> Option<Mode> {
        match (mode & 7, register & 7) {
            (0, _) => Some(Mode::Dn),
            (1, _) => Some(Mode::An),
            (2, _) => Some(Mode::Ind),
            (3, _) => Some(Mode::PostInc),
            (4, _) => Some(Mode::PreDec),
            (5, _) => Some(Mode::Disp),
            (6, _) => Some(Mode::Index),
            (_, 0) => Some(Mode::AbsW),
            (_, 1) => Some(Mode::AbsL),
            (_, 2) => Some(Mode::PcDisp),
            (_, 3) => Some(Mode::PcIndex),
            (_, 4) => Some(Mode::Imm),
            _ => None,
        }
    }

    const fn bit(self) -> u16 {
        1 << (self as u16)
    }
}

/// A set of addressing modes: the modes one operand accepts, or the modes a timing row covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ModeSet(u16);

impl ModeSet {
    /// No mode.
    pub const NONE: ModeSet = ModeSet(0);
    /// Every mode.
    pub const ALL: ModeSet = ModeSet(0x0FFF);
    /// Data addressing: every mode except `An`.
    pub const DATA: ModeSet = ModeSet::ALL.without(Mode::An);
    /// Memory addressing: every mode except `Dn` and `An`.
    pub const MEMORY: ModeSet = ModeSet::ALL.without(Mode::Dn).without(Mode::An);
    /// Control addressing: `(An)`, `d16(An)`, `d8(An,Xn)`, `xxx.W`, `xxx.L`, `d16(PC)`,
    /// `d8(PC,Xn)`.
    pub const CONTROL: ModeSet = ModeSet::of(&[
        Mode::Ind,
        Mode::Disp,
        Mode::Index,
        Mode::AbsW,
        Mode::AbsL,
        Mode::PcDisp,
        Mode::PcIndex,
    ]);
    /// Alterable addressing: every mode except `d16(PC)`, `d8(PC,Xn)` and `#imm`.
    pub const ALTERABLE: ModeSet = ModeSet::ALL
        .without(Mode::PcDisp)
        .without(Mode::PcIndex)
        .without(Mode::Imm);
    /// Data alterable: data and alterable.
    pub const DATA_ALTERABLE: ModeSet = ModeSet::DATA.intersection(ModeSet::ALTERABLE);
    /// Memory alterable: memory and alterable.
    pub const MEMORY_ALTERABLE: ModeSet = ModeSet::MEMORY.intersection(ModeSet::ALTERABLE);
    /// Control alterable: control and alterable.
    pub const CONTROL_ALTERABLE: ModeSet = ModeSet::CONTROL.intersection(ModeSet::ALTERABLE);

    /// Returns the set holding only `mode`.
    pub const fn only(mode: Mode) -> ModeSet {
        ModeSet(mode.bit())
    }

    /// Returns the set of the listed modes.
    pub const fn of(modes: &[Mode]) -> ModeSet {
        let mut bits = 0;
        let mut i = 0;
        while i < modes.len() {
            bits |= modes[i].bit();
            i += 1;
        }
        ModeSet(bits)
    }

    /// Returns this set with `mode` added.
    pub const fn with(self, mode: Mode) -> ModeSet {
        ModeSet(self.0 | mode.bit())
    }

    /// Returns this set with `mode` removed.
    pub const fn without(self, mode: Mode) -> ModeSet {
        ModeSet(self.0 & !mode.bit())
    }

    /// Returns the modes of either set.
    pub const fn union(self, other: ModeSet) -> ModeSet {
        ModeSet(self.0 | other.0)
    }

    /// Returns the modes of both sets.
    pub const fn intersection(self, other: ModeSet) -> ModeSet {
        ModeSet(self.0 & other.0)
    }

    /// Returns the modes of this set that are not in `other`.
    pub const fn minus(self, other: ModeSet) -> ModeSet {
        ModeSet(self.0 & !other.0)
    }

    /// Returns whether `mode` is in the set.
    pub const fn contains(self, mode: Mode) -> bool {
        self.0 & mode.bit() != 0
    }

    /// Returns whether every mode of this set is in `other`.
    pub const fn is_subset(self, other: ModeSet) -> bool {
        self.0 & !other.0 == 0
    }

    /// Returns whether the set has no mode.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns the number of modes in the set.
    pub const fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Returns the raw bits: bit `i` stands for the mode of [`Mode::index`] `i`.
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Iterates over the modes of the set, in Motorola order.
    pub fn iter(self) -> impl Iterator<Item = Mode> {
        Mode::ALL
            .into_iter()
            .filter(move |&mode| self.contains(mode))
    }
}
