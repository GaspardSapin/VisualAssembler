//! The 68000 as a `va_core::arch::Architecture` (m1-core-isa-lexer.md §3.6): the single entry
//! point through which the apps and the CPU-agnostic crates reach the lexer, the instruction
//! tables, the registers, the diagnostic codes and the documentation of this CPU.

use va_core::arch::Architecture;
use va_core::diag::DiagCode;
use va_core::isa::InstructionSet;
use va_core::regs::RegisterSpec;
use va_core::syntax::SyntaxProvider;

use crate::{isa, locales, registers, syntax};

/// The Motorola 68000, as registered by the apps through [`M68K`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct M68k;

/// The 68000 architecture instance (`&va_arch_m68k::M68K` in the apps' registries).
pub static M68K: M68k = M68k;

impl Architecture for M68k {
    /// Always `"m68k"`; it also prefixes the documentation ids (`isa-m68k-…`).
    fn id(&self) -> &'static str {
        "m68k"
    }

    /// The A68k-dialect lexer ([`syntax::M68kSyntax`]).
    fn syntax(&self) -> &dyn SyntaxProvider {
        &syntax::M68kSyntax
    }

    /// The 68000 instruction tables ([`isa::M68kIsa`]).
    fn instruction_set(&self) -> &dyn InstructionSet {
        &isa::M68kIsa
    }

    /// D0–D7, A0–A7 (A7 alias `SP`), USP, SSP, PC and SR with its flag fields.
    fn registers(&self) -> &'static [RegisterSpec] {
        &registers::REGISTERS
    }

    /// The lexer codes [`syntax::DIAG_CODES`]; M2 adds the assembler codes.
    fn diagnostic_codes(&self) -> &'static [DiagCode] {
        &syntax::DIAG_CODES
    }

    /// The embedded `isa.ftl` and `diagnostics.ftl` of `"en"` or `"fr"`; empty for any other
    /// language code.
    fn locale_resources(&self, lang: &str) -> &'static [&'static str] {
        locales::resources(lang)
    }
}
