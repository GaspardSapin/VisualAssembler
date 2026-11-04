//! The generated forms corpus (m2 §10.1, shared with the a68k comparison of §10.4): one line per
//! entry, form, size and operand mode, assembled without diagnostics and decoded back to its
//! form (WP-C).
//!
//! **Shared file (§15.3 G2).** `tests/a68k.rs` includes this file with
//! `#[path = "asm/corpus.rs"] mod corpus;` and calls [`lines`]. So it is self-contained (only
//! `va_arch_m68k`, `va_core` and `std`), and every item is used by its own tests here.
//!
//! **The program.** `org $1000`, then one instruction line per (entry, form, condition of a
//! family, size, addressing mode of each `<ea>` operand), in table order. Every condition of
//! Bcc, DBcc and Scc gets its lines. Representative operands (§10.1): `d1 a2 (a3) (a4)+ -(a5)
//! $10(a6) $10(a0,d1.w) ($1234).w ($00012345).l` and `#$12` for the `<ea>` modes; register
//! fields `d6`/`d7`, `a1`/`a2`, `(a3)+`/`(a4)+`, `-(a5)`/`-(a6)`, `$10(a6)` (MOVEP), by operand
//! position; `#5` for bit numbers and trap vectors, `#8` for quick values (stored as 0), `#-2`
//! for MOVEQ, `d0-d2/a3` for MOVEM lists. A line that has a PC-relative or branch operand
//! carries its own label `l<n>` in column 1 and targets it (`l7 bra.s l7`, `l9 lea
//! l9(pc,d2.l),a1`), so every displacement fits whatever the corpus length.
//!
//! **Generic variant** (`lines(true)`, assembled with optimizations in §10.4): ADDA, ADDI,
//! ADDQ → ADD; SUBA, SUBI, SUBQ → SUB; CMPA, CMPI, CMPM → CMP; ANDI → AND; ORI → OR;
//! EORI → EOR; MOVEA, MOVEQ → MOVE; branches (Bcc, BRA, BSR) without size.
//!
//! **Checks here.** The lines lex without error; every form is covered; every line selects a
//! form and encodes to words whose opword that form matches, with the length `length` gives
//! (the generic lines after `optimize`). A non-generic line selects its own form except the 15
//! immediate-source lines of ADD, SUB, CMP, AND and OR `<ea>,Dn`, which a68k (and §4.10) assemble
//! as ADDI, SUBI, CMPI, ANDI and ORI. The join with the driver and the disassembler lives in
//! `tests/roundtrip.rs` (§16.2), which includes this file the same way: `lines(false)` assembles
//! through `asm::assemble` and each instruction decodes back to its length and words.
#![allow(clippy::unwrap_used)]

use va_arch_m68k::asm::encode::{encode, length, select};
use va_arch_m68k::asm::expr::{Expr, Lookup, Scope, SymRef};
use va_arch_m68k::asm::operand::{Ea, Operand, Spanned, Xn};
use va_arch_m68k::asm::optimize::optimize;
use va_arch_m68k::isa::{
    CONDITIONS, Form, INSTRUCTIONS, Instruction, Mode, Operand as Slot, QuickRange, Size,
};
use va_arch_m68k::syntax::lex_line;
use va_core::source::TextRange;
use va_core::syntax::TokenKind;

/// The origin of the corpus program.
const ORIGIN: i64 = 0x1000;

/// The number of instruction lines of the corpus (golden: a change of the tables changes it).
const ITEMS: usize = 1_526;

/// One instruction line of the corpus: its text pieces and the operands they stand for.
struct Item {
    entry: &'static Instruction,
    form: usize,
    condition: Option<u8>,
    /// Uppercase, as `select` takes it.
    spelling: String,
    size: Option<Size>,
    /// The line's own label, when an operand targets it.
    label: Option<String>,
    operands: Vec<(String, Operand)>,
}

impl Item {
    fn text(&self) -> String {
        let mut line = self.label.clone().unwrap_or_default();
        line.push('\t');
        line.push_str(&self.spelling.to_ascii_lowercase());
        if let Some(size) = self.size {
            line.push('.');
            line.push_str(&size.letter().to_ascii_lowercase());
        }
        if !self.operands.is_empty() {
            let texts: Vec<&str> = self
                .operands
                .iter()
                .map(|(text, _)| text.as_str())
                .collect();
            line.push('\t');
            line.push_str(&texts.join(","));
        }
        line
    }

    fn spanned(&self) -> Vec<Spanned<Operand>> {
        self.operands
            .iter()
            .map(|(_, op)| Spanned {
                item: op.clone(),
                range: TextRange::new(0, 0),
            })
            .collect()
    }
}

/// The source lines of the forms corpus; `generic` gives the variant with the generic
/// mnemonics (assembled with optimizations by §10.4).
pub fn lines(generic: bool) -> Vec<String> {
    let mut out = vec![format!("\torg\t${ORIGIN:X}")];
    out.extend(items(generic).iter().map(Item::text));
    out
}

fn sym(name: &str) -> Expr {
    Expr::Sym(SymRef {
        name: name.to_owned(),
        local: false,
        range: TextRange::new(0, 0),
    })
}

/// The representative operand of an `<ea>` mode (§10.1); `label` names the line's label.
fn ea_example(mode: Mode, label: &str) -> (String, Operand) {
    let (text, ea) = match mode {
        Mode::Dn => ("d1".to_owned(), Ea::Dn(1)),
        Mode::An => ("a2".to_owned(), Ea::An(2)),
        Mode::Ind => ("(a3)".to_owned(), Ea::Ind(3)),
        Mode::PostInc => ("(a4)+".to_owned(), Ea::PostInc(4)),
        Mode::PreDec => ("-(a5)".to_owned(), Ea::PreDec(5)),
        Mode::Disp => ("$10(a6)".to_owned(), Ea::Disp(Expr::Num(0x10), 6)),
        Mode::Index => (
            "$10(a0,d1.w)".to_owned(),
            Ea::Index(
                Expr::Num(0x10),
                0,
                Xn {
                    reg: 1,
                    long: false,
                },
            ),
        ),
        Mode::AbsW => (
            "($1234).w".to_owned(),
            Ea::Abs(Expr::Num(0x1234), Some(Size::Word)),
        ),
        Mode::AbsL => (
            "($00012345).l".to_owned(),
            Ea::Abs(Expr::Num(0x12345), Some(Size::Long)),
        ),
        Mode::PcDisp => (format!("{label}(pc)"), Ea::PcDisp(sym(label))),
        Mode::PcIndex => (
            format!("{label}(pc,d2.l)"),
            Ea::PcIndex(sym(label), Xn { reg: 2, long: true }),
        ),
        Mode::Imm => ("#$12".to_owned(), Ea::Imm(Expr::Num(0x12))),
    };
    (text, Operand::Ea(ea))
}

/// The representative operand of a non-`<ea>` slot at `position`.
fn slot_example(entry: &Instruction, slot: Slot, position: u8, label: &str) -> (String, Operand) {
    let immediate = |value: i64| (format!("#{value}"), Operand::Ea(Ea::Imm(Expr::Num(value))));
    match slot {
        Slot::Dn => (
            format!("d{}", 6 + position),
            Operand::Ea(Ea::Dn(6 + position)),
        ),
        Slot::An => (
            format!("a{}", 1 + position),
            Operand::Ea(Ea::An(1 + position)),
        ),
        Slot::PostIncAn => (
            format!("(a{})+", 3 + position),
            Operand::Ea(Ea::PostInc(3 + position)),
        ),
        Slot::PreDecAn => (
            format!("-(a{})", 5 + position),
            Operand::Ea(Ea::PreDec(5 + position)),
        ),
        Slot::DispAn => (
            "$10(a6)".to_owned(),
            Operand::Ea(Ea::Disp(Expr::Num(0x10), 6)),
        ),
        Slot::Imm if matches!(entry.name, "BTST" | "BCHG" | "BCLR" | "BSET") => immediate(5),
        Slot::Imm => ("#$12".to_owned(), Operand::Ea(Ea::Imm(Expr::Num(0x12)))),
        Slot::Quick(QuickRange::OneToEight) => immediate(8),
        Slot::Quick(QuickRange::Signed8) => immediate(-2),
        Slot::Quick(QuickRange::Vector4) => immediate(5),
        Slot::RegList => ("d0-d2/a3".to_owned(), Operand::RegList(0x0807)),
        Slot::Label => (label.to_owned(), Operand::Ea(Ea::Abs(sym(label), None))),
        Slot::Ccr => ("ccr".to_owned(), Operand::Ccr),
        Slot::Sr => ("sr".to_owned(), Operand::Sr),
        Slot::Usp => ("usp".to_owned(), Operand::Usp),
        Slot::Ea(_) => unreachable!("an <ea> slot takes ea_example"),
    }
}

/// The spellings of an entry: its name, or one per condition of its family.
fn spellings(entry: &Instruction) -> Vec<(String, Option<u8>)> {
    match entry.family {
        Some(family) => (family.first_condition()..16)
            .map(|code| {
                let condition = &CONDITIONS[usize::from(code)];
                (format!("{}{}", family.prefix(), condition.name), Some(code))
            })
            .collect(),
        None => vec![(entry.name.to_owned(), None)],
    }
}

/// The generic spelling of an explicit variant (§10.4).
fn generic_spelling(spelling: &str) -> &str {
    match spelling {
        "ADDA" | "ADDI" | "ADDQ" => "ADD",
        "SUBA" | "SUBI" | "SUBQ" => "SUB",
        "CMPA" | "CMPI" | "CMPM" => "CMP",
        "ANDI" => "AND",
        "ORI" => "OR",
        "EORI" => "EOR",
        "MOVEA" | "MOVEQ" => "MOVE",
        other => other,
    }
}

fn is_branch(entry: &Instruction) -> bool {
    entry
        .forms
        .iter()
        .any(|form| form.sizes.contains(Size::Short))
}

/// Every combination of one mode per `<ea>` slot of `form` (one empty combination without).
fn mode_combinations(form: &Form) -> Vec<Vec<Mode>> {
    let mut combinations = vec![Vec::new()];
    for slot in form.operands {
        if let Slot::Ea(set) = slot {
            combinations = combinations
                .iter()
                .flat_map(|prefix| {
                    set.iter().map(move |mode| {
                        let mut next = prefix.clone();
                        next.push(mode);
                        next
                    })
                })
                .collect();
        }
    }
    combinations
}

fn items(generic: bool) -> Vec<Item> {
    let mut out = Vec::new();
    for entry in &INSTRUCTIONS {
        for (form_index, form) in entry.forms.iter().enumerate() {
            for (spelling, condition) in spellings(entry) {
                let sizes: Vec<Option<Size>> = if form.sizes.is_empty() {
                    vec![None]
                } else {
                    form.sizes.iter().map(Some).collect()
                };
                for size in sizes {
                    for modes in mode_combinations(form) {
                        let label = format!("l{}", out.len() + 1);
                        let mut modes = modes.into_iter();
                        let mut uses_label = false;
                        let operands = form
                            .operands
                            .iter()
                            .zip(0u8..)
                            .map(|(&slot, position)| match slot {
                                Slot::Ea(_) => {
                                    let mode = modes.next().unwrap();
                                    uses_label |= matches!(mode, Mode::PcDisp | Mode::PcIndex);
                                    ea_example(mode, &label)
                                }
                                other => {
                                    uses_label |= other == Slot::Label;
                                    slot_example(entry, other, position, &label)
                                }
                            })
                            .collect();
                        let (spelling, size) = if generic {
                            let size = if is_branch(entry) { None } else { size };
                            (generic_spelling(&spelling).to_owned(), size)
                        } else {
                            (spelling.clone(), size)
                        };
                        out.push(Item {
                            entry,
                            form: form_index,
                            condition,
                            spelling,
                            size,
                            label: uses_label.then_some(label),
                            operands,
                        });
                    }
                }
            }
        }
    }
    out
}

/// Every symbol is the label of the line being encoded, so it resolves to the line's address.
struct LineScope {
    here: i64,
}

impl Scope for LineScope {
    fn lookup(&self, _sym: &SymRef) -> Lookup {
        Lookup::Known(self.here)
    }

    fn here(&self) -> i64 {
        self.here
    }
}

#[test]
fn the_corpus_covers_every_form_and_starts_at_its_origin() {
    for generic in [false, true] {
        let items = items(generic);
        assert_eq!(items.len(), ITEMS, "generic {generic}");
        let lines = lines(generic);
        assert_eq!(lines.len(), ITEMS + 1);
        assert_eq!(lines[0], "\torg\t$1000");
        for entry in &INSTRUCTIONS {
            for form in 0..entry.forms.len() {
                assert!(
                    items
                        .iter()
                        .any(|item| std::ptr::eq(item.entry, entry) && item.form == form),
                    "{} form {form} has no line",
                    entry.name
                );
            }
        }
    }
    let plain = lines(false);
    assert!(plain.contains(&"\tmove.b\t$10(a0,d1.w),($00012345).l".to_owned()));
    assert!(plain.iter().any(|line| line.contains("\tdbf.w\td6,l")));
    assert!(plain.contains(&"\tmovem.w\td0-d2/a3,-(a5)".to_owned()));
    let generic = lines(true);
    assert!(generic.contains(&"\tadd.w\t#8,a2".to_owned()));
    assert!(generic.contains(&"\tmove.l\t#-2,d7".to_owned()));
    assert!(generic.iter().any(|line| line.contains("\tbeq\tl")));
}

#[test]
fn every_line_lexes_without_error() {
    let mut tokens = Vec::new();
    for generic in [false, true] {
        for line in lines(generic) {
            tokens.clear();
            lex_line(&line, &mut tokens);
            for token in &tokens {
                assert!(
                    !matches!(
                        token.kind,
                        TokenKind::Invalid(_) | TokenKind::SuspectComment(_) | TokenKind::Comment
                    ),
                    "{line:?}: {token:?}"
                );
            }
            let operations = tokens
                .iter()
                .filter(|t| matches!(t.kind, TokenKind::Mnemonic | TokenKind::Directive))
                .count();
            assert_eq!(operations, 1, "{line:?}");
            let labelled = !line.starts_with('\t');
            assert_eq!(
                tokens.first().map(|t| t.kind) == Some(TokenKind::Label),
                labelled,
                "{line:?}"
            );
        }
    }
}

/// Selects, optionally optimizes, and encodes every item at its address; returns, per item,
/// the entry and form that were encoded.
fn encode_all(generic: bool) -> Vec<(&'static str, usize)> {
    let mut address = ORIGIN;
    let mut encoded = Vec::new();
    for item in items(generic) {
        let line = item.text();
        let ops = item.spanned();
        let scope = LineScope { here: address };
        let mut choice = select(&item.spelling, item.size, &ops)
            .unwrap_or_else(|problem| panic!("{line:?}: {problem:?}"));
        let mut ops = ops;
        if generic && let Some((new_choice, new_ops, _)) = optimize(&choice, &ops, &scope, true) {
            choice = new_choice;
            ops = new_ops;
        }
        let mut words = Vec::new();
        encode(
            &choice,
            &ops,
            &scope,
            u64::try_from(address).unwrap(),
            &mut words,
        )
        .unwrap_or_else(|problem| panic!("{line:?}: {problem:?}"));
        let form = &choice.entry.forms[choice.form];
        assert!(
            form.matches(words[0], choice.condition),
            "{line:?}: {:04X} is not {} form {}",
            words[0],
            choice.entry.name,
            choice.form
        );
        if item.entry.family.is_some() && !generic {
            assert_eq!(choice.condition, item.condition, "{line:?}");
        }
        let bytes = length(&choice, &ops);
        assert_eq!(bytes, 2 * u32::try_from(words.len()).unwrap(), "{line:?}");
        address += i64::from(bytes);
        encoded.push((choice.entry.name, choice.form));
    }
    encoded
}

#[test]
fn every_line_encodes_to_its_form() {
    let items = items(false);
    let encoded = encode_all(false);
    let mut shadowed = Vec::new();
    for (item, &(entry, form)) in items.iter().zip(&encoded) {
        if entry != item.entry.name || form != item.form {
            shadowed.push((item.entry.name, item.form, entry, item.text()));
        }
    }
    // In table order: `<op> #$12,d7` at .B, .W and .L selects the immediate entry.
    let mut expected = Vec::new();
    for (name, immediate) in [
        ("ADD", "ADDI"),
        ("AND", "ANDI"),
        ("CMP", "CMPI"),
        ("OR", "ORI"),
        ("SUB", "SUBI"),
    ] {
        for size in ["b", "w", "l"] {
            let text = format!("\t{}.{size}\t#$12,d7", name.to_lowercase());
            expected.push((name, immediate, text));
        }
    }
    let got: Vec<(&str, &str, String)> = shadowed
        .iter()
        .map(|(own, _, selected, text)| (*own, *selected, text.clone()))
        .collect();
    assert_eq!(got, expected);
}

#[test]
fn every_generic_line_encodes_with_optimizations() {
    let encoded = encode_all(true);
    assert_eq!(encoded.len(), ITEMS);
    // The quick and MOVEQ lines come back to their own entry through the optimizations.
    assert!(encoded.contains(&("ADDQ", 1)));
    assert!(encoded.contains(&("SUBQ", 0)));
    assert!(encoded.contains(&("MOVEQ", 0)));
    assert!(encoded.contains(&("LEA", 0)));
}
