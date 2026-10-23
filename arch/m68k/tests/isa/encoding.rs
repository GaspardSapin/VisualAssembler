//! Tests for `src/isa/encoding.rs`: field layout, uniqueness over all 65,536 opwords, and the
//! golden opwords observed in a68k 2.71.F3w listings (m1 §4.5, §4.7).
#![allow(clippy::unwrap_used)]

use va_arch_m68k::isa::{
    EaField, Form, INSTRUCTIONS, Instruction, Mode, Operand, Size, SizeField, instruction_of,
};

use crate::{all_forms, operands};

/// The number of opwords the table calls valid (matched by exactly one form and condition).
/// Golden constant, pinned pending the QA review of m1 §4.7. Per opword line, counting `<ea>`
/// encodings (all 61, data 53, memory 45, control 28, alterable 58, data alterable 50, memory
/// alterable 42, control alterable 26): line 0 2,988; MOVE/MOVEA 9,726; line 4 1,958; line 5
/// 3,584; line 6 4,096; MOVEQ 2,048; line 8 3,256; lines 9, B and D 3,768 each; line C 3,448;
/// line E 3,408.
const VALID_OPWORDS: usize = 45_816;

/// One (entry, form index, form, condition) candidate: family forms once per condition code
/// of the family, the other forms once with `None`.
type Candidate = (&'static Instruction, usize, &'static Form, Option<u8>);

fn candidates() -> Vec<Candidate> {
    let mut out = Vec::new();
    for (entry, index, form) in all_forms() {
        match entry.family {
            Some(family) => {
                for code in family.first_condition()..16 {
                    out.push((entry, index, form, Some(code)));
                }
            }
            None => out.push((entry, index, form, None)),
        }
    }
    out
}

fn matching(candidates: &[Candidate], opword: u16) -> Vec<&Candidate> {
    candidates
        .iter()
        .filter(|(_, _, form, condition)| form.matches(opword, *condition))
        .collect()
}

#[test]
fn fixed_bits_and_fields_do_not_overlap() {
    for (entry, index, form) in all_forms() {
        let at = format!("{} form {index}", entry.name);
        let encoding = form.encoding;
        assert_eq!(
            encoding.opword & !encoding.mask,
            0,
            "{at}: opword outside mask"
        );
        assert_eq!(
            encoding.size.bits() & encoding.mask,
            0,
            "{at}: size field is fixed"
        );
        assert_eq!(
            encoding.ea.bits() & encoding.mask,
            0,
            "{at}: EA field is fixed"
        );
        assert_eq!(
            encoding.size.bits() & encoding.ea.bits(),
            0,
            "{at}: fields overlap"
        );
        if entry.family.is_some() {
            assert_eq!(encoding.mask & 0x0F00, 0, "{at}: condition field is fixed");
        }
        let ea_operands = form
            .operands
            .iter()
            .filter(|op| matches!(op, Operand::Ea(_)))
            .count();
        let expected = match encoding.ea {
            EaField::None => 0,
            EaField::Low | EaField::MoveDest => 1,
            EaField::Both => 2,
        };
        assert_eq!(ea_operands, expected, "{at}: EA operands vs EA field");
        if encoding.size == SizeField::None {
            assert!(
                form.sizes.len() <= 1,
                "{at}: several sizes without a size field"
            );
        } else {
            assert!(!form.sizes.is_empty(), "{at}: size field without size");
        }
    }
}

#[test]
fn every_opword_matches_at_most_one_form_and_every_form_matches() {
    let candidates = candidates();
    let mut hits = vec![0_usize; candidates.len()];
    let mut valid = 0;
    let mut conflicts = Vec::new();
    for opword in 0..=u16::MAX {
        let mut count = 0;
        for (slot, (entry, index, form, condition)) in candidates.iter().enumerate() {
            if form.matches(opword, *condition) {
                count += 1;
                hits[slot] += 1;
                if count > 1 && conflicts.len() < 20 {
                    conflicts.push(format!(
                        "{opword:04X}: {} form {index} {condition:?}",
                        entry.name
                    ));
                }
            }
        }
        if count > 0 {
            valid += 1;
        }
    }
    println!("valid 68000 opwords: {valid}");
    assert!(
        conflicts.is_empty(),
        "opwords matched twice: {conflicts:#?}"
    );
    for ((entry, index, _, condition), hits) in candidates.iter().zip(&hits) {
        assert!(
            *hits > 0,
            "{} form {index} {condition:?} matches no opword",
            entry.name
        );
    }
    assert_eq!(valid, VALID_OPWORDS, "pinned count of valid opwords");
}

/// Golden opwords: (word, spelling, size, operands of the matching form).
#[rustfmt::skip]
const GOLDEN: &[(u16, &str, Option<Size>, &str)] = &[
    (0x4E71, "NOP", None, ""),
    (0x4E75, "RTS", None, ""),
    (0x4AFC, "ILLEGAL", None, ""),
    (0x747F, "MOVEQ", Some(Size::Long), "#<-128..127>,Dn"),
    (0x2200, "MOVE", Some(Size::Long), "<ea>,<ea>"),
    (0x3040, "MOVEA", Some(Size::Word), "<ea>,An"),
    (0x51C8, "DBF", Some(Size::Word), "Dn,<label>"),
    (0x64D8, "BCC", Some(Size::Short), "<label>"),
    (0x6500, "BCS", Some(Size::Word), "<label>"),
    (0x60DC, "BRA", Some(Size::Short), "<label>"),
    (0x61C4, "BSR", Some(Size::Short), "<label>"),
    (0x55C0, "SCS", Some(Size::Byte), "<ea>"),
    (0xB308, "CMPM", Some(Size::Byte), "(An)+,(An)+"),
    (0xC308, "ABCD", Some(Size::Byte), "-(An),-(An)"),
    (0xD380, "ADDX", Some(Size::Long), "Dn,Dn"),
    (0xC141, "EXG", Some(Size::Long), "Dn,Dn"),
    (0x4840, "SWAP", Some(Size::Word), "Dn"),
    (0x4880, "EXT", Some(Size::Word), "Dn"),
    (0x48C0, "EXT", Some(Size::Long), "Dn"),
    (0x43D0, "LEA", Some(Size::Long), "<ea>,An"),
    (0x4E68, "MOVE", Some(Size::Long), "USP,An"),
    (0x4E60, "MOVE", Some(Size::Long), "An,USP"),
    (0x44C0, "MOVE", Some(Size::Word), "<ea>,CCR"),
    (0x40C0, "MOVE", Some(Size::Word), "SR,<ea>"),
    (0x007C, "ORI", Some(Size::Word), "#<data>,SR"),
    (0x023C, "ANDI", Some(Size::Byte), "#<data>,CCR"),
    (0x0A3C, "EORI", Some(Size::Byte), "#<data>,CCR"),
    (0x4190, "CHK", Some(Size::Word), "<ea>,Dn"),
    (0x48E7, "MOVEM", Some(Size::Long), "<list>,<ea>"),
    (0x4C9F, "MOVEM", Some(Size::Word), "<ea>,<list>"),
    (0x0108, "MOVEP", Some(Size::Word), "d16(An),Dn"),
    (0x5240, "ADDQ", Some(Size::Word), "#<1..8>,<ea>"),
    (0x5181, "SUBQ", Some(Size::Long), "#<1..8>,<ea>"),
    (0x0681, "ADDI", Some(Size::Long), "#<data>,<ea>"),
    (0xD0C0, "ADDA", Some(Size::Word), "<ea>,An"),
    (0x0C50, "CMPI", Some(Size::Word), "#<data>,<ea>"),
    (0xE348, "LSL", Some(Size::Word), "#<1..8>,Dn"),
    (0xE3D0, "LSL", Some(Size::Word), "<ea>"),
    (0xE362, "ASL", Some(Size::Word), "Dn,Dn"),
    (0x0800, "BTST", Some(Size::Long), "#<data>,Dn"),
    (0x0310, "BTST", Some(Size::Byte), "Dn,<ea>"),
    (0x08D0, "BSET", Some(Size::Byte), "#<data>,<ea>"),
    (0x4E56, "LINK", Some(Size::Word), "An,#<data>"),
    (0x4E4F, "TRAP", None, "#<0..15>"),
    (0x4E72, "STOP", None, "#<data>"),
    (0x4A40, "TST", Some(Size::Word), "<ea>"),
    (0x4250, "CLR", Some(Size::Word), "<ea>"),
];

#[test]
fn golden_opwords_decode_to_the_observed_instruction() {
    let candidates = candidates();
    for &(opword, spelling, size, operand_text) in GOLDEN {
        let found = matching(&candidates, opword);
        assert_eq!(
            found.len(),
            1,
            "{opword:04X} ({spelling}): {} matches",
            found.len()
        );
        let (entry, _, form, condition) = *found[0];
        let (expected_entry, expected_condition) = instruction_of(spelling).unwrap();
        assert_eq!(entry.name, expected_entry.name, "{opword:04X}");
        assert_eq!(
            condition,
            expected_condition.map(|c| c.code),
            "{opword:04X}"
        );
        assert_eq!(operands(form), operand_text, "{opword:04X} ({spelling})");
        match size {
            None => assert!(form.sizes.is_empty(), "{opword:04X}: sized"),
            Some(size) => {
                assert!(form.sizes.contains(size), "{opword:04X}: size {size:?}");
                if form.encoding.size != SizeField::None {
                    assert_eq!(
                        form.encoding.size.decode(opword),
                        Some(size),
                        "{opword:04X}"
                    );
                }
            }
        }
    }
}

#[test]
fn reserved_and_later_cpu_opwords_match_nothing() {
    let candidates = candidates();
    // Line A, line F, RTD, MOVEC, EXTB.L, TAS d8(PC,Xn), MULU.L/MULS.L.
    for opword in [0xA000, 0xF000, 0x4E74, 0x4E7A, 0x49C0, 0x4AFB, 0x4C00] {
        let found = matching(&candidates, opword);
        assert!(
            found.is_empty(),
            "{opword:04X} matches {} form(s)",
            found.len()
        );
    }
    // ILLEGAL is not TAS #imm; line A and line F are entirely unassigned.
    for opword in (0xA000..=0xAFFF).chain(0xF000..=0xFFFF) {
        assert!(matching(&candidates, opword).is_empty(), "{opword:04X}");
    }
}

#[test]
fn condition_argument_selects_bits_11_to_8() {
    let bcc_word = &INSTRUCTIONS.iter().find(|e| e.name == "Bcc").unwrap().forms[1];
    assert!(bcc_word.matches(0x6700, Some(7)));
    assert!(!bcc_word.matches(0x6700, Some(6)));
    assert!(bcc_word.matches(0x6700, None));
    assert!(
        !bcc_word.matches(0x6701, Some(7)),
        ".W needs a zero low byte"
    );
}

#[test]
fn mode_fields_round_trip() {
    for mode in Mode::ALL {
        let (field, register) = mode.fields();
        assert_eq!(Mode::from_fields(field, register.unwrap_or(5)), Some(mode));
    }
    for register in 5..8 {
        assert_eq!(Mode::from_fields(7, register), None);
    }
}

// QA (m1 §12.5 item 2): plausibility of `VALID_OPWORDS`, broken down per opword line and per
// table entry. The expected numbers are derived independently of the tables, from the 68000
// encoding rules and the `<ea>` category sizes (all 61, data 53, memory 45, control 28,
// alterable 58, data alterable 50, memory alterable 42, control alterable 26). Examples: ORI is
// 3 sizes x 50 + to CCR + to SR = 152; BTST is #,<ea> (data except #imm, 52) + 8 x Dn,<ea>
// (data, 53; a68k 2.71.F3w assembles `btst d0,#5` to 013C 0005) = 476; MOVE is .B 53 x 50,
// .W and .L 61 x 50 each, to CCR 53, to SR 53, from SR 50, USP 16 = 8,922; each shift or rotate
// is 3 sizes x 2 count kinds x 8 x 8 + memory 42 = 426. The binding check is M3's (m1 §12.3).

/// Valid opwords per top nibble (line 0 to line F).
const VALID_PER_LINE: [usize; 16] = [
    2_988, 2_650, 3_538, 3_538, 1_958, 3_584, 4_096, 2_048, 3_256, 3_768, 0, 3_768, 3_448, 3_768,
    3_408, 0,
];

/// Valid opwords per table entry, keyed by doc id (families count every condition code).
const VALID_PER_ENTRY: [(&str, usize); 74] = [
    ("isa-m68k-abcd", 128),
    ("isa-m68k-add", 2_408),
    ("isa-m68k-adda", 976),
    ("isa-m68k-addi", 150),
    ("isa-m68k-addq", 1_328),
    ("isa-m68k-addx", 384),
    ("isa-m68k-and", 2_280),
    ("isa-m68k-andi", 152),
    ("isa-m68k-asl", 426),
    ("isa-m68k-asr", 426),
    ("isa-m68k-bcc", 3_584),
    ("isa-m68k-bchg", 450),
    ("isa-m68k-bclr", 450),
    ("isa-m68k-bra", 256),
    ("isa-m68k-bset", 450),
    ("isa-m68k-bsr", 256),
    ("isa-m68k-btst", 476),
    ("isa-m68k-chk", 424),
    ("isa-m68k-clr", 150),
    ("isa-m68k-cmp", 1_400),
    ("isa-m68k-cmpa", 976),
    ("isa-m68k-cmpi", 150),
    ("isa-m68k-cmpm", 192),
    ("isa-m68k-dbcc", 128),
    ("isa-m68k-divs", 424),
    ("isa-m68k-divu", 424),
    ("isa-m68k-eor", 1_200),
    ("isa-m68k-eori", 152),
    ("isa-m68k-exg", 192),
    ("isa-m68k-ext", 16),
    ("isa-m68k-illegal", 1),
    ("isa-m68k-jmp", 28),
    ("isa-m68k-jsr", 28),
    ("isa-m68k-lea", 224),
    ("isa-m68k-link", 8),
    ("isa-m68k-lsl", 426),
    ("isa-m68k-lsr", 426),
    ("isa-m68k-move", 8_922),
    ("isa-m68k-movea", 976),
    ("isa-m68k-movem", 140),
    ("isa-m68k-movep", 256),
    ("isa-m68k-moveq", 2_048),
    ("isa-m68k-muls", 424),
    ("isa-m68k-mulu", 424),
    ("isa-m68k-nbcd", 50),
    ("isa-m68k-neg", 150),
    ("isa-m68k-negx", 150),
    ("isa-m68k-nop", 1),
    ("isa-m68k-not", 150),
    ("isa-m68k-or", 2_280),
    ("isa-m68k-ori", 152),
    ("isa-m68k-pea", 28),
    ("isa-m68k-reset", 1),
    ("isa-m68k-rol", 426),
    ("isa-m68k-ror", 426),
    ("isa-m68k-roxl", 426),
    ("isa-m68k-roxr", 426),
    ("isa-m68k-rte", 1),
    ("isa-m68k-rtr", 1),
    ("isa-m68k-rts", 1),
    ("isa-m68k-sbcd", 128),
    ("isa-m68k-scc", 800),
    ("isa-m68k-stop", 1),
    ("isa-m68k-sub", 2_408),
    ("isa-m68k-suba", 976),
    ("isa-m68k-subi", 150),
    ("isa-m68k-subq", 1_328),
    ("isa-m68k-subx", 384),
    ("isa-m68k-swap", 8),
    ("isa-m68k-tas", 50),
    ("isa-m68k-trap", 16),
    ("isa-m68k-trapv", 1),
    ("isa-m68k-tst", 150),
    ("isa-m68k-unlk", 8),
];

#[test]
fn valid_opwords_per_line_and_per_entry_match_an_independent_count() {
    assert_eq!(VALID_PER_LINE.iter().sum::<usize>(), VALID_OPWORDS);
    assert_eq!(
        VALID_PER_ENTRY
            .iter()
            .map(|&(_, count)| count)
            .sum::<usize>(),
        VALID_OPWORDS
    );
    let candidates = candidates();
    let mut per_line = [0_usize; 16];
    let mut per_entry: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for opword in 0..=u16::MAX {
        let found = matching(&candidates, opword);
        if let Some((entry, _, _, _)) = found.first() {
            per_line[usize::from(opword >> 12)] += 1;
            *per_entry.entry(entry.doc_id).or_default() += 1;
        }
    }
    for (line, (&actual, &expected)) in per_line.iter().zip(&VALID_PER_LINE).enumerate() {
        assert_eq!(actual, expected, "line {line:X}");
    }
    let expected: std::collections::BTreeMap<&str, usize> = VALID_PER_ENTRY.into_iter().collect();
    assert_eq!(
        expected.len(),
        INSTRUCTIONS.len(),
        "one row per table entry"
    );
    let mismatches: Vec<String> = INSTRUCTIONS
        .iter()
        .filter_map(|entry| {
            let actual = per_entry.get(entry.doc_id).copied().unwrap_or(0);
            let wanted = expected.get(entry.doc_id).copied();
            (wanted != Some(actual))
                .then(|| format!("{}: table {actual}, expected {wanted:?}", entry.doc_id))
        })
        .collect();
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}
