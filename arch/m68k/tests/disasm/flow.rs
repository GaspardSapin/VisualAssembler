//! Flow kinds and targets of decoded instructions (m2 §5.1, §10.1): branches, jumps, calls with
//! their return address, returns and traps, and the branch, jump, call and PC-relative targets
//! (WP-E). Also the golden file `fixtures/disasm/flow.{s68,dis}`.
#![allow(clippy::unwrap_used)]

use va_arch_m68k::M68K;
use va_arch_m68k::isa::{EaField, Family, Mode, Operand as Slot};
use va_core::arch::Architecture as _;
use va_core::disasm::{DecodedInsn, FlowKind};

use crate::decode::{ORIGIN, candidates, check_golden, decode_words, insn, matching};

/// The flow kind §5.1 gives an entry (a call returns to `return_address`).
fn expected_flow(name: &str, family: Option<Family>, return_address: u64) -> FlowKind {
    match (family, name) {
        (Some(Family::Bcc | Family::DBcc), _) => FlowKind::Branch,
        (_, "BRA" | "JMP") => FlowKind::Jump,
        (_, "BSR" | "JSR") => FlowKind::Call { return_address },
        (_, "RTS" | "RTE" | "RTR") => FlowKind::Return,
        (_, "TRAP" | "TRAPV" | "ILLEGAL") => FlowKind::Trap,
        _ => FlowKind::Sequential,
    }
}

#[test]
fn every_opword_has_the_flow_of_its_entry() {
    let candidates = candidates();
    let disassembler = M68K.disassembler().unwrap();
    for opword in 0..=u16::MAX {
        let Some(&(entry, _, _, _)) = matching(&candidates, opword).first() else {
            continue;
        };
        // A non-zero MOVEM mask: an empty one is written as words, `Sequential` too.
        let insn = decode_words(&[opword, 0x0102, 0x0102, 0x0102, 0x0102], 0x2000).unwrap();
        let return_address = 0x2000 + u64::from(insn.len);
        assert_eq!(
            insn.flow,
            expected_flow(entry.name, entry.family, return_address),
            "{opword:04X} {}",
            insn.text
        );
        assert_eq!(disassembler.flow_kind(&insn), insn.flow);
    }
}

#[test]
fn calls_return_after_the_whole_instruction() {
    for (words, address, return_address) in [
        (&[0x6110][..], 0x1000, 0x1002),
        (&[0x6100, 0x0100][..], 0x1000, 0x1004),
        (&[0x4E90][..], 0x2000, 0x2002),
        (&[0x4EA8, 0x0010][..], 0x2000, 0x2004),
        (&[0x4EB0, 0x1004][..], 0x2000, 0x2004),
        (&[0x4EB8, 0x0400][..], 0x2000, 0x2004),
        (&[0x4EB9, 0x0001, 0x0000][..], 0x2000, 0x2006),
        (&[0x4EBA, 0x0100][..], 0x2000, 0x2004),
        (&[0x4EBB, 0x2000][..], 0x2000, 0x2004),
    ] {
        let insn = decode_words(words, address).unwrap();
        assert_eq!(
            insn.flow,
            FlowKind::Call { return_address },
            "{}",
            insn.text
        );
    }
}

/// Decodes `words` at [`ORIGIN`] and returns its flow kind and target.
fn flow_and_target(words: &[u16]) -> (FlowKind, Option<u64>) {
    let DecodedInsn { flow, target, .. } = insn(words);
    (flow, target)
}

#[test]
fn branches_target_the_opword_address_plus_two_plus_the_displacement() {
    let branch = FlowKind::Branch;
    // Bcc .S forward and backward, .W forward and backward.
    assert_eq!(flow_and_target(&[0x6702]), (branch, Some(0x1004)));
    assert_eq!(flow_and_target(&[0x6680]), (branch, Some(0x0F82)));
    assert_eq!(flow_and_target(&[0x6C00, 0x7FFE]), (branch, Some(0x9000)));
    assert_eq!(
        flow_and_target(&[0x6D00, 0x8000]),
        (branch, Some(0xFFFF_9002))
    );
    assert_eq!(
        flow_and_target(&[0x6000, 0x0000]),
        (FlowKind::Jump, Some(0x1002))
    );
    // DBcc: a loop branch.
    assert_eq!(flow_and_target(&[0x51C9, 0xFFFE]), (branch, Some(0x1000)));
    assert_eq!(flow_and_target(&[0x50CA, 0x0010]), (branch, Some(0x1012)));
    // BRA and BSR.
    assert_eq!(flow_and_target(&[0x60FE]), (FlowKind::Jump, Some(0x1000)));
    let (flow, target) = flow_and_target(&[0x61FE]);
    assert_eq!(target, Some(0x1000));
    assert_eq!(
        flow,
        FlowKind::Call {
            return_address: 0x1002
        }
    );
    // An odd short displacement is decoded as it is (the target is odd).
    assert_eq!(flow_and_target(&[0x6001]), (FlowKind::Jump, Some(0x1003)));
}

#[test]
fn jmp_jsr_lea_and_pea_target_their_absolute_or_pc_relative_address() {
    let jump = FlowKind::Jump;
    assert_eq!(flow_and_target(&[0x4ED0]), (jump, None));
    assert_eq!(flow_and_target(&[0x4EE8, 0x0010]), (jump, None));
    assert_eq!(flow_and_target(&[0x4EF0, 0x1004]), (jump, None));
    assert_eq!(flow_and_target(&[0x4EF8, 0x0400]), (jump, Some(0x0400)));
    assert_eq!(
        flow_and_target(&[0x4EF8, 0x8000]),
        (jump, Some(0xFFFF_8000))
    );
    assert_eq!(
        flow_and_target(&[0x4EF9, 0x0001, 0x2345]),
        (jump, Some(0x12345))
    );
    assert_eq!(flow_and_target(&[0x4EFA, 0x0100]), (jump, Some(0x1102)));
    assert_eq!(flow_and_target(&[0x4EFB, 0x00FE]), (jump, Some(0x1000)));
    let call = FlowKind::Call {
        return_address: 0x1004,
    };
    assert_eq!(flow_and_target(&[0x4EB8, 0x0400]), (call, Some(0x0400)));
    let sequential = FlowKind::Sequential;
    assert_eq!(
        flow_and_target(&[0x41F8, 0x0400]),
        (sequential, Some(0x0400))
    );
    assert_eq!(
        flow_and_target(&[0x43F9, 0x00FF, 0x0000]),
        (sequential, Some(0xFF_0000))
    );
    assert_eq!(
        flow_and_target(&[0x45FA, 0xFFFE]),
        (sequential, Some(0x1000))
    );
    assert_eq!(
        flow_and_target(&[0x4878, 0x1234]),
        (sequential, Some(0x1234))
    );
    assert_eq!(
        flow_and_target(&[0x487A, 0x0010]),
        (sequential, Some(0x1012))
    );
    assert_eq!(flow_and_target(&[0x41D0]), (sequential, None));
    assert_eq!(flow_and_target(&[0x4850]), (sequential, None));
    // Other instructions have no target for an absolute operand.
    assert_eq!(
        flow_and_target(&[0x3039, 0x0001, 0x2345]),
        (sequential, None)
    );
    assert_eq!(flow_and_target(&[0x4278, 0x0400]), (sequential, None));
}

/// The addressing mode of the EA field at bits 5–0 of `opword`.
fn low_mode(opword: u16) -> Option<Mode> {
    Mode::from_fields(((opword >> 3) & 7) as u8, (opword & 7) as u8)
}

#[test]
fn every_pc_relative_operand_has_its_target() {
    let candidates = candidates();
    let mut seen = 0;
    for opword in 0..=u16::MAX {
        let Some(&(entry, _, form, _)) = matching(&candidates, opword).first() else {
            continue;
        };
        let Some(mode @ (Mode::PcDisp | Mode::PcIndex)) = low_mode(opword) else {
            continue;
        };
        if !matches!(form.encoding.ea, EaField::Low | EaField::Both) {
            continue;
        }
        // The PC-relative word follows the extension words written before it: the MOVEM mask
        // or a bit number, which `0x0010` stands for here.
        let before = usize::from(
            form.operands.contains(&Slot::RegList)
                || matches!(form.operands.first(), Some(Slot::Imm)),
        );
        let mut words = vec![opword];
        words.extend(std::iter::repeat_n(0x0010, before));
        let extension: u16 = if mode == Mode::PcDisp { 0xFFF0 } else { 0x00F0 };
        words.push(extension);
        words.extend([0, 0, 0]);
        let insn = decode_words(&words, ORIGIN).unwrap();
        let word_address = ORIGIN + 2 + 2 * before as u64;
        assert_eq!(
            insn.target,
            Some(word_address - 16),
            "{opword:04X} {} {}",
            entry.name,
            insn.text
        );
        assert!(
            insn.text
                .contains(&format!("${:08X}(pc", word_address - 16))
        );
        seen += 1;
    }
    assert!(seen > 500, "{seen} PC-relative opwords");
}

#[test]
fn returns_and_traps() {
    assert_eq!(flow_and_target(&[0x4E75]), (FlowKind::Return, None));
    assert_eq!(flow_and_target(&[0x4E73]), (FlowKind::Return, None));
    assert_eq!(flow_and_target(&[0x4E77]), (FlowKind::Return, None));
    assert_eq!(flow_and_target(&[0x4E40]), (FlowKind::Trap, None));
    assert_eq!(flow_and_target(&[0x4E4F]), (FlowKind::Trap, None));
    assert_eq!(flow_and_target(&[0x4E76]), (FlowKind::Trap, None));
    assert_eq!(flow_and_target(&[0x4AFC]), (FlowKind::Trap, None));
    // STOP, RESET, CHK and NOP are sequential.
    assert_eq!(
        flow_and_target(&[0x4E72, 0x2700]),
        (FlowKind::Sequential, None)
    );
    assert_eq!(flow_and_target(&[0x4E70]), (FlowKind::Sequential, None));
    assert_eq!(flow_and_target(&[0x4180]), (FlowKind::Sequential, None));
    assert_eq!(flow_and_target(&[0x4E71]), (FlowKind::Sequential, None));
}

#[test]
fn the_flow_golden_file_matches() {
    check_golden("flow");
}
