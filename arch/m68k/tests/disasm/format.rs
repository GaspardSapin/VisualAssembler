//! Text of decoded instructions (`src/disasm/format.rs`; m2 §5.2, §10.1): spellings, sizes,
//! number notation, register lists, and `tokens` covering `text` (WP-E).
//!
//! The exhaustive check reassembles the text of every valid opword, with the extension words
//! of the §10.3 pattern, through the front end (`asm::parser`, `asm::operand`) and the encoder
//! (`asm::encode`, `optimize = false`): the words come back, except that the 120 immediate
//! opwords of §16.2 C8 come back as their immediate instruction. `tests/roundtrip.rs` repeats
//! it through the whole assembler.
#![allow(clippy::unwrap_used)]

use va_arch_m68k::asm::codes::ASM_ODD_BRANCH_TARGET;
use va_arch_m68k::asm::encode::{encode, select};
use va_arch_m68k::asm::expr::{Expr, Lookup, Scope, SymRef};
use va_arch_m68k::asm::operand::{Ea, Operand, parse_operand};
use va_arch_m68k::asm::parser::{OpField, parse_line};
use va_arch_m68k::isa::{
    CONDITIONS, EaField, Form, Instruction, Mode, Operand as Slot, Place, Size, SizeField,
    instruction_of, places,
};
use va_arch_m68k::syntax::lex_line;
use va_core::syntax::{Token, TokenKind};

use crate::decode::{ORIGIN, candidates, decode_words, matching, text};

/// No symbol is defined: the texts hold numbers only.
struct NoSymbols {
    here: i64,
}

impl Scope for NoSymbols {
    fn lookup(&self, _sym: &SymRef) -> Lookup {
        Lookup::Undefined
    }

    fn here(&self) -> i64 {
        self.here
    }
}

/// Assembles the one-instruction `text` at `address` through the front end and the encoder
/// (`optimize = false`, no symbol): its words, and whether the warning `asm-odd-branch-target`
/// came with them. A `dc.w` text gives its words.
pub fn reencode(text: &str, address: u64) -> Result<(Vec<u16>, bool), String> {
    let line = format!("\t{text}");
    let mut tokens = Vec::new();
    lex_line(&line, &mut tokens);
    let parsed = parse_line(&line, &tokens);
    if !parsed.problems.is_empty() {
        return Err(format!("{:?}", parsed.problems));
    }
    let field = parsed.operand_field;
    let field: Vec<Token> = tokens
        .iter()
        .filter(|token| {
            !field.is_empty() && token.range.start >= field.start && token.range.end <= field.end
        })
        .copied()
        .collect();
    let mut ops = Vec::new();
    let mut at = 0;
    while at < field.len() {
        let (op, used) = parse_operand(&line, &field[at..]).map_err(|p| format!("{p:?}"))?;
        ops.push(op);
        at += used;
        if let Some(comma) = field.get(at) {
            if comma.kind != TokenKind::Punctuation || comma.range.slice(&line) != Some(",") {
                return Err(format!("no comma at {comma:?}"));
            }
            at += 1;
        }
    }
    match parsed.op.map(|op| op.item) {
        Some(OpField::Mnemonic { spelling, size }) => {
            let choice = select(spelling, size, &ops).map_err(|p| format!("{p:?}"))?;
            let mut words = Vec::new();
            let scope = NoSymbols {
                here: address as i64,
            };
            match encode(&choice, &ops, &scope, address, &mut words) {
                Ok(()) => Ok((words, false)),
                Err(problem) if problem.code == ASM_ODD_BRANCH_TARGET => Ok((words, true)),
                Err(problem) => Err(format!("{problem:?}")),
            }
        }
        Some(OpField::Directive {
            name: "DC",
            size: Some(Size::Word),
        }) => ops
            .iter()
            .map(|op| match &op.item {
                Operand::Ea(Ea::Abs(Expr::Num(value), None)) => Ok((*value as u16, false)),
                other => Err(format!("{other:?}")),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|words| (words.into_iter().map(|(word, _)| word).collect(), false)),
        other => Err(format!("operation {other:?}")),
    }
}

/// The operation size of `form` for `opword` (its size field, else its only size).
fn size_of(form: &Form, opword: u16) -> Option<Size> {
    match form.encoding.size {
        SizeField::None if form.sizes.len() == 1 => form.sizes.iter().next(),
        SizeField::None => None,
        field => field.decode(opword),
    }
}

/// The addressing mode of the `Ea` operand at `position`, read from the opword's EA fields.
fn mode_at(form: &Form, opword: u16, position: usize) -> Option<Mode> {
    let eas: Vec<usize> = (0..form.operands.len())
        .filter(|&p| matches!(form.operands[p], Slot::Ea(_)))
        .collect();
    let low = Mode::from_fields(((opword >> 3) & 7) as u8, (opword & 7) as u8);
    let dest = Mode::from_fields(((opword >> 6) & 7) as u8, ((opword >> 9) & 7) as u8);
    match form.encoding.ea {
        EaField::None => None,
        EaField::Low => (eas.first() == Some(&position)).then_some(low?),
        EaField::MoveDest => (eas.last() == Some(&position)).then_some(dest?),
        EaField::Both if eas.first() == Some(&position) => low,
        EaField::Both => (eas.get(1) == Some(&position)).then_some(dest?),
    }
}

/// The words of `opword` (a form of `entry`) followed by the extension words of the fixed
/// pattern of m2 §10.3: d16 `$0102`, index word `D1.W` with displacement 2, immediates `.B`
/// `$0012`, `.W` `$1234`, `.L` `$12345678`, abs.W `$1234`, abs.L `$00012345`, branch word
/// `$0102`, MOVEM mask `$0102`, STOP, LINK and the SR forms `$2700`, the CCR forms and bit
/// numbers `$0012`.
pub fn pattern(entry: &Instruction, form_index: usize, form: &Form, opword: u16) -> Vec<u16> {
    let size = size_of(form, opword);
    let immediate: &[u16] = match size {
        Some(Size::Byte) => &[0x0012],
        Some(Size::Long) => &[0x1234, 0x5678],
        _ => &[0x1234],
    };
    let row = places(entry, form_index);
    let mut words = vec![opword];
    if row.contains(&Place::RegMask) {
        words.push(0x0102);
    }
    for (position, &place) in row.iter().enumerate() {
        let extension: &[u16] = match place {
            Place::Ea => match mode_at(form, opword, position) {
                Some(Mode::Disp | Mode::PcDisp) => &[0x0102],
                Some(Mode::Index | Mode::PcIndex) => &[0x1002],
                Some(Mode::AbsW) => &[0x1234],
                Some(Mode::AbsL) => &[0x0001, 0x2345],
                Some(Mode::Imm) => immediate,
                _ => &[],
            },
            Place::Reg9 | Place::Reg0 if form.operands[position] == Slot::DispAn => &[0x0102],
            Place::Disp16 => &[0x0102],
            Place::Imm => immediate,
            Place::ImmWord => &[0x2700],
            Place::ImmByte => &[0x0012],
            _ => &[],
        };
        words.extend_from_slice(extension);
    }
    words
}

/// For one of the 120 immediate opwords of m2 §16.2 C8 (ADD, SUB, CMP, AND, OR `<ea>,Dn` with
/// `#imm`), the opword of the immediate instruction the text selects: `I | size | n`.
pub fn immediate_counterpart(entry: &Instruction, form: &Form, opword: u16) -> Option<u16> {
    let base = match entry.name {
        "ADD" => 0x0600,
        "SUB" => 0x0400,
        "CMP" => 0x0C00,
        "AND" => 0x0200,
        "OR" => 0x0000,
        _ => return None,
    };
    let to_dn = matches!(form.operands, [Slot::Ea(_), Slot::Dn]);
    if !to_dn || mode_at(form, opword, 0) != Some(Mode::Imm) {
        return None;
    }
    let size = match size_of(form, opword)? {
        Size::Byte => 0x00,
        Size::Word => 0x40,
        Size::Long => 0x80,
        Size::Short => return None,
    };
    Some(base | size | ((opword >> 9) & 7))
}

#[test]
fn every_text_reassembles_to_its_words() {
    let candidates = candidates();
    let mut immediate = 0;
    let mut checked = 0;
    let mut failures = Vec::new();
    for opword in 0..=u16::MAX {
        let Some(&(entry, index, form, _)) = matching(&candidates, opword).first() else {
            continue;
        };
        let words = pattern(entry, index, form, opword);
        let insn = decode_words(&words, ORIGIN).unwrap();
        assert_eq!(insn.len as usize, 2 * words.len(), "{opword:04X}");
        let mut expected = words.clone();
        if let Some(first) = immediate_counterpart(entry, form, opword) {
            expected[0] = first;
            immediate += 1;
        }
        checked += 1;
        match reencode(&insn.text, ORIGIN) {
            Ok((got, _)) if got == expected => {}
            other => {
                if failures.len() < 20 {
                    failures.push(format!("{words:04X?} {:?}: {other:04X?}", insn.text));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    assert_eq!(checked, crate::decode::VALID_OPWORDS);
    assert_eq!(immediate, 120);
}

#[test]
fn the_120_immediate_opwords_print_as_what_they_are() {
    assert_eq!(text(&[0xD03C, 0x0012]), "add.b #$12,d0");
    assert_eq!(text(&[0xD27C, 0x1234]), "add.w #$1234,d1");
    assert_eq!(text(&[0x94BC, 0x1234, 0x5678]), "sub.l #$12345678,d2");
    assert_eq!(text(&[0xB63C, 0x00FF]), "cmp.b #$FF,d3");
    assert_eq!(text(&[0xC87C, 0x00FF]), "and.w #$00FF,d4");
    assert_eq!(text(&[0x8ABC, 0x0000, 0x0001]), "or.l #$00000001,d5");
    // They reassemble to their immediate instruction (same result, flags and length).
    let (words, _) = reencode("add.w #$1234,d1", ORIGIN).unwrap();
    assert_eq!(words, [0x0641, 0x1234]);
}

/// The 16 condition names, by code, lowercase.
const CONDITION_NAMES: [&str; 16] = [
    "t", "f", "hi", "ls", "cc", "cs", "ne", "eq", "vc", "vs", "pl", "mi", "ge", "lt", "gt", "le",
];

#[test]
fn families_write_every_condition_and_dbf_as_dbra() {
    for code in 0u16..16 {
        let name = CONDITION_NAMES[usize::from(code)];
        assert_eq!(CONDITIONS[usize::from(code)].name.to_lowercase(), name);
        let db = if code == 1 { "ra" } else { name };
        assert_eq!(
            text(&[0x50C8 | code << 8 | 3, 0xFFFE]),
            format!("db{db}.w d3,$00001000")
        );
        assert_eq!(
            text(&[0x50C0 | code << 8 | 0x10]),
            format!("s{name}.b (a0)")
        );
        if code >= 2 {
            assert_eq!(text(&[0x6010 | code << 8]), format!("b{name}.s $00001012"));
            assert_eq!(
                text(&[0x6000 | code << 8, 0x0010]),
                format!("b{name}.w $00001012")
            );
        }
    }
    assert_eq!(text(&[0x6010]), "bra.s $00001012");
    assert_eq!(text(&[0x6100, 0x0010]), "bsr.w $00001012");
    // ROXL and ROXR, never their aliases ROLX and RORX.
    assert_eq!(text(&[0xE311]), "roxl.b #1,d1");
    assert_eq!(text(&[0xE4D0]), "roxr.w (a0)");
}

#[test]
fn every_spelling_is_canonical_and_sized_exactly_when_the_form_is() {
    let candidates = candidates();
    for opword in 0..=u16::MAX {
        let Some(&(entry, index, form, condition)) = matching(&candidates, opword).first() else {
            continue;
        };
        let words = pattern(entry, index, form, opword);
        let insn = decode_words(&words, ORIGIN).unwrap();
        if insn.text.starts_with("dc.w") {
            continue;
        }
        let operation = insn.text.split(' ').next().unwrap();
        assert_eq!(operation, operation.to_lowercase(), "{opword:04X}");
        let (stem, suffix) = match operation.split_once('.') {
            Some((stem, suffix)) => (stem, Some(suffix)),
            None => (operation, None),
        };
        let (found, found_condition) = instruction_of(stem).unwrap();
        assert_eq!(found.name, entry.name, "{opword:04X} {operation}");
        assert_eq!(found_condition.map(|c| c.code), condition, "{opword:04X}");
        let canonical = va_arch_m68k::isa::spelling_info(stem).unwrap().canonical;
        let expected = if canonical == "DBF" {
            "DBRA"
        } else {
            canonical
        };
        assert_eq!(stem.to_uppercase(), expected, "{opword:04X}");
        match suffix {
            None => assert!(form.sizes.is_empty(), "{opword:04X} {operation}: no size"),
            Some(letter) => {
                let size = Size::from_letter(letter).unwrap();
                assert!(form.sizes.contains(size), "{opword:04X} {operation}");
                assert_eq!(
                    Some(size),
                    size_of(form, opword),
                    "{opword:04X} {operation}"
                );
            }
        }
    }
}

#[test]
fn numbers_follow_the_notation() {
    // Immediates in hexadecimal by size; the CCR forms on 2 digits, STOP and SR on 4.
    assert_eq!(text(&[0x0000, 0x00FF]), "ori.b #$FF,d0");
    assert_eq!(text(&[0x0440, 0x8000]), "subi.w #$8000,d0");
    assert_eq!(text(&[0x0C80, 0xFFFF, 0xFFFF]), "cmpi.l #$FFFFFFFF,d0");
    assert_eq!(text(&[0x003C, 0x0005]), "ori.b #$05,ccr");
    assert_eq!(text(&[0x0A7C, 0x0700]), "eori.w #$0700,sr");
    assert_eq!(text(&[0x4E72, 0x0000]), "stop #$0000");
    // Quick values, MOVEQ, vectors, bit numbers and LINK in signed decimal.
    assert_eq!(text(&[0x5E40]), "addq.w #7,d0");
    assert_eq!(text(&[0x5048]), "addq.w #8,a0");
    assert_eq!(text(&[0xE018]), "ror.b #8,d0");
    assert_eq!(text(&[0x7080]), "moveq.l #-128,d0");
    assert_eq!(text(&[0x70FF]), "moveq.l #-1,d0");
    assert_eq!(text(&[0x4E40]), "trap #0");
    assert_eq!(text(&[0x0880, 0x00FF]), "bclr.l #255,d0");
    assert_eq!(text(&[0x4E50, 0x7FFF]), "link.w a0,#32767");
    assert_eq!(text(&[0x4E50, 0x8000]), "link.w a0,#-32768");
    // Displacements in signed decimal.
    assert_eq!(text(&[0x3028, 0x8000]), "move.w -32768(a0),d0");
    assert_eq!(text(&[0x3028, 0x0000]), "move.w 0(a0),d0");
    assert_eq!(text(&[0x3030, 0x0080]), "move.w -128(a0,d0.w),d0");
    assert_eq!(text(&[0x3030, 0xF87F]), "move.w 127(a0,a7.l),d0");
    // Absolute short, sign-extended on 8 digits when bit 15 is set; absolute long on 8 digits.
    assert_eq!(text(&[0x3038, 0x7FFF]), "move.w ($7FFF).w,d0");
    assert_eq!(text(&[0x3038, 0x8000]), "move.w ($FFFF8000).w,d0");
    assert_eq!(text(&[0x3038, 0x0000]), "move.w ($0000).w,d0");
    assert_eq!(text(&[0x3039, 0x0000, 0x0400]), "move.w $00000400,d0");
    // Targets as absolute addresses.
    assert_eq!(text(&[0x303A, 0xFFFE]), "move.w $00001000(pc),d0");
    assert_eq!(text(&[0x303B, 0x0000]), "move.w $00001002(pc,d0.w),d0");
    assert_eq!(text(&[0x60FE]), "bra.s $00001000");
    // MOVEP's displacement goes with its address register.
    assert_eq!(text(&[0x01C8, 0xFFFC]), "movep.l d0,-4(a0)");
    assert_eq!(text(&[0x0349, 0x0010]), "movep.l 16(a1),d1");
}

#[test]
fn register_lists_use_canonical_ranges() {
    for (mask, list) in [
        (0x0001, "d0"),
        (0x0080, "d7"),
        (0x0100, "a0"),
        (0x00FF, "d0-d7"),
        (0xFF00, "a0-a7"),
        (0xFFFF, "d0-d7/a0-a7"),
        (0x7FFF, "d0-d7/a0-a6"),
        (0x0505, "d0/d2/a0/a2"),
        (0x8080, "d7/a7"),
        (0x0306, "d1-d2/a0-a1"),
        (0x00C0, "d6-d7"),
        (0x0180, "d7/a0"),
        (0x6D5A, "d1/d3-d4/d6/a0/a2-a3/a5-a6"),
    ] {
        assert_eq!(text(&[0x4C90, mask]), format!("movem.w (a0),{list}"));
        // -(An) stores the mask bit-reversed: the list reads the same.
        assert_eq!(
            text(&[0x48E0, mask.reverse_bits()]),
            format!("movem.l {list},-(a0)")
        );
    }
}

#[test]
fn tokens_are_the_lexer_tokens_of_the_text() {
    for words in [
        &[0x4E71][..],
        &[0x23FC, 0x1234, 0x5678, 0x0001, 0x2345][..],
        &[0x48E7, 0xFFFE][..],
        &[0x303B, 0xA8F0][..],
        &[0x4C90, 0x0000][..],
        &[0x4E56, 0xFFF0][..],
    ] {
        let insn = decode_words(words, ORIGIN).unwrap();
        let line = format!("\t{}", insn.text);
        let mut expected = Vec::new();
        lex_line(&line, &mut expected);
        assert_eq!(expected[0].kind, TokenKind::Whitespace);
        assert_eq!(expected[0].range.end, 1);
        assert_eq!(insn.tokens.len(), expected.len() - 1);
        for (token, lexed) in insn.tokens.iter().zip(&expected[1..]) {
            assert_eq!(token.kind, lexed.kind, "{}", insn.text);
            assert_eq!(token.range.start + 1, lexed.range.start);
            assert_eq!(token.range.end + 1, lexed.range.end);
        }
        let operation = if insn.text.starts_with("dc.") {
            TokenKind::Directive
        } else {
            TokenKind::Mnemonic
        };
        assert_eq!(insn.tokens[0].kind, operation, "{}", insn.text);
        assert!(
            insn.tokens
                .iter()
                .all(|t| !matches!(t.kind, TokenKind::Invalid(_) | TokenKind::Comment)),
            "{}: {:?}",
            insn.text,
            insn.tokens
        );
    }
}
