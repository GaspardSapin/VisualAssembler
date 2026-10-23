//! The data model of the 68000 instruction tables (m1-core-isa-lexer.md §4.3, frozen for WP-C,
//! WP-D and M2).
//!
//! An [`Instruction`] is one table entry (one documentation id). It has one or more [`Form`]s:
//! an operand pattern with its sizes, flag effects, privilege, opword encoding and cycle counts.
//! The conditional families (Bcc, DBcc, Scc) are one entry each, whose forms leave the condition
//! field free.

use va_core::isa::FlagEffect;

use super::encoding::Encoding;
use super::modes::{Mode, ModeSet};
use super::timing::{Outcome, Timing};

/// An operation size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Size {
    /// Byte, `.B` (8 bits).
    Byte,
    /// Word, `.W` (16 bits).
    Word,
    /// Long word, `.L` (32 bits).
    Long,
    /// Short branch, `.S`: an 8-bit displacement inside the opword.
    Short,
}

impl Size {
    /// Every size, in display order (byte, short, word, long).
    pub const ALL: [Size; 4] = [Size::Byte, Size::Short, Size::Word, Size::Long];

    /// Returns the suffix letter, uppercase, without the dot: `"B"`, `"W"`, `"L"` or `"S"`.
    pub const fn letter(self) -> &'static str {
        match self {
            Size::Byte => "B",
            Size::Word => "W",
            Size::Long => "L",
            Size::Short => "S",
        }
    }

    /// Parses a suffix letter without the dot, ASCII case-insensitive.
    pub fn from_letter(letter: &str) -> Option<Size> {
        Size::ALL
            .into_iter()
            .find(|size| size.letter().eq_ignore_ascii_case(letter))
    }

    const fn bit(self) -> u8 {
        1 << (self as u8)
    }
}

/// A set of operation sizes. [`SizeSet::NONE`] marks an unsized form, which takes no suffix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct SizeSet(u8);

impl SizeSet {
    /// No size: the form is unsized.
    pub const NONE: SizeSet = SizeSet(0);

    /// Returns the set of the listed sizes.
    pub const fn of(sizes: &[Size]) -> SizeSet {
        let mut bits = 0;
        let mut i = 0;
        while i < sizes.len() {
            bits |= sizes[i].bit();
            i += 1;
        }
        SizeSet(bits)
    }

    /// Returns whether `size` is in the set.
    pub const fn contains(self, size: Size) -> bool {
        self.0 & size.bit() != 0
    }

    /// Returns whether every size of this set is in `other`.
    pub const fn is_subset(self, other: SizeSet) -> bool {
        self.0 & !other.0 == 0
    }

    /// Returns whether the set has no size.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns the number of sizes in the set.
    pub const fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Iterates over the sizes of the set, in display order (byte, short, word, long).
    pub fn iter(self) -> impl Iterator<Item = Size> {
        Size::ALL
            .into_iter()
            .filter(move |&size| self.contains(size))
    }
}

/// The value range of an immediate encoded inside the opword.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QuickRange {
    /// 1 to 8, stored as 1–7 and 0 (ADDQ, SUBQ, shift counts).
    OneToEight,
    /// −128 to 127 (MOVEQ).
    Signed8,
    /// 0 to 15 (TRAP vector).
    Vector4,
}

impl QuickRange {
    /// Returns the smallest and the largest accepted value.
    pub const fn bounds(self) -> (i32, i32) {
        match self {
            QuickRange::OneToEight => (1, 8),
            QuickRange::Signed8 => (-128, 127),
            QuickRange::Vector4 => (0, 15),
        }
    }
}

/// One operand of a form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operand {
    /// An effective address restricted to a set of modes.
    Ea(ModeSet),
    /// A data register in a register field.
    Dn,
    /// An address register in a register field.
    An,
    /// `(An)+` with the register in a register field (CMPM).
    PostIncAn,
    /// `-(An)` with the register in a register field (ABCD, SBCD, ADDX, SUBX).
    PreDecAn,
    /// `d16(An)` with the register in a register field (MOVEP).
    DispAn,
    /// Immediate data in extension words.
    Imm,
    /// Immediate data inside the opword.
    Quick(QuickRange),
    /// A MOVEM register list (a 16-bit mask in an extension word).
    RegList,
    /// A branch target (a displacement from the PC).
    Label,
    /// The condition code register.
    Ccr,
    /// The status register.
    Sr,
    /// The user stack pointer.
    Usp,
}

impl Operand {
    /// Returns the operand in notation, e.g. `"<ea>"`, `"Dn"`, `"#<data>"`, `"#<1..8>"`.
    pub const fn notation(self) -> &'static str {
        match self {
            Operand::Ea(_) => "<ea>",
            Operand::Dn => "Dn",
            Operand::An => "An",
            Operand::PostIncAn => "(An)+",
            Operand::PreDecAn => "-(An)",
            Operand::DispAn => "d16(An)",
            Operand::Imm => "#<data>",
            Operand::Quick(QuickRange::OneToEight) => "#<1..8>",
            Operand::Quick(QuickRange::Signed8) => "#<-128..127>",
            Operand::Quick(QuickRange::Vector4) => "#<0..15>",
            Operand::RegList => "<list>",
            Operand::Label => "<label>",
            Operand::Ccr => "CCR",
            Operand::Sr => "SR",
            Operand::Usp => "USP",
        }
    }

    /// Returns the addressing modes the operand stands for: the set of an `Ea`, the single
    /// mode of a register or immediate operand, and no mode for a list, label or special
    /// register.
    pub const fn modes(self) -> ModeSet {
        match self {
            Operand::Ea(set) => set,
            Operand::Dn => ModeSet::only(Mode::Dn),
            Operand::An => ModeSet::only(Mode::An),
            Operand::PostIncAn => ModeSet::only(Mode::PostInc),
            Operand::PreDecAn => ModeSet::only(Mode::PreDec),
            Operand::DispAn => ModeSet::only(Mode::Disp),
            Operand::Imm | Operand::Quick(_) => ModeSet::only(Mode::Imm),
            Operand::RegList | Operand::Label | Operand::Ccr | Operand::Sr | Operand::Usp => {
                ModeSet::NONE
            }
        }
    }
}

/// The effect of a form on the five condition flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Flags {
    /// Extend.
    pub x: FlagEffect,
    /// Negative.
    pub n: FlagEffect,
    /// Zero.
    pub z: FlagEffect,
    /// Overflow.
    pub v: FlagEffect,
    /// Carry.
    pub c: FlagEffect,
}

impl Flags {
    /// Returns the flags with their names, in display order X N Z V C.
    pub const fn named(self) -> [(&'static str, FlagEffect); 5] {
        [
            ("X", self.x),
            ("N", self.n),
            ("Z", self.z),
            ("V", self.v),
            ("C", self.c),
        ]
    }
}

/// One operand pattern of an instruction, with everything that depends on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Form {
    /// The operands, in source order (0 to 2).
    pub operands: &'static [Operand],
    /// The accepted sizes; [`SizeSet::NONE`] for an unsized form.
    pub sizes: SizeSet,
    /// The size used without a suffix; `None` when a suffix is required, or when the assembler
    /// chooses (branches).
    pub default_size: Option<Size>,
    /// The effect on X N Z V C.
    pub flags: Flags,
    /// Whether the form needs supervisor mode.
    pub privileged: bool,
    /// The outcomes its cycle count depends on (`[Outcome::Always]` when none).
    pub outcomes: &'static [Outcome],
    /// The opword facts.
    pub encoding: Encoding,
    /// The cycle counts.
    pub timing: Timing,
}

impl Form {
    /// Returns an unprivileged form whose cycle count depends on no outcome.
    pub(super) const fn new(
        operands: &'static [Operand],
        sizes: SizeSet,
        default_size: Option<Size>,
        flags: Flags,
        encoding: Encoding,
        timing: Timing,
    ) -> Form {
        Form {
            operands,
            sizes,
            default_size,
            flags,
            privileged: false,
            outcomes: &[Outcome::Always],
            encoding,
            timing,
        }
    }

    /// Returns this form marked as needing supervisor mode.
    pub(super) const fn supervisor(self) -> Form {
        Form {
            privileged: true,
            ..self
        }
    }

    /// Returns this form with the outcomes its cycle count depends on.
    pub(super) const fn with_outcomes(self, outcomes: &'static [Outcome]) -> Form {
        Form { outcomes, ..self }
    }
}

/// A conditional instruction family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    /// Conditional branch, `Bcc` (conditions 2–15; codes 0 and 1 are BRA and BSR).
    Bcc,
    /// Test condition, decrement and branch, `DBcc` (all 16 conditions).
    DBcc,
    /// Set according to condition, `Scc` (all 16 conditions).
    Scc,
}

impl Family {
    /// Returns the family name, `"Bcc"`, `"DBcc"` or `"Scc"`.
    pub const fn name(self) -> &'static str {
        match self {
            Family::Bcc => "Bcc",
            Family::DBcc => "DBcc",
            Family::Scc => "Scc",
        }
    }

    /// Returns the spelling prefix put before a condition name: `"B"`, `"DB"` or `"S"`.
    pub const fn prefix(self) -> &'static str {
        match self {
            Family::Bcc => "B",
            Family::DBcc => "DB",
            Family::Scc => "S",
        }
    }

    /// Returns the lowest condition code of the family (2 for Bcc, 0 otherwise); every code
    /// from it up to 15 belongs to the family.
    pub const fn first_condition(self) -> u8 {
        match self {
            Family::Bcc => 2,
            Family::DBcc | Family::Scc => 0,
        }
    }
}

/// One entry of the instruction table (one documentation id).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Instruction {
    /// The entry name: the mnemonic, or the family name (`"Bcc"`, `"DBcc"`, `"Scc"`).
    pub name: &'static str,
    /// The conditional family, if the entry is one.
    pub family: Option<Family>,
    /// Fluent id of the documentation, `isa-m68k-<name in lowercase>`.
    pub doc_id: &'static str,
    /// The forms of the entry.
    pub forms: &'static [Form],
    /// The entries whose forms this mnemonic also accepts (A68k picks the variant from the
    /// operands, e.g. ADD with an address register destination is ADDA).
    pub also_accepts: &'static [&'static str],
}

/// What a spelling resolves to, as returned by `spelling_info`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpellingInfo {
    /// The canonical spelling (e.g. `"BCC"` for `BHS`, `"DBF"` for `DBRA`, itself otherwise).
    pub canonical: &'static str,
    /// Whether the instruction takes operands (false for NOP, RESET, RTE, RTR, RTS, TRAPV and
    /// ILLEGAL: what follows them is a comment).
    pub takes_operands: bool,
    /// Fluent id of the entry's documentation (e.g. `"isa-m68k-bcc"` for `BEQ`).
    pub doc_id: &'static str,
}

/// `.B`.
pub(super) const SZ_B: SizeSet = SizeSet::of(&[Size::Byte]);
/// `.W`.
pub(super) const SZ_W: SizeSet = SizeSet::of(&[Size::Word]);
/// `.L`.
pub(super) const SZ_L: SizeSet = SizeSet::of(&[Size::Long]);
/// `.S`.
pub(super) const SZ_S: SizeSet = SizeSet::of(&[Size::Short]);
/// `.B` and `.W` (timing rows that share a cost).
pub(super) const SZ_BW: SizeSet = SizeSet::of(&[Size::Byte, Size::Word]);
/// `.W` and `.L`.
pub(super) const SZ_WL: SizeSet = SizeSet::of(&[Size::Word, Size::Long]);
/// `.B`, `.W` and `.L`.
pub(super) const SZ_BWL: SizeSet = SizeSet::of(&[Size::Byte, Size::Word, Size::Long]);

const fn flags(x: FlagEffect, n: FlagEffect, z: FlagEffect, v: FlagEffect, c: FlagEffect) -> Flags {
    Flags { x, n, z, v, c }
}

use FlagEffect::{Cleared as CLR, Result as RES, Set as SET, Special as SPC};
use FlagEffect::{Unaffected as UNA, Undefined as UND};

/// `- - - - -`: no flag changes.
pub(super) const UNAFFECTED: Flags = flags(UNA, UNA, UNA, UNA, UNA);
/// `* * * * *`: arithmetic (ADD, SUB, NEG, ASL).
pub(super) const ARITH: Flags = flags(RES, RES, RES, RES, RES);
/// `- * * * *`: comparisons.
pub(super) const COMPARE: Flags = flags(UNA, RES, RES, RES, RES);
/// `- * * 0 0`: moves, logic operations and tests.
pub(super) const LOGIC: Flags = flags(UNA, RES, RES, CLR, CLR);
/// `* * ! * *`: multi-precision arithmetic (ADDX, SUBX, NEGX), Z only cleared.
pub(super) const EXTENDED: Flags = flags(RES, RES, SPC, RES, RES);
/// `* U ! U *`: decimal arithmetic (ABCD, SBCD, NBCD), Z only cleared.
pub(super) const DECIMAL: Flags = flags(RES, UND, SPC, UND, RES);
/// `- 0 1 0 0`: CLR.
pub(super) const CLEARED: Flags = flags(UNA, CLR, SET, CLR, CLR);
/// `- * * * 0`: divisions.
pub(super) const DIVIDE: Flags = flags(UNA, RES, RES, RES, CLR);
/// `- - * - -`: bit operations.
pub(super) const BIT: Flags = flags(UNA, UNA, RES, UNA, UNA);
/// `- ! U U U`: CHK.
pub(super) const CHECK: Flags = flags(UNA, SPC, UND, UND, UND);
/// `* * * 0 *`: ASR, logical shifts, and rotations through X.
pub(super) const SHIFT: Flags = flags(RES, RES, RES, CLR, RES);
/// `- * * 0 *`: rotations without X.
pub(super) const ROTATE: Flags = flags(UNA, RES, RES, CLR, RES);
/// `! ! ! ! !`: logic operations on CCR or SR, each flag combined with its immediate bit.
pub(super) const STATUS_LOGIC: Flags = flags(SPC, SPC, SPC, SPC, SPC);
/// `* * * * *`: the status register is loaded (MOVE to CCR/SR, RTE, RTR, STOP).
pub(super) const STATUS_LOAD: Flags = flags(RES, RES, RES, RES, RES);
