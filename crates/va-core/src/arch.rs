//! The CPU registry trait (m1-core-isa-lexer.md §3.6, architecture §3): everything the
//! CPU-agnostic crates and the apps know about a CPU goes through a `&dyn Architecture`.
//!
//! M2 adds `assembler`/`disassembler` and M3 `emulators`/`io_profiles`, with the signatures of
//! m1 §3.7; they are not defined yet.

use crate::diag::DiagCode;
use crate::isa::InstructionSet;
use crate::regs::RegisterSpec;
use crate::syntax::SyntaxProvider;

/// One supported CPU, registered by the apps (e.g. `va_arch_m68k::M68K`).
pub trait Architecture: Send + Sync {
    /// Stable identifier of the CPU, e.g. `"m68k"`.
    fn id(&self) -> &'static str;

    /// The lexer of the CPU's assembler dialect.
    fn syntax(&self) -> &dyn SyntaxProvider;

    /// The CPU-agnostic view of its instruction tables.
    fn instruction_set(&self) -> &dyn InstructionSet;

    /// Its registers, in panel order.
    fn registers(&self) -> &'static [RegisterSpec];

    /// Every diagnostic code this CPU can emit (drives the localization completeness test).
    fn diagnostic_codes(&self) -> &'static [DiagCode];

    /// The embedded Fluent resources (`.ftl` sources) of language `lang` (e.g. `"en"`), or an
    /// empty slice for a language the CPU does not provide.
    fn locale_resources(&self, lang: &str) -> &'static [&'static str];
}
