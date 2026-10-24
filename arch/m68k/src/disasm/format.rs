//! The text of a decoded instruction (m2-assembler.md §5.2): lowercase canonical spelling with
//! its size, one space, the operands joined by `,`, numbers as §5.2 writes them, and the lexer's
//! tokens of that text. The text reassembles to the same bytes with `optimize = false`.
//!
//! WP0 scaffold: no frozen signature (§5 leaves this module to WP-E); WP-E writes it.
