//! 68000: ISA tables, syntax/lexer, A68k-dialect assembler, disassembler, emulator adapter over
//! the `m68k` crate, I/O profiles, its own `.ftl` resources. Internal layout:
//! `src/{isa,syntax,asm,disasm,emu,io}/`.
//!
//! Allowed internal dependencies: `va-core`.
//!
//! M1 (m1-core-isa-lexer.md §3.6, §4, §5, §6): the instruction tables ([`isa`]), the shared
//! A68k-dialect lexer ([`syntax`]), and [`M68k`], which registers them as a
//! `va_core::arch::Architecture` together with the register file and the embedded English and
//! French documentation (`locales/{en,fr}/{isa,diagnostics}.ftl`). The apps register the
//! instance [`M68K`]. M2 adds `asm` and `disasm`, M3 `emu` and `io`.

pub mod isa;
pub mod syntax;

mod arch;
mod locales;
mod registers;

pub use arch::{M68K, M68k};
