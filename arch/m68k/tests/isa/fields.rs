//! Tests for `src/isa/fields.rs` (m2-assembler.md §4.9): one row of places per form with its
//! operand count, places that agree with the operand kinds, the free opword bits partitioned
//! exactly by the register and data fields, and every field value giving an opword the form
//! matches.
#![allow(clippy::unwrap_used)]

use va_arch_m68k::isa::{
    EaField, Form, INSTRUCTIONS, Instruction, Operand, Place, QuickRange, Size, fields,
    instruction_of, places,
};

use crate::all_forms;

/// The opword bits a place occupies (none for the places that live in extension words only).
fn place_bits(place: Place) -> u16 {
    match place {
        Place::Reg9 | Place::Quick9 => 0x0E00,
        Place::Reg0 => 0x0007,
        Place::Data8 | Place::Disp8 => 0x00FF,
        Place::Vector => 0x000F,
        Place::Ea
        | Place::Disp16
        | Place::Imm
        | Place::ImmWord
        | Place::ImmByte
        | Place::RegMask
        | Place::Implied => 0,
    }
}

/// The values a place's opword field takes in a valid opword, and the shift of the field.
fn field_values(place: Place) -> Option<(std::ops::RangeInclusive<u16>, u32)> {
    match place {
        Place::Reg9 | Place::Quick9 => Some((0..=7, 9)),
        Place::Reg0 => Some((0..=7, 0)),
        Place::Data8 => Some((0..=255, 0)),
        // A zero byte announces the word displacement (the `.W` form).
        Place::Disp8 => Some((1..=255, 0)),
        Place::Vector => Some((0..=15, 0)),
        _ => None,
    }
}

/// The condition codes of the entry's family, or `[None]` for an entry without family.
fn conditions(entry: &Instruction) -> Vec<Option<u8>> {
    match entry.family {
        Some(family) => (family.first_condition()..16).map(Some).collect(),
        None => vec![None],
    }
}

/// The first opword the form matches with `condition`.
fn first_match(form: &Form, condition: Option<u8>) -> u16 {
    (0..=u16::MAX)
        .find(|&word| form.matches(word, condition))
        .unwrap()
}

#[test]
fn every_form_has_a_row_with_its_operand_count() {
    let mut forms = 0usize;
    for (entry, index, form) in all_forms() {
        let row = places(entry, index);
        assert_eq!(
            row.len(),
            form.operands.len(),
            "{} form {index}: places {row:?}",
            entry.name
        );
        forms += 1;
    }
    assert_eq!(
        forms,
        INSTRUCTIONS.iter().map(|e| e.forms.len()).sum::<usize>()
    );
    // Past the last form, and for a name outside the table, the row is empty.
    for entry in &INSTRUCTIONS {
        assert!(
            places(entry, entry.forms.len()).is_empty(),
            "{}",
            entry.name
        );
    }
    let unknown = Instruction {
        name: "NOSUCH",
        ..INSTRUCTIONS[0]
    };
    assert!(places(&unknown, 0).is_empty());
}

#[test]
fn the_row_is_found_by_name_also_for_a_copy_of_an_entry() {
    let (move_entry, _) = instruction_of("MOVE").unwrap();
    let copy = *move_entry;
    assert_eq!(places(&copy, 5), &[Place::Implied, Place::Reg0]);
    let lowercase = Instruction {
        name: "dbcc",
        ..*instruction_of("DBRA").unwrap().0
    };
    assert_eq!(places(&lowercase, 0), &[Place::Reg0, Place::Disp16]);
    // The re-export at `isa` and the module path name the same function.
    assert_eq!(fields::places(&copy, 0), places(&copy, 0));
}

#[test]
fn places_agree_with_the_operand_kinds() {
    for (entry, index, form) in all_forms() {
        let row = places(entry, index);
        let context = format!("{} form {index}", entry.name);
        for (operand, place) in form.operands.iter().zip(row) {
            let fits = match (operand, place) {
                (Operand::Ea(_), Place::Ea) => true,
                (
                    Operand::Dn
                    | Operand::An
                    | Operand::PostIncAn
                    | Operand::PreDecAn
                    | Operand::DispAn,
                    Place::Reg9 | Place::Reg0,
                ) => true,
                (Operand::Quick(QuickRange::OneToEight), Place::Quick9) => true,
                (Operand::Quick(QuickRange::Signed8), Place::Data8) => true,
                (Operand::Quick(QuickRange::Vector4), Place::Vector) => true,
                (Operand::Label, Place::Disp8) => form.sizes.contains(Size::Short),
                (Operand::Label, Place::Disp16) => !form.sizes.contains(Size::Short),
                (Operand::Imm, Place::Imm | Place::ImmWord | Place::ImmByte) => true,
                (Operand::RegList, Place::RegMask) => true,
                (Operand::Ccr | Operand::Sr | Operand::Usp, Place::Implied) => true,
                _ => false,
            };
            assert!(fits, "{context}: {operand:?} placed as {place:?}");
        }
        let ea_places = row.iter().filter(|&&p| p == Place::Ea).count();
        let ea_operands = form
            .operands
            .iter()
            .filter(|op| matches!(op, Operand::Ea(_)))
            .count();
        assert_eq!(ea_places, ea_operands, "{context}");
        let expected_fields = match form.encoding.ea {
            EaField::None => 0,
            EaField::Low | EaField::MoveDest => 1,
            EaField::Both => 2,
        };
        assert_eq!(ea_places, expected_fields, "{context}: EA field");
        assert!(
            row.iter().filter(|&&p| p == Place::RegMask).count() <= 1,
            "{context}"
        );
    }
}

#[test]
fn free_bits_are_exactly_the_register_and_data_fields() {
    for (entry, index, form) in all_forms() {
        let encoding = &form.encoding;
        let condition_bits = if entry.family.is_some() { 0x0F00 } else { 0 };
        let free = !encoding.mask & !encoding.size.bits() & !encoding.ea.bits() & !condition_bits;
        let mut claimed = 0u16;
        for &place in places(entry, index) {
            let bits = place_bits(place);
            assert_eq!(
                claimed & bits,
                0,
                "{} form {index}: {place:?} claims a bit twice",
                entry.name
            );
            claimed |= bits;
        }
        assert_eq!(
            free, claimed,
            "{} form {index}: free bits {free:04X}, claimed {claimed:04X}",
            entry.name
        );
        // The size, EA and condition fields never overlap the fixed bits.
        assert_eq!(
            encoding.mask & (encoding.size.bits() | encoding.ea.bits() | condition_bits),
            0,
            "{} form {index}",
            entry.name
        );
    }
}

#[test]
fn every_field_value_gives_an_opword_the_form_matches() {
    for (entry, index, form) in all_forms() {
        for condition in conditions(entry) {
            let base = first_match(form, condition);
            for &place in places(entry, index) {
                let Some((values, shift)) = field_values(place) else {
                    continue;
                };
                let bits = place_bits(place);
                for value in values {
                    let word = (base & !bits) | (value << shift);
                    assert!(
                        form.matches(word, condition),
                        "{} form {index} {condition:?}: {place:?} = {value} gives {word:04X}",
                        entry.name
                    );
                }
            }
        }
    }
}

/// The places of a few forms, pinned (operand order, word order exceptions, register fields).
#[test]
fn pinned_rows() {
    use Place::{
        Data8, Disp8, Disp16, Ea, Imm, ImmByte, ImmWord, Implied, Quick9, Reg0, Reg9, RegMask,
        Vector,
    };
    let rows: &[(&str, usize, &[Place])] = &[
        ("ADD", 0, &[Ea, Reg9]),
        ("ADD", 2, &[Reg9, Ea]),
        ("ADDI", 0, &[Imm, Ea]),
        ("ADDQ", 1, &[Quick9, Reg0]),
        ("ABCD", 1, &[Reg0, Reg9]),
        ("CMPM", 0, &[Reg0, Reg9]),
        ("ANDI", 1, &[ImmByte, Implied]),
        ("ORI", 2, &[ImmWord, Implied]),
        ("LSL", 0, &[Quick9, Reg0]),
        ("LSL", 1, &[Reg9, Reg0]),
        ("LSL", 2, &[Ea]),
        ("BTST", 0, &[Reg9, Reg0]),
        ("BTST", 3, &[ImmByte, Ea]),
        ("BRA", 0, &[Disp8]),
        ("BSR", 1, &[Disp16]),
        ("BEQ", 0, &[Disp8]),
        ("DBRA", 0, &[Reg0, Disp16]),
        ("EXG", 2, &[Reg9, Reg0]),
        ("LINK", 0, &[Reg0, ImmWord]),
        ("MOVE", 0, &[Ea, Ea]),
        ("MOVE", 4, &[Implied, Ea]),
        ("MOVE", 6, &[Reg0, Implied]),
        ("MOVEM", 0, &[RegMask, Ea]),
        ("MOVEM", 1, &[Ea, RegMask]),
        ("MOVEP", 0, &[Reg0, Reg9]),
        ("MOVEP", 1, &[Reg9, Reg0]),
        ("MOVEQ", 0, &[Data8, Reg9]),
        ("NOP", 0, &[]),
        ("STOP", 0, &[ImmWord]),
        ("TRAP", 0, &[Vector]),
        ("SEQ", 0, &[Ea]),
    ];
    for &(spelling, form, expected) in rows {
        let (entry, _) = instruction_of(spelling).unwrap();
        assert_eq!(places(entry, form), expected, "{spelling} form {form}");
    }
}

/// Reads the register fields of golden a68k opwords back through the places.
#[test]
fn golden_opwords_hold_their_registers_in_the_placed_fields() {
    // (opword, spelling, form, field values in operand order; `None` for a place without
    // opword bits).
    let cases: &[(u16, &str, usize, &[Option<u16>])] = &[
        // abcd -(a0),-(a1): source a0 in bits 2-0, destination a1 in bits 11-9.
        (0xC308, "ABCD", 1, &[Some(0), Some(1)]),
        // cmpm.b (a0)+,(a1)+
        (0xB308, "CMPM", 0, &[Some(0), Some(1)]),
        // addx.l d0,d1
        (0xD380, "ADDX", 0, &[Some(0), Some(1)]),
        // exg d0,d1 and exg d0,a0
        (0xC141, "EXG", 0, &[Some(0), Some(1)]),
        (0xC188, "EXG", 2, &[Some(0), Some(0)]),
        // moveq #127,d2
        (0x747F, "MOVEQ", 0, &[Some(0x7F), Some(2)]),
        // asl.w d1,d2: count register d1, destination d2
        (0xE362, "ASL", 1, &[Some(1), Some(2)]),
        // lsl.w #1,d0
        (0xE348, "LSL", 0, &[Some(1), Some(0)]),
        // btst d1,(a0)
        (0x0310, "BTST", 1, &[Some(1), None]),
        // trap #15, link a6,#…, dbf d0,…
        (0x4E4F, "TRAP", 0, &[Some(15)]),
        (0x4E56, "LINK", 0, &[Some(6), None]),
        (0x51C8, "DBF", 0, &[Some(0), None]),
        // bcc.s with displacement $D8
        (0x64D8, "BCC", 0, &[Some(0xD8)]),
        // movep.w d16(a0),d0
        (0x0108, "MOVEP", 0, &[Some(0), Some(0)]),
    ];
    for &(opword, spelling, form, values) in cases {
        let (entry, condition) = instruction_of(spelling).unwrap();
        let entry_form = &entry.forms[form];
        assert!(
            entry_form.matches(opword, condition.map(|c| c.code)),
            "{opword:04X} {spelling}"
        );
        let row = places(entry, form);
        assert_eq!(row.len(), values.len(), "{spelling}");
        for (&place, &value) in row.iter().zip(values) {
            let read = field_values(place).map(|(_, shift)| (opword & place_bits(place)) >> shift);
            assert_eq!(read, value, "{opword:04X} {spelling} {place:?}");
        }
    }
}
