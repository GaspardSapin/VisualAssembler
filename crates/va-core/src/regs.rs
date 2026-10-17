//! Register descriptions of a CPU (m1-core-isa-lexer.md §3.6), for the register panel (M4), the
//! emulator snapshots (M3) and the flag names of the instruction view.

/// Identifies a register of a CPU: an index chosen by the CPU crate, stable across releases.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RegId(pub u16);

/// The panel group a register belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RegisterGroup {
    /// General-purpose data registers.
    General,
    /// Address registers and stack pointers.
    Address,
    /// Control registers (e.g. program counter, other stack pointers).
    Control,
    /// Status registers, usually with flag fields.
    Status,
}

/// A named bit field of a register, e.g. a condition flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FlagField {
    /// Field name, uppercase (e.g. `"Z"`).
    pub name: &'static str,
    /// Lowest bit of the field.
    pub bit: u8,
    /// Width in bits.
    pub width: u8,
}

/// The description of one register.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegisterSpec {
    /// Stable identifier.
    pub id: RegId,
    /// Name, uppercase (e.g. `"A7"`).
    pub name: &'static str,
    /// Other names of the same register (e.g. `"SP"`).
    pub aliases: &'static [&'static str],
    /// Width in bits.
    pub bits: u8,
    /// Panel group.
    pub group: RegisterGroup,
    /// Named bit fields, empty for a plain register.
    pub fields: &'static [FlagField],
}
