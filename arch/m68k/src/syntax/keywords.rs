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
/// its operand policy and its doc id, the Fluent message `isa-m68k-dir-<name>` in lowercase
/// (`=` is `isa-m68k-dir-equals`) of `locales/{en,fr}/directives.ftl` (m2-assembler.md §7.1).
static DIRECTIVES: [(&str, OperandPolicy, &str); 49] = [
    ("=", OperandPolicy::Operands, "isa-m68k-dir-equals"),
    ("BSS", OperandPolicy::None, "isa-m68k-dir-bss"),
    ("CNOP", OperandPolicy::Operands, "isa-m68k-dir-cnop"),
    ("CODE", OperandPolicy::None, "isa-m68k-dir-code"),
    ("CSEG", OperandPolicy::None, "isa-m68k-dir-cseg"),
    ("DATA", OperandPolicy::None, "isa-m68k-dir-data"),
    ("DC", OperandPolicy::Operands, "isa-m68k-dir-dc"),
    ("DCB", OperandPolicy::Operands, "isa-m68k-dir-dcb"),
    ("DS", OperandPolicy::Operands, "isa-m68k-dir-ds"),
    ("DSEG", OperandPolicy::None, "isa-m68k-dir-dseg"),
    ("END", OperandPolicy::Operands, "isa-m68k-dir-end"),
    ("ENDC", OperandPolicy::None, "isa-m68k-dir-endc"),
    ("ENDIF", OperandPolicy::None, "isa-m68k-dir-endif"),
    ("ENDM", OperandPolicy::None, "isa-m68k-dir-endm"),
    ("EQU", OperandPolicy::Operands, "isa-m68k-dir-equ"),
    ("EQUR", OperandPolicy::Operands, "isa-m68k-dir-equr"),
    ("EVEN", OperandPolicy::None, "isa-m68k-dir-even"),
    ("FAR", OperandPolicy::None, "isa-m68k-dir-far"),
    ("IDNT", OperandPolicy::Operands, "isa-m68k-dir-idnt"),
    ("IFC", OperandPolicy::Operands, "isa-m68k-dir-ifc"),
    ("IFD", OperandPolicy::Operands, "isa-m68k-dir-ifd"),
    ("IFEQ", OperandPolicy::Operands, "isa-m68k-dir-ifeq"),
    ("IFGE", OperandPolicy::Operands, "isa-m68k-dir-ifge"),
    ("IFGT", OperandPolicy::Operands, "isa-m68k-dir-ifgt"),
    ("IFLE", OperandPolicy::Operands, "isa-m68k-dir-ifle"),
    ("IFLT", OperandPolicy::Operands, "isa-m68k-dir-iflt"),
    ("IFNC", OperandPolicy::Operands, "isa-m68k-dir-ifnc"),
    ("IFND", OperandPolicy::Operands, "isa-m68k-dir-ifnd"),
    ("IFNE", OperandPolicy::Operands, "isa-m68k-dir-ifne"),
    ("INCBIN", OperandPolicy::Path, "isa-m68k-dir-incbin"),
    ("INCLUDE", OperandPolicy::Path, "isa-m68k-dir-include"),
    ("LIST", OperandPolicy::None, "isa-m68k-dir-list"),
    ("MACRO", OperandPolicy::None, "isa-m68k-dir-macro"),
    ("MEXIT", OperandPolicy::None, "isa-m68k-dir-mexit"),
    ("NEAR", OperandPolicy::Operands, "isa-m68k-dir-near"),
    ("NOL", OperandPolicy::None, "isa-m68k-dir-nol"),
    ("NOLIST", OperandPolicy::None, "isa-m68k-dir-nolist"),
    ("ORG", OperandPolicy::Operands, "isa-m68k-dir-org"),
    ("PAGE", OperandPolicy::None, "isa-m68k-dir-page"),
    ("PUBLIC", OperandPolicy::Operands, "isa-m68k-dir-public"),
    ("REG", OperandPolicy::Operands, "isa-m68k-dir-reg"),
    ("RORG", OperandPolicy::Operands, "isa-m68k-dir-rorg"),
    ("SECTION", OperandPolicy::Operands, "isa-m68k-dir-section"),
    ("SET", OperandPolicy::Operands, "isa-m68k-dir-set"),
    ("SPC", OperandPolicy::Operands, "isa-m68k-dir-spc"),
    ("TITLE", OperandPolicy::Operands, "isa-m68k-dir-title"),
    ("TTL", OperandPolicy::Operands, "isa-m68k-dir-ttl"),
    ("XDEF", OperandPolicy::Operands, "isa-m68k-dir-xdef"),
    ("XREF", OperandPolicy::Operands, "isa-m68k-dir-xref"),
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
        .binary_search_by(|(entry, _, _)| cmp_folded(entry, name))
        .ok()
        .and_then(|index| DIRECTIVES.get(index))
        .map(|&(_, policy, _)| policy)
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
/// their doc id, the 49 directives with theirs (m2 §7.1), the 21 registers and the 4 size
/// suffixes. Built once, on first use.
pub(crate) fn keywords() -> &'static [KeywordInfo] {
    static KEYWORDS: OnceLock<Vec<KeywordInfo>> = OnceLock::new();
    KEYWORDS.get_or_init(|| {
        let mnemonics = SPELLINGS.iter().map(|&name| KeywordInfo {
            name,
            kind: KeywordKind::Mnemonic,
            doc_id: spelling_info(name).map(|info| info.doc_id),
        });
        let directives = DIRECTIVES.iter().map(|&(name, _, doc_id)| KeywordInfo {
            name,
            kind: KeywordKind::Directive,
            doc_id: Some(doc_id),
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
