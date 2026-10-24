//! Macros and conditional assembly (m2-assembler.md §4.6, §4.7): `MACRO` … `ENDM` bodies,
//! argument splitting (`<…>` groups), the `\0`–`\9`, `\@` and `NARG` substitutions, `MEXIT`,
//! the depth and statement limits, and the `IFxx` … `ENDC` blocks evaluated in pass 1.
//!
//! WP0 scaffold: no frozen signature (§4.2 leaves this module to WP-D); WP-D writes it.
