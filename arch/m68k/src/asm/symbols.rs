//! The symbol table of one assembly (m2-assembler.md §4.5, §4.13): labels, local labels and
//! their blocks, `EQU`, `SET` and `=` constants, macro names, pass-1 and pass-2 values, poisoned
//! definitions and the references of every symbol, ending as a `va_core::asm::SymbolTable`.
//!
//! WP0 scaffold: no frozen signature (§4.2 leaves this module to WP-D); WP-D writes it.
