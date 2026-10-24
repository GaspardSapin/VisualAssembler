//! The strict-A68k 68000 assembler (m2-assembler.md §4): [`M68kAssembler`], reached through
//! `M68K.assembler()`, and [`assemble`], the same entry point as a function.
//!
//! Pipeline (§4.1): pass 1 expands the source in order (lexing with `syntax::lex_line`, parsing,
//! INCLUDE, macros, conditionals, END), sizes and places every statement; relaxation shortens
//! branches when optimizing; pass 2 evaluates, encodes and builds the image, the source map, the
//! symbols and the listing. Modules:
//!
//! - front end (WP-B): [`parser`] (fields of a line, [`parser::Problem`]), [`operand`] (operand
//!   syntax), [`expr`] (expressions and their evaluation);
//! - encoder (WP-C): [`encode`] (form selection, length, encoding), [`optimize`] (the §4.10
//!   rewrites);
//! - driver (WP-D): [`codes`] (diagnostic codes), [`symbols`], [`directives`], [`macros`],
//!   [`layout`] (addresses and branch relaxation), [`listing`], [`srec`] (S-record and binary
//!   output).
//!
//! The interfaces between the three groups are frozen by §4.2. WP0 scaffold: the entry points
//! with a `todo!()` body; WP-D implements them.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-D implements m2 §4.1"
)]

pub mod codes;
pub mod directives;
pub mod encode;
pub mod expr;
pub mod layout;
pub mod listing;
pub mod macros;
pub mod operand;
pub mod optimize;
pub mod parser;
pub mod srec;
pub mod symbols;

use va_core::asm::{AssembleInput, AssembleOutput, Assembler, CancelToken};

/// The 68000 assembler of the strict A68k dialect (§4), as `M68K.assembler()` returns it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct M68kAssembler;

impl Assembler for M68kAssembler {
    /// Assembles `input` with [`assemble`].
    fn assemble(&mut self, input: &AssembleInput<'_>, cancel: &CancelToken) -> AssembleOutput {
        assemble(input, cancel)
    }
}

/// Assembles `input`, polling `cancel` every 256 statements in every pass (§4.1).
///
/// The output holds an image iff no diagnostic is blocking and the assembly was not cancelled.
pub fn assemble(input: &AssembleInput<'_>, cancel: &CancelToken) -> AssembleOutput {
    todo!()
}
