//! Tests for `src/isa.rs`: the ids of `FlagEffect` and `Bound`, `Cycles` defaults, and an
//! `InstructionSet` used as a trait object.
#![allow(clippy::unwrap_used)]

use std::collections::HashSet;

use va_core::diag::DiagCode;
use va_core::isa::{
    Bound, ConditionInfo, Cycles, EaTiming, FlagEffect, FlagInfo, FormInfo, InstructionInfo,
    InstructionSet, LookupError, ModeCondition, OperandInfo, PerUnit, TimingInfo,
};

#[test]
fn flag_effect_ids() {
    let all = [
        (FlagEffect::Unaffected, "unaffected"),
        (FlagEffect::Result, "result"),
        (FlagEffect::Cleared, "cleared"),
        (FlagEffect::Set, "set"),
        (FlagEffect::Undefined, "undefined"),
        (FlagEffect::Special, "special"),
    ];
    for (effect, id) in all {
        assert_eq!(effect.id(), id);
        assert!(DiagCode(effect.id()).is_valid(), "{id} is not kebab-case");
    }
    let unique: HashSet<&str> = all.iter().map(|(effect, _)| effect.id()).collect();
    assert_eq!(unique.len(), all.len());
}

#[test]
fn bound_ids() {
    assert_eq!(Bound::Exact.id(), "exact");
    assert_eq!(Bound::Max.id(), "max");
}

#[test]
fn cycles_default_to_zero() {
    assert_eq!(
        Cycles::default(),
        Cycles {
            clocks: 0,
            reads: 0,
            writes: 0
        }
    );
}

/// A one-instruction set: `NOP`, unsized, 4(1/0).
struct TinyIsa;

/// The spellings of [`TinyIsa`].
static TINY_SPELLINGS: [&str; 1] = ["NOP"];

/// Cost of [`TinyIsa`]'s NOP.
const NOP_CYCLES: Cycles = Cycles {
    clocks: 4,
    reads: 1,
    writes: 0,
};

impl InstructionSet for TinyIsa {
    fn spellings(&self) -> &[&'static str] {
        &TINY_SPELLINGS
    }

    fn describe(&self, spelling: &str, size: Option<&str>) -> Result<InstructionInfo, LookupError> {
        if !spelling.eq_ignore_ascii_case("nop") {
            return Err(LookupError::Unknown);
        }
        if size.is_some() {
            return Err(LookupError::SizeNotAccepted);
        }
        Ok(InstructionInfo {
            spelling: "NOP",
            canonical: "NOP",
            family: None,
            condition: None,
            aliases: Vec::new(),
            also_accepts: Vec::new(),
            size: None,
            doc_id: "isa-tiny-nop",
            forms: vec![FormInfo {
                syntax: "NOP".to_owned(),
                sizes: Vec::new(),
                default_size: None,
                privileged: false,
                operands: Vec::new(),
                flags: ["X", "N", "Z", "V", "C"]
                    .into_iter()
                    .map(|flag| FlagInfo {
                        flag,
                        effect: FlagEffect::Unaffected,
                    })
                    .collect(),
                timing: vec![TimingInfo {
                    sizes: Vec::new(),
                    when: Vec::new(),
                    outcome: None,
                    base: NOP_CYCLES,
                    plus_ea: Vec::new(),
                    per_unit: None,
                    bound: Bound::Exact,
                }],
            }],
        })
    }

    fn ea_timing(&self) -> &[EaTiming] {
        &[]
    }
}

#[test]
fn an_instruction_set_works_as_a_trait_object() {
    let isa: &dyn InstructionSet = &TinyIsa;
    assert_eq!(isa.spellings(), ["NOP"]);
    let info = isa.describe("nOp", None).unwrap();
    assert_eq!(info.canonical, "NOP");
    assert_eq!(info.forms[0].timing[0].base, NOP_CYCLES);
    assert_eq!(info.forms[0].flags.len(), 5);
    assert_eq!(isa.describe("MOV", None), Err(LookupError::Unknown));
    assert_eq!(
        isa.describe("NOP", Some("w")),
        Err(LookupError::SizeNotAccepted)
    );
    assert!(isa.ea_timing().is_empty());
}

#[test]
fn view_types_hold_the_documented_data() {
    let condition = ConditionInfo {
        name: "EQ",
        test: "Z",
        doc_id: "isa-m68k-cond-eq",
    };
    let operand = OperandInfo {
        notation: "<ea>",
        modes: vec!["Dn", "(An)"],
    };
    let row = TimingInfo {
        sizes: vec!["B", "W"],
        when: vec![ModeCondition {
            operand: 0,
            modes: vec!["Dn"],
        }],
        outcome: Some("taken"),
        base: Cycles {
            clocks: 6,
            reads: 1,
            writes: 0,
        },
        plus_ea: vec![0],
        per_unit: Some(PerUnit {
            cycles: Cycles {
                clocks: 2,
                reads: 0,
                writes: 0,
            },
            unit: "shift-count",
        }),
        bound: Bound::Max,
    };
    let ea = EaTiming {
        mode: "(An)",
        sizes: vec!["B", "W"],
        cycles: Cycles {
            clocks: 4,
            reads: 1,
            writes: 0,
        },
    };
    assert_eq!(condition, condition.clone());
    assert_eq!(operand.clone(), operand);
    assert_eq!(row.clone(), row);
    assert_eq!(ea.clone(), ea);
    assert_eq!(row.per_unit.unwrap().unit, "shift-count");
}
