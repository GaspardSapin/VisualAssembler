//! The CPU registry trait (m1-core-isa-lexer.md §3.6, architecture §3): everything the
//! CPU-agnostic crates and the apps know about a CPU goes through a `&dyn Architecture`.
//!
//! The M1 methods are required. Capabilities added later are provided methods with a neutral
//! default (architecture §3), so a CPU, or a test fake, that lacks one keeps compiling: M2 adds
//! `assembler`/`disassembler` (m2-assembler.md §3.4, defaults `None`); M3 will add
//! `emulators`/`io_profiles`, which are not defined yet.

use crate::asm::Assembler;
use crate::diag::DiagCode;
use crate::disasm::Disassembler;
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

    /// A new assembler for the CPU's dialect, independent of any other, or `None` if the CPU
    /// has none.
    ///
    /// Default: `None` (callers hide what a CPU does not offer).
    fn assembler(&self) -> Option<Box<dyn Assembler>> {
        None
    }

    /// A disassembler for the CPU, or `None` if the CPU has none.
    ///
    /// Default: `None` (callers hide what a CPU does not offer).
    fn disassembler(&self) -> Option<Box<dyn Disassembler>> {
        None
    }
}
