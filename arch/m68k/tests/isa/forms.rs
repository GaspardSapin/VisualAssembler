//! Tests for the forms of the table (`src/isa/model.rs`, `src/isa/table_*.rs`): sizes and
//! defaults, operand counts, addressing categories, privilege, and flag facts (m1 §4.1, §4.7).
#![allow(clippy::unwrap_used)]

use std::collections::{BTreeSet, HashSet};

use va_arch_m68k::isa::{Form, INSTRUCTIONS, Mode, ModeSet, Operand, Size, SizeSet};
use va_core::isa::FlagEffect;

use crate::{all_forms, form_of, operands};

/// The entries whose forms are unsized (§4.1).
const UNSIZED: [&str; 12] = [
    "NOP", "RTS", "RTE", "RTR", "RESET", "TRAPV", "ILLEGAL", "STOP", "TRAP", "JMP", "JSR", "UNLK",
];

/// The privileged forms of §4.1, as `<entry> <operands>`.
const PRIVILEGED: [&str; 9] = [
    "ANDI #<data>,SR",
    "EORI #<data>,SR",
    "ORI #<data>,SR",
    "MOVE <ea>,SR",
    "MOVE USP,An",
    "MOVE An,USP",
    "RESET",
    "RTE",
    "STOP #<data>",
];

/// A claimed category: (entry, form index, operand index, category, modes the 68000 adds to
/// it, modes it removes).
type Claim = (
    &'static str,
    usize,
    usize,
    ModeSet,
    &'static [Mode],
    &'static [Mode],
);

/// The Motorola category each `Ea` operand claims, typed from the Programmer's Reference
/// Manual independently of the tables.
#[rustfmt::skip]
const CLAIMS: &[Claim] = &[
    ("ADD", 0, 0, ModeSet::ALL, &[], &[Mode::An]),
    ("ADD", 1, 0, ModeSet::ALL, &[], &[]),
    ("ADD", 2, 1, ModeSet::MEMORY_ALTERABLE, &[], &[]),
    ("ADDA", 0, 0, ModeSet::ALL, &[], &[]),
    ("ADDI", 0, 1, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("ADDQ", 0, 1, ModeSet::ALTERABLE, &[], &[Mode::An]),
    ("AND", 0, 0, ModeSet::DATA, &[], &[]),
    ("AND", 1, 1, ModeSet::MEMORY_ALTERABLE, &[], &[]),
    ("ANDI", 0, 1, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("ASL", 2, 0, ModeSet::MEMORY_ALTERABLE, &[], &[]),
    ("ASR", 2, 0, ModeSet::MEMORY_ALTERABLE, &[], &[]),
    ("BCHG", 1, 1, ModeSet::DATA_ALTERABLE, &[], &[Mode::Dn]),
    ("BCHG", 3, 1, ModeSet::DATA_ALTERABLE, &[], &[Mode::Dn]),
    ("BCLR", 1, 1, ModeSet::DATA_ALTERABLE, &[], &[Mode::Dn]),
    ("BCLR", 3, 1, ModeSet::DATA_ALTERABLE, &[], &[Mode::Dn]),
    ("BSET", 1, 1, ModeSet::DATA_ALTERABLE, &[], &[Mode::Dn]),
    ("BSET", 3, 1, ModeSet::DATA_ALTERABLE, &[], &[Mode::Dn]),
    ("BTST", 1, 1, ModeSet::DATA, &[], &[Mode::Dn]),
    ("BTST", 3, 1, ModeSet::DATA, &[], &[Mode::Dn, Mode::Imm]),
    ("CHK", 0, 0, ModeSet::DATA, &[], &[]),
    ("CLR", 0, 0, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("CMP", 0, 0, ModeSet::ALL, &[], &[Mode::An]),
    ("CMP", 1, 0, ModeSet::ALL, &[], &[]),
    ("CMPA", 0, 0, ModeSet::ALL, &[], &[]),
    ("CMPI", 0, 1, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("DIVS", 0, 0, ModeSet::DATA, &[], &[]),
    ("DIVU", 0, 0, ModeSet::DATA, &[], &[]),
    ("EOR", 0, 1, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("EORI", 0, 1, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("JMP", 0, 0, ModeSet::CONTROL, &[], &[]),
    ("JSR", 0, 0, ModeSet::CONTROL, &[], &[]),
    ("LEA", 0, 0, ModeSet::CONTROL, &[], &[]),
    ("LSL", 2, 0, ModeSet::MEMORY_ALTERABLE, &[], &[]),
    ("LSR", 2, 0, ModeSet::MEMORY_ALTERABLE, &[], &[]),
    ("MOVE", 0, 0, ModeSet::ALL, &[], &[Mode::An]),
    ("MOVE", 0, 1, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("MOVE", 1, 0, ModeSet::ALL, &[], &[]),
    ("MOVE", 1, 1, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("MOVE", 2, 0, ModeSet::DATA, &[], &[]),
    ("MOVE", 3, 0, ModeSet::DATA, &[], &[]),
    ("MOVE", 4, 1, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("MOVEA", 0, 0, ModeSet::ALL, &[], &[]),
    ("MOVEM", 0, 1, ModeSet::CONTROL_ALTERABLE, &[Mode::PreDec], &[]),
    ("MOVEM", 1, 0, ModeSet::CONTROL, &[Mode::PostInc], &[]),
    ("MULS", 0, 0, ModeSet::DATA, &[], &[]),
    ("MULU", 0, 0, ModeSet::DATA, &[], &[]),
    ("NBCD", 0, 0, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("NEG", 0, 0, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("NEGX", 0, 0, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("NOT", 0, 0, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("OR", 0, 0, ModeSet::DATA, &[], &[]),
    ("OR", 1, 1, ModeSet::MEMORY_ALTERABLE, &[], &[]),
    ("ORI", 0, 1, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("PEA", 0, 0, ModeSet::CONTROL, &[], &[]),
    ("ROL", 2, 0, ModeSet::MEMORY_ALTERABLE, &[], &[]),
    ("ROR", 2, 0, ModeSet::MEMORY_ALTERABLE, &[], &[]),
    ("ROXL", 2, 0, ModeSet::MEMORY_ALTERABLE, &[], &[]),
    ("ROXR", 2, 0, ModeSet::MEMORY_ALTERABLE, &[], &[]),
    ("Scc", 0, 0, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("SUB", 0, 0, ModeSet::ALL, &[], &[Mode::An]),
    ("SUB", 1, 0, ModeSet::ALL, &[], &[]),
    ("SUB", 2, 1, ModeSet::MEMORY_ALTERABLE, &[], &[]),
    ("SUBA", 0, 0, ModeSet::ALL, &[], &[]),
    ("SUBI", 0, 1, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("SUBQ", 0, 1, ModeSet::ALTERABLE, &[], &[Mode::An]),
    ("TAS", 0, 0, ModeSet::DATA_ALTERABLE, &[], &[]),
    ("TST", 0, 0, ModeSet::DATA_ALTERABLE, &[], &[]),
];

/// Renders the flags of a form as `X N Z V C` symbols: `-` unaffected, `*` result, `0`
/// cleared, `1` set, `U` undefined, `!` special.
fn flag_string(form: &Form) -> String {
    let symbols: Vec<&str> = form
        .flags
        .named()
        .into_iter()
        .map(|(_, effect)| match effect {
            FlagEffect::Unaffected => "-",
            FlagEffect::Result => "*",
            FlagEffect::Cleared => "0",
            FlagEffect::Set => "1",
            FlagEffect::Undefined => "U",
            FlagEffect::Special => "!",
        })
        .collect();
    symbols.join(" ")
}

fn forms_of(name: &str) -> &'static [Form] {
    INSTRUCTIONS
        .iter()
        .find(|entry| entry.name == name)
        .unwrap()
        .forms
}

#[test]
fn default_size_is_an_accepted_size() {
    for (entry, index, form) in all_forms() {
        if let Some(size) = form.default_size {
            assert!(form.sizes.contains(size), "{} form {index}", entry.name);
        }
    }
}

#[test]
fn unsized_forms_are_exactly_those_of_the_spec_list() {
    let mut unsized_entries = BTreeSet::new();
    for (entry, index, form) in all_forms() {
        let is_unsized = form.sizes == SizeSet::NONE;
        assert_eq!(is_unsized, form.sizes.is_empty());
        if is_unsized {
            assert_eq!(form.default_size, None, "{} form {index}", entry.name);
            unsized_entries.insert(entry.name);
        }
        let all_unsized = entry.forms.iter().all(|f| f.sizes.is_empty());
        assert_eq!(
            is_unsized, all_unsized,
            "{} mixes sized and unsized forms",
            entry.name
        );
    }
    assert_eq!(unsized_entries, UNSIZED.into_iter().collect());
}

#[test]
fn defaults_follow_the_a68k_rules() {
    for (entry, index, form) in all_forms() {
        let at = format!("{} form {index}", entry.name);
        if matches!(entry.name, "Bcc" | "BRA" | "BSR") {
            // Branches: the assembler chooses .S or .W.
            assert_eq!(form.default_size, None, "{at}");
            assert_eq!(form.sizes.len(), 1, "{at}");
        } else if form.sizes.len() >= 2 {
            assert_eq!(form.default_size, Some(Size::Word), "{at}");
        } else if let Some(size) = form.sizes.iter().next() {
            if form.default_size.is_none() {
                // A byte form split from a word/long form with the same encoding.
                assert_eq!(size, Size::Byte, "{at}");
                let sibling = entry.forms.iter().any(|other| {
                    other.encoding.opword == form.encoding.opword
                        && other.encoding.mask == form.encoding.mask
                        && other.default_size == Some(Size::Word)
                });
                assert!(sibling, "{at}");
            } else {
                assert_eq!(form.default_size, Some(size), "{at}");
            }
        }
    }
    let branch_sizes = |name: &str| -> BTreeSet<Size> {
        forms_of(name).iter().flat_map(|f| f.sizes.iter()).collect()
    };
    for name in ["Bcc", "BRA", "BSR"] {
        assert_eq!(branch_sizes(name), [Size::Short, Size::Word].into());
    }
    assert_eq!(branch_sizes("DBcc"), [Size::Word].into());
    // Single-size forms observed in A68k.
    assert_eq!(
        form_of("LEA", "<ea>,An", None).sizes,
        SizeSet::of(&[Size::Long])
    );
    assert_eq!(
        form_of("SWAP", "Dn", None).sizes,
        SizeSet::of(&[Size::Word])
    );
    assert_eq!(
        form_of("EXG", "Dn,Dn", None).sizes,
        SizeSet::of(&[Size::Long])
    );
    let moveq = form_of("MOVEQ", "#<-128..127>,Dn", None);
    assert_eq!(moveq.sizes, SizeSet::of(&[Size::Long]));
    // Bit operations: long on a data register, byte on memory.
    for name in ["BTST", "BCHG", "BCLR", "BSET"] {
        for form in forms_of(name) {
            let expected = if matches!(form.operands[1], Operand::Dn) {
                Size::Long
            } else {
                Size::Byte
            };
            assert_eq!(form.sizes, SizeSet::of(&[expected]), "{name}");
        }
    }
}

#[test]
fn forms_have_zero_to_two_operands() {
    for (entry, index, form) in all_forms() {
        assert!(form.operands.len() <= 2, "{} form {index}", entry.name);
    }
}

#[test]
fn ea_sets_are_non_empty_and_inside_their_claimed_category() {
    let mut claimed = HashSet::new();
    for &(name, form_index, operand_index, category, added, removed) in CLAIMS {
        let form = &forms_of(name)[form_index];
        let Operand::Ea(set) = form.operands[operand_index] else {
            panic!("{name} form {form_index} operand {operand_index} is not an <ea>");
        };
        let added = ModeSet::of(added);
        let removed = ModeSet::of(removed);
        assert!(
            removed.is_subset(category),
            "{name}: removes modes outside its category"
        );
        assert!(
            added.intersection(category).is_empty(),
            "{name}: adds modes already there"
        );
        assert!(!set.is_empty(), "{name} form {form_index}");
        assert!(
            set.is_subset(category.union(added)),
            "{name} form {form_index}"
        );
        assert_eq!(
            set,
            category.union(added).minus(removed),
            "{name} form {form_index}"
        );
        claimed.insert((name, form_index, operand_index));
    }
    for (entry, index, form) in all_forms() {
        for (operand_index, operand) in form.operands.iter().enumerate() {
            if let Operand::Ea(set) = operand {
                assert!(!set.is_empty());
                assert!(
                    claimed.contains(&(entry.name, index, operand_index)),
                    "{} form {index} operand {operand_index} claims no category",
                    entry.name
                );
            }
        }
    }
}

#[test]
fn privileged_forms_are_exactly_those_of_the_spec() {
    let mut found = BTreeSet::new();
    for (entry, _, form) in all_forms() {
        if form.privileged {
            let text = format!("{} {}", entry.name, operands(form));
            found.insert(text.trim_end().to_owned());
        }
    }
    let expected: BTreeSet<String> = PRIVILEGED.iter().map(|&s| s.to_owned()).collect();
    assert_eq!(found, expected);
}

#[test]
fn flag_spot_checks() {
    for form in forms_of("MOVE").iter().take(2) {
        assert_eq!(flag_string(form), "- * * 0 0");
    }
    for form in forms_of("ADD") {
        assert_eq!(flag_string(form), "* * * * *");
    }
    for form in forms_of("ADDA") {
        assert_eq!(flag_string(form), "- - - - -");
    }
    for form in forms_of("CMP") {
        assert_eq!(flag_string(form), "- * * * *");
    }
    for form in forms_of("CLR") {
        assert_eq!(flag_string(form), "- 0 1 0 0");
    }
    for form in forms_of("BTST") {
        assert_eq!(flag_string(form), "- - * - -");
    }
    for form in forms_of("ABCD") {
        assert_eq!(form.flags.z, FlagEffect::Special);
    }
    for form in forms_of("CHK") {
        assert_eq!(form.flags.n, FlagEffect::Special);
    }
    for name in ["LEA", "Scc"] {
        for form in forms_of(name) {
            assert_eq!(flag_string(form), "- - - - -", "{name}");
        }
    }
    // ADDQ and SUBQ to an address register leave the flags alone.
    assert_eq!(
        flag_string(form_of("ADDQ", "#<1..8>,An", None)),
        "- - - - -"
    );
    assert_eq!(
        flag_string(form_of("SUBQ", "#<1..8>,An", None)),
        "- - - - -"
    );
}

#[test]
fn special_flags_are_exactly_the_documented_rules() {
    let mut found = BTreeSet::new();
    for (entry, index, form) in all_forms() {
        for (flag, effect) in form.flags.named() {
            if effect == FlagEffect::Special {
                found.insert(format!("{} {} {flag}", entry.name, index));
            }
        }
    }
    let mut expected = BTreeSet::new();
    for name in ["ABCD", "SBCD", "ADDX", "SUBX"] {
        for index in 0..2 {
            expected.insert(format!("{name} {index} Z"));
        }
    }
    for name in ["NBCD", "NEGX"] {
        expected.insert(format!("{name} 0 Z"));
    }
    expected.insert("CHK 0 N".to_owned());
    for name in ["ANDI", "EORI", "ORI"] {
        for index in 1..3 {
            for flag in ["X", "N", "Z", "V", "C"] {
                expected.insert(format!("{name} {index} {flag}"));
            }
        }
    }
    assert_eq!(found, expected);
    // The CCR/SR forms are the ones at indexes 1 and 2.
    for name in ["ANDI", "EORI", "ORI"] {
        let forms = forms_of(name);
        assert!(matches!(forms[1].operands, [Operand::Imm, Operand::Ccr]));
        assert!(matches!(forms[2].operands, [Operand::Imm, Operand::Sr]));
    }
}

#[test]
fn no_68010_or_later_form_leaks_in() {
    // CHK is word only.
    for form in forms_of("CHK") {
        assert_eq!(form.sizes, SizeSet::of(&[Size::Word]));
    }
    // TST and CMPI: no An, PC-relative or immediate operand.
    let excluded = ModeSet::of(&[Mode::An, Mode::PcDisp, Mode::PcIndex, Mode::Imm]);
    for name in ["TST", "CMPI"] {
        for form in forms_of(name) {
            for operand in form.operands {
                if let Operand::Ea(set) = operand {
                    assert!(set.intersection(excluded).is_empty(), "{name}");
                }
            }
        }
    }
    // MOVE from SR is not privileged on the 68000.
    assert!(!form_of("MOVE", "SR,<ea>", None).privileged);
    // No byte operation on an address register.
    for (entry, index, form) in all_forms() {
        if form.sizes.contains(Size::Byte) {
            for operand in form.operands {
                assert!(
                    !operand.modes().contains(Mode::An),
                    "{} form {index}",
                    entry.name
                );
            }
        }
    }
}
