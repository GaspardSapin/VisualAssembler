//! CPU-agnostic instruction view (m1-core-isa-lexer.md §3.5): what `va isa` (M1) and hover (M5,
//! in `va-lang`, which may not depend on `arch/*`) show about an instruction.
//!
//! Every string is an identifier or notation, never prose: mode and operand strings are
//! assembler notation (never translated), outcome and unit ids are documented by the CPU crate
//! as the Fluent messages `isa-<arch>-outcome-<id>` and `isa-<arch>-unit-<id>`, and `doc_id`
//! fields are Fluent ids.

/// The instruction tables of one CPU, seen through CPU-agnostic types.
pub trait InstructionSet: Send + Sync {
    /// Every accepted spelling, uppercase, sorted.
    fn spellings(&self) -> &[&'static str];

    /// `spelling` without suffix, any case; `size` is the suffix letter(s) without the dot, any
    /// case.
    ///
    /// Returns the instruction with only the forms that accept `size` (every form when `size` is
    /// `None`).
    fn describe(&self, spelling: &str, size: Option<&str>) -> Result<InstructionInfo, LookupError>;

    /// EA calculation times, referenced by [`TimingInfo::plus_ea`].
    fn ea_timing(&self) -> &[EaTiming];
}

/// Why [`InstructionSet::describe`] found nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LookupError {
    /// The spelling is not an instruction of this CPU.
    Unknown,
    /// The instruction exists, but no form accepts the requested size.
    SizeNotAccepted,
}

/// What one spelling means, as returned by [`InstructionSet::describe`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstructionInfo {
    /// The spelling asked for, normalized to uppercase (e.g. `"BHS"`).
    pub spelling: &'static str,
    /// The canonical spelling of the same instruction (e.g. `"BCC"`).
    pub canonical: &'static str,
    /// The conditional family, if any: `"Bcc"`, `"DBcc"` or `"Scc"`.
    pub family: Option<&'static str>,
    /// The condition of a conditional spelling.
    pub condition: Option<ConditionInfo>,
    /// The other spellings of `canonical`.
    pub aliases: Vec<&'static str>,
    /// The table entries whose forms this spelling also accepts (e.g. ADD accepts ADDA, ADDI).
    pub also_accepts: Vec<&'static str>,
    /// The normalized size filter (e.g. `"S"` for `BRA.B`), or `None` when no size was given.
    pub size: Option<&'static str>,
    /// Fluent id of the documentation entry, e.g. `"isa-m68k-bcc"`.
    pub doc_id: &'static str,
    /// The forms that accept `size`.
    pub forms: Vec<FormInfo>,
}

/// A condition of a conditional instruction family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConditionInfo {
    /// Condition name, uppercase (e.g. `"EQ"`).
    pub name: &'static str,
    /// The flag test, in notation (e.g. `"Z"`, `"!C & !Z"`).
    pub test: &'static str,
    /// Fluent id of the condition's documentation.
    pub doc_id: &'static str,
}

/// One operand form of an instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormInfo {
    /// The form in notation, e.g. `"ADD.B/W/L <ea>,Dn"`.
    pub syntax: String,
    /// Accepted size letters (empty for an unsized form).
    pub sizes: Vec<&'static str>,
    /// The size used without a suffix, if the form has one.
    pub default_size: Option<&'static str>,
    /// Whether the form needs supervisor mode.
    pub privileged: bool,
    /// The operands, in source order.
    pub operands: Vec<OperandInfo>,
    /// Effect on each condition flag, in the CPU's display order.
    pub flags: Vec<FlagInfo>,
    /// Cycle counts.
    pub timing: Vec<TimingInfo>,
}

/// One operand of a form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperandInfo {
    /// The operand in notation, e.g. `"<ea>"`, `"Dn"`, `"#<data>"`.
    pub notation: &'static str,
    /// The addressing modes it accepts, in notation (e.g. `"(An)+"`); empty when the notation
    /// says it all.
    pub modes: Vec<&'static str>,
}

/// The effect of a form on one condition flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FlagInfo {
    /// Flag name, e.g. `"Z"`.
    pub flag: &'static str,
    /// What the form does to it.
    pub effect: FlagEffect,
}

/// What an instruction does to a condition flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FlagEffect {
    /// Left unchanged.
    Unaffected,
    /// Set from the result.
    Result,
    /// Always cleared.
    Cleared,
    /// Always set.
    Set,
    /// Left undefined.
    Undefined,
    /// An instruction-specific rule, described by the documentation entry's `.flags` attribute.
    Special,
}

impl FlagEffect {
    /// Returns the identifier of the effect: `"unaffected"`, `"result"`, `"cleared"`, `"set"`,
    /// `"undefined"` or `"special"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Unaffected => "unaffected",
            Self::Result => "result",
            Self::Cleared => "cleared",
            Self::Set => "set",
            Self::Undefined => "undefined",
            Self::Special => "special",
        }
    }
}

/// A cost in clock periods with its bus cycles, Motorola's `n(r/w)` notation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Cycles {
    /// Clock periods.
    pub clocks: u16,
    /// Bus read cycles.
    pub reads: u8,
    /// Bus write cycles.
    pub writes: u8,
}

/// Whether a cycle count is exact or an upper bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bound {
    /// The count is exact.
    Exact,
    /// The count is the maximum (e.g. a division whose time depends on the operands).
    Max,
}

impl Bound {
    /// Returns the identifier of the bound: `"exact"` or `"max"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Max => "max",
        }
    }
}

/// One timing row of a form: the cost for some sizes, operand modes and outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimingInfo {
    /// Size letters the row applies to.
    pub sizes: Vec<&'static str>,
    /// Operand modes the row applies to; empty means any operands.
    pub when: Vec<ModeCondition>,
    /// Outcome id the row applies to (e.g. `"taken"`), if the cost depends on one.
    pub outcome: Option<&'static str>,
    /// The base cost.
    pub base: Cycles,
    /// Indexes of the operands whose EA calculation time ([`InstructionSet::ea_timing`]) is
    /// added.
    pub plus_ea: Vec<u8>,
    /// A cost added per counted unit, if any.
    pub per_unit: Option<PerUnit>,
    /// Whether the total is exact or an upper bound.
    pub bound: Bound,
}

/// A restriction of a timing row to some modes of one operand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModeCondition {
    /// Index of the operand.
    pub operand: u8,
    /// The modes, in notation.
    pub modes: Vec<&'static str>,
}

/// A cost added once per counted unit (e.g. per shift).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PerUnit {
    /// The cost of one unit.
    pub cycles: Cycles,
    /// Unit id, e.g. `"shift-count"`.
    pub unit: &'static str,
}

/// The EA calculation time of one addressing mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EaTiming {
    /// The mode, in notation.
    pub mode: &'static str,
    /// Size letters this cost applies to.
    pub sizes: Vec<&'static str>,
    /// The cost.
    pub cycles: Cycles,
}
