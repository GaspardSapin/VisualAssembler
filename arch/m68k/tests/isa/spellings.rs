//! Tests for the spelling catalogue (`src/isa/mod.rs`, `src/isa/conditions.rs`): the §4.1 list,
//! family expansion plus aliases, resolution, conditions and `also_accepts`.
#![allow(clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet, HashSet};

use va_arch_m68k::isa::{
    CONDITIONS, Family, INSTRUCTIONS, SPELLINGS, instruction_of, spelling_info,
};

/// The 126 spellings of m1-core-isa-lexer.md §4.1, typed from the spec.
const SPEC_LIST: &str = "
ABCD ADD ADDA ADDI ADDQ ADDX AND ANDI ASL ASR BCC BCHG BCLR BCS BEQ BGE BGT BHI BHS BLE BLO BLS
BLT BMI BNE BPL BRA BSET BSR BTST BVC BVS CHK CLR CMP CMPA CMPI CMPM DBCC DBCS DBEQ DBF DBGE DBGT
DBHI DBHS DBLE DBLO DBLS DBLT DBMI DBNE DBPL DBRA DBT DBVC DBVS DIVS DIVU EOR EORI EXG EXT ILLEGAL
JMP JSR LEA LINK LSL LSR MOVE MOVEA MOVEM MOVEP MOVEQ MULS MULU NBCD NEG NEGX NOP NOT OR ORI PEA
RESET ROL ROLX ROR RORX ROXL ROXR RTE RTR RTS SBCD SCC SCS SEQ SF SGE SGT SHI SHS SLE SLO SLS SLT
SMI SNE SPL ST STOP SUB SUBA SUBI SUBQ SUBX SVC SVS SWAP TAS TRAP TRAPV TST UNLK
";

/// The aliases of §4.1: (alias, canonical spelling).
const SPEC_ALIASES: [(&str, &str); 9] = [
    ("BHS", "BCC"),
    ("BLO", "BCS"),
    ("DBHS", "DBCC"),
    ("DBLO", "DBCS"),
    ("DBRA", "DBF"),
    ("SHS", "SCC"),
    ("SLO", "SCS"),
    ("ROLX", "ROXL"),
    ("RORX", "ROXR"),
];

/// The conditions of §4.1: (name, code, test).
const SPEC_CONDITIONS: [(&str, u8, &str); 16] = [
    ("T", 0, "1"),
    ("F", 1, "0"),
    ("HI", 2, "!C & !Z"),
    ("LS", 3, "C | Z"),
    ("CC", 4, "!C"),
    ("CS", 5, "C"),
    ("NE", 6, "!Z"),
    ("EQ", 7, "Z"),
    ("VC", 8, "!V"),
    ("VS", 9, "V"),
    ("PL", 10, "!N"),
    ("MI", 11, "N"),
    ("GE", 12, "N == V"),
    ("LT", 13, "N != V"),
    ("GT", 14, "!Z & N == V"),
    ("LE", 15, "Z | N != V"),
];

/// The mnemonics without operands (§5.2).
const NO_OPERANDS: [&str; 7] = ["NOP", "RESET", "RTE", "RTR", "RTS", "TRAPV", "ILLEGAL"];

#[test]
fn spellings_equal_the_spec_list() {
    let expected: Vec<&str> = SPEC_LIST.split_whitespace().collect();
    assert_eq!(expected.len(), 126);
    assert_eq!(SPELLINGS.to_vec(), expected);
}

#[test]
fn spellings_are_sorted_unique_uppercase_ascii() {
    for pair in SPELLINGS.windows(2) {
        assert!(pair[0] < pair[1], "{} is not before {}", pair[0], pair[1]);
    }
    for spelling in SPELLINGS {
        assert!(!spelling.is_empty());
        assert!(
            spelling.bytes().all(|byte| byte.is_ascii_uppercase()),
            "{spelling} is not uppercase ASCII"
        );
    }
}

#[test]
fn family_expansion_plus_aliases_gives_the_list() {
    let mut built = BTreeSet::new();
    for entry in &INSTRUCTIONS {
        match entry.family {
            None => {
                assert!(built.insert(entry.name.to_owned()), "{}", entry.name);
            }
            Some(family) => {
                for condition in &CONDITIONS[usize::from(family.first_condition())..] {
                    let spelling = format!("{}{}", family.prefix(), condition.name);
                    assert!(built.insert(spelling.clone()), "{spelling}");
                }
            }
        }
    }
    for (alias, _) in SPEC_ALIASES {
        assert!(built.insert(alias.to_owned()), "{alias}");
    }
    let listed: BTreeSet<String> = SPELLINGS.iter().map(|&s| s.to_owned()).collect();
    assert_eq!(built, listed);
}

#[test]
fn the_table_has_74_entries_with_unique_names_and_doc_ids() {
    assert_eq!(INSTRUCTIONS.len(), 74);
    let names: HashSet<&str> = INSTRUCTIONS.iter().map(|entry| entry.name).collect();
    assert_eq!(names.len(), 74);
    let doc_ids: HashSet<&str> = INSTRUCTIONS.iter().map(|entry| entry.doc_id).collect();
    assert_eq!(doc_ids.len(), 74);
    for entry in &INSTRUCTIONS {
        assert_eq!(
            entry.doc_id,
            format!("isa-m68k-{}", entry.name.to_ascii_lowercase())
        );
        assert!(!entry.forms.is_empty(), "{} has no form", entry.name);
        if let Some(family) = entry.family {
            assert_eq!(entry.name, family.name());
        }
    }
    let families: Vec<Family> = INSTRUCTIONS.iter().filter_map(|e| e.family).collect();
    assert_eq!(families, [Family::Bcc, Family::DBcc, Family::Scc]);
    for pair in INSTRUCTIONS.windows(2) {
        assert!(
            pair[0].name.to_ascii_uppercase() < pair[1].name.to_ascii_uppercase(),
            "{} is not before {}",
            pair[0].name,
            pair[1].name
        );
    }
}

#[test]
fn every_spelling_resolves_in_any_case() {
    for spelling in SPELLINGS {
        let (entry, condition) = instruction_of(spelling).unwrap();
        let info = spelling_info(spelling).unwrap();
        assert_eq!(info.doc_id, entry.doc_id, "{spelling}");
        match entry.family {
            Some(family) => {
                let condition = condition.unwrap();
                assert!(condition.code >= family.first_condition(), "{spelling}");
                assert_eq!(
                    format!("{}{}", family.prefix(), condition.name),
                    info.canonical
                );
            }
            None => {
                assert!(condition.is_none(), "{spelling}");
                assert_eq!(info.canonical, entry.name, "{spelling}");
            }
        }
        let lower = spelling.to_ascii_lowercase();
        let mut mixed = lower.clone();
        mixed.replace_range(..1, &spelling[..1]);
        for variant in [lower, mixed] {
            assert_eq!(spelling_info(&variant), Some(info), "{variant}");
            assert_eq!(
                instruction_of(&variant),
                Some((entry, condition)),
                "{variant}"
            );
        }
    }
}

#[test]
fn other_words_do_not_resolve() {
    for name in [
        "", "MOV", "B", "S", "DB", "BT", "BF", "BHSS", "DBCCC", "ADD.W", "ADDQ ", " NOP", "MOVEC",
        "RTD", "BKPT", "EXTB", "CHK2", "TRAPCC", "Ä", "nop\0", "DBEQ.", "ST2",
    ] {
        assert!(spelling_info(name).is_none(), "{name:?}");
        assert!(instruction_of(name).is_none(), "{name:?}");
    }
}

#[test]
fn aliases_resolve_to_their_canonical_spelling() {
    for (alias, target) in SPEC_ALIASES {
        assert!(SPELLINGS.contains(&target), "{target}");
        assert_eq!(spelling_info(alias).unwrap().canonical, target, "{alias}");
        assert_eq!(spelling_info(target).unwrap().canonical, target, "{target}");
        assert_eq!(instruction_of(alias), instruction_of(target), "{alias}");
    }
    let aliases: HashSet<&str> = SPEC_ALIASES.iter().map(|&(alias, _)| alias).collect();
    for spelling in SPELLINGS {
        if !aliases.contains(spelling) {
            assert_eq!(spelling_info(spelling).unwrap().canonical, spelling);
        }
    }
}

#[test]
fn also_accepts_targets_are_entries() {
    let expected: BTreeMap<&str, Vec<&str>> = [
        ("ADD", vec!["ADDA", "ADDI"]),
        ("SUB", vec!["SUBA", "SUBI"]),
        ("CMP", vec!["CMPA", "CMPI", "CMPM"]),
        ("AND", vec!["ANDI"]),
        ("OR", vec!["ORI"]),
        ("EOR", vec!["EORI"]),
        ("MOVE", vec!["MOVEA"]),
    ]
    .into_iter()
    .collect();
    let mut found = BTreeMap::new();
    for entry in &INSTRUCTIONS {
        if entry.also_accepts.is_empty() {
            continue;
        }
        for target in entry.also_accepts {
            let (other, condition) = instruction_of(target).unwrap();
            assert_eq!(other.name, *target, "{} names a non-entry", entry.name);
            assert!(condition.is_none() && other.name != entry.name);
        }
        found.insert(entry.name, entry.also_accepts.to_vec());
    }
    assert_eq!(found, expected);
}

#[test]
fn conditions_match_the_spec_and_appear_once_per_family() {
    assert_eq!(CONDITIONS.len(), 16);
    for (condition, (name, code, test)) in CONDITIONS.iter().zip(SPEC_CONDITIONS) {
        assert_eq!(
            (condition.name, condition.code, condition.test),
            (name, code, test)
        );
        assert_eq!(
            condition.doc_id,
            format!("isa-m68k-cond-{}", name.to_ascii_lowercase())
        );
    }
    let aliases: HashSet<&str> = SPEC_ALIASES.iter().map(|&(alias, _)| alias).collect();
    let mut codes: BTreeMap<&str, Vec<u8>> = BTreeMap::new();
    for spelling in SPELLINGS.iter().filter(|s| !aliases.contains(*s)) {
        let (entry, condition) = instruction_of(spelling).unwrap();
        if let (Some(family), Some(condition)) = (entry.family, condition) {
            codes.entry(family.name()).or_default().push(condition.code);
        }
    }
    for codes in codes.values_mut() {
        codes.sort_unstable();
    }
    assert_eq!(codes["Bcc"], (2..16).collect::<Vec<u8>>());
    assert_eq!(codes["DBcc"], (0..16).collect::<Vec<u8>>());
    assert_eq!(codes["Scc"], (0..16).collect::<Vec<u8>>());
}

#[test]
fn takes_operands_is_false_exactly_for_the_no_operand_mnemonics() {
    for spelling in SPELLINGS {
        let info = spelling_info(spelling).unwrap();
        assert_eq!(
            info.takes_operands,
            !NO_OPERANDS.contains(&spelling),
            "{spelling}"
        );
        let (entry, _) = instruction_of(spelling).unwrap();
        let has_operands = entry.forms.iter().map(|form| !form.operands.is_empty());
        for form_has_operands in has_operands {
            assert_eq!(form_has_operands, info.takes_operands, "{spelling}");
        }
    }
}
