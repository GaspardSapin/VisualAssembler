//! 68000 instruction tables: model, addressing modes, conditions, encodings, cycle counts, and
//! the CPU-agnostic view (m1-core-isa-lexer.md §4).
//!
//! The tables are hand-written `const` data (§4.2), checked by `tests/isa/`. This module holds
//! the catalogue: the 74 entries ([`INSTRUCTIONS`]), the 126 accepted spellings
//! ([`SPELLINGS`], the opcode table of A68k 2.71.F3w) and their resolution
//! ([`spelling_info`], [`instruction_of`]). Each spelling resolves at compile time: a spelling
//! that names no entry stops the build.
//!
//! Factual sources (§4.6; numbers, bit patterns and names only, every description in our own
//! words): Motorola *M68000 Family Programmer's Reference Manual* (M68000PM/AD rev. 1, 1992) for
//! the instruction list, sizes, addressing categories, condition codes, flag effects and opword
//! bit patterns; Motorola *MC68000 8-/16-/32-Bit Microprocessors User's Manual* (M68000UM/AD
//! rev. 8) §8, tables 8-1 to 8-14, for clock periods and bus cycles; J. Cwik, *68000 Yacht*,
//! to cross-check timing footnotes only; `a68k.exe` 2.71.F3w run locally (read-only) for the
//! spellings, aliases, size defaults and golden opwords.

mod conditions;
mod encoding;
mod model;
mod modes;
mod table_arith;
mod table_flow;
mod table_logic;
mod table_move;
mod timing;
mod view;

pub use conditions::{CONDITIONS, Condition};
pub use encoding::{EaField, Encoding, FieldRule, SizeField};
pub use model::{
    Family, Flags, Form, Instruction, Operand, QuickRange, Size, SizeSet, SpellingInfo,
};
pub use modes::{Mode, ModeSet};
pub use timing::{
    EA_TIME, MOVE_BW, MOVE_L, Outcome, Timing, TimingRow, Unit, cycles, ea_time, move_column,
};
pub use view::M68kIsa;

use conditions::CONDITION_TABLE;
use table_arith as arith;
use table_flow as flow;
use table_logic as logic;
use table_move as moves;

const fn plain(name: &'static str, doc_id: &'static str, forms: &'static [Form]) -> Instruction {
    Instruction {
        name,
        family: None,
        doc_id,
        forms,
        also_accepts: &[],
    }
}

const fn borrowing(
    name: &'static str,
    doc_id: &'static str,
    forms: &'static [Form],
    also_accepts: &'static [&'static str],
) -> Instruction {
    Instruction {
        name,
        family: None,
        doc_id,
        forms,
        also_accepts,
    }
}

const fn family(family: Family, doc_id: &'static str, forms: &'static [Form]) -> Instruction {
    Instruction {
        name: family.name(),
        family: Some(family),
        doc_id,
        forms,
        also_accepts: &[],
    }
}

/// The 74 entries, in ASCII case-insensitive order of their names (read in constant evaluation).
const TABLE: [Instruction; 74] = [
    plain("ABCD", "isa-m68k-abcd", arith::ABCD),
    borrowing("ADD", "isa-m68k-add", arith::ADD, &["ADDA", "ADDI"]),
    plain("ADDA", "isa-m68k-adda", arith::ADDA),
    plain("ADDI", "isa-m68k-addi", arith::ADDI),
    plain("ADDQ", "isa-m68k-addq", arith::ADDQ),
    plain("ADDX", "isa-m68k-addx", arith::ADDX),
    borrowing("AND", "isa-m68k-and", logic::AND, &["ANDI"]),
    plain("ANDI", "isa-m68k-andi", logic::ANDI),
    plain("ASL", "isa-m68k-asl", logic::ASL),
    plain("ASR", "isa-m68k-asr", logic::ASR),
    family(Family::Bcc, "isa-m68k-bcc", flow::BCC),
    plain("BCHG", "isa-m68k-bchg", logic::BCHG),
    plain("BCLR", "isa-m68k-bclr", logic::BCLR),
    plain("BRA", "isa-m68k-bra", flow::BRA),
    plain("BSET", "isa-m68k-bset", logic::BSET),
    plain("BSR", "isa-m68k-bsr", flow::BSR),
    plain("BTST", "isa-m68k-btst", logic::BTST),
    plain("CHK", "isa-m68k-chk", arith::CHK),
    plain("CLR", "isa-m68k-clr", arith::CLR),
    borrowing("CMP", "isa-m68k-cmp", arith::CMP, &["CMPA", "CMPI", "CMPM"]),
    plain("CMPA", "isa-m68k-cmpa", arith::CMPA),
    plain("CMPI", "isa-m68k-cmpi", arith::CMPI),
    plain("CMPM", "isa-m68k-cmpm", arith::CMPM),
    family(Family::DBcc, "isa-m68k-dbcc", flow::DBCC),
    plain("DIVS", "isa-m68k-divs", arith::DIVS),
    plain("DIVU", "isa-m68k-divu", arith::DIVU),
    borrowing("EOR", "isa-m68k-eor", logic::EOR, &["EORI"]),
    plain("EORI", "isa-m68k-eori", logic::EORI),
    plain("EXG", "isa-m68k-exg", moves::EXG),
    plain("EXT", "isa-m68k-ext", arith::EXT),
    plain("ILLEGAL", "isa-m68k-illegal", flow::ILLEGAL),
    plain("JMP", "isa-m68k-jmp", flow::JMP),
    plain("JSR", "isa-m68k-jsr", flow::JSR),
    plain("LEA", "isa-m68k-lea", moves::LEA),
    plain("LINK", "isa-m68k-link", moves::LINK),
    plain("LSL", "isa-m68k-lsl", logic::LSL),
    plain("LSR", "isa-m68k-lsr", logic::LSR),
    borrowing("MOVE", "isa-m68k-move", moves::MOVE, &["MOVEA"]),
    plain("MOVEA", "isa-m68k-movea", moves::MOVEA),
    plain("MOVEM", "isa-m68k-movem", moves::MOVEM),
    plain("MOVEP", "isa-m68k-movep", moves::MOVEP),
    plain("MOVEQ", "isa-m68k-moveq", moves::MOVEQ),
    plain("MULS", "isa-m68k-muls", arith::MULS),
    plain("MULU", "isa-m68k-mulu", arith::MULU),
    plain("NBCD", "isa-m68k-nbcd", arith::NBCD),
    plain("NEG", "isa-m68k-neg", arith::NEG),
    plain("NEGX", "isa-m68k-negx", arith::NEGX),
    plain("NOP", "isa-m68k-nop", flow::NOP),
    plain("NOT", "isa-m68k-not", logic::NOT),
    borrowing("OR", "isa-m68k-or", logic::OR, &["ORI"]),
    plain("ORI", "isa-m68k-ori", logic::ORI),
    plain("PEA", "isa-m68k-pea", moves::PEA),
    plain("RESET", "isa-m68k-reset", flow::RESET),
    plain("ROL", "isa-m68k-rol", logic::ROL),
    plain("ROR", "isa-m68k-ror", logic::ROR),
    plain("ROXL", "isa-m68k-roxl", logic::ROXL),
    plain("ROXR", "isa-m68k-roxr", logic::ROXR),
    plain("RTE", "isa-m68k-rte", flow::RTE),
    plain("RTR", "isa-m68k-rtr", flow::RTR),
    plain("RTS", "isa-m68k-rts", flow::RTS),
    plain("SBCD", "isa-m68k-sbcd", arith::SBCD),
    family(Family::Scc, "isa-m68k-scc", logic::SCC),
    plain("STOP", "isa-m68k-stop", flow::STOP),
    borrowing("SUB", "isa-m68k-sub", arith::SUB, &["SUBA", "SUBI"]),
    plain("SUBA", "isa-m68k-suba", arith::SUBA),
    plain("SUBI", "isa-m68k-subi", arith::SUBI),
    plain("SUBQ", "isa-m68k-subq", arith::SUBQ),
    plain("SUBX", "isa-m68k-subx", arith::SUBX),
    plain("SWAP", "isa-m68k-swap", moves::SWAP),
    plain("TAS", "isa-m68k-tas", logic::TAS),
    plain("TRAP", "isa-m68k-trap", flow::TRAP),
    plain("TRAPV", "isa-m68k-trapv", flow::TRAPV),
    plain("TST", "isa-m68k-tst", arith::TST),
    plain("UNLK", "isa-m68k-unlk", moves::UNLK),
];

/// The 74 table entries (one documentation id each), in ASCII case-insensitive name order.
pub static INSTRUCTIONS: [Instruction; 74] = TABLE;

/// The spellings, read in constant evaluation.
#[rustfmt::skip]
const SPELLING_TABLE: [&str; 126] = [
    "ABCD", "ADD", "ADDA", "ADDI", "ADDQ", "ADDX", "AND", "ANDI", "ASL", "ASR",
    "BCC", "BCHG", "BCLR", "BCS", "BEQ", "BGE", "BGT", "BHI", "BHS", "BLE",
    "BLO", "BLS", "BLT", "BMI", "BNE", "BPL", "BRA", "BSET", "BSR", "BTST",
    "BVC", "BVS", "CHK", "CLR", "CMP", "CMPA", "CMPI", "CMPM", "DBCC", "DBCS",
    "DBEQ", "DBF", "DBGE", "DBGT", "DBHI", "DBHS", "DBLE", "DBLO", "DBLS", "DBLT",
    "DBMI", "DBNE", "DBPL", "DBRA", "DBT", "DBVC", "DBVS", "DIVS", "DIVU", "EOR",
    "EORI", "EXG", "EXT", "ILLEGAL", "JMP", "JSR", "LEA", "LINK", "LSL", "LSR",
    "MOVE", "MOVEA", "MOVEM", "MOVEP", "MOVEQ", "MULS", "MULU", "NBCD", "NEG", "NEGX",
    "NOP", "NOT", "OR", "ORI", "PEA", "RESET", "ROL", "ROLX", "ROR", "RORX",
    "ROXL", "ROXR", "RTE", "RTR", "RTS", "SBCD", "SCC", "SCS", "SEQ", "SF",
    "SGE", "SGT", "SHI", "SHS", "SLE", "SLO", "SLS", "SLT", "SMI", "SNE",
    "SPL", "ST", "STOP", "SUB", "SUBA", "SUBI", "SUBQ", "SUBX", "SVC", "SVS",
    "SWAP", "TAS", "TRAP", "TRAPV", "TST", "UNLK",
];

/// Every accepted spelling, uppercase ASCII, sorted: the opcode table of A68k 2.71.F3w (§4.1).
pub static SPELLINGS: [&str; 126] = SPELLING_TABLE;

/// The alias spellings and the canonical spelling each one stands for (§4.1).
const ALIASES: [(&str, &str); 9] = [
    ("BHS", "BCC"),
    ("BLO", "BCS"),
    ("DBHS", "DBCC"),
    ("DBLO", "DBCS"),
    ("DBRA", "DBF"),
    ("ROLX", "ROXL"),
    ("RORX", "ROXR"),
    ("SHS", "SCC"),
    ("SLO", "SCS"),
];

/// The entries that take no operand: whatever follows them is a comment (§5.2).
const NO_OPERANDS: [&str; 7] = ["ILLEGAL", "NOP", "RESET", "RTE", "RTR", "RTS", "TRAPV"];

/// How one spelling resolves: indexes into [`TABLE`] and [`CONDITION_TABLE`].
#[derive(Clone, Copy)]
struct Resolution {
    canonical: &'static str,
    entry: usize,
    condition: Option<usize>,
    takes_operands: bool,
}

const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Returns whether `text` is `prefix` immediately followed by `rest`.
const fn is_concat(text: &str, prefix: &str, rest: &str) -> bool {
    let (text, prefix, rest) = (text.as_bytes(), prefix.as_bytes(), rest.as_bytes());
    if text.len() != prefix.len() + rest.len() {
        return false;
    }
    let mut i = 0;
    while i < text.len() {
        let expected = if i < prefix.len() {
            prefix[i]
        } else {
            rest[i - prefix.len()]
        };
        if text[i] != expected {
            return false;
        }
        i += 1;
    }
    true
}

const fn canonical_of(spelling: &'static str) -> &'static str {
    let mut i = 0;
    while i < ALIASES.len() {
        if str_eq(ALIASES[i].0, spelling) {
            return ALIASES[i].1;
        }
        i += 1;
    }
    spelling
}

const fn takes_operands(entry_name: &str) -> bool {
    let mut i = 0;
    while i < NO_OPERANDS.len() {
        if str_eq(NO_OPERANDS[i], entry_name) {
            return false;
        }
        i += 1;
    }
    true
}

const fn resolve_spelling(spelling: &'static str) -> Resolution {
    let canonical = canonical_of(spelling);
    let mut e = 0;
    while e < TABLE.len() {
        let entry = &TABLE[e];
        match entry.family {
            Some(family) => {
                let mut c = family.first_condition() as usize;
                while c < CONDITION_TABLE.len() {
                    if is_concat(canonical, family.prefix(), CONDITION_TABLE[c].name) {
                        return Resolution {
                            canonical,
                            entry: e,
                            condition: Some(c),
                            takes_operands: true,
                        };
                    }
                    c += 1;
                }
            }
            None => {
                if str_eq(entry.name, canonical) {
                    return Resolution {
                        canonical,
                        entry: e,
                        condition: None,
                        takes_operands: takes_operands(entry.name),
                    };
                }
            }
        }
        e += 1;
    }
    panic!("a spelling of SPELLINGS names no table entry");
}

const fn resolve_all() -> [Resolution; 126] {
    let mut out = [Resolution {
        canonical: "",
        entry: 0,
        condition: None,
        takes_operands: false,
    }; 126];
    let mut i = 0;
    while i < SPELLING_TABLE.len() {
        out[i] = resolve_spelling(SPELLING_TABLE[i]);
        i += 1;
    }
    out
}

/// The resolution of each spelling, parallel to [`SPELLINGS`], computed at compile time.
static RESOLUTIONS: [Resolution; 126] = resolve_all();

/// Finds `name` in [`SPELLINGS`], ASCII case-insensitive, without allocating.
fn spelling_index(name: &str) -> Option<usize> {
    SPELLINGS
        .binary_search_by(|probe| {
            probe
                .bytes()
                .cmp(name.bytes().map(|byte| byte.to_ascii_uppercase()))
        })
        .ok()
}

/// A spelling with everything it resolves to.
pub(super) struct Resolved {
    /// The spelling, normalized to uppercase.
    pub(super) spelling: &'static str,
    /// Its canonical spelling.
    pub(super) canonical: &'static str,
    /// Its table entry.
    pub(super) instruction: &'static Instruction,
    /// Its condition, for a conditional spelling.
    pub(super) condition: Option<&'static Condition>,
}

/// Resolves `name` (without suffix, ASCII case-insensitive).
pub(super) fn resolve(name: &str) -> Option<Resolved> {
    let index = spelling_index(name)?;
    let resolution = RESOLUTIONS.get(index)?;
    let condition = match resolution.condition {
        Some(code) => Some(CONDITIONS.get(code)?),
        None => None,
    };
    Some(Resolved {
        spelling: SPELLINGS.get(index)?,
        canonical: resolution.canonical,
        instruction: INSTRUCTIONS.get(resolution.entry)?,
        condition,
    })
}

/// Iterates over the spellings whose canonical spelling is `canonical` (itself included).
pub(super) fn spellings_of(canonical: &str) -> impl Iterator<Item = &'static str> + '_ {
    SPELLINGS
        .iter()
        .zip(RESOLUTIONS.iter())
        .filter(move |(_, resolution)| resolution.canonical == canonical)
        .map(|(&spelling, _)| spelling)
}

/// Describes the spelling `name` (without suffix): its canonical spelling, whether it takes
/// operands, and its documentation id.
///
/// ASCII case-insensitive and allocation-free; `None` if `name` is not one of [`SPELLINGS`].
pub fn spelling_info(name: &str) -> Option<SpellingInfo> {
    let index = spelling_index(name)?;
    let resolution = RESOLUTIONS.get(index)?;
    Some(SpellingInfo {
        canonical: resolution.canonical,
        takes_operands: resolution.takes_operands,
        doc_id: INSTRUCTIONS.get(resolution.entry)?.doc_id,
    })
}

/// Returns the table entry of the spelling `name` (without suffix) and, for a conditional
/// spelling, its condition (e.g. `BHS` gives the Bcc entry and the condition CC).
///
/// ASCII case-insensitive and allocation-free; `None` if `name` is not one of [`SPELLINGS`].
pub fn instruction_of(name: &str) -> Option<(&'static Instruction, Option<&'static Condition>)> {
    let resolved = resolve(name)?;
    Some((resolved.instruction, resolved.condition))
}
