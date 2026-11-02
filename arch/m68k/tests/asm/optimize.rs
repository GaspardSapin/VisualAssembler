//! Optimizations (`src/asm/optimize.rs`; m2 §4.10, §10.1): every rule on and off, with the
//! forward-reference cases of §12.2 (WP-C).
//!
//! "On" is `optimize` with the generic spelling, then `encode` of its result; "off" is the
//! same instruction without `optimize` (a68k `-n`) and with an explicit spelling, which is
//! never rewritten. Pass 1 is modelled by `Lookup::Unknown` for a symbol defined further down,
//! pass 2 by `Lookup::Later`. Operands are built through the §4.2 types (rule G1, §15.3).
#![allow(clippy::unwrap_used)]

use va_arch_m68k::asm::encode::{encode, length, select};
use va_arch_m68k::asm::expr::{Expr, Lookup, Scope, SymRef};
use va_arch_m68k::asm::operand::{Ea, Operand, Spanned, Xn};
use va_arch_m68k::asm::optimize::optimize;
use va_arch_m68k::isa::Size;
use va_core::source::TextRange;

/// A scope with a location counter and the symbols a test uses.
struct Symbols {
    here: i64,
    symbols: Vec<(&'static str, Lookup)>,
}

impl Scope for Symbols {
    fn lookup(&self, sym: &SymRef) -> Lookup {
        self.symbols
            .iter()
            .find(|(name, _)| *name == sym.name)
            .map(|(_, lookup)| *lookup)
            .unwrap()
    }

    fn here(&self) -> i64 {
        self.here
    }
}

fn at(here: i64) -> Symbols {
    Symbols {
        here,
        symbols: Vec::new(),
    }
}

/// A scope at `$1000` where `name` resolves as `lookup`.
fn with(name: &'static str, lookup: Lookup) -> Symbols {
    Symbols {
        here: 0x1000,
        symbols: vec![(name, lookup)],
    }
}

fn num(value: i64) -> Expr {
    Expr::Num(value)
}

fn sym(name: &str) -> Expr {
    Expr::Sym(SymRef {
        name: name.to_owned(),
        local: false,
        range: TextRange::new(0, 0),
    })
}

fn d(n: u8) -> Operand {
    Operand::Ea(Ea::Dn(n))
}
fn a(n: u8) -> Operand {
    Operand::Ea(Ea::An(n))
}
fn ind(n: u8) -> Operand {
    Operand::Ea(Ea::Ind(n))
}
fn post(n: u8) -> Operand {
    Operand::Ea(Ea::PostInc(n))
}
fn pre(n: u8) -> Operand {
    Operand::Ea(Ea::PreDec(n))
}
fn disp(expr: Expr, n: u8) -> Operand {
    Operand::Ea(Ea::Disp(expr, n))
}
fn abs(expr: Expr) -> Operand {
    Operand::Ea(Ea::Abs(expr, None))
}
fn imm(expr: Expr) -> Operand {
    Operand::Ea(Ea::Imm(expr))
}

fn spans(items: Vec<Operand>) -> Vec<Spanned<Operand>> {
    items
        .into_iter()
        .zip(0u32..)
        .map(|(item, i)| Spanned {
            item,
            range: TextRange::new(10 * i, 10 * i + 4),
        })
        .collect()
}

/// Encodes `ops` with the form `spelling` selects, without optimization.
fn plain(scope: &Symbols, spelling: &str, size: Option<Size>, items: Vec<Operand>) -> Vec<u16> {
    let ops = spans(items);
    let choice = select(spelling, size, &ops).unwrap();
    let mut out = Vec::new();
    encode(
        &choice,
        &ops,
        scope,
        u64::try_from(scope.here).unwrap(),
        &mut out,
    )
    .unwrap();
    out
}

/// Optimizes with the generic spelling and encodes the result: the rule id and the words, or
/// `None` when no rule applies. Also checks that an explicit spelling is never rewritten and
/// that the rewrite keeps the operand ranges.
fn optimized(
    scope: &Symbols,
    spelling: &str,
    size: Option<Size>,
    items: Vec<Operand>,
) -> Option<(&'static str, Vec<u16>)> {
    let ops = spans(items);
    let choice = select(spelling, size, &ops).unwrap();
    assert_eq!(
        optimize(&choice, &ops, scope, false),
        None,
        "explicit {spelling}"
    );
    let (new_choice, new_ops, id) = optimize(&choice, &ops, scope, true)?;
    assert_eq!(new_ops.len(), ops.len(), "{spelling}");
    for (new, old) in new_ops.iter().zip(&ops) {
        assert_eq!(new.range, old.range, "{spelling}: operand range kept");
    }
    let mut out = Vec::new();
    let address = u64::try_from(scope.here).unwrap();
    encode(&new_choice, &new_ops, scope, address, &mut out).unwrap();
    assert_eq!(
        length(&new_choice, &new_ops),
        2 * u32::try_from(out.len()).unwrap()
    );
    Some((id, out))
}

const B: Option<Size> = Some(Size::Byte);
const W: Option<Size> = Some(Size::Word);
const L: Option<Size> = Some(Size::Long);

fn rule(id: &'static str, words: &[u16]) -> Option<(&'static str, Vec<u16>)> {
    Some((id, words.to_vec()))
}

#[test]
fn moveq() {
    let s = at(0x1000);
    let cases: Vec<(i64, Option<Vec<u16>>)> = vec![
        (5, Some(vec![0x7005])),
        (127, Some(vec![0x707F])),
        (-128, Some(vec![0x7080])),
        // A 32-bit value: #$FFFFFFFF is moveq #-1.
        (0xFFFF_FFFF, Some(vec![0x70FF])),
        (128, None),
        (-129, None),
    ];
    for (n, expected) in cases {
        let got = optimized(&s, "MOVE", L, vec![imm(num(n)), d(0)]);
        assert_eq!(got, expected.map(|w| ("moveq", w)), "move.l #{n},d0");
    }
    assert_eq!(
        plain(&s, "MOVE", L, vec![imm(num(5)), d(0)]),
        [0x203C, 0, 5]
    );
    assert_eq!(optimized(&s, "MOVE", W, vec![imm(num(5)), d(0)]), None);
    assert_eq!(optimized(&s, "MOVE", L, vec![imm(num(5)), a(0)]), None);
    assert_eq!(optimized(&s, "MOVE", L, vec![imm(num(5)), ind(0)]), None);
}

#[test]
fn moveq_needs_a_value_known_at_the_line() {
    // §12.2: backward `move.l #F,d0` gives 7003, forward gives 203C (pass 1 cannot shrink it,
    // and pass 2 keeps the size).
    let backward = with("F", Lookup::Known(3));
    assert_eq!(
        optimized(&backward, "MOVE", L, vec![imm(sym("F")), d(0)]),
        rule("moveq", &[0x7003])
    );
    for lookup in [Lookup::Unknown, Lookup::Later(3)] {
        let forward = with("F", lookup);
        assert_eq!(
            optimized(&forward, "MOVE", L, vec![imm(sym("F")), d(0)]),
            None
        );
    }
    let pass2 = with("F", Lookup::Later(3));
    assert_eq!(
        plain(&pass2, "MOVE", L, vec![imm(sym("F")), d(0)]),
        [0x203C, 0, 3]
    );
}

#[test]
fn addq_and_subq() {
    let s = at(0x1000);
    assert_eq!(
        optimized(&s, "ADD", L, vec![imm(num(4)), a(0)]),
        rule("addq", &[0x5888])
    );
    assert_eq!(
        optimized(&s, "ADD", W, vec![imm(num(3)), d(3)]),
        rule("addq", &[0x5643])
    );
    assert_eq!(
        optimized(&s, "SUB", B, vec![imm(num(8)), ind(0)]),
        rule("subq", &[0x5110])
    );
    assert_eq!(
        optimized(&s, "SUB", W, vec![imm(num(1)), a(0)]),
        rule("subq", &[0x5348])
    );
    assert_eq!(
        optimized(&s, "ADD", L, vec![imm(num(1)), post(2)]),
        rule("addq", &[0x529A])
    );
    // n <= 0 or n > 8 stays ADDI (§12.2: add.w #-3,d0 is 0640 FFFD).
    for n in [-3, 0, 9] {
        assert_eq!(
            optimized(&s, "ADD", W, vec![imm(num(n)), d(0)]),
            None,
            "#{n}"
        );
    }
    assert_eq!(
        plain(&s, "ADD", W, vec![imm(num(-3)), d(0)]),
        [0x0640, 0xFFFD]
    );
    assert_eq!(plain(&s, "ADD", L, vec![imm(num(4)), a(0)]), [0xD1FC, 0, 4]);
    // A value not known at the line never changes the size.
    let pass2 = with("G", Lookup::Later(4));
    assert_eq!(optimized(&pass2, "ADD", W, vec![imm(sym("G")), d(0)]), None);
}

#[test]
fn lea_from_add_and_sub_to_an_address_register() {
    let s = at(0x1000);
    // §12.2: add.w #100,a0 and sub.l #100,a0.
    assert_eq!(
        optimized(&s, "ADD", W, vec![imm(num(100)), a(0)]),
        rule("lea", &[0x41E8, 0x0064])
    );
    assert_eq!(
        optimized(&s, "SUB", L, vec![imm(num(100)), a(0)]),
        rule("lea", &[0x41E8, 0xFF9C])
    );
    assert_eq!(
        optimized(&s, "ADD", L, vec![imm(num(9)), a(3)]),
        rule("lea", &[0x47EB, 0x0009])
    );
    assert_eq!(
        optimized(&s, "ADD", L, vec![imm(num(32767)), a(0)]),
        rule("lea", &[0x41E8, 0x7FFF])
    );
    for n in [32768, -100, 0] {
        assert_eq!(
            optimized(&s, "ADD", W, vec![imm(num(n)), a(0)]),
            None,
            "#{n}"
        );
        assert_eq!(
            optimized(&s, "ADD", L, vec![imm(num(n)), a(0)]),
            None,
            "#{n}"
        );
    }
    assert_eq!(
        plain(&s, "ADD", W, vec![imm(num(100)), a(0)]),
        [0xD0FC, 0x0064]
    );
    // §12.2: a forward `add.w #G,a0` (G = 100) still becomes LEA in pass 2 (same size); a small
    // forward value too (1…32767), never ADDQ, which would change the size.
    let pass2 = with("G", Lookup::Later(100));
    assert_eq!(
        optimized(&pass2, "ADD", W, vec![imm(sym("G")), a(0)]),
        rule("lea", &[0x41E8, 0x0064])
    );
    let small = with("G", Lookup::Later(4));
    assert_eq!(
        optimized(&small, "ADD", W, vec![imm(sym("G")), a(0)]),
        rule("lea", &[0x41E8, 0x0004])
    );
    assert_eq!(
        optimized(
            &with("G", Lookup::Later(8)),
            "SUB",
            W,
            vec![imm(sym("G")), a(0)]
        ),
        rule("lea", &[0x41E8, 0xFFF8])
    );
    // `.L` needs the value at the line (the rewrite shortens it); pass 1 decides nothing.
    assert_eq!(optimized(&pass2, "ADD", L, vec![imm(sym("G")), a(0)]), None);
    let pass1 = with("G", Lookup::Unknown);
    assert_eq!(optimized(&pass1, "ADD", W, vec![imm(sym("G")), a(0)]), None);
}

#[test]
fn addq_lea_and_subq_lea() {
    let s = at(0x1000);
    // §12.2: lea 4(a0),a0 is 5848, lea -8(a1),a1 is 5149.
    assert_eq!(
        optimized(&s, "LEA", None, vec![disp(num(4), 0), a(0)]),
        rule("addq-lea", &[0x5848])
    );
    assert_eq!(
        optimized(&s, "LEA", None, vec![disp(num(-8), 1), a(1)]),
        rule("subq-lea", &[0x5149])
    );
    assert_eq!(
        optimized(&s, "LEA", None, vec![disp(num(8), 7), a(7)]),
        rule("addq-lea", &[0x504F])
    );
    assert_eq!(
        optimized(&s, "LEA", None, vec![disp(num(4), 0), a(1)]),
        None
    );
    assert_eq!(
        optimized(&s, "LEA", None, vec![disp(num(9), 0), a(0)]),
        None
    );
    assert_eq!(
        optimized(&s, "LEA", None, vec![disp(num(-9), 0), a(0)]),
        None
    );
    let forward = with("D", Lookup::Later(4));
    assert_eq!(
        optimized(&forward, "LEA", None, vec![disp(sym("D"), 0), a(0)]),
        None
    );
    assert_eq!(
        plain(&s, "LEA", None, vec![disp(num(4), 0), a(0)]),
        [0x41E8, 0x0004]
    );
}

#[test]
fn zero_displacement() {
    let s = at(0x1000);
    // §12.2: move.w 0(a0),d0 is 3010, lea 0(a1),a1 is 43D1.
    assert_eq!(
        optimized(&s, "MOVE", W, vec![disp(num(0), 0), d(0)]),
        rule("zero-displacement", &[0x3010])
    );
    assert_eq!(
        optimized(&s, "LEA", None, vec![disp(num(0), 1), a(1)]),
        rule("zero-displacement", &[0x43D1])
    );
    assert_eq!(
        optimized(&s, "MOVE", W, vec![d(0), disp(num(0), 1)]),
        rule("zero-displacement", &[0x3280])
    );
    // 0(a0,d0) and MOVEP's d16(An) stay; so does an unknown displacement.
    let index = Operand::Ea(Ea::Index(
        num(0),
        0,
        Xn {
            reg: 0,
            long: false,
        },
    ));
    assert_eq!(optimized(&s, "MOVE", W, vec![index, d(1)]), None);
    assert_eq!(optimized(&s, "MOVEP", W, vec![disp(num(0), 0), d(0)]), None);
    let pass1 = with("E", Lookup::Unknown);
    assert_eq!(
        optimized(&pass1, "MOVE", W, vec![disp(sym("E"), 0), d(0)]),
        None
    );
    assert_eq!(
        plain(&s, "MOVE", W, vec![disp(num(0), 0), d(0)]),
        [0x3028, 0x0000]
    );
}

#[test]
fn pc_relative() {
    // §12.2: move.w $400,d0 gives 303A F3E2 and jmp $400 gives 4EFA F3B0 (their addresses
    // follow from the displacements).
    assert_eq!(
        optimized(&at(0x101C), "MOVE", W, vec![abs(num(0x400)), d(0)]),
        rule("pc-relative", &[0x303A, 0xF3E2])
    );
    assert_eq!(
        optimized(&at(0x104E), "JMP", None, vec![abs(num(0x400))]),
        rule("pc-relative", &[0x4EFA, 0xF3B0])
    );
    // `lea lab,a0` for a backward lab; `btst #1,$400`, whose EA word follows the bit number.
    let backward = with("lab", Lookup::Known(0x0F00));
    assert_eq!(
        optimized(&backward, "LEA", None, vec![abs(sym("lab")), a(0)]),
        rule("pc-relative", &[0x41FA, 0xFEFE])
    );
    assert_eq!(
        optimized(
            &at(0x1000),
            "BTST",
            None,
            vec![imm(num(1)), abs(num(0x400))]
        ),
        rule("pc-relative", &[0x083A, 0x0001, 0xF3FC])
    );
    // Destinations never qualify; neither do forms without d16(PC) (TST on the 68000).
    let s = at(0x1000);
    assert_eq!(optimized(&s, "MOVE", W, vec![d(0), abs(num(0x400))]), None);
    assert_eq!(optimized(&s, "TST", W, vec![abs(num(0x400))]), None);
    assert_eq!(
        plain(&s, "MOVE", W, vec![d(0), abs(num(0x400))]),
        [0x33C0, 0, 0x0400]
    );
    assert_eq!(
        plain(&s, "TST", W, vec![abs(num(0x400))]),
        [0x4A79, 0, 0x0400]
    );
    // Out of reach, past the address space, or not known: absolute long.
    assert_eq!(optimized(&s, "MOVE", W, vec![abs(num(0x9002)), d(0)]), None);
    assert_eq!(
        optimized(&s, "MOVE", W, vec![abs(num(0x9001)), d(0)]),
        rule("pc-relative", &[0x303A, 0x7FFF])
    );
    assert_eq!(optimized(&s, "MOVE", W, vec![abs(num(-2)), d(0)]), None);
    assert_eq!(
        plain(&s, "JMP", None, vec![abs(num(0x400))]),
        [0x4EF9, 0, 0x0400]
    );
    // §12.2: `self: jmp self` stays absolute long (its own label counts as not known).
    for lookup in [Lookup::Unknown, Lookup::Later(0x1000)] {
        let own = with("self", lookup);
        assert_eq!(optimized(&own, "JMP", None, vec![abs(sym("self"))]), None);
    }
    assert_eq!(
        plain(
            &with("self", Lookup::Later(0x1000)),
            "JMP",
            None,
            vec![abs(sym("self"))]
        ),
        [0x4EF9, 0, 0x1000]
    );
    // An explicit size is never rewritten.
    let sized = Operand::Ea(Ea::Abs(num(0x400), Some(Size::Long)));
    assert_eq!(optimized(&s, "MOVE", W, vec![sized, d(0)]), None);
}

#[test]
fn movem_move() {
    let s = at(0x1000);
    // §12.2: movem.w d0,(a0) → 3080, movem.w (a0),d0 → 3010, movem.l a2,(a0) → 208A;
    // §13.3 Q2: movem.l d0,-(sp) → move.l d0,-(sp).
    let cases: Vec<(Option<Size>, Vec<Operand>, &[u16])> = vec![
        (W, vec![d(0), ind(0)], &[0x3080]),
        (W, vec![ind(0), d(0)], &[0x3010]),
        (L, vec![a(2), ind(0)], &[0x208A]),
        (L, vec![d(0), pre(7)], &[0x2F00]),
        // Memory to an address register is MOVEA (identical effect).
        (W, vec![ind(0), a(1)], &[0x3250]),
        // A list of one register.
        (L, vec![post(7), Operand::RegList(0x0008)], &[0x261F]),
        (W, vec![Operand::RegList(0x0200), pre(7)], &[0x3F09]),
    ];
    for (size, items, expected) in cases {
        assert_eq!(
            optimized(&s, "MOVEM", size, items.clone()),
            rule("movem-move", expected),
            "{items:?}"
        );
    }
    assert_eq!(
        optimized(&s, "MOVEM", L, vec![Operand::RegList(0x0003), ind(0)]),
        None
    );
    // Without optimization the MOVEM is kept, correct (§4.11).
    assert_eq!(plain(&s, "MOVEM", W, vec![d(0), ind(0)]), [0x4890, 0x0001]);
    assert_eq!(plain(&s, "MOVEM", L, vec![d(0), pre(7)]), [0x48E7, 0x8000]);
}

#[test]
fn one_call_applies_the_instruction_rule_then_the_operand_rules() {
    let s = at(0x1000);
    // addq and zero-displacement: the id is the instruction rule's.
    assert_eq!(
        optimized(&s, "ADD", W, vec![imm(num(4)), disp(num(0), 0)]),
        rule("addq", &[0x5850])
    );
    assert_eq!(
        optimized(&s, "MOVEM", L, vec![d(0), disp(num(0), 0)]),
        rule("movem-move", &[0x2080])
    );
    // movem-move then pc-relative, with the displacement of the final MOVE (word at +2).
    assert_eq!(
        optimized(&s, "MOVEM", W, vec![abs(num(0x400)), d(1)]),
        rule("movem-move", &[0x323A, 0xF3FE])
    );
    // Two operand rules: the first id.
    assert_eq!(
        optimized(
            &at(0x101C),
            "MOVE",
            W,
            vec![abs(num(0x400)), disp(num(0), 1)]
        ),
        rule("pc-relative", &[0x32BA, 0xF3E2])
    );
}

#[test]
fn explicit_spellings_and_other_instructions_are_never_rewritten() {
    let s = at(0x1000);
    let cases: Vec<(&str, Option<Size>, Vec<Operand>)> = vec![
        ("ADDI", W, vec![imm(num(3)), d(3)]),
        ("ADDA", L, vec![imm(num(4)), a(0)]),
        ("SUBA", W, vec![imm(num(100)), a(0)]),
        ("MOVEA", L, vec![imm(num(5)), a(0)]),
        ("ADDQ", W, vec![imm(num(3)), d(3)]),
        ("CMP", W, vec![imm(num(3)), a(0)]),
        ("AND", W, vec![imm(num(3)), d(0)]),
        ("BRA", None, vec![abs(num(0x1010))]),
    ];
    for (spelling, size, items) in cases {
        let ops = spans(items);
        let choice = select(spelling, size, &ops).unwrap();
        assert_eq!(optimize(&choice, &ops, &s, false), None, "{spelling}");
    }
    // With the generic spelling, CMP, AND and branches have no rule here (branches are
    // shortened by the relaxation).
    for (spelling, size, items) in [
        ("CMP", W, vec![imm(num(3)), a(0)]),
        ("AND", W, vec![imm(num(3)), d(0)]),
        ("BRA", None, vec![abs(num(0x1010))]),
    ] {
        let ops = spans(items);
        let choice = select(spelling, size, &ops).unwrap();
        assert_eq!(optimize(&choice, &ops, &s, true), None, "{spelling}");
    }
}
