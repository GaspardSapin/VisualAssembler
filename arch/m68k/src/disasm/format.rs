//! The text of a decoded instruction (m2-assembler.md §5.2, completed by §16.2): lowercase
//! canonical spelling with its size, one space, the operands joined by `,`, numbers as §5.2
//! writes them, and the lexer's tokens of that text. The text reassembles to the same bytes
//! with `optimize = false`.
//!
//! **Spelling.** The canonical spelling of the entry (`ROXL`, never `ROLX`); a conditional
//! family writes its prefix and the condition name (`bhi`, `dbcs`, `st`), and DBF is written
//! `dbra`. The size suffix is written whenever the form is sized (`bra.s`, `bra.w`, `addq.w`,
//! `dbra.w`, `lea.l`, `moveq.l`) and never on an unsized form (`jmp`, `stop`, `trap`, `nop`).
//!
//! **Numbers.** Immediates `#$` and 2, 4 or 8 uppercase hexadecimal digits by operation size
//! (the CCR forms `#$XX`, STOP and the SR forms `#$XXXX`); MOVEQ, ADDQ/SUBQ, shift counts, TRAP
//! vectors, bit numbers and LINK's displacement in signed decimal (`#-16`); `d16` and `d8`
//! displacements in signed decimal (`-2(a6)`, `16(a0,d1.w)`); absolute long `$` and 8 digits;
//! absolute short `($1234).w`, with bit 15 set sign-extended on 8 digits (`($FFFF8000).w`);
//! branch and PC-relative targets as the absolute address, `$` and 8 digits (`bra.s
//! $00001000`, `$00001016(pc)`, `$00001016(pc,d2.l)`). Registers are `d0`–`d7` and `a0`–`a7`,
//! an index register always carries its size (`d1.w`, `a2.l`).
//!
//! **Register lists** in canonical ranges: the data registers, then the address registers,
//! each run of consecutive registers as `first-last` (`d0-d7/a0-a6`, `d1/d3-d4/a0`).
//!
//! **Exceptions to the round trip (§16.2).** The 120 opwords of ADD, SUB, CMP, AND and OR
//! `<ea>,Dn` with `#imm` are written as what they are (`add.w #$1234,d0`) and reassemble to the
//! immediate instruction (ADDI, SUBI, CMPI, ANDI, ORI), which has the same result, flags and
//! length. A target or absolute long address outside `$000000`–`$FFFFFF` is written as it is
//! and does not reassemble.
//!
//! **Words.** A MOVEM with an empty mask has no A68k spelling and is written as its words
//! (`dc.w $4C90,$0000`), which reassemble to the same bytes (§16.2).
//!
//! **Tokens** are the lexer's tokens of `"\t" + text`, shifted left by one, without the leading
//! whitespace token, so they cover `text` exactly.

use std::fmt::Write as _;

use va_core::source::TextRange;
use va_core::syntax::Token;

use crate::isa::{CONDITIONS, Instruction, Size};
use crate::syntax::lex_line;

/// An index register of `d8(An,Xn)` or `d8(PC,Xn)`, as the brief extension word gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct IndexReg {
    /// The register: 0–7 are D0–D7, 8–15 are A0–A7.
    pub(crate) reg: u8,
    /// Whether the index is `.L` (else `.W`).
    pub(crate) long: bool,
}

/// One decoded operand, ready to be written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Arg {
    /// `dn`.
    Dn(u8),
    /// `an`.
    An(u8),
    /// `(an)`.
    Ind(u8),
    /// `(an)+`.
    PostInc(u8),
    /// `-(an)`.
    PreDec(u8),
    /// `d16(an)`: the displacement and the register.
    Disp(i16, u8),
    /// `d8(an,xn)`: the displacement, the address register and the index.
    Index(i8, u8, IndexReg),
    /// `(xxxx).w`: the extension word as read (sign-extended when written).
    AbsW(u16),
    /// `$xxxxxxxx`: the 32-bit address.
    AbsL(u32),
    /// `$xxxxxxxx(pc)`: the target address.
    PcDisp(u32),
    /// `$xxxxxxxx(pc,xn)`: the target address of the displacement (index not added) and the
    /// index.
    PcIndex(u32, IndexReg),
    /// `#$xx`, `#$xxxx` or `#$xxxxxxxx` by size: the value (already cut to the size).
    Imm(u32, Size),
    /// `#n` in signed decimal: quick values, MOVEQ, TRAP vectors, bit numbers, LINK.
    Decimal(i32),
    /// A branch target, `$xxxxxxxx`.
    Target(u32),
    /// A register list in canonical order (bit n is Dn, bit 8 + n is An), never empty.
    List(u16),
    /// `ccr`.
    Ccr,
    /// `sr`.
    Sr,
    /// `usp`.
    Usp,
}

/// Returns the lowercase canonical spelling of `entry` with condition `condition` (a family
/// entry needs one; DBF is written `dbra`).
pub(crate) fn spelling(entry: &Instruction, condition: Option<u8>) -> String {
    let name = match (entry.family, condition) {
        (Some(family), Some(code)) => {
            let condition = CONDITIONS
                .get(usize::from(code & 0xF))
                .map_or("", |c| c.name);
            if family.prefix() == "DB" && condition == "F" {
                "DBRA".to_owned()
            } else {
                format!("{}{condition}", family.prefix())
            }
        }
        _ => entry.name.to_owned(),
    };
    name.to_ascii_lowercase()
}

/// Returns the text of an instruction: `spelling`, its size suffix when `size` is given, and
/// the operands after one space, joined by `,`.
pub(crate) fn instruction_text(spelling: &str, size: Option<Size>, args: &[Arg]) -> String {
    let mut text = String::with_capacity(32);
    text.push_str(spelling);
    if let Some(size) = size {
        text.push('.');
        text.push_str(&size.letter().to_ascii_lowercase());
    }
    for (index, arg) in args.iter().enumerate() {
        text.push(if index == 0 { ' ' } else { ',' });
        write_arg(&mut text, *arg);
    }
    text
}

/// Returns `dc.w` and the words in uppercase hexadecimal, joined by `,` (`dc.w $4C90,$0000`).
pub(crate) fn words_text(words: &[u16]) -> String {
    let mut text = String::from("dc.w");
    for (index, word) in words.iter().enumerate() {
        text.push(if index == 0 { ' ' } else { ',' });
        let _ = write!(text, "${word:04X}");
    }
    text
}

/// Returns the lexer's tokens of `text`: those of `"\t" + text` without the leading whitespace
/// token, shifted left by one byte.
pub(crate) fn tokens(text: &str) -> Vec<Token> {
    let mut line = String::with_capacity(text.len() + 1);
    line.push('\t');
    line.push_str(text);
    let mut tokens = Vec::with_capacity(8);
    lex_line(&line, &mut tokens);
    tokens
        .into_iter()
        .filter(|token| token.range.start > 0)
        .map(|token| Token {
            kind: token.kind,
            range: TextRange::new(token.range.start - 1, token.range.end - 1),
        })
        .collect()
}

/// Writes one operand.
fn write_arg(out: &mut String, arg: Arg) {
    let _ = match arg {
        Arg::Dn(reg) => write!(out, "d{reg}"),
        Arg::An(reg) => write!(out, "a{reg}"),
        Arg::Ind(reg) => write!(out, "(a{reg})"),
        Arg::PostInc(reg) => write!(out, "(a{reg})+"),
        Arg::PreDec(reg) => write!(out, "-(a{reg})"),
        Arg::Disp(displacement, reg) => write!(out, "{displacement}(a{reg})"),
        Arg::Index(displacement, reg, index) => {
            write!(out, "{displacement}(a{reg},{})", index_text(index))
        }
        Arg::AbsW(word) if word < 0x8000 => write!(out, "(${word:04X}).w"),
        Arg::AbsW(word) => write!(out, "(${:08X}).w", 0xFFFF_0000 | u32::from(word)),
        Arg::AbsL(address) | Arg::Target(address) => write!(out, "${address:08X}"),
        Arg::PcDisp(target) => write!(out, "${target:08X}(pc)"),
        Arg::PcIndex(target, index) => write!(out, "${target:08X}(pc,{})", index_text(index)),
        Arg::Imm(value, Size::Byte) => write!(out, "#${:02X}", value & 0xFF),
        Arg::Imm(value, Size::Long) => write!(out, "#${value:08X}"),
        Arg::Imm(value, Size::Word | Size::Short) => write!(out, "#${:04X}", value & 0xFFFF),
        Arg::Decimal(value) => write!(out, "#{value}"),
        Arg::List(mask) => {
            write_list(out, mask);
            Ok(())
        }
        Arg::Ccr => write!(out, "ccr"),
        Arg::Sr => write!(out, "sr"),
        Arg::Usp => write!(out, "usp"),
    };
}

/// Returns an index register with its size, `d1.w` or `a2.l`.
fn index_text(index: IndexReg) -> String {
    let kind = if index.reg >= 8 { 'a' } else { 'd' };
    let size = if index.long { 'l' } else { 'w' };
    format!("{kind}{}.{size}", index.reg & 7)
}

/// Writes a register list in canonical ranges: the data registers then the address registers,
/// each run of two or more consecutive registers as `first-last`, items joined by `/`.
fn write_list(out: &mut String, mask: u16) {
    let mut first = true;
    for (kind, bits) in [('d', mask & 0xFF), ('a', mask >> 8)] {
        let mut reg = 0u16;
        while reg < 8 {
            if bits & (1 << reg) == 0 {
                reg += 1;
                continue;
            }
            let start = reg;
            while reg + 1 < 8 && bits & (1 << (reg + 1)) != 0 {
                reg += 1;
            }
            if !first {
                out.push('/');
            }
            first = false;
            let _ = if reg == start {
                write!(out, "{kind}{start}")
            } else {
                write!(out, "{kind}{start}-{kind}{reg}")
            };
            reg += 1;
        }
    }
}
