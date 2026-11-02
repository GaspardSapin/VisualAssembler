//! Form selection and encoding (`src/asm/encode.rs`; m2 §4.10, §10.1): the m1 §4.7 golden
//! opwords assembled from text, the selection examples, every `Place`, MOVEM mask reversal and
//! the selection error codes (WP-C).
//!
//! The operands are built directly through the §4.2 types (rule G1 of §15.3: no test calls the
//! front end or the driver of another package before they land), with leaf expressions only.
#![allow(clippy::unwrap_used)]

use va_arch_m68k::asm::codes::{
    ASM_BRANCH_OUT_OF_RANGE, ASM_MODE_NOT_ALLOWED, ASM_NO_MATCHING_FORM, ASM_ODD_BRANCH_TARGET,
    ASM_OPERAND_COUNT, ASM_SHORT_BRANCH_ZERO, ASM_SIZE_NOT_ALLOWED, ASM_UNKNOWN_OPERATION,
    ASM_VALUE_OUT_OF_RANGE,
};
use va_arch_m68k::asm::encode::{Choice, encode, length, select};
use va_arch_m68k::asm::expr::{Expr, Lookup, Scope, SymRef};
use va_arch_m68k::asm::operand::{Ea, Operand, Spanned, Xn};
use va_arch_m68k::asm::parser::Problem;
use va_arch_m68k::isa::{Size, instruction_of};
use va_core::diag::{DiagArg, DiagCode};
use va_core::source::TextRange;

/// A scope with a location counter and a few symbols (every name used must be listed: an
/// undefined name would go to the front end's evaluator).
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
fn disp(value: i64, n: u8) -> Operand {
    Operand::Ea(Ea::Disp(num(value), n))
}
fn index(value: i64, n: u8, reg: u8, long: bool) -> Operand {
    Operand::Ea(Ea::Index(num(value), n, Xn { reg, long }))
}
fn pc(target: i64) -> Operand {
    Operand::Ea(Ea::PcDisp(num(target)))
}
fn pc_index(target: i64, reg: u8, long: bool) -> Operand {
    Operand::Ea(Ea::PcIndex(num(target), Xn { reg, long }))
}
fn abs(value: i64) -> Operand {
    Operand::Ea(Ea::Abs(num(value), None))
}
fn abs_w(value: i64) -> Operand {
    Operand::Ea(Ea::Abs(num(value), Some(Size::Word)))
}
fn abs_l(value: i64) -> Operand {
    Operand::Ea(Ea::Abs(num(value), Some(Size::Long)))
}
fn imm(value: i64) -> Operand {
    Operand::Ea(Ea::Imm(num(value)))
}
fn list(mask: u16) -> Operand {
    Operand::RegList(mask)
}

/// Operand `i` covers bytes `10 i .. 10 i + 4` of the line.
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

/// Selects and encodes at `address`; checks that `length` agrees with the words.
fn words_in(scope: &Symbols, spelling: &str, size: Option<Size>, items: Vec<Operand>) -> Vec<u16> {
    let ops = spans(items);
    let choice = select(spelling, size, &ops).unwrap();
    let mut out = Vec::new();
    let address = u64::try_from(scope.here).unwrap();
    encode(&choice, &ops, scope, address, &mut out).unwrap();
    assert_eq!(
        length(&choice, &ops),
        2 * u32::try_from(out.len()).unwrap(),
        "{spelling} {size:?}: length"
    );
    out
}

/// [`words_in`] at `$1000`.
fn words(spelling: &str, size: Option<Size>, items: Vec<Operand>) -> Vec<u16> {
    words_in(&at(0x1000), spelling, size, items)
}

/// Selects (which must succeed) and encodes at `$1000`, which must fail without appending.
fn encode_error(spelling: &str, size: Option<Size>, items: Vec<Operand>) -> Problem {
    let ops = spans(items);
    let choice = select(spelling, size, &ops).unwrap();
    let mut out = vec![0xABCD];
    let problem = encode(&choice, &ops, &at(0x1000), 0x1000, &mut out).unwrap_err();
    assert_eq!(out, [0xABCD], "{spelling}: words appended on error");
    problem
}

fn select_error(spelling: &str, size: Option<Size>, items: Vec<Operand>) -> Problem {
    select(spelling, size, &spans(items)).unwrap_err()
}

fn arg<'p>(problem: &'p Problem, name: &str) -> &'p DiagArg {
    &problem
        .args
        .iter()
        .find(|(key, _)| *key == name)
        .unwrap_or_else(|| panic!("no argument {name} in {problem:?}"))
        .1
}

fn text(value: &str) -> DiagArg {
    DiagArg::Str(value.to_owned())
}

fn assert_out_of_range(problem: &Problem, value: &str, min: &str, max: &str) {
    assert_eq!(problem.code, ASM_VALUE_OUT_OF_RANGE, "{problem:?}");
    assert_eq!(arg(problem, "value"), &text(value), "{problem:?}");
    assert_eq!(arg(problem, "min"), &text(min), "{problem:?}");
    assert_eq!(arg(problem, "max"), &text(max), "{problem:?}");
}

/// A spelling, a size, operands, the entry selected and the words.
type SelectionCase = (
    &'static str,
    Option<Size>,
    Vec<Operand>,
    &'static str,
    Vec<u16>,
);

/// A spelling, a size, operands and the `value`, `min`, `max` arguments of the range problem.
type RangeCase = (&'static str, Option<Size>, Vec<Operand>, [&'static str; 3]);

const B: Option<Size> = Some(Size::Byte);
const W: Option<Size> = Some(Size::Word);
const L: Option<Size> = Some(Size::Long);
const S: Option<Size> = Some(Size::Short);

/// The golden opwords of m1 §4.7 (observed in a68k listings), assembled from operands.
#[test]
fn golden_opwords_of_m1() {
    // Branch targets: displacement = target - ($1000 + 2).
    let back = |d: i64| abs(0x1002 + d);
    let cases: Vec<(u16, &str, Option<Size>, Vec<Operand>)> = vec![
        (0x4E71, "NOP", None, vec![]),
        (0x4E75, "RTS", None, vec![]),
        (0x4AFC, "ILLEGAL", None, vec![]),
        (0x747F, "MOVEQ", None, vec![imm(127), d(2)]),
        (0x2200, "MOVE", L, vec![d(0), d(1)]),
        (0x3040, "MOVEA", None, vec![d(0), a(0)]),
        (0x51C8, "DBF", None, vec![d(0), back(-2)]),
        (0x64D8, "BCC", S, vec![back(-40)]),
        (0x6500, "BCS", W, vec![back(0x100)]),
        (0x60DC, "BRA", S, vec![back(-36)]),
        (0x61C4, "BSR", S, vec![back(-60)]),
        (0x55C0, "SCS", None, vec![d(0)]),
        (0xB308, "CMPM", B, vec![post(0), post(1)]),
        (0xC308, "ABCD", None, vec![pre(0), pre(1)]),
        (0xD380, "ADDX", L, vec![d(0), d(1)]),
        (0xC141, "EXG", None, vec![d(0), d(1)]),
        (0x4840, "SWAP", None, vec![d(0)]),
        (0x4880, "EXT", W, vec![d(0)]),
        (0x48C0, "EXT", L, vec![d(0)]),
        (0x43D0, "LEA", None, vec![ind(0), a(1)]),
        (0x4E68, "MOVE", None, vec![Operand::Usp, a(0)]),
        (0x4E60, "MOVE", None, vec![a(0), Operand::Usp]),
        (0x44C0, "MOVE", None, vec![d(0), Operand::Ccr]),
        (0x40C0, "MOVE", None, vec![Operand::Sr, d(0)]),
        (0x007C, "ORI", None, vec![imm(0x0700), Operand::Sr]),
        (0x023C, "ANDI", None, vec![imm(0xFE), Operand::Ccr]),
        (0x0A3C, "EORI", None, vec![imm(1), Operand::Ccr]),
        (0x4190, "CHK", None, vec![ind(0), d(0)]),
        (0x48E7, "MOVEM", L, vec![list(0x4001), pre(7)]),
        (0x4C9F, "MOVEM", W, vec![post(7), list(0x0003)]),
        (0x0108, "MOVEP", W, vec![disp(4, 0), d(0)]),
        (0x5240, "ADDQ", W, vec![imm(1), d(0)]),
        (0x5181, "SUBQ", L, vec![imm(8), d(1)]),
        (0x0681, "ADDI", L, vec![imm(5), d(1)]),
        (0xD0C0, "ADDA", W, vec![d(0), a(0)]),
        (0x0C50, "CMPI", W, vec![imm(3), ind(0)]),
        (0xE348, "LSL", W, vec![imm(1), d(0)]),
        (0xE3D0, "LSL", None, vec![ind(0)]),
        (0xE362, "ASL", W, vec![d(1), d(2)]),
        (0x0800, "BTST", None, vec![imm(3), d(0)]),
        (0x0310, "BTST", None, vec![d(1), ind(0)]),
        (0x08D0, "BSET", None, vec![imm(7), ind(0)]),
        (0x4E56, "LINK", None, vec![a(6), imm(-8)]),
        (0x4E4F, "TRAP", None, vec![imm(15)]),
        (0x4E72, "STOP", None, vec![imm(0x2700)]),
        (0x4A40, "TST", W, vec![d(0)]),
        (0x4250, "CLR", W, vec![ind(0)]),
    ];
    for (opword, spelling, size, items) in cases {
        let words = words(spelling, size, items);
        assert_eq!(words[0], opword, "{spelling}: {words:04X?}");
    }
}

/// The selection examples of §4.10 and the `-n` encodings of §12.2.
#[test]
fn selection_follows_also_accepts_then_table_order() {
    let cases: Vec<SelectionCase> = vec![
        ("ADD", W, vec![imm(3), d(3)], "ADDI", vec![0x0643, 0x0003]),
        ("ADD", W, vec![imm(3), a(0)], "ADDA", vec![0xD0FC, 0x0003]),
        ("CMP", W, vec![imm(3), a(0)], "CMPA", vec![0xB0FC, 0x0003]),
        ("MOVE", W, vec![imm(5), a(0)], "MOVEA", vec![0x307C, 0x0005]),
        ("AND", W, vec![imm(3), d(0)], "ANDI", vec![0x0240, 0x0003]),
        ("EXG", None, vec![a(0), d(0)], "EXG", vec![0xC188]),
        ("EXG", None, vec![d(0), a(0)], "EXG", vec![0xC188]),
        ("EXG", None, vec![a(1), a(2)], "EXG", vec![0xC34A]),
        // The data register is Rx (bits 11-9) in either order.
        ("EXG", None, vec![a(1), d(2)], "EXG", vec![0xC589]),
        ("EXG", None, vec![d(2), a(1)], "EXG", vec![0xC589]),
        ("EXG", None, vec![d(3), d(4)], "EXG", vec![0xC744]),
        ("MOVE", L, vec![imm(5), a(0)], "MOVEA", vec![0x207C, 0, 5]),
        ("MOVE", L, vec![imm(5), d(0)], "MOVE", vec![0x203C, 0, 5]),
        ("CMP", None, vec![post(0), post(1)], "CMPM", vec![0xB348]),
        ("OR", L, vec![d(0), d(1)], "OR", vec![0x8280]),
        ("ADD", L, vec![d(1), ind(0)], "ADD", vec![0xD390]),
        ("ADD", W, vec![imm(-3), d(0)], "ADDI", vec![0x0640, 0xFFFD]),
        ("ADD", W, vec![imm(0), d(0)], "ADDI", vec![0x0640, 0x0000]),
        (
            "MOVE",
            W,
            vec![abs_w(0x400), d(0)],
            "MOVE",
            vec![0x3038, 0x0400],
        ),
        (
            "MOVE",
            W,
            vec![d(0), abs(0x400)],
            "MOVE",
            vec![0x33C0, 0, 0x0400],
        ),
        (
            "MOVE",
            W,
            vec![abs(0x400), d(0)],
            "MOVE",
            vec![0x3039, 0, 0x0400],
        ),
        ("TST", W, vec![abs(0x400)], "TST", vec![0x4A79, 0, 0x0400]),
        (
            "JMP",
            None,
            vec![abs(0x400)],
            "JMP",
            vec![0x4EF9, 0, 0x0400],
        ),
        (
            "LEA",
            None,
            vec![disp(4, 0), a(0)],
            "LEA",
            vec![0x41E8, 0x0004],
        ),
        (
            "MOVE",
            W,
            vec![disp(0, 0), d(0)],
            "MOVE",
            vec![0x3028, 0x0000],
        ),
        // A one-register MOVEM stays a correct MOVEM without optimization (§4.11).
        (
            "MOVEM",
            W,
            vec![d(0), ind(0)],
            "MOVEM",
            vec![0x4890, 0x0001],
        ),
        (
            "MOVEM",
            W,
            vec![ind(0), d(0)],
            "MOVEM",
            vec![0x4C90, 0x0001],
        ),
        // Lowercase spellings resolve too; no suffix takes the default size.
        ("move", None, vec![d(0), d(1)], "MOVE", vec![0x3200]),
        ("add", None, vec![a(1), d(2)], "ADD", vec![0xD449]),
    ];
    for (spelling, size, items, entry, expected) in cases {
        let ops = spans(items.clone());
        let choice = select(spelling, size, &ops).unwrap();
        assert_eq!(choice.entry.name, entry, "{spelling} {size:?}");
        assert_eq!(
            words(spelling, size, items),
            expected,
            "{spelling} {size:?}"
        );
    }
}

#[test]
fn choice_carries_condition_and_size() {
    let label = vec![abs(0x1000)];
    let choice = select("BEQ", None, &spans(label.clone())).unwrap();
    let (bcc, _) = instruction_of("BEQ").unwrap();
    assert_eq!(
        choice,
        Choice {
            entry: bcc,
            form: 1,
            condition: Some(7),
            size: W,
        }
    );
    // On a branch, `.B` means `.S` and `.L` means `.W` (A68k); `.S` and `.W` as written.
    assert_eq!(select("BRA", B, &spans(label.clone())).unwrap().size, S);
    assert_eq!(select("BRA", L, &spans(label.clone())).unwrap().size, W);
    assert_eq!(
        select("BHS", S, &spans(label.clone())).unwrap().condition,
        Some(4)
    );
    assert_eq!(select("BRA", None, &spans(label)).unwrap().condition, None);
    let choice = select("SEQ", None, &spans(vec![d(0)])).unwrap();
    assert_eq!((choice.condition, choice.size), (Some(7), B));
    let choice = select("DBRA", None, &spans(vec![d(0), abs(0x1000)])).unwrap();
    assert_eq!((choice.condition, choice.size), (Some(1), W));
    assert_eq!(select("NOP", None, &[]).unwrap().size, None);
    assert_eq!(
        select("LEA", L, &spans(vec![ind(0), a(1)])).unwrap().size,
        L
    );
}

#[test]
fn every_place_is_encoded() {
    // Ea: every mode of a MOVE source, extension words in operand order.
    assert_eq!(words("MOVE", W, vec![a(3), d(1)]), [0x320B]);
    assert_eq!(words("MOVE", W, vec![post(4), pre(5)]), [0x3B1C]);
    assert_eq!(
        words("MOVE", W, vec![index(0x10, 0, 1, false), d(2)]),
        [0x3430, 0x1010]
    );
    assert_eq!(
        words("MOVE", L, vec![abs_l(0x12345), abs_l(0x0F_0000)]),
        [0x23F9, 0x0001, 0x2345, 0x000F, 0x0000]
    );
    assert_eq!(
        words("MOVE", B, vec![disp(-2, 6), abs_w(0x1234)]),
        [0x11EE, 0xFFFE, 0x1234]
    );
    assert_eq!(words("MOVE", B, vec![imm(-1), d(0)]), [0x103C, 0x00FF]);
    // d16(PC): the expression is the target; displacement from the extension word.
    assert_eq!(
        words_in(&at(0x100A), "MOVE", W, vec![pc(0x1010), d(0)]),
        [0x303A, 0x0004]
    );
    // d8(PC,Xn) with an address register index, long.
    assert_eq!(
        words("LEA", None, vec![pc_index(0x1010, 11, true), a(0)]),
        [0x41FB, 0xB80E]
    );
    // Reg9 and Reg0, and MOVEP's d16(An) in a register field.
    assert_eq!(
        words("MOVEP", W, vec![disp(0x10, 0), d(0)]),
        [0x0108, 0x0010]
    );
    assert_eq!(words("MOVEP", L, vec![d(3), disp(-4, 1)]), [0x07C9, 0xFFFC]);
    // Quick9: 8 is stored as 0.
    assert_eq!(words("ADDQ", L, vec![imm(8), d(1)]), [0x5081]);
    assert_eq!(words("LSR", L, vec![imm(8), d(7)]), [0xE08F]);
    assert_eq!(words("SUBQ", W, vec![imm(1), a(0)]), [0x5348]);
    // Data8 and Vector.
    assert_eq!(words("MOVEQ", None, vec![imm(-1), d(0)]), [0x70FF]);
    assert_eq!(words("MOVEQ", L, vec![imm(-128), d(7)]), [0x7E80]);
    assert_eq!(words("TRAP", None, vec![imm(0)]), [0x4E40]);
    // Imm by operation size (`.B`: the low byte of one word).
    assert_eq!(words("ADDI", B, vec![imm(-1), d(0)]), [0x0600, 0x00FF]);
    assert_eq!(words("ADDI", W, vec![imm(0xFFFF), d(0)]), [0x0640, 0xFFFF]);
    assert_eq!(
        words("ADDI", L, vec![imm(0x1234_5678), d(0)]),
        [0x0680, 0x1234, 0x5678]
    );
    assert_eq!(
        words("CMPI", L, vec![imm(-2), disp(8, 2)]),
        [0x0CAA, 0xFFFF, 0xFFFE, 0x0008]
    );
    // ImmWord: STOP, LINK, the SR forms.
    assert_eq!(words("STOP", None, vec![imm(0x2700)]), [0x4E72, 0x2700]);
    assert_eq!(words("LINK", None, vec![a(6), imm(-8)]), [0x4E56, 0xFFF8]);
    assert_eq!(
        words("ORI", None, vec![imm(0x8000), Operand::Sr]),
        [0x007C, 0x8000]
    );
    // ImmByte: the CCR forms and bit numbers.
    assert_eq!(
        words("ANDI", None, vec![imm(-2), Operand::Ccr]),
        [0x023C, 0x00FE]
    );
    assert_eq!(words("BTST", None, vec![imm(31), d(0)]), [0x0800, 0x001F]);
    assert_eq!(
        words("BCHG", None, vec![imm(7), disp(2, 1)]),
        [0x0869, 0x0007, 0x0002]
    );
    // MOVE to CCR takes a word immediate.
    assert_eq!(
        words("MOVE", None, vec![imm(0x1F), Operand::Ccr]),
        [0x44FC, 0x001F]
    );
    // Implied: the SR, CCR and USP forms carry no operand bits.
    assert_eq!(words("MOVE", None, vec![Operand::Sr, ind(2)]), [0x40D2]);
    assert_eq!(words("MOVE", None, vec![Operand::Usp, a(5)]), [0x4E6D]);
    // Disp8 and Disp16.
    assert_eq!(words("BRA", S, vec![abs(0x1002 + 0x7E)]), [0x607E]);
    assert_eq!(words("BRA", S, vec![abs(0x1002 - 128)]), [0x6080]);
    assert_eq!(
        words("BNE", W, vec![abs(0x1002 + 0x7FFE)]),
        [0x6600, 0x7FFE]
    );
    assert_eq!(
        words("DBRA", None, vec![d(1), abs(0x1000)]),
        [0x51C9, 0xFFFE]
    );
    assert_eq!(words("BSR", L, vec![abs(0x0F00)]), [0x6100, 0xFEFE]);
}

#[test]
fn movem_mask_is_reversed_for_predecrement_and_comes_first() {
    // d0-d2/a3 = $0807; -(An) reads the mask bit-reversed.
    assert_eq!(
        words("MOVEM", L, vec![list(0x0807), pre(7)]),
        [0x48E7, 0xE010]
    );
    assert_eq!(
        words("MOVEM", L, vec![list(0x0807), ind(0)]),
        [0x48D0, 0x0807]
    );
    assert_eq!(
        words("MOVEM", W, vec![post(7), list(0x0807)]),
        [0x4C9F, 0x0807]
    );
    // The mask word precedes the EA extension, also when the list is the second operand.
    assert_eq!(
        words("MOVEM", W, vec![disp(0x10, 6), list(0x0003)]),
        [0x4CAE, 0x0003, 0x0010]
    );
    assert_eq!(
        words("MOVEM", W, vec![pc(0x1010), list(0x0003)]),
        [0x4CBA, 0x0003, 0x000C]
    );
    // A single register is a one-register list.
    assert_eq!(words("MOVEM", L, vec![a(2), pre(7)]), [0x48E7, 0x0020]);
    assert_eq!(
        words("MOVEM", L, vec![list(0x0100), ind(1)]),
        [0x48D1, 0x0100]
    );
}

#[test]
fn branch_problems() {
    let problem = encode_error("BRA", S, vec![abs(0x1002 + 128)]);
    assert_eq!(problem.code, ASM_BRANCH_OUT_OF_RANGE);
    assert_eq!(arg(&problem, "distance"), &DiagArg::Int(128));
    assert_eq!(arg(&problem, "size"), &text("S"));
    assert_eq!(problem.range, TextRange::new(0, 4));
    let problem = encode_error("BEQ", S, vec![abs(0x1002 - 129)]);
    assert_eq!(arg(&problem, "distance"), &DiagArg::Int(-129));
    let problem = encode_error("BSR", W, vec![abs(0x1002 + 32768)]);
    assert_eq!(problem.code, ASM_BRANCH_OUT_OF_RANGE);
    assert_eq!(arg(&problem, "size"), &text("W"));
    let problem = encode_error("DBRA", None, vec![d(0), abs(0x1002 + 40000)]);
    assert_eq!(arg(&problem, "distance"), &DiagArg::Int(40000));
    assert_eq!(problem.range, TextRange::new(10, 14));
    // An explicit `.S` to the next instruction would have a zero displacement.
    let problem = encode_error("BRA", S, vec![abs(0x1002)]);
    assert_eq!(problem.code, ASM_SHORT_BRANCH_ZERO);
    assert!(problem.args.is_empty());
    // A target past the 24-bit address space.
    let problem = encode_error("BRA", W, vec![abs(0x0100_0000)]);
    assert_out_of_range(&problem, "$1000000", "$0", "$FFFFFF");
}

#[test]
fn an_odd_branch_target_warns_after_the_words() {
    for (size, expected) in [(W, vec![0x6000, 0xFFFF]), (S, vec![0x6003])] {
        let target = if size == W { 0x1001 } else { 0x1005 };
        let ops = spans(vec![abs(target)]);
        let choice = select("BRA", size, &ops).unwrap();
        let mut out = Vec::new();
        let warning = encode(&choice, &ops, &at(0x1000), 0x1000, &mut out).unwrap_err();
        assert_eq!(warning.code, ASM_ODD_BRANCH_TARGET);
        assert_eq!(arg(&warning, "target"), &text(&format!("${target:X}")));
        assert_eq!(out, expected, "the words are kept with the warning");
    }
}

#[test]
fn values_out_of_range() {
    let cases: Vec<RangeCase> = vec![
        ("ADDQ", W, vec![imm(9), d(0)], ["9", "1", "8"]),
        ("ADDQ", W, vec![imm(0), d(0)], ["0", "1", "8"]),
        ("LSL", W, vec![imm(9), d(0)], ["9", "1", "8"]),
        ("MOVEQ", None, vec![imm(128), d(0)], ["128", "-128", "127"]),
        ("TRAP", None, vec![imm(16)], ["16", "0", "15"]),
        ("ADDI", B, vec![imm(256), d(0)], ["256", "-128", "255"]),
        ("ADDI", B, vec![imm(-129), d(0)], ["-129", "-128", "255"]),
        (
            "ADDI",
            W,
            vec![imm(65536), d(0)],
            ["65536", "-32768", "65535"],
        ),
        (
            "ADDI",
            W,
            vec![imm(-32769), d(0)],
            ["-32769", "-32768", "65535"],
        ),
        (
            "MOVE",
            W,
            vec![disp(32768, 0), d(0)],
            ["32768", "-32768", "32767"],
        ),
        (
            "MOVE",
            W,
            vec![index(128, 0, 0, false), d(1)],
            ["128", "-128", "127"],
        ),
        (
            "MOVE",
            W,
            vec![abs_w(0x8000), d(0)],
            ["32768", "-32768", "32767"],
        ),
        (
            "JMP",
            None,
            vec![abs(0x0100_0000)],
            ["$1000000", "$0", "$FFFFFF"],
        ),
        ("JMP", None, vec![abs(-2)], ["$FFFFFFFE", "$0", "$FFFFFF"]),
        (
            "LINK",
            None,
            vec![a(6), imm(32768)],
            ["32768", "-32768", "32767"],
        ),
        ("STOP", None, vec![imm(65536)], ["65536", "-32768", "65535"]),
        (
            "ANDI",
            None,
            vec![imm(256), Operand::Ccr],
            ["256", "-128", "255"],
        ),
        ("BTST", None, vec![imm(256), d(0)], ["256", "0", "255"]),
        ("BSET", None, vec![imm(-1), ind(0)], ["-1", "0", "255"]),
        (
            "MOVE",
            W,
            vec![pc(0x1002 + 32768), d(0)],
            ["32768", "-32768", "32767"],
        ),
        (
            "MOVE",
            W,
            vec![pc_index(0x1002 + 128, 0, false), d(0)],
            ["128", "-128", "127"],
        ),
        (
            "MOVEP",
            W,
            vec![disp(-32769, 0), d(0)],
            ["-32769", "-32768", "32767"],
        ),
    ];
    for (spelling, size, items, [value, min, max]) in cases {
        let problem = encode_error(spelling, size, items);
        assert_out_of_range(&problem, value, min, max);
    }
    // The problem covers the operand whose value does not fit.
    let problem = encode_error("MOVE", W, vec![d(0), disp(-40000, 1)]);
    assert_eq!(problem.range, TextRange::new(10, 14));
    // Values are 32-bit: ($FFFF8000).w is -32768, #$FFFFFFFF.w is -1.
    assert_eq!(
        words("MOVE", W, vec![abs_w(0xFFFF_8000), d(0)]),
        [0x3038, 0x8000]
    );
    assert_eq!(
        words("ADDI", W, vec![imm(0xFFFF_FFFF), d(0)]),
        [0x0640, 0xFFFF]
    );
}

#[test]
fn symbols_resolve_through_the_scope() {
    let scope = Symbols {
        here: 0x2000,
        symbols: vec![
            ("back", Lookup::Known(0x1F00)),
            ("later", Lookup::Later(0x2010)),
            ("value", Lookup::Known(0xFFFF_FFFF)),
        ],
    };
    let label = |name: &str| Operand::Ea(Ea::Abs(sym(name), None));
    assert_eq!(
        words_in(&scope, "BRA", W, vec![label("back")]),
        [0x6000, 0xFEFE]
    );
    assert_eq!(words_in(&scope, "BSR", S, vec![label("later")]), [0x610E]);
    assert_eq!(
        words_in(
            &scope,
            "MOVE",
            L,
            vec![Operand::Ea(Ea::Imm(sym("value"))), d(0)]
        ),
        [0x203C, 0xFFFF, 0xFFFF]
    );
    // `*` is the location counter.
    assert_eq!(
        words_in(
            &scope,
            "JMP",
            None,
            vec![Operand::Ea(Ea::Abs(Expr::Here, None))]
        ),
        [0x4EF9, 0x0000, 0x2000]
    );
    assert_eq!(
        words_in(
            &scope,
            "MOVE",
            W,
            vec![Operand::Ea(Ea::Imm(Expr::Char(0x6162))), d(1)]
        ),
        [0x323C, 0x6162]
    );
}

#[test]
fn length_depends_on_operand_shapes_only() {
    let cases: Vec<(&str, Option<Size>, Vec<Operand>, u32)> = vec![
        ("NOP", None, vec![], 2),
        ("MOVE", L, vec![abs_l(1), abs_l(2)], 10),
        ("MOVE", L, vec![imm(1), d(0)], 6),
        ("MOVE", W, vec![abs(0x400), d(0)], 6),
        ("MOVE", W, vec![abs_w(0x400), d(0)], 4),
        ("MOVEM", W, vec![disp(0x10, 6), list(3)], 6),
        ("MOVEP", W, vec![disp(0x10, 0), d(0)], 4),
        ("BRA", S, vec![abs(0x1010)], 2),
        ("BRA", W, vec![abs(0x1010)], 4),
        ("DBRA", None, vec![d(0), abs(0x1010)], 4),
        ("ADDI", L, vec![imm(1), disp(2, 0)], 8),
        ("ANDI", None, vec![imm(1), Operand::Ccr], 4),
        ("STOP", None, vec![imm(0)], 4),
        ("BTST", None, vec![imm(1), abs(0x400)], 8),
    ];
    for (spelling, size, items, bytes) in cases {
        let ops = spans(items);
        let choice = select(spelling, size, &ops).unwrap();
        assert_eq!(length(&choice, &ops), bytes, "{spelling} {size:?}");
    }
}

fn assert_code(problem: &Problem, code: DiagCode, range: TextRange) {
    assert_eq!(problem.code, code, "{problem:?}");
    assert_eq!(problem.range, range, "{problem:?}");
}

#[test]
fn selection_reports_the_closest_failure() {
    // The count fits no form: the count nearest to the one written.
    let problem = select_error("MOVE", W, vec![d(0)]);
    assert_code(&problem, ASM_OPERAND_COUNT, TextRange::new(0, 4));
    assert_eq!(arg(&problem, "mnemonic"), &text("MOVE"));
    assert_eq!(arg(&problem, "expected"), &DiagArg::Int(2));
    assert_eq!(arg(&problem, "found"), &DiagArg::Int(1));
    let problem = select_error("LSL", W, vec![d(0), d(1), d(2)]);
    assert_eq!(arg(&problem, "expected"), &DiagArg::Int(2));
    let problem = select_error("NOP", None, vec![d(0)]);
    assert_eq!(arg(&problem, "expected"), &DiagArg::Int(0));
    // The operands fit but the size does not: the sizes of the forms they fit.
    let problem = select_error("CHK", L, vec![d(0), d(1)]);
    assert_code(&problem, ASM_SIZE_NOT_ALLOWED, TextRange::new(0, 14));
    assert_eq!(arg(&problem, "size"), &text("L"));
    assert_eq!(arg(&problem, "allowed"), &text(".W"));
    let problem = select_error("MOVE", L, vec![d(0), Operand::Sr]);
    assert_eq!(arg(&problem, "allowed"), &text(".W"));
    let problem = select_error("MOVE", B, vec![a(0), d(0)]);
    assert_eq!(problem.code, ASM_SIZE_NOT_ALLOWED);
    assert_eq!(arg(&problem, "allowed"), &text(".W, .L"));
    let problem = select_error("DBRA", L, vec![d(0), abs(0x1000)]);
    assert_eq!(arg(&problem, "allowed"), &text(".W"));
    let problem = select_error("NOP", W, vec![]);
    assert_code(&problem, ASM_SIZE_NOT_ALLOWED, TextRange::new(0, 0));
    assert_eq!(arg(&problem, "allowed"), &text("-"));
    // Only a mode does not fit: every mode accepted at that operand.
    let problem = select_error("ADD", W, vec![d(0), imm(3)]);
    assert_code(&problem, ASM_MODE_NOT_ALLOWED, TextRange::new(10, 14));
    assert_eq!(arg(&problem, "mode"), &text("#imm"));
    assert_eq!(
        arg(&problem, "allowed"),
        &text("Dn, An, (An), (An)+, -(An), d16(An), d8(An,Xn), xxx.W, xxx.L")
    );
    let problem = select_error("TST", W, vec![a(0)]);
    assert_eq!(arg(&problem, "mode"), &text("An"));
    let problem = select_error("BRA", None, vec![ind(0)]);
    assert_eq!(arg(&problem, "mode"), &text("(An)"));
    assert_eq!(arg(&problem, "allowed"), &text("<label>"));
    let problem = select_error("MOVEM", W, vec![ind(0), ind(1)]);
    assert_code(&problem, ASM_MODE_NOT_ALLOWED, TextRange::new(0, 4));
    assert_eq!(arg(&problem, "allowed"), &text("<list>"));
    let problem = select_error("JMP", None, vec![d(0)]);
    assert_eq!(
        arg(&problem, "allowed"),
        &text("(An), d16(An), d8(An,Xn), xxx.W, xxx.L, d16(PC), d8(PC,Xn)")
    );
    // A kind fits no form: the forms of every candidate entry.
    let problem = select_error("EXG", None, vec![d(0), Operand::Ccr]);
    assert_code(&problem, ASM_NO_MATCHING_FORM, TextRange::new(0, 14));
    assert_eq!(arg(&problem, "forms"), &text("Dn,Dn; An,An; Dn,An"));
    let problem = select_error("MOVE", W, vec![Operand::Ccr, d(0)]);
    assert_eq!(problem.code, ASM_NO_MATCHING_FORM);
    let problem = select_error("ADD", W, vec![Operand::Bytes(vec![1]), d(0)]);
    assert_eq!(
        arg(&problem, "forms"),
        &text("<ea>,An; #<data>,<ea>; <ea>,Dn; Dn,<ea>")
    );
    // A spelling that is no instruction.
    let problem = select_error("FOO", None, vec![]);
    assert_eq!(problem.code, ASM_UNKNOWN_OPERATION);
    assert_eq!(arg(&problem, "name"), &text("FOO"));
}
