//! 68000: ISA tables, syntax/lexer, A68k-dialect assembler, disassembler, emulator adapter over
//! the `m68k` crate, I/O profiles, its own `.ftl` resources. Internal layout:
//! `src/{isa,syntax,asm,disasm,emu,io}/`.
//!
//! Allowed internal dependencies: `va-core`.
//!
//! **WP0 stub (m1-core-isa-lexer.md §1), owned by WP-D in wave 2.** It only makes the crate
//! compile and register a `va_core::arch::Architecture`: [`M68k`] answers its id and hands out
//! the [`isa`] and [`syntax`] stubs, with no register, diagnostic code or locale resource yet.
//! WP-D moves [`M68k`] and [`M68K`] into the private `arch` module (re-exported here, so the
//! paths `va_arch_m68k::{M68k, M68K}` stay), and adds the private `registers` and `locales`
//! modules (m1 §3.6, §6.1).

pub mod isa;
pub mod syntax;

use va_core::arch::Architecture;
use va_core::diag::DiagCode;
use va_core::isa::InstructionSet;
use va_core::regs::RegisterSpec;
use va_core::syntax::SyntaxProvider;

/// The Motorola 68000, as registered by the apps through [`M68K`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct M68k;

/// The 68000 architecture instance (`&va_arch_m68k::M68K` in the apps' registries).
pub static M68K: M68k = M68k;

impl Architecture for M68k {
    fn id(&self) -> &'static str {
        "m68k"
    }

    fn syntax(&self) -> &dyn SyntaxProvider {
        &syntax::M68kSyntax
    }

    fn instruction_set(&self) -> &dyn InstructionSet {
        &isa::M68kIsa
    }

    /// Stub: empty until WP-D adds D0–D7, A0–A7, USP, SSP, PC and SR (m1 §3.6).
    fn registers(&self) -> &'static [RegisterSpec] {
        &[]
    }

    /// Stub: empty until WP-D returns WP-C's `syntax::DIAG_CODES` (m1 §5, §9.1).
    fn diagnostic_codes(&self) -> &'static [DiagCode] {
        &[]
    }

    /// Stub: empty until WP-D embeds `locales/{en,fr}/{isa,diagnostics}.ftl` (m1 §6.1).
    fn locale_resources(&self, _lang: &str) -> &'static [&'static str] {
        &[]
    }
}
