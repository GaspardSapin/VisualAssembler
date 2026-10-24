//! Addresses and branch relaxation (m2-assembler.md §4.5, §4.10): the location counter,
//! automatic alignment, `EVEN`/`CNOP` padding, segments, and the relaxation sweeps that shorten
//! relaxable branches (`short-branch`) up to `AsmLimits::max_relaxation_passes`.
//!
//! WP0 scaffold: no frozen signature (§4.2 leaves this module to WP-D); WP-D writes it.
