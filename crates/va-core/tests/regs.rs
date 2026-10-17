//! Tests for `src/regs.rs`: static register descriptions and `RegId` order, read through an
//! `Architecture` trait object (`src/arch.rs` has no logic of its own).
#![allow(clippy::unwrap_used)]

use va_core::arch::Architecture;
use va_core::diag::DiagCode;
use va_core::isa::{EaTiming, InstructionInfo, InstructionSet, LookupError};
use va_core::regs::{FlagField, RegId, RegisterGroup, RegisterSpec};
use va_core::syntax::{KeywordInfo, LineState, SyntaxProvider, Token};

/// Fields of the test status register.
static FLAGS: [FlagField; 2] = [
    FlagField {
        name: "Z",
        bit: 2,
        width: 1,
    },
    FlagField {
        name: "I",
        bit: 8,
        width: 3,
    },
];

/// Registers of the test CPU.
static REGISTERS: [RegisterSpec; 3] = [
    RegisterSpec {
        id: RegId(0),
        name: "D0",
        aliases: &[],
        bits: 32,
        group: RegisterGroup::General,
        fields: &[],
    },
    RegisterSpec {
        id: RegId(15),
        name: "A7",
        aliases: &["SP"],
        bits: 32,
        group: RegisterGroup::Address,
        fields: &[],
    },
    RegisterSpec {
        id: RegId(19),
        name: "SR",
        aliases: &[],
        bits: 16,
        group: RegisterGroup::Status,
        fields: &FLAGS,
    },
];

/// Diagnostic codes of the test CPU.
static CODES: [DiagCode; 1] = [DiagCode("lex-bad-number")];

/// English resources of the test CPU.
static EN: [&str; 1] = ["isa-test-nop = Does nothing.\n"];

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

/// The test CPU.
struct TestCpu;

impl Architecture for TestCpu {
    fn id(&self) -> &'static str {
        "test"
    }

    fn syntax(&self) -> &dyn SyntaxProvider {
        &NoSyntax
    }

    fn instruction_set(&self) -> &dyn InstructionSet {
        &NoIsa
    }

    fn registers(&self) -> &'static [RegisterSpec] {
        &REGISTERS
    }

    fn diagnostic_codes(&self) -> &'static [DiagCode] {
        &CODES
    }

    fn locale_resources(&self, lang: &str) -> &'static [&'static str] {
        if lang == "en" { &EN } else { &[] }
    }
}

/// The test CPU as the apps see it.
static CPU: &dyn Architecture = &TestCpu;

#[test]
fn registers_are_read_through_the_architecture() {
    let registers = CPU.registers();
    assert_eq!(registers.len(), 3);
    assert_eq!(registers[1].aliases, ["SP"]);
    assert_eq!(registers[2].group, RegisterGroup::Status);
    assert_eq!(registers[2].fields[1], FLAGS[1]);
    assert_eq!(registers[2].bits, 16);
}

#[test]
fn register_ids_are_ordered() {
    let mut ids: Vec<RegId> = CPU.registers().iter().rev().map(|spec| spec.id).collect();
    ids.sort();
    assert_eq!(ids, vec![RegId(0), RegId(15), RegId(19)]);
}

#[test]
fn flag_fields_fit_their_register() {
    for spec in CPU.registers() {
        for field in spec.fields {
            assert!(field.width >= 1);
            assert!(field.bit + field.width <= spec.bits, "{}", field.name);
        }
    }
}

#[test]
fn every_architecture_method_is_reachable_through_dyn() {
    assert_eq!(CPU.id(), "test");
    assert!(CPU.syntax().keywords().is_empty());
    let mut tokens = Vec::new();
    assert_eq!(
        CPU.syntax().lex_line("", LineState::INITIAL, &mut tokens),
        LineState::INITIAL
    );
    assert!(tokens.is_empty());
    assert_eq!(
        CPU.instruction_set().describe("NOP", None),
        Err(LookupError::Unknown)
    );
    assert_eq!(CPU.diagnostic_codes(), [DiagCode("lex-bad-number")]);
    assert_eq!(CPU.locale_resources("en").len(), 1);
    assert!(CPU.locale_resources("fr").is_empty());
}

#[test]
fn architectures_can_be_shared_across_threads() {
    let id = std::thread::scope(|scope| scope.spawn(|| CPU.id()).join().unwrap());
    assert_eq!(id, "test");
}
