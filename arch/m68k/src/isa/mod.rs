//! 68000 instruction tables: model, addressing modes, conditions, encodings, cycle counts, and
//! the CPU-agnostic view (m1-core-isa-lexer.md §4).
//!
//! **WP0 stub (m1 §1), owned by WP-B in wave 2.** It only provides [`M68kIsa`] implementing
//! `va_core::isa::InstructionSet` with an empty table: no spelling, every lookup is
//! `LookupError::Unknown`. WP-B replaces it with the tables of §4 (first `SPELLINGS` and
//! `spelling_info`, which WP-C needs) and moves the trait impl into `view.rs`.

use va_core::isa::{EaTiming, InstructionInfo, InstructionSet, LookupError};

/// The 68000 instruction set, seen through `va_core::isa::InstructionSet`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct M68kIsa;

impl InstructionSet for M68kIsa {
    /// Stub: no spelling until WP-B lands `SPELLINGS`.
    fn spellings(&self) -> &[&'static str] {
        &[]
    }

    /// Stub: every spelling is unknown until WP-B lands the tables.
    fn describe(
        &self,
        _spelling: &str,
        _size: Option<&str>,
    ) -> Result<InstructionInfo, LookupError> {
        Err(LookupError::Unknown)
    }

    /// Stub: no EA timing until WP-B lands `EA_TIME`.
    fn ea_timing(&self) -> &[EaTiming] {
        &[]
    }
}
