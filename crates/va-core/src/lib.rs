//! CPU-agnostic traits and data types (architecture §3, finalized by m1-core-isa-lexer.md §3
//! and m2-assembler.md §3). No I/O, no threads, no UI, no dependency, no user-facing text: error
//! enums have no `Display`, and the ids returned by `name()`/`id()` methods are identifiers, not
//! prose.
//!
//! Allowed internal dependencies: none.
//!
//! One module per contract (m1 §2.1): [`source`] (texts, byte ranges, lines), [`diag`]
//! (diagnostics), [`syntax`] (the lexer contract), [`isa`] (the instruction view), [`regs`]
//! (register descriptions), [`image`] (program image and source map), [`asm`] (the assembler,
//! its sources, options and output), [`disasm`] (the disassembler) and [`arch`] (the CPU
//! registry trait). The M3 contracts of m1 §3.7 (emulator, I/O profiles) are not defined yet.

pub mod arch;
pub mod asm;
pub mod diag;
pub mod disasm;
pub mod image;
pub mod isa;
pub mod regs;
pub mod source;
pub mod syntax;
