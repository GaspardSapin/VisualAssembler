//! Disassembler contracts (m2-assembler.md §3.3, architecture §3): the [`Disassembler`] trait,
//! the [`DecodedInsn`] it produces, why decoding fails ([`DecodeError`]) and how an instruction
//! changes the flow of control ([`FlowKind`], which drives step over/out in the debugger).

use crate::syntax::Token;

/// How an instruction changes the flow of control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FlowKind {
    /// Execution continues with the next instruction.
    Sequential,
    /// An unconditional jump (on the 68000: BRA, JMP).
    Jump,
    /// A conditional branch, taken or not (on the 68000: Bcc, DBcc).
    Branch,
    /// A subroutine call (on the 68000: BSR, JSR).
    Call {
        /// Address of the instruction after the call, where the matching return resumes.
        return_address: u64,
    },
    /// A return from a subroutine or an exception (on the 68000: RTS, RTE, RTR).
    Return,
    /// An exception raised on purpose (on the 68000: TRAP, TRAPV, ILLEGAL).
    Trap,
}

impl FlowKind {
    /// Returns the identifier of the kind: `"sequential"`, `"jump"`, `"branch"`, `"call"`,
    /// `"return"` or `"trap"` (the return address is not part of it).
    pub const fn id(self) -> &'static str {
        match self {
            Self::Sequential => "sequential",
            Self::Jump => "jump",
            Self::Branch => "branch",
            Self::Call { .. } => "call",
            Self::Return => "return",
            Self::Trap => "trap",
        }
    }
}

/// One decoded instruction.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DecodedInsn {
    /// Address of its first byte.
    pub address: u64,
    /// Its length in bytes, extension words included.
    pub len: u32,
    /// The instruction in the CPU's assembler syntax: the operation, one space, the operands
    /// (`"move.l d0,d1"`). Assembling it gives the same bytes (m2 §5.2).
    pub text: String,
    /// The lexer's tokens of `text`, ranges relative to `text`: they cover it exactly
    /// (`check_line_tokens(&text, &tokens)` holds).
    pub tokens: Vec<Token>,
    /// The address a branch, jump or call goes to, or that a PC-relative operand designates.
    pub target: Option<u64>,
    /// How it changes the flow of control.
    pub flow: FlowKind,
}

/// Why [`Disassembler::decode`] gives no instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DecodeError {
    /// The bytes end inside the instruction.
    Truncated {
        /// The length in bytes the instruction needs, counted from the start of the given
        /// bytes; it is larger than their count.
        needed: u32,
    },
    /// The first word is no instruction of the CPU.
    Illegal {
        /// That word, as the CPU reads it from the bytes.
        word: u16,
    },
}

/// The disassembler of one CPU.
pub trait Disassembler: Send {
    /// Decodes the instruction at the start of `bytes`, whose first byte is at `address`.
    fn decode(&self, bytes: &[u8], address: u64) -> Result<DecodedInsn, DecodeError>;

    /// Returns how `insn` changes the flow of control (the 68000 returns `insn.flow`).
    fn flow_kind(&self, insn: &DecodedInsn) -> FlowKind;
}
