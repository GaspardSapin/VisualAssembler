//! Decoding (m2-assembler.md §5.1): a lazily built 65,536-entry table that maps each opword to
//! its (entry, form, condition) through `Form::matches`, the extension words read through
//! `isa::fields::places` and the EA fields (the brief index word as the 68000 reads it), and the
//! flow kind and target of each instruction.
//!
//! WP0 scaffold: no frozen signature (§5 leaves this module to WP-E); WP-E writes it.
