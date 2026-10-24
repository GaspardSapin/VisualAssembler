//! The 68000 disassembler (m2-assembler.md §5): [`M68kDisassembler`], reached through
//! `M68K.disassembler()`, built on the same tables as the assembler.
//!
//! Modules: [`decode`] (the opword table over `Form::matches`, extension words through
//! `isa::fields::places`, flow and target) and [`format`] (the canonical text, which reassembles
//! to the same bytes with `optimize = false`, and its tokens).
//!
//! WP0 scaffold: [`Disassembler::decode`] has a `todo!()` body; WP-E implements it.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-E implements m2 §5"
)]

pub mod decode;
pub mod format;

use va_core::disasm::{DecodeError, DecodedInsn, Disassembler, FlowKind};

/// The 68000 disassembler (§5), as `M68K.disassembler()` returns it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct M68kDisassembler;

impl Disassembler for M68kDisassembler {
    /// Decodes the instruction at the start of `bytes` (§5.1): `Truncated` when the bytes end
    /// inside it, `Illegal` when no form matches its opword.
    fn decode(&self, bytes: &[u8], address: u64) -> Result<DecodedInsn, DecodeError> {
        todo!()
    }

    /// Returns `insn.flow`, which [`decode`](Disassembler::decode) sets (§3.3).
    fn flow_kind(&self, insn: &DecodedInsn) -> FlowKind {
        insn.flow
    }
}
