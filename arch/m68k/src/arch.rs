//! The 68000 as a `va_core::arch::Architecture` (m1-core-isa-lexer.md §3.6, m2-assembler.md
//! §3.4): the single entry point through which the apps and the CPU-agnostic crates reach the
//! lexer, the instruction tables, the registers, the diagnostic codes, the documentation, the
//! assembler and the disassembler of this CPU.

use va_core::arch::Architecture;
use va_core::asm::Assembler;
use va_core::diag::DiagCode;
use va_core::disasm::Disassembler;
use va_core::isa::InstructionSet;
use va_core::regs::RegisterSpec;
use va_core::syntax::SyntaxProvider;

use crate::{asm, disasm, isa, locales, registers, syntax};

/// The Motorola 68000, as registered by the apps through [`M68K`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct M68k;

/// The 68000 architecture instance (`&va_arch_m68k::M68K` in the apps' registries).
pub static M68K: M68k = M68k;

/// Number of codes in [`DIAG_CODES`]: the lexer's, then the assembler's.
const DIAG_CODE_COUNT: usize = syntax::DIAG_CODES.len() + asm::codes::ASM_DIAG_CODES.len();

/// Every diagnostic code of the 68000 (m2 §7.1): [`syntax::DIAG_CODES`] followed by
/// [`asm::codes::ASM_DIAG_CODES`].
static DIAG_CODES: [DiagCode; DIAG_CODE_COUNT] =
    concat_codes(&syntax::DIAG_CODES, &asm::codes::ASM_DIAG_CODES);

/// Returns `first` followed by `second`; `N` must be the sum of their lengths (checked at
/// compile time: a mismatch stops the build).
const fn concat_codes<const N: usize>(first: &[DiagCode], second: &[DiagCode]) -> [DiagCode; N] {
    assert!(
        first.len() + second.len() == N,
        "N is not the sum of the lengths"
    );
    let mut out = [DiagCode(""); N];
    let mut i = 0;
    while i < first.len() {
        out[i] = first[i];
        i += 1;
    }
    let mut j = 0;
    while j < second.len() {
        out[first.len() + j] = second[j];
        j += 1;
    }
    out
}

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

    /// The lexer codes [`syntax::DIAG_CODES`] followed by the assembler codes
    /// [`asm::codes::ASM_DIAG_CODES`] (m2 §7.1).
    fn diagnostic_codes(&self) -> &'static [DiagCode] {
        &DIAG_CODES
    }

    /// The embedded `isa.ftl` and `diagnostics.ftl` of `"en"` or `"fr"`; empty for any other
    /// language code.
    fn locale_resources(&self, lang: &str) -> &'static [&'static str] {
        locales::resources(lang)
    }

    /// A new [`asm::M68kAssembler`] (m2 §4.1).
    fn assembler(&self) -> Option<Box<dyn Assembler>> {
        Some(Box::new(asm::M68kAssembler))
    }

    /// The [`disasm::M68kDisassembler`] (m2 §5.1).
    fn disassembler(&self) -> Option<Box<dyn Disassembler>> {
        Some(Box::new(disasm::M68kDisassembler))
    }
}
