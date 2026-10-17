//! Opword encodings of the forms (m1-core-isa-lexer.md §4.5).
//!
//! Each form states which opword bits are fixed (`mask`, with their values in `opword`), where
//! its size and effective-address fields sit, and one extra rule if any. Every other bit is a
//! free register, data or condition field. [`Form::matches`] decides whether a 16-bit word is
//! an instance of the form; the consistency tests run it over all 65,536 words to prove that
//! no two forms claim the same word. M2's encoder and disassembler start from the same facts.

use super::model::{Form, Operand, Size};
use super::modes::{Mode, ModeSet};

/// The opword facts of one form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Encoding {
    /// The values of the fixed bits; every bit outside `mask` is zero.
    pub opword: u16,
    /// The fixed bits.
    pub mask: u16,
    /// Where the operation size is encoded.
    pub size: SizeField,
    /// Where the effective-address operands are encoded.
    pub ea: EaField,
    /// An extra condition on the free bits.
    pub rule: FieldRule,
}

impl Encoding {
    /// Returns an encoding without extra rule.
    pub(super) const fn new(opword: u16, mask: u16, size: SizeField, ea: EaField) -> Encoding {
        Encoding {
            opword,
            mask,
            size,
            ea,
            rule: FieldRule::None,
        }
    }

    /// Returns this encoding with `rule`.
    pub(super) const fn with_rule(self, rule: FieldRule) -> Encoding {
        Encoding { rule, ..self }
    }
}

/// Where a form encodes its operation size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SizeField {
    /// No size field: the form has one size, or none.
    None,
    /// Bits 7–6: `00` byte, `01` word, `10` long (`11` is another instruction).
    Bits7_6,
    /// Bits 13–12 of MOVE and MOVEA: `01` byte, `11` word, `10` long.
    Move13_12,
    /// Bit 8 of ADDA, SUBA and CMPA: `0` word, `1` long.
    Bit8,
    /// Bit 6 of MOVEM and MOVEP: `0` word, `1` long.
    Bit6,
    /// Bits 8–6 of EXT: `010` word, `011` long.
    Ext8_6,
}

impl SizeField {
    /// Returns the bits the field occupies (zero for [`SizeField::None`]).
    pub const fn bits(self) -> u16 {
        match self {
            SizeField::None => 0,
            SizeField::Bits7_6 => 0x00C0,
            SizeField::Move13_12 => 0x3000,
            SizeField::Bit8 => 0x0100,
            SizeField::Bit6 => 0x0040,
            SizeField::Ext8_6 => 0x01C0,
        }
    }

    /// Decodes the size held by the field in `opword`.
    ///
    /// Returns `None` for [`SizeField::None`] and for a value that encodes no size.
    pub const fn decode(self, opword: u16) -> Option<Size> {
        match self {
            SizeField::None => None,
            SizeField::Bits7_6 => match (opword >> 6) & 3 {
                0 => Some(Size::Byte),
                1 => Some(Size::Word),
                2 => Some(Size::Long),
                _ => None,
            },
            SizeField::Move13_12 => match (opword >> 12) & 3 {
                1 => Some(Size::Byte),
                3 => Some(Size::Word),
                2 => Some(Size::Long),
                _ => None,
            },
            SizeField::Bit8 => Some(if opword & 0x0100 == 0 {
                Size::Word
            } else {
                Size::Long
            }),
            SizeField::Bit6 => Some(if opword & 0x0040 == 0 {
                Size::Word
            } else {
                Size::Long
            }),
            SizeField::Ext8_6 => match (opword >> 6) & 7 {
                2 => Some(Size::Word),
                3 => Some(Size::Long),
                _ => None,
            },
        }
    }
}

/// Where a form encodes its effective-address operands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EaField {
    /// No effective-address field.
    None,
    /// Bits 5–0 (mode 5–3, register 2–0) hold the form's first `Ea` operand.
    Low,
    /// Bits 11–6 (register 11–9, mode 8–6: swapped, as in a MOVE destination) hold the form's
    /// last `Ea` operand.
    MoveDest,
    /// MOVE: bits 5–0 hold the first `Ea` operand and bits 11–6 the second.
    Both,
}

impl EaField {
    /// Returns the bits the field occupies (zero for [`EaField::None`]).
    pub const fn bits(self) -> u16 {
        match self {
            EaField::None => 0,
            EaField::Low => 0x003F,
            EaField::MoveDest => 0x0FC0,
            EaField::Both => 0x0FFF,
        }
    }
}

/// An extra condition on the free bits of an opword.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldRule {
    /// No extra condition.
    None,
    /// The low byte (the 8-bit branch displacement) is not zero: `Bcc.S`, `BRA.S`, `BSR.S`.
    /// A zero byte announces a 16-bit displacement, which is the `.W` form.
    Disp8NonZero,
}

/// Decodes the effective-address field at bits 5–0.
const fn low_mode(opword: u16) -> Option<Mode> {
    Mode::from_fields(((opword >> 3) & 7) as u8, (opword & 7) as u8)
}

/// Decodes the effective-address field at bits 11–6 (register 11–9, mode 8–6).
const fn move_dest_mode(opword: u16) -> Option<Mode> {
    Mode::from_fields(((opword >> 6) & 7) as u8, ((opword >> 9) & 7) as u8)
}

fn decodes_into(mode: Option<Mode>, set: Option<ModeSet>) -> bool {
    match (mode, set) {
        (Some(mode), Some(set)) => set.contains(mode),
        _ => false,
    }
}

impl Form {
    /// Returns whether `opword` is an instance of this form.
    ///
    /// True when the fixed bits equal [`Encoding::opword`], the size field holds a size of
    /// [`Form::sizes`], every effective-address field decodes to a mode of its operand's
    /// [`ModeSet`] (mode 7 accepts registers 0–4 only), and the [`FieldRule`] holds. Free
    /// register and data fields accept any value. `condition`, when given, must also equal
    /// bits 11–8: the condition field of the Bcc, DBcc and Scc forms, whose mask leaves those
    /// bits free. Pass `None` for the forms of the other entries.
    pub fn matches(&self, opword: u16, condition: Option<u8>) -> bool {
        let encoding = &self.encoding;
        if opword & encoding.mask != encoding.opword {
            return false;
        }
        if let Some(code) = condition
            && (opword >> 8) & 0xF != u16::from(code)
        {
            return false;
        }
        let size_holds = match encoding.size {
            SizeField::None => true,
            field => field
                .decode(opword)
                .is_some_and(|size| self.sizes.contains(size)),
        };
        if !size_holds {
            return false;
        }
        let mut ea_sets = self.operands.iter().filter_map(|operand| match operand {
            Operand::Ea(set) => Some(*set),
            _ => None,
        });
        let ea_holds = match encoding.ea {
            EaField::None => true,
            EaField::Low => decodes_into(low_mode(opword), ea_sets.next()),
            EaField::MoveDest => decodes_into(move_dest_mode(opword), ea_sets.next_back()),
            EaField::Both => {
                let source = ea_sets.next();
                let destination = ea_sets.next();
                decodes_into(low_mode(opword), source)
                    && decodes_into(move_dest_mode(opword), destination)
            }
        };
        if !ea_holds {
            return false;
        }
        match encoding.rule {
            FieldRule::None => true,
            FieldRule::Disp8NonZero => opword & 0x00FF != 0,
        }
    }
}
