//! Tests for `src/disasm.rs`: `FlowKind` ids, `DecodeError` values, a `Disassembler` used as a
//! trait object (decoded text covered by its tokens, targets, flow, truncated and illegal
//! input), reached through a fake `Architecture`, and the default `Architecture::disassembler`.
#![allow(clippy::unwrap_used)]

use std::collections::HashSet;

use va_core::arch::Architecture;
use va_core::diag::DiagCode;
use va_core::disasm::{DecodeError, DecodedInsn, Disassembler, FlowKind};
use va_core::isa::{EaTiming, InstructionInfo, InstructionSet, LookupError};
use va_core::regs::RegisterSpec;
use va_core::source::TextRange;
use va_core::syntax::{
    KeywordInfo, LineState, SyntaxProvider, Token, TokenKind, check_line_tokens,
};

/// Tells whether `id` is a kebab-case identifier (`[a-z]+(-[a-z]+)*`).
fn is_kebab(id: &str) -> bool {
    !id.is_empty()
        && id
            .split('-')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_lowercase()))
}

// ---------------------------------------------------------------------------------------------
// Ids and values
// ---------------------------------------------------------------------------------------------

#[test]
fn flow_kind_ids_are_kebab_case_and_unique() {
    let expected = [
        (FlowKind::Sequential, "sequential"),
        (FlowKind::Jump, "jump"),
        (FlowKind::Branch, "branch"),
        (FlowKind::Call { return_address: 0 }, "call"),
        (FlowKind::Return, "return"),
        (FlowKind::Trap, "trap"),
    ];
    let mut seen = HashSet::new();
    for (kind, id) in expected {
        assert_eq!(kind.id(), id, "{kind:?}");
        assert!(is_kebab(id), "{id:?}");
        assert!(seen.insert(id), "duplicate id {id:?}");
    }
}

#[test]
fn a_call_id_does_not_depend_on_its_return_address() {
    let near = FlowKind::Call {
        return_address: 0x1004,
    };
    let far = FlowKind::Call {
        return_address: u64::MAX,
    };
    assert_eq!(near.id(), far.id());
    assert_ne!(near, far);
}

#[test]
fn decode_errors_carry_their_details() {
    let errors = [
        DecodeError::Truncated { needed: 2 },
        DecodeError::Truncated { needed: 4 },
        DecodeError::Illegal { word: 0x4AFC },
        DecodeError::Illegal { word: 0xFFFF },
    ];
    let distinct: HashSet<DecodeError> = errors.iter().copied().collect();
    assert_eq!(distinct.len(), errors.len());
}

// ---------------------------------------------------------------------------------------------
// A tiny disassembler
// ---------------------------------------------------------------------------------------------

/// Builds tokens of consecutive `(kind, byte length)` parts, from offset 0.
fn tokens(parts: &[(TokenKind, u32)]) -> Vec<Token> {
    let mut start = 0;
    parts
        .iter()
        .map(|&(kind, len)| {
            let token = Token {
                kind,
                range: TextRange::new(start, start + len),
            };
            start += len;
            token
        })
        .collect()
}

/// A decoder for five 68000-like words: NOP `4E71`, RTS `4E75`, TRAP #15 `4E4F`, and BRA.W
/// `6000`/BSR.W `6100` with a 16-bit displacement word; anything else is illegal.
struct TinyDisassembler;

impl TinyDisassembler {
    /// A word branch or call at `address` with displacement `disp`.
    fn branch(call: bool, address: u64, disp: u16) -> DecodedInsn {
        let target = address
            .wrapping_add(2)
            .wrapping_add_signed(i64::from(disp.cast_signed()));
        let operation = if call { "bsr" } else { "bra" };
        let flow = if call {
            FlowKind::Call {
                return_address: address + 4,
            }
        } else {
            FlowKind::Jump
        };
        DecodedInsn {
            address,
            len: 4,
            text: format!("{operation}.w ${target:08X}"),
            tokens: tokens(&[
                (TokenKind::Mnemonic, 3),
                (TokenKind::SizeSuffix, 2),
                (TokenKind::Whitespace, 1),
                (TokenKind::Number, 9),
            ]),
            target: Some(target),
            flow,
        }
    }

    /// A one-word instruction at `address`.
    fn simple(address: u64, text: &str, parts: &[(TokenKind, u32)], flow: FlowKind) -> DecodedInsn {
        DecodedInsn {
            address,
            len: 2,
            text: text.to_owned(),
            tokens: tokens(parts),
            target: None,
            flow,
        }
    }
}

impl Disassembler for TinyDisassembler {
    fn decode(&self, bytes: &[u8], address: u64) -> Result<DecodedInsn, DecodeError> {
        let [high, low, rest @ ..] = bytes else {
            return Err(DecodeError::Truncated { needed: 2 });
        };
        let word = u16::from_be_bytes([*high, *low]);
        match word {
            0x4E71 => Ok(Self::simple(
                address,
                "nop",
                &[(TokenKind::Mnemonic, 3)],
                FlowKind::Sequential,
            )),
            0x4E75 => Ok(Self::simple(
                address,
                "rts",
                &[(TokenKind::Mnemonic, 3)],
                FlowKind::Return,
            )),
            0x4E4F => Ok(Self::simple(
                address,
                "trap #15",
                &[
                    (TokenKind::Mnemonic, 4),
                    (TokenKind::Whitespace, 1),
                    (TokenKind::Punctuation, 1),
                    (TokenKind::Number, 2),
                ],
                FlowKind::Trap,
            )),
            0x6000 | 0x6100 => match rest {
                [disp_high, disp_low, ..] => Ok(Self::branch(
                    word == 0x6100,
                    address,
                    u16::from_be_bytes([*disp_high, *disp_low]),
                )),
                _ => Err(DecodeError::Truncated { needed: 4 }),
            },
            _ => Err(DecodeError::Illegal { word }),
        }
    }

    fn flow_kind(&self, insn: &DecodedInsn) -> FlowKind {
        insn.flow
    }
}

/// The tiny disassembler as callers see it.
static TINY: &(dyn Disassembler + Sync) = &TinyDisassembler;

#[test]
fn decoded_text_is_covered_by_its_tokens() {
    let program: [&[u8]; 5] = [
        &[0x4E, 0x71],
        &[0x4E, 0x75],
        &[0x4E, 0x4F],
        &[0x60, 0x00, 0x00, 0x10],
        &[0x61, 0x00, 0xFF, 0xFE],
    ];
    for bytes in program {
        let insn = TINY.decode(bytes, 0x1000).unwrap();
        assert_eq!(
            check_line_tokens(&insn.text, &insn.tokens),
            Ok(()),
            "{insn:?}"
        );
        assert_eq!(insn.len as usize, bytes.len());
        assert_eq!(insn.address, 0x1000);
        assert_eq!(TINY.flow_kind(&insn), insn.flow);
    }
}

#[test]
fn branches_and_calls_have_targets() {
    let jump = TINY
        .decode(&[0x60, 0x00, 0x00, 0x10, 0x4E, 0x71], 0x1000)
        .unwrap();
    assert_eq!(jump.text, "bra.w $00001012");
    assert_eq!(jump.target, Some(0x1012));
    assert_eq!(jump.flow, FlowKind::Jump);
    assert_eq!(jump.len, 4, "the following bytes are not part of it");

    let call = TINY.decode(&[0x61, 0x00, 0xFF, 0xFE], 0x2000).unwrap();
    assert_eq!(call.text, "bsr.w $00002000");
    assert_eq!(call.target, Some(0x2000));
    assert_eq!(
        TINY.flow_kind(&call),
        FlowKind::Call {
            return_address: 0x2004
        }
    );
    assert_eq!(TINY.flow_kind(&call).id(), "call");
}

#[test]
fn other_instructions_have_no_target() {
    let nop = TINY.decode(&[0x4E, 0x71], 0).unwrap();
    assert_eq!(nop.text, "nop");
    assert_eq!(nop.target, None);
    assert_eq!(nop.flow.id(), "sequential");
    assert_eq!(
        TINY.decode(&[0x4E, 0x75], 0).unwrap().flow,
        FlowKind::Return
    );
    assert_eq!(TINY.decode(&[0x4E, 0x4F], 0).unwrap().flow, FlowKind::Trap);
}

#[test]
fn short_input_is_truncated_with_the_length_needed() {
    assert_eq!(
        TINY.decode(&[], 0),
        Err(DecodeError::Truncated { needed: 2 })
    );
    assert_eq!(
        TINY.decode(&[0x4E], 0),
        Err(DecodeError::Truncated { needed: 2 })
    );
    assert_eq!(
        TINY.decode(&[0x60, 0x00, 0x00], 0),
        Err(DecodeError::Truncated { needed: 4 })
    );
}

#[test]
fn an_unknown_word_is_illegal() {
    assert_eq!(
        TINY.decode(&[0x4A, 0xFC, 0x4E, 0x71], 0),
        Err(DecodeError::Illegal { word: 0x4AFC })
    );
}

// ---------------------------------------------------------------------------------------------
// Disassembler through an Architecture
// ---------------------------------------------------------------------------------------------

/// A lexer that appends nothing (only valid for empty lines; enough for these tests).
struct NoSyntax;

impl SyntaxProvider for NoSyntax {
    fn lex_line(&self, _line: &str, state: LineState, _out: &mut Vec<Token>) -> LineState {
        state
    }

    fn keywords(&self) -> &[KeywordInfo] {
        &[]
    }
}

/// An instruction set without instructions.
struct NoIsa;

impl InstructionSet for NoIsa {
    fn spellings(&self) -> &[&'static str] {
        &[]
    }

    fn describe(
        &self,
        _spelling: &str,
        _size: Option<&str>,
    ) -> Result<InstructionInfo, LookupError> {
        Err(LookupError::Unknown)
    }

    fn ea_timing(&self) -> &[EaTiming] {
        &[]
    }
}

/// A CPU that implements only the required (M1) methods.
struct PlainCpu;

impl Architecture for PlainCpu {
    fn id(&self) -> &'static str {
        "plain"
    }

    fn syntax(&self) -> &dyn SyntaxProvider {
        &NoSyntax
    }

    fn instruction_set(&self) -> &dyn InstructionSet {
        &NoIsa
    }

    fn registers(&self) -> &'static [RegisterSpec] {
        &[]
    }

    fn diagnostic_codes(&self) -> &'static [DiagCode] {
        &[]
    }

    fn locale_resources(&self, _lang: &str) -> &'static [&'static str] {
        &[]
    }
}

/// A CPU that provides the tiny disassembler and no assembler.
struct DisasmCpu;

impl Architecture for DisasmCpu {
    fn id(&self) -> &'static str {
        "disasm"
    }

    fn syntax(&self) -> &dyn SyntaxProvider {
        &NoSyntax
    }

    fn instruction_set(&self) -> &dyn InstructionSet {
        &NoIsa
    }

    fn registers(&self) -> &'static [RegisterSpec] {
        &[]
    }

    fn diagnostic_codes(&self) -> &'static [DiagCode] {
        &[]
    }

    fn locale_resources(&self, _lang: &str) -> &'static [&'static str] {
        &[]
    }

    fn disassembler(&self) -> Option<Box<dyn Disassembler>> {
        Some(Box::new(TinyDisassembler))
    }
}

#[test]
fn a_cpu_without_a_disassembler_returns_none() {
    let cpu: &dyn Architecture = &PlainCpu;
    assert!(cpu.disassembler().is_none());
    assert!(cpu.assembler().is_none());
}

#[test]
fn a_disassembler_is_reached_through_the_architecture() {
    let cpu: &dyn Architecture = &DisasmCpu;
    assert!(cpu.assembler().is_none(), "the other default stays");
    let disassembler = cpu.disassembler().unwrap();
    let insn = disassembler.decode(&[0x4E, 0x75], 0x400).unwrap();
    assert_eq!(insn.text, "rts");
    assert_eq!(disassembler.flow_kind(&insn), FlowKind::Return);
}

#[test]
fn a_disassembler_can_move_to_a_worker_thread() {
    let disassembler = DisasmCpu.disassembler().unwrap();
    let insn = std::thread::spawn(move || disassembler.decode(&[0x4E, 0x71], 0x10))
        .join()
        .unwrap()
        .unwrap();
    assert_eq!(insn.address, 0x10);
    assert_eq!(insn.flow, FlowKind::Sequential);
}
