//! The 68000 disassembler (m2-assembler.md §5, completed by §16.2): [`M68kDisassembler`],
//! reached through `M68K.disassembler()`, built on the same tables as the assembler.
//!
//! Modules: [`decode`] (the opword table over `Form::matches`, [`decode::form_of`], the
//! extension words read through `isa::fields::places`, flow and target) and [`format`] (the
//! canonical text, which reassembles to the same bytes with `optimize = false`, and its
//! tokens).

pub mod decode;
pub mod format;

use va_core::disasm::{DecodeError, DecodedInsn, Disassembler, FlowKind};

/// The 68000 disassembler (§5), as `M68K.disassembler()` returns it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct M68kDisassembler;

impl Disassembler for M68kDisassembler {
    /// Decodes the instruction at the start of `bytes` (§5.1): `Truncated` when the bytes end
    /// inside it (or hold less than an opword), `Illegal` when no form matches its opword (even
    /// when bytes are missing after it, §15.1 A3).
    fn decode(&self, bytes: &[u8], address: u64) -> Result<DecodedInsn, DecodeError> {
        decode::decode(bytes, address)
    }

    /// Returns `insn.flow`, which [`decode`](Disassembler::decode) sets (§3.3).
    fn flow_kind(&self, insn: &DecodedInsn) -> FlowKind {
        insn.flow
    }
}
