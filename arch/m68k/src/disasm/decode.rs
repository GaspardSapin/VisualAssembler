//! Decoding (m2-assembler.md §5.1, completed by §16.2): a lazily built 65,536-entry table that
//! maps each opword to its (entry, form, condition) through `Form::matches`, the extension
//! words read through `isa::fields::places` and the EA fields (the brief index word as the
//! 68000 reads it), and the flow kind and target of each instruction.
//!
//! **Table.** Built once, on first use, from every (entry, form, condition) of
//! `isa::INSTRUCTIONS` (a family form once per condition code of its family): each opword the
//! form matches gets that form. The M1 tests prove that no opword matches two forms, so the
//! table is exactly "some form matches"; [`form_of`] reads it.
//!
//! **Length.** The opword alone gives the length (§15.1 A3): the extension words of each
//! operand by its place and addressing mode, in the word order of `isa::fields` (the MOVEM
//! mask word first, then the operands in order). An opword that no form matches is
//! `Illegal { word }`, even when bytes are missing after it; a matching opword whose extension
//! words are cut is `Truncated { needed }`, `needed` being its full length; fewer than 2 bytes
//! are `Truncated { needed: 2 }`.
//!
//! **Arbitrary words.** The high byte of a `.B` immediate word (and of the CCR and bit-number
//! words) and bits 10–8 of a brief index word are ignored, as the 68000 ignores them, so the
//! text reassembles to the canonical words. A MOVEM mask of 0 decodes with its full length as
//! `dc.w` of all its words, `Sequential`, without target (§16.2). Decoding never panics.
//!
//! **Addresses.** Branch and PC-relative targets are computed as the 68000 computes them, on
//! its 32-bit program counter: the address of the opword + 2 (branches) or of the extension
//! word (PC-relative operands) plus the sign-extended displacement, modulo 2^32. A target
//! outside `$000000`–`$FFFFFF` is still decoded and printed; its text then does not reassemble.
//!
//! **Flow** (§5.1): `Branch` for Bcc and DBcc, `Jump` for BRA and JMP, `Call` for BSR and JSR
//! (returning to `address + len`), `Return` for RTS, RTE and RTR, `Trap` for TRAP, TRAPV and
//! ILLEGAL, else `Sequential`. **Target:** the branch target of Bcc, BRA, BSR and DBcc; the
//! target of every PC-relative operand (for `d8(PC,Xn)`, the address without the index); the
//! absolute address of JMP, JSR, LEA and PEA (an absolute short address sign-extended to 32
//! bits).

use std::sync::OnceLock;

use va_core::disasm::{DecodeError, DecodedInsn, FlowKind};

use super::format::{self, Arg, IndexReg};
use crate::isa::{
    EaField, Family, Form, INSTRUCTIONS, Instruction, Mode, Operand as Slot, Place, Size,
    SizeField, places,
};

/// The form an opword is an instance of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FormRef {
    /// The table entry.
    pub entry: &'static Instruction,
    /// The index of the form in `entry.forms`.
    pub form: usize,
    /// The condition code (opword bits 11–8) of a Bcc, DBcc or Scc opword.
    pub condition: Option<u8>,
}

impl FormRef {
    /// Returns the form, `entry.forms[form]` (`None` when `form` is out of range).
    pub fn form(&self) -> Option<&'static Form> {
        self.entry.forms.get(self.form)
    }
}

/// Returns the form `opword` is an instance of, with its entry and condition; `None` when no
/// form of the 68000 tables matches it (an illegal opword).
///
/// The 65,536-entry table behind it is built on the first call.
pub fn form_of(opword: u16) -> Option<FormRef> {
    let slot = *table().get(usize::from(opword))?;
    let entry = INSTRUCTIONS.get(usize::from(slot >> 4).checked_sub(1)?)?;
    Some(FormRef {
        entry,
        form: usize::from(slot & 0xF),
        condition: entry.family.map(|_| ((opword >> 8) & 0xF) as u8),
    })
}

/// One slot per opword: 0 when no form matches, else `(entry index + 1) << 4 | form index`.
static TABLE: OnceLock<Box<[u16]>> = OnceLock::new();

fn table() -> &'static [u16] {
    TABLE.get_or_init(build_table)
}

/// Fills the table from every (entry, form, condition), enumerating the opwords that agree
/// with the fixed bits (and the condition field) of each form.
fn build_table() -> Box<[u16]> {
    let mut table = vec![0u16; 0x1_0000].into_boxed_slice();
    for (entry_index, entry) in INSTRUCTIONS.iter().enumerate() {
        let (codes, condition_bits) = match entry.family {
            Some(family) => (family.first_condition()..16, 0x0F00),
            None => (0..1, 0),
        };
        for (form_index, form) in entry.forms.iter().enumerate() {
            let (Ok(entry_slot), Ok(form_slot)) =
                (u16::try_from(entry_index + 1), u16::try_from(form_index))
            else {
                continue;
            };
            if entry_slot >= 0x1000 || form_slot >= 0x10 {
                continue;
            }
            let slot = (entry_slot << 4) | form_slot;
            let free = !form.encoding.mask & !condition_bits;
            for code in codes.clone() {
                let condition = entry.family.map(|_| code);
                let fixed = form.encoding.opword | ((u16::from(code) << 8) & condition_bits);
                let mut subset: u16 = 0;
                loop {
                    let word = fixed | subset;
                    if form.matches(word, condition)
                        && let Some(cell) = table.get_mut(usize::from(word))
                    {
                        *cell = slot;
                    }
                    if subset == free {
                        break;
                    }
                    subset = subset.wrapping_sub(free) & free;
                }
            }
        }
    }
    table
}

/// Reads the big-endian word at byte `offset`.
fn word_at(bytes: &[u8], offset: u32) -> Option<u16> {
    let start = usize::try_from(offset).ok()?;
    let high = *bytes.get(start)?;
    let low = *bytes.get(start.checked_add(1)?)?;
    Some(u16::from_be_bytes([high, low]))
}

/// The operation size of `form` for `opword`: its size field, else its only size; `None` for
/// an unsized form.
fn operation_size(form: &Form, opword: u16) -> Option<Size> {
    match form.encoding.size {
        SizeField::None if form.sizes.len() == 1 => form.sizes.iter().next(),
        SizeField::None => None,
        field => field.decode(opword),
    }
}

/// Most operands of a 68000 form.
const MAX_OPERANDS: usize = 2;

/// The addressing mode and register of each `Ea` operand, by operand position.
fn ea_fields(form: &Form, opword: u16) -> [Option<(Mode, u8)>; MAX_OPERANDS] {
    let low = || {
        Mode::from_fields(((opword >> 3) & 7) as u8, (opword & 7) as u8)
            .map(|mode| (mode, (opword & 7) as u8))
    };
    let dest = || {
        Mode::from_fields(((opword >> 6) & 7) as u8, ((opword >> 9) & 7) as u8)
            .map(|mode| (mode, ((opword >> 9) & 7) as u8))
    };
    let positions: Vec<usize> = form
        .operands
        .iter()
        .enumerate()
        .filter(|(_, slot)| matches!(slot, Slot::Ea(_)))
        .map(|(position, _)| position)
        .collect();
    let mut fields = [None; MAX_OPERANDS];
    let mut set = |position: Option<&usize>, value: Option<(Mode, u8)>| {
        if let Some(field) = position.and_then(|&position| fields.get_mut(position)) {
            *field = value;
        }
    };
    match form.encoding.ea {
        EaField::None => {}
        EaField::Low => set(positions.first(), low()),
        EaField::MoveDest => set(positions.last(), dest()),
        EaField::Both => {
            set(positions.first(), low());
            set(positions.get(1), dest());
        }
    }
    fields
}

/// The bytes of extension words of an immediate of `size`.
fn immediate_bytes(size: Option<Size>) -> u32 {
    if size == Some(Size::Long) { 4 } else { 2 }
}

/// The bytes of extension words of the operand at `place` (see `isa::fields`).
fn extension_bytes(place: Place, slot: Slot, ea: Option<(Mode, u8)>, size: Option<Size>) -> u32 {
    match place {
        Place::Ea => match ea.map(|(mode, _)| mode) {
            Some(Mode::Disp | Mode::Index | Mode::AbsW | Mode::PcDisp | Mode::PcIndex) => 2,
            Some(Mode::AbsL) => 4,
            Some(Mode::Imm) => immediate_bytes(size),
            _ => 0,
        },
        Place::Reg9 | Place::Reg0 if slot == Slot::DispAn => 2,
        Place::Disp16 | Place::ImmWord | Place::ImmByte | Place::RegMask => 2,
        Place::Imm => immediate_bytes(size),
        _ => 0,
    }
}

/// One operand as the opword describes it.
#[derive(Clone, Copy)]
struct Field {
    place: Place,
    slot: Slot,
    ea: Option<(Mode, u8)>,
    /// Byte offset of its first extension word from the opword.
    offset: u32,
}

/// What the opword alone tells: the operation size, the operands and the length.
struct Layout {
    size: Option<Size>,
    fields: Vec<Field>,
    len: u32,
}

impl Layout {
    fn of(found: FormRef, form: &Form, opword: u16) -> Option<Layout> {
        let places = places(found.entry, found.form);
        if places.len() != form.operands.len() || places.len() > MAX_OPERANDS {
            return None;
        }
        let size = operation_size(form, opword);
        let eas = ea_fields(form, opword);
        let mut fields: Vec<Field> = places
            .iter()
            .zip(form.operands)
            .zip(eas)
            .map(|((&place, &slot), ea)| Field {
                place,
                slot,
                ea,
                offset: 0,
            })
            .collect();
        let mut len = 2;
        if let Some(mask) = fields
            .iter_mut()
            .find(|field| field.place == Place::RegMask)
        {
            mask.offset = len;
            len += 2;
        }
        for field in fields
            .iter_mut()
            .filter(|field| field.place != Place::RegMask)
        {
            field.offset = len;
            len += extension_bytes(field.place, field.slot, field.ea, size);
        }
        Some(Layout { size, fields, len })
    }
}

/// Decodes the instruction at the start of `bytes`, whose first byte is at `address` (§5.1).
pub(crate) fn decode(bytes: &[u8], address: u64) -> Result<DecodedInsn, DecodeError> {
    let opword = word_at(bytes, 0).ok_or(DecodeError::Truncated { needed: 2 })?;
    let illegal = DecodeError::Illegal { word: opword };
    let found = form_of(opword).ok_or(illegal)?;
    let form = found.form().ok_or(illegal)?;
    let layout = Layout::of(found, form, opword).ok_or(illegal)?;
    let truncated = DecodeError::Truncated { needed: layout.len };
    if usize::try_from(layout.len).map_or(true, |len| bytes.len() < len) {
        return Err(truncated);
    }
    let reader = Reader {
        bytes,
        address,
        opword,
        size: layout.size,
        entry: found.entry,
    };
    // MOVEM stores its mask bit-reversed when its EA is `-(An)`.
    let predecrement = layout
        .fields
        .iter()
        .any(|field| matches!(field.ea, Some((Mode::PreDec, _))));
    let mut args = Vec::with_capacity(layout.fields.len());
    let mut empty_mask = false;
    for field in &layout.fields {
        let arg = reader.arg(*field, predecrement).ok_or(truncated)?;
        empty_mask |= arg == Arg::List(0);
        args.push(arg);
    }
    let len = layout.len;
    let (text, flow, target) = if empty_mask {
        let words: Vec<u16> = (0..len / 2)
            .map(|index| word_at(bytes, 2 * index).unwrap_or(0))
            .collect();
        (format::words_text(&words), FlowKind::Sequential, None)
    } else {
        let spelling = format::spelling(found.entry, found.condition);
        let size = if form.sizes.is_empty() {
            None
        } else {
            layout.size
        };
        let text = format::instruction_text(&spelling, size, &args);
        (
            text,
            flow(found.entry, address, len),
            target(found.entry, &args),
        )
    };
    let tokens = format::tokens(&text);
    Ok(DecodedInsn {
        address,
        len,
        text,
        tokens,
        target,
        flow,
    })
}

/// Reads the operands of one instruction whose bytes are all present.
struct Reader<'a> {
    bytes: &'a [u8],
    address: u64,
    opword: u16,
    size: Option<Size>,
    entry: &'static Instruction,
}

impl Reader<'_> {
    fn word(&self, offset: u32) -> Option<u16> {
        word_at(self.bytes, offset)
    }

    /// The 32-bit address of the byte at `offset` from the opword (the 68000 program counter).
    fn pc(&self, offset: u32) -> u32 {
        (self.address.wrapping_add(u64::from(offset)) & 0xFFFF_FFFF) as u32
    }

    /// The operand of `field`; `predecrement` tells whether the instruction's EA is `-(An)`
    /// (MOVEM reverses its mask then). `None` if a word is missing.
    fn arg(&self, field: Field, predecrement: bool) -> Option<Arg> {
        let opword = self.opword;
        let reg9 = ((opword >> 9) & 7) as u8;
        let reg0 = (opword & 7) as u8;
        Some(match field.place {
            Place::Ea => self.ea(field)?,
            Place::Reg9 => self.register(field, reg9)?,
            Place::Reg0 => self.register(field, reg0)?,
            Place::Quick9 => Arg::Decimal(if reg9 == 0 { 8 } else { i32::from(reg9) }),
            Place::Data8 => Arg::Decimal(i32::from(opword as u8 as i8)),
            Place::Vector => Arg::Decimal(i32::from(opword & 0xF)),
            Place::Disp8 => Arg::Target(
                self.pc(2)
                    .wrapping_add_signed(i32::from(opword as u8 as i8)),
            ),
            Place::Disp16 => {
                let displacement = self.word(field.offset)? as i16;
                Arg::Target(self.pc(2).wrapping_add_signed(i32::from(displacement)))
            }
            Place::Imm => self.immediate(field.offset)?,
            Place::ImmWord => {
                let word = self.word(field.offset)?;
                if self.entry.name == "LINK" {
                    Arg::Decimal(i32::from(word as i16))
                } else {
                    Arg::Imm(u32::from(word), Size::Word)
                }
            }
            Place::ImmByte => {
                let byte = self.word(field.offset)? & 0xFF;
                if matches!(self.entry.name, "BTST" | "BCHG" | "BCLR" | "BSET") {
                    Arg::Decimal(i32::from(byte))
                } else {
                    Arg::Imm(u32::from(byte), Size::Byte)
                }
            }
            Place::RegMask => {
                let mask = self.word(field.offset)?;
                Arg::List(if predecrement {
                    mask.reverse_bits()
                } else {
                    mask
                })
            }
            Place::Implied => match field.slot {
                Slot::Ccr => Arg::Ccr,
                Slot::Sr => Arg::Sr,
                _ => Arg::Usp,
            },
        })
    }

    /// A register operand (bits 11–9 or 2–0) as its slot writes it.
    fn register(&self, field: Field, reg: u8) -> Option<Arg> {
        Some(match field.slot {
            Slot::An => Arg::An(reg),
            Slot::PostIncAn => Arg::PostInc(reg),
            Slot::PreDecAn => Arg::PreDec(reg),
            Slot::DispAn => Arg::Disp(self.word(field.offset)? as i16, reg),
            _ => Arg::Dn(reg),
        })
    }

    /// An immediate of the operation size at `offset` (a `.B` value is the low byte).
    fn immediate(&self, offset: u32) -> Option<Arg> {
        let first = self.word(offset)?;
        Some(match self.size {
            Some(Size::Byte) => Arg::Imm(u32::from(first & 0xFF), Size::Byte),
            Some(Size::Long) => {
                let second = self.word(offset + 2)?;
                Arg::Imm((u32::from(first) << 16) | u32::from(second), Size::Long)
            }
            _ => Arg::Imm(u32::from(first), Size::Word),
        })
    }

    /// An effective address with its extension words.
    fn ea(&self, field: Field) -> Option<Arg> {
        let (mode, reg) = field.ea?;
        let offset = field.offset;
        Some(match mode {
            Mode::Dn => Arg::Dn(reg),
            Mode::An => Arg::An(reg),
            Mode::Ind => Arg::Ind(reg),
            Mode::PostInc => Arg::PostInc(reg),
            Mode::PreDec => Arg::PreDec(reg),
            Mode::Disp => Arg::Disp(self.word(offset)? as i16, reg),
            Mode::Index => {
                let (displacement, index) = brief(self.word(offset)?);
                Arg::Index(displacement, reg, index)
            }
            Mode::AbsW => Arg::AbsW(self.word(offset)?),
            Mode::AbsL => {
                let high = self.word(offset)?;
                let low = self.word(offset + 2)?;
                Arg::AbsL((u32::from(high) << 16) | u32::from(low))
            }
            Mode::PcDisp => {
                let displacement = self.word(offset)? as i16;
                Arg::PcDisp(self.pc(offset).wrapping_add_signed(i32::from(displacement)))
            }
            Mode::PcIndex => {
                let (displacement, index) = brief(self.word(offset)?);
                Arg::PcIndex(
                    self.pc(offset).wrapping_add_signed(i32::from(displacement)),
                    index,
                )
            }
            Mode::Imm => self.immediate(offset)?,
        })
    }
}

/// Reads a brief extension word: its 8-bit displacement and index register (bits 10–8 are
/// ignored, as on the 68000).
fn brief(word: u16) -> (i8, IndexReg) {
    let reg = ((word >> 12) & 7) as u8 + if word & 0x8000 != 0 { 8 } else { 0 };
    (
        word as u8 as i8,
        IndexReg {
            reg,
            long: word & 0x0800 != 0,
        },
    )
}

/// The flow kind of an instruction of `entry` at `address`, `len` bytes long (§5.1).
fn flow(entry: &Instruction, address: u64, len: u32) -> FlowKind {
    match (entry.family, entry.name) {
        (Some(Family::Bcc | Family::DBcc), _) => FlowKind::Branch,
        (_, "BRA" | "JMP") => FlowKind::Jump,
        (_, "BSR" | "JSR") => FlowKind::Call {
            return_address: address.wrapping_add(u64::from(len)),
        },
        (_, "RTS" | "RTE" | "RTR") => FlowKind::Return,
        (_, "TRAP" | "TRAPV" | "ILLEGAL") => FlowKind::Trap,
        _ => FlowKind::Sequential,
    }
}

/// The target of an instruction of `entry` with `args` (§5.1): a branch target or a
/// PC-relative target, else the absolute address of JMP, JSR, LEA and PEA.
fn target(entry: &Instruction, args: &[Arg]) -> Option<u64> {
    let relative = args.iter().find_map(|arg| match *arg {
        Arg::Target(target) | Arg::PcDisp(target) | Arg::PcIndex(target, _) => Some(target),
        _ => None,
    });
    let absolute = || {
        if !matches!(entry.name, "JMP" | "JSR" | "LEA" | "PEA") {
            return None;
        }
        args.iter().find_map(|arg| match *arg {
            Arg::AbsW(word) => Some(word as i16 as i32 as u32),
            Arg::AbsL(address) => Some(address),
            _ => None,
        })
    };
    relative.or_else(absolute).map(u64::from)
}
