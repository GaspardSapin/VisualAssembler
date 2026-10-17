//! Keyword tables of the A68k dialect (m1-core-isa-lexer.md §5.2, §5.3): the 49 directives with
//! their operand policy, the 21 register names, the 4 size suffixes, and the keyword list of
//! `SyntaxProvider::keywords`. Mnemonics come from the ISA tables (`isa::SPELLINGS`,
//! `isa::spelling_info`, §4.3).
//!
//! Lookups are ASCII case-insensitive and allocation-free: fixed tables compared byte by byte
//! with ASCII case folding, never an uppercased copy of the input.

use std::cmp::Ordering;
use std::sync::OnceLock;

use va_core::syntax::{KeywordInfo, KeywordKind};

use crate::isa::{SPELLINGS, spelling_info};

/// What follows a directive or a mnemonic in the operation field (§5.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum OperandPolicy {
    /// No operand: the rest of the line is a comment (`EVEN`, `RTS`).
    None,
    /// A file path, quoted or not, lexed as `String` (`INCLUDE`, `INCBIN`).
    Path,
    /// Operands, required or optional, lexed with the operand rules.
    Operands,
}

/// The A68k 2.71.F3w directive table (§5.3): 49 uppercase spellings in ASCII order, each with
/// its operand policy.
static DIRECTIVES: [(&str, OperandPolicy); 49] = [
    ("=", OperandPolicy::Operands),
    ("BSS", OperandPolicy::None),
    ("CNOP", OperandPolicy::Operands),
    ("CODE", OperandPolicy::None),
    ("CSEG", OperandPolicy::None),
    ("DATA", OperandPolicy::None),
    ("DC", OperandPolicy::Operands),
    ("DCB", OperandPolicy::Operands),
    ("DS", OperandPolicy::Operands),
    ("DSEG", OperandPolicy::None),
    ("END", OperandPolicy::Operands),
    ("ENDC", OperandPolicy::None),
    ("ENDIF", OperandPolicy::None),
    ("ENDM", OperandPolicy::None),
    ("EQU", OperandPolicy::Operands),
    ("EQUR", OperandPolicy::Operands),
    ("EVEN", OperandPolicy::None),
    ("FAR", OperandPolicy::None),
    ("IDNT", OperandPolicy::Operands),
    ("IFC", OperandPolicy::Operands),
    ("IFD", OperandPolicy::Operands),
    ("IFEQ", OperandPolicy::Operands),
    ("IFGE", OperandPolicy::Operands),
    ("IFGT", OperandPolicy::Operands),
    ("IFLE", OperandPolicy::Operands),
    ("IFLT", OperandPolicy::Operands),
    ("IFNC", OperandPolicy::Operands),
    ("IFND", OperandPolicy::Operands),
    ("IFNE", OperandPolicy::Operands),
    ("INCBIN", OperandPolicy::Path),
    ("INCLUDE", OperandPolicy::Path),
    ("LIST", OperandPolicy::None),
    ("MACRO", OperandPolicy::None),
    ("MEXIT", OperandPolicy::None),
    ("NEAR", OperandPolicy::Operands),
    ("NOL", OperandPolicy::None),
    ("NOLIST", OperandPolicy::None),
    ("ORG", OperandPolicy::Operands),
    ("PAGE", OperandPolicy::None),
    ("PUBLIC", OperandPolicy::Operands),
    ("REG", OperandPolicy::Operands),
    ("RORG", OperandPolicy::Operands),
    ("SECTION", OperandPolicy::Operands),
    ("SET", OperandPolicy::Operands),
    ("SPC", OperandPolicy::Operands),
    ("TITLE", OperandPolicy::Operands),
    ("TTL", OperandPolicy::Operands),
    ("XDEF", OperandPolicy::Operands),
    ("XREF", OperandPolicy::Operands),
];

/// Length of the longest directive spelling (`INCLUDE`, `SECTION`).
const LONGEST_DIRECTIVE: usize = 7;

/// The 21 register names the operand rules recognize (§5.2), uppercase.
static REGISTERS: [&str; 21] = [
    "D0", "D1", "D2", "D3", "D4", "D5", "D6", "D7", "A0", "A1", "A2", "A3", "A4", "A5", "A6", "A7",
    "SP", "USP", "CCR", "SR", "PC",
];

/// The 4 size suffixes, dot included, uppercase.
static SIZE_SUFFIXES: [&str; 4] = [".B", ".W", ".L", ".S"];

/// Compares an uppercase table entry with `name`, folding the ASCII case of `name`.
fn cmp_folded(entry: &str, name: &str) -> Ordering {
    entry
        .bytes()
        .cmp(name.bytes().map(|byte| byte.to_ascii_uppercase()))
}

/// Returns the operand policy of the directive spelled `name` (any case), or `None` if `name`
/// is not one of the 49 directives.
pub(crate) fn directive(name: &str) -> Option<OperandPolicy> {
    if name.len() > LONGEST_DIRECTIVE {
        return None;
    }
    DIRECTIVES
        .binary_search_by(|(entry, _)| cmp_folded(entry, name))
        .ok()
        .and_then(|index| DIRECTIVES.get(index))
        .map(|&(_, policy)| policy)
}

/// Returns the operand policy of the mnemonic spelled `name` (any case), or `None` if `name`
/// is not one of the 126 spellings of §4.1.
pub(crate) fn mnemonic(name: &str) -> Option<OperandPolicy> {
    spelling_info(name).map(|info| {
        if info.takes_operands {
            OperandPolicy::Operands
        } else {
            OperandPolicy::None
        }
    })
}

/// Tells whether `word` (any case) is one of the 21 register names: D0–D7, A0–A7, SP, USP,
/// CCR, SR, PC.
pub(crate) fn is_register(word: &[u8]) -> bool {
    match *word {
        [first, second] => matches!(
            (first.to_ascii_uppercase(), second.to_ascii_uppercase()),
            (b'D' | b'A', b'0'..=b'7') | (b'S', b'P' | b'R') | (b'P', b'C')
        ),
        [first, second, third] => matches!(
            (
                first.to_ascii_uppercase(),
                second.to_ascii_uppercase(),
                third.to_ascii_uppercase()
            ),
            (b'U', b'S', b'P') | (b'C', b'C', b'R')
        ),
        _ => false,
    }
}

/// Returns the keyword list of the dialect (§5.3): the 126 mnemonics in `SPELLINGS` order with
/// their doc id, the 49 directives (no doc id until M2), the 21 registers and the 4 size
/// suffixes. Built once, on first use.
pub(crate) fn keywords() -> &'static [KeywordInfo] {
    static KEYWORDS: OnceLock<Vec<KeywordInfo>> = OnceLock::new();
    KEYWORDS.get_or_init(|| {
        let mnemonics = SPELLINGS.iter().map(|&name| KeywordInfo {
            name,
            kind: KeywordKind::Mnemonic,
            doc_id: spelling_info(name).map(|info| info.doc_id),
        });
        let directives = DIRECTIVES.iter().map(|&(name, _)| KeywordInfo {
            name,
            kind: KeywordKind::Directive,
            doc_id: None,
        });
        let registers = REGISTERS.iter().map(|&name| KeywordInfo {
            name,
            kind: KeywordKind::Register,
            doc_id: None,
        });
        let suffixes = SIZE_SUFFIXES.iter().map(|&name| KeywordInfo {
            name,
            kind: KeywordKind::SizeSuffix,
            doc_id: None,
        });
        mnemonics
            .chain(directives)
            .chain(registers)
            .chain(suffixes)
            .collect()
    })
}
