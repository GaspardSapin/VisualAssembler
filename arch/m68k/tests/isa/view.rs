//! Tests for `src/isa/view.rs`: `M68kIsa` as a `va_core::isa::InstructionSet` (m1 §3.5, §4.7),
//! including the agreement of the rendered timing rows with `cycles()`.
#![allow(clippy::unwrap_used)]

use va_arch_m68k::M68K;
use va_arch_m68k::isa::{M68kIsa, Mode, Outcome, SPELLINGS, Size, cycles, instruction_of};
use va_core::arch::Architecture;
use va_core::diag::DiagCode;
use va_core::isa::{Bound, Cycles, FormInfo, InstructionSet, LookupError, TimingInfo};

/// The frozen mode notation of m1 §4.3.
const NOTATION: [&str; 12] = [
    "Dn",
    "An",
    "(An)",
    "(An)+",
    "-(An)",
    "d16(An)",
    "d8(An,Xn)",
    "xxx.W",
    "xxx.L",
    "d16(PC)",
    "d8(PC,Xn)",
    "#imm",
];

const OUTCOMES: [Outcome; 9] = [
    Outcome::Always,
    Outcome::Taken,
    Outcome::NotTaken,
    Outcome::True,
    Outcome::False,
    Outcome::NotExpired,
    Outcome::Expired,
    Outcome::Trap,
    Outcome::NoTrap,
];

#[test]
fn spellings_are_the_table_spellings() {
    assert_eq!(M68kIsa.spellings(), &SPELLINGS[..]);
    assert_eq!(M68K.instruction_set().spellings(), &SPELLINGS[..]);
}

#[test]
fn describe_is_case_insensitive() {
    let upper = M68kIsa.describe("BEQ", Some("S")).unwrap();
    for (spelling, size) in [("beq", Some("s")), ("bEq", Some("S")), ("Beq", Some("s"))] {
        assert_eq!(M68kIsa.describe(spelling, size).unwrap(), upper);
    }
    assert_eq!(upper.spelling, "BEQ");
    assert_eq!(upper.canonical, "BEQ");
    assert_eq!(upper.family, Some("Bcc"));
    let condition = upper.condition.unwrap();
    assert_eq!((condition.name, condition.test), ("EQ", "Z"));
    assert_eq!(condition.doc_id, "isa-m68k-cond-eq");
    assert_eq!(upper.doc_id, "isa-m68k-bcc");
    assert!(upper.aliases.is_empty() && upper.also_accepts.is_empty());
}

#[test]
fn branch_suffixes_follow_a68k() {
    let short = M68kIsa.describe("BRA", Some("b")).unwrap();
    assert_eq!(short.size, Some("S"));
    assert!(short.forms.iter().all(|form| form.sizes == ["S"]));
    let word = M68kIsa.describe("BRA", Some("l")).unwrap();
    assert_eq!(word.size, Some("W"));
    assert!(word.forms.iter().all(|form| form.sizes == ["W"]));
    let all = M68kIsa.describe("BRA", None).unwrap();
    assert_eq!(all.size, None);
    assert_eq!(all.forms.len(), 2);
    assert_eq!(all.forms[0].syntax, "BRA.S <label>");
    assert_eq!(all.forms[1].syntax, "BRA.W <label>");
}

#[test]
fn lookup_errors() {
    assert_eq!(
        M68kIsa.describe("MOVEQ", Some("b")),
        Err(LookupError::SizeNotAccepted)
    );
    assert_eq!(M68kIsa.describe("MOV", None), Err(LookupError::Unknown));
    assert_eq!(
        M68kIsa.describe("MOV", Some("w")),
        Err(LookupError::Unknown)
    );
    assert_eq!(
        M68kIsa.describe("NOP", Some("w")),
        Err(LookupError::SizeNotAccepted)
    );
    assert_eq!(
        M68kIsa.describe("ADD", Some("x")),
        Err(LookupError::SizeNotAccepted)
    );
    assert_eq!(
        M68kIsa.describe("ADD", Some("WL")),
        Err(LookupError::SizeNotAccepted)
    );
    assert_eq!(
        M68kIsa.describe("BTST", Some("w")),
        Err(LookupError::SizeNotAccepted)
    );
}

#[test]
fn aliases_canonical_and_borrowed_entries() {
    let dbra = M68kIsa.describe("dbra", None).unwrap();
    assert_eq!((dbra.spelling, dbra.canonical), ("DBRA", "DBF"));
    assert_eq!(dbra.aliases, ["DBF"]);
    assert_eq!(dbra.doc_id, "isa-m68k-dbcc");
    let dbf = M68kIsa.describe("DBF", None).unwrap();
    assert_eq!(dbf.aliases, ["DBRA"]);
    assert_eq!(
        dbf.forms,
        dbra.forms
            .iter()
            .map(|f| FormInfo {
                syntax: f.syntax.replace("DBRA", "DBF"),
                ..f.clone()
            })
            .collect::<Vec<_>>()
    );
    let add = M68kIsa.describe("ADD", None).unwrap();
    assert_eq!(add.also_accepts, ["ADDA", "ADDI"]);
    assert_eq!(add.forms[1].syntax, "ADD.W/L <ea>,Dn");
    assert_eq!(add.forms[1].default_size, Some("W"));
    let long = M68kIsa.describe("add", Some("L")).unwrap();
    assert_eq!(long.size, Some("L"));
    assert_eq!(long.forms.len(), 2, "the byte-only form is filtered out");
    let rolx = M68kIsa.describe("ROLX", None).unwrap();
    assert_eq!((rolx.canonical, rolx.doc_id), ("ROXL", "isa-m68k-roxl"));
}

#[test]
fn every_spelling_describes_with_each_accepted_size() {
    for spelling in SPELLINGS {
        let info = M68kIsa.describe(spelling, None).unwrap();
        assert!(!info.forms.is_empty(), "{spelling}");
        assert_eq!(
            info.forms.len(),
            instruction_of(spelling).unwrap().0.forms.len()
        );
        for form in &info.forms {
            assert!(form.syntax.starts_with(spelling), "{}", form.syntax);
            if let Some(default) = form.default_size {
                assert!(form.sizes.contains(&default), "{spelling}");
            }
            for &letter in &form.sizes {
                let filtered = M68kIsa.describe(spelling, Some(letter)).unwrap();
                assert_eq!(filtered.size, Some(letter));
                assert!(filtered.forms.iter().all(|f| f.sizes.contains(&letter)));
            }
        }
    }
}

#[test]
fn flag_names_are_the_sr_condition_fields() {
    let sr = M68K
        .registers()
        .iter()
        .find(|register| register.name == "SR")
        .unwrap();
    let mut ccr: Vec<_> = sr
        .fields
        .iter()
        .filter(|field| field.bit <= 4 && field.width == 1)
        .collect();
    ccr.sort_by_key(|field| std::cmp::Reverse(field.bit));
    let ccr: Vec<&str> = ccr.iter().map(|field| field.name).collect();
    assert_eq!(ccr, ["X", "N", "Z", "V", "C"]);
    for spelling in SPELLINGS {
        for form in M68kIsa.describe(spelling, None).unwrap().forms {
            let names: Vec<&str> = form.flags.iter().map(|flag| flag.flag).collect();
            assert_eq!(names, ccr, "{spelling}");
        }
    }
}

#[test]
fn every_mode_string_is_frozen_notation_and_ids_are_kebab_case() {
    for entry in M68kIsa.ea_timing() {
        assert!(NOTATION.contains(&entry.mode), "{}", entry.mode);
    }
    assert_eq!(M68kIsa.ea_timing().len(), 24);
    for spelling in SPELLINGS {
        for form in M68kIsa.describe(spelling, None).unwrap().forms {
            for operand in &form.operands {
                for mode in &operand.modes {
                    assert!(NOTATION.contains(mode), "{spelling}: {mode}");
                }
            }
            for row in &form.timing {
                for condition in &row.when {
                    assert!(usize::from(condition.operand) < form.operands.len());
                    for mode in &condition.modes {
                        assert!(NOTATION.contains(mode), "{spelling}: {mode}");
                    }
                }
                if let Some(outcome) = row.outcome {
                    assert!(DiagCode(outcome).is_valid(), "{outcome}");
                }
                if let Some(per) = row.per_unit {
                    assert!(DiagCode(per.unit).is_valid(), "{}", per.unit);
                }
            }
        }
    }
    for (mode, notation) in Mode::ALL.into_iter().zip(NOTATION) {
        assert_eq!(mode.notation(), notation);
    }
}

/// The total and bound of a rendered form for one case, from its `TimingInfo` rows and
/// `ea_timing()`.
fn rendered_total(
    form: &FormInfo,
    size: Option<&str>,
    modes: [Option<Mode>; 2],
    outcome: Outcome,
    n: u32,
) -> Option<(Cycles, Bound)> {
    let applies = |row: &&TimingInfo| {
        let size_holds = match size {
            None => row.sizes.is_empty(),
            Some(letter) => row.sizes.contains(&letter),
        };
        let modes_hold = row.when.iter().all(|condition| {
            modes[usize::from(condition.operand)]
                .is_some_and(|mode| condition.modes.contains(&mode.notation()))
        });
        let outcome_holds = match row.outcome {
            None => outcome == Outcome::Always,
            Some(id) => id == outcome.id(),
        };
        size_holds && modes_hold && outcome_holds
    };
    let rows: Vec<&TimingInfo> = form.timing.iter().filter(applies).collect();
    assert!(
        rows.len() <= 1,
        "{}: {} rows apply",
        form.syntax,
        rows.len()
    );
    let row = rows.first()?;
    let mut total = row.base;
    for &operand in &row.plus_ea {
        let mode = modes[usize::from(operand)]?;
        let letter = size.unwrap_or("W");
        let ea = M68kIsa
            .ea_timing()
            .iter()
            .find(|entry| entry.mode == mode.notation() && entry.sizes.contains(&letter))
            .unwrap();
        total.clocks += ea.cycles.clocks;
        total.reads += ea.cycles.reads;
        total.writes += ea.cycles.writes;
    }
    if let Some(per) = row.per_unit {
        let n16 = u16::try_from(n).unwrap();
        let n8 = u8::try_from(n).unwrap();
        total.clocks += per.cycles.clocks * n16;
        total.reads += per.cycles.reads * n8;
        total.writes += per.cycles.writes * n8;
    }
    Some((total, row.bound))
}

#[test]
fn rendered_timing_rows_agree_with_cycles() {
    let mut cases = 0;
    for spelling in SPELLINGS {
        let (entry, _) = instruction_of(spelling).unwrap();
        let info = M68kIsa.describe(spelling, None).unwrap();
        for (form, rendered) in entry.forms.iter().zip(&info.forms) {
            let sizes: Vec<Option<Size>> = if form.sizes.is_empty() {
                vec![None]
            } else {
                form.sizes.iter().map(Some).collect()
            };
            let domain = |index: usize| -> Vec<Option<Mode>> {
                let modes = form
                    .operands
                    .get(index)
                    .map(|op| op.modes())
                    .unwrap_or_default();
                if modes.is_empty() {
                    vec![None]
                } else {
                    modes.iter().map(Some).collect()
                }
            };
            for size in &sizes {
                for first in domain(0) {
                    for second in domain(1) {
                        for outcome in OUTCOMES {
                            for n in [0, 3] {
                                let modes = [first, second];
                                let size_arg = size.unwrap_or(Size::Word);
                                let expected = cycles(form, size_arg, modes, outcome, n);
                                let letter = size.map(Size::letter);
                                let found = rendered_total(rendered, letter, modes, outcome, n);
                                assert_eq!(
                                    found, expected,
                                    "{} {modes:?} {outcome:?}",
                                    rendered.syntax
                                );
                                cases += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(cases > 10_000, "{cases} cases");
}
