//! The 16 condition codes of the conditional families Bcc, DBcc and Scc
//! (m1-core-isa-lexer.md §4.1, §4.3).
//!
//! The code is the value of opword bits 11–8. The test is written in a small notation over the
//! flags: `!` negates, `&` and `|` combine, `==` and `!=` compare two flags.

/// One condition of the conditional families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Condition {
    /// Condition name, uppercase (e.g. `"EQ"`).
    pub name: &'static str,
    /// The 4-bit condition code (opword bits 11–8).
    pub code: u8,
    /// The flag test, in notation (e.g. `"!C & !Z"`).
    pub test: &'static str,
    /// Fluent id of the condition's documentation, `isa-m68k-cond-<name>`.
    pub doc_id: &'static str,
}

const fn condition(
    name: &'static str,
    code: u8,
    test: &'static str,
    doc_id: &'static str,
) -> Condition {
    Condition {
        name,
        code,
        test,
        doc_id,
    }
}

/// The conditions, indexed by code (the table read in constant evaluation).
pub(super) const CONDITION_TABLE: [Condition; 16] = [
    condition("T", 0, "1", "isa-m68k-cond-t"),
    condition("F", 1, "0", "isa-m68k-cond-f"),
    condition("HI", 2, "!C & !Z", "isa-m68k-cond-hi"),
    condition("LS", 3, "C | Z", "isa-m68k-cond-ls"),
    condition("CC", 4, "!C", "isa-m68k-cond-cc"),
    condition("CS", 5, "C", "isa-m68k-cond-cs"),
    condition("NE", 6, "!Z", "isa-m68k-cond-ne"),
    condition("EQ", 7, "Z", "isa-m68k-cond-eq"),
    condition("VC", 8, "!V", "isa-m68k-cond-vc"),
    condition("VS", 9, "V", "isa-m68k-cond-vs"),
    condition("PL", 10, "!N", "isa-m68k-cond-pl"),
    condition("MI", 11, "N", "isa-m68k-cond-mi"),
    condition("GE", 12, "N == V", "isa-m68k-cond-ge"),
    condition("LT", 13, "N != V", "isa-m68k-cond-lt"),
    condition("GT", 14, "!Z & N == V", "isa-m68k-cond-gt"),
    condition("LE", 15, "Z | N != V", "isa-m68k-cond-le"),
];

/// The 16 conditions, indexed by code: `CONDITIONS[c].code == c`.
pub static CONDITIONS: [Condition; 16] = CONDITION_TABLE;
