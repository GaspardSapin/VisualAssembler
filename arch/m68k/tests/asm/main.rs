//! Tests for `src/asm/` (m2-assembler.md §10.1, exit criterion E3), run by
//! `cargo test -p va-arch-m68k --test asm`: line fields and operands (`parser`), expressions
//! (`expr`), form selection and encoding (`encode`), the optimizations (`optimize`), the
//! generated forms corpus (`corpus`), directives (`directives`), macros and conditionals
//! (`macros`), branch relaxation (`relax`), S-record and binary output (`output`), the listing
//! (`listing`), the golden fixtures of `fixtures/asm*/` (`fixtures`) and the programmatic
//! diagnostic cases with the code coverage check (`diagnostics`).
//!
//! This file belongs to the WP0 scaffold and stays as written (§1): each wave-2 work package
//! owns its module files only, and keeps its helpers there.
#![allow(clippy::unwrap_used)]

mod corpus;
mod diagnostics;
mod directives;
mod encode;
mod expr;
mod fixtures;
mod listing;
mod macros;
mod optimize;
mod output;
mod parser;
mod relax;
