//! Tests for `src/isa/timing.rs`: row partitions, cycle sanity, EA times, MOVE matrices and
//! `cycles()` spot checks against the User's Manual §8 (m1 §4.4, §4.7).
#![allow(clippy::unwrap_used)]

use va_arch_m68k::isa::{
    EA_TIME, Form, MOVE_BW, MOVE_L, Mode, Operand, Outcome, Size, Timing, cycles, instruction_of,
};
use va_core::isa::{Bound, Cycles};

use crate::{all_forms, form_of};

fn cost(clocks: u16, reads: u8, writes: u8) -> Cycles {
    Cycles {
        clocks,
        reads,
        writes,
    }
}

fn add(a: Cycles, b: Cycles) -> Cycles {
    cost(a.clocks + b.clocks, a.reads + b.reads, a.writes + b.writes)
}

/// The sizes to try on a form: its sizes, or `None` once for an unsized form.
fn sizes_of(form: &Form) -> Vec<Option<Size>> {
    if form.sizes.is_empty() {
        vec![None]
    } else {
        form.sizes.iter().map(Some).collect()
    }
}

/// The modes to try on an operand: its modes, or `None` once when it has none.
fn modes_of(operand: Option<&Operand>) -> Vec<Option<Mode>> {
    let modes = operand.map(|op| op.modes()).unwrap_or_default();
    if modes.is_empty() {
        vec![None]
    } else {
        modes.iter().map(Some).collect()
    }
}

#[test]
fn rows_partition_size_mode_and_outcome() {
    for (entry, index, form) in all_forms() {
        let at = format!("{} form {index}", entry.name);
        assert!(!form.outcomes.is_empty(), "{at}: no outcome");
        if form.outcomes.contains(&Outcome::Always) {
            assert_eq!(
                form.outcomes,
                [Outcome::Always],
                "{at}: Always mixed with outcomes"
            );
        }
        let rows = match form.timing {
            Timing::Rows(rows) => rows,
            Timing::MoveMatrix => {
                assert_eq!(form.outcomes, [Outcome::Always], "{at}");
                assert_eq!(form.operands.len(), 2, "{at}");
                assert!(matches!(form.operands[0], Operand::Ea(_)), "{at}");
                for size in sizes_of(form) {
                    for source in modes_of(form.operands.first()) {
                        for destination in modes_of(form.operands.get(1)) {
                            let total = cycles(
                                form,
                                size.unwrap(),
                                [source, destination],
                                Outcome::Always,
                                0,
                            );
                            assert!(total.is_some(), "{at}: {source:?} -> {destination:?}");
                        }
                    }
                }
                continue;
            }
        };
        assert!(!rows.is_empty(), "{at}: no row");
        let operand = rows[0].operand;
        let mode_independent = rows.iter().all(|row| row.modes.is_empty());
        for row in rows {
            assert_eq!(row.operand, operand, "{at}: several varying operands");
            assert!(
                row.sizes.is_subset(form.sizes),
                "{at}: row size outside the form"
            );
            assert_eq!(row.sizes.is_empty(), form.sizes.is_empty(), "{at}");
            assert!(
                form.outcomes.contains(&row.outcome),
                "{at}: undeclared outcome"
            );
            if !row.modes.is_empty() {
                let declared = form.operands[usize::from(operand)].modes();
                assert!(
                    row.modes.is_subset(declared),
                    "{at}: row modes outside the operand"
                );
            }
            if row.plus_ea {
                assert!(
                    matches!(form.operands[usize::from(operand)], Operand::Ea(_)),
                    "{at}: EA time of a non-EA operand"
                );
                assert!(!row.modes.is_empty(), "{at}: EA time without a mode");
            }
        }
        let modes = if mode_independent {
            vec![None]
        } else {
            modes_of(form.operands.get(usize::from(operand)))
        };
        for size in sizes_of(form) {
            for &mode in &modes {
                for &outcome in form.outcomes {
                    let count = rows
                        .iter()
                        .filter(|row| row.applies(size, mode, outcome))
                        .count();
                    assert_eq!(count, 1, "{at}: {size:?} {mode:?} {outcome:?}");
                }
            }
        }
    }
}

fn plausible(cycles: Cycles) -> bool {
    cycles.clocks.is_multiple_of(2)
        && u32::from(cycles.clocks) >= 4 * (u32::from(cycles.reads) + u32::from(cycles.writes))
}

#[test]
fn every_cost_has_even_clocks_and_room_for_its_bus_cycles() {
    for (entry, index, form) in all_forms() {
        if let Timing::Rows(rows) = form.timing {
            for row in rows {
                assert!(
                    plausible(row.base),
                    "{} form {index}: {:?}",
                    entry.name,
                    row.base
                );
                if let Some((per, _)) = row.per {
                    assert!(plausible(per), "{} form {index}: {per:?}", entry.name);
                }
            }
        }
    }
    let ea = EA_TIME.iter().flatten();
    let moves = MOVE_BW.iter().chain(MOVE_L.iter()).flatten();
    for cell in ea.chain(moves) {
        assert!(plausible(*cell), "{cell:?}");
    }
}

#[test]
fn ea_time_spot_checks() {
    let checks = [
        (Mode::Dn, cost(0, 0, 0), cost(0, 0, 0)),
        (Mode::An, cost(0, 0, 0), cost(0, 0, 0)),
        (Mode::Ind, cost(4, 1, 0), cost(8, 2, 0)),
        (Mode::PreDec, cost(6, 1, 0), cost(10, 2, 0)),
        (Mode::Index, cost(10, 2, 0), cost(14, 3, 0)),
        (Mode::AbsL, cost(12, 3, 0), cost(16, 4, 0)),
        (Mode::Imm, cost(4, 1, 0), cost(8, 2, 0)),
    ];
    for (mode, byte_word, long) in checks {
        assert_eq!(EA_TIME[mode.index()], [byte_word, long], "{mode:?}");
    }
}

#[test]
fn move_matrices_are_the_dn_row_plus_the_source_ea_time() {
    for source in Mode::ALL {
        for column in 0..9 {
            let bw = add(
                MOVE_BW[Mode::Dn.index()][column],
                EA_TIME[source.index()][0],
            );
            assert_eq!(
                MOVE_BW[source.index()][column],
                bw,
                "B/W {source:?} col {column}"
            );
            let long = add(MOVE_L[Mode::Dn.index()][column], EA_TIME[source.index()][1]);
            assert_eq!(
                MOVE_L[source.index()][column],
                long,
                "L {source:?} col {column}"
            );
        }
    }
}

/// `cycles()` of the form of `spelling` whose operands read `operands` and which accepts `size`
/// (or is unsized).
fn total(
    spelling: &str,
    operands: &str,
    size: Size,
    modes: [Option<Mode>; 2],
    outcome: Outcome,
    n: u32,
) -> (Cycles, Bound) {
    let (entry, _) = instruction_of(spelling).unwrap();
    let forms: Vec<&Form> = entry
        .forms
        .iter()
        .filter(|form| crate::operands(form) == operands)
        .filter(|form| form.sizes.is_empty() || form.sizes.contains(size))
        .collect();
    assert_eq!(forms.len(), 1, "{spelling} {operands} {size:?}");
    cycles(forms[0], size, modes, outcome, n).unwrap()
}

#[test]
fn cycles_spot_checks() {
    use Outcome::{Always, Expired, NotExpired, NotTaken, Taken, True};
    use Size::{Long, Short, Word};
    let none = [None, None];
    let dn = Some(Mode::Dn);
    let ind = Some(Mode::Ind);
    let exact = |c, r, w| (cost(c, r, w), Bound::Exact);
    let checks = [
        (total("NOP", "", Word, none, Always, 0), exact(4, 1, 0)),
        (total("RTS", "", Word, none, Always, 0), exact(16, 4, 0)),
        (total("RTE", "", Word, none, Always, 0), exact(20, 5, 0)),
        (
            total("MOVEQ", "#<-128..127>,Dn", Long, none, Always, 0),
            exact(4, 1, 0),
        ),
        (total("EXG", "Dn,Dn", Long, none, Always, 0), exact(6, 1, 0)),
        (total("SWAP", "Dn", Word, none, Always, 0), exact(4, 1, 0)),
        (
            total("LINK", "An,#<data>", Word, none, Always, 0),
            exact(16, 2, 2),
        ),
        (total("UNLK", "An", Word, none, Always, 0), exact(12, 3, 0)),
        (
            total("TRAP", "#<0..15>", Word, none, Always, 0),
            exact(34, 4, 3),
        ),
        (
            total("BEQ", "<label>", Short, none, Taken, 0),
            exact(10, 2, 0),
        ),
        (
            total("BEQ", "<label>", Short, none, NotTaken, 0),
            exact(8, 1, 0),
        ),
        (
            total("BNE", "<label>", Word, none, NotTaken, 0),
            exact(12, 2, 0),
        ),
        (
            total("BSR", "<label>", Short, none, Always, 0),
            exact(18, 2, 2),
        ),
        (
            total("BSR", "<label>", Word, none, Always, 0),
            exact(18, 2, 2),
        ),
        (
            total("DBF", "Dn,<label>", Word, none, True, 0),
            exact(12, 2, 0),
        ),
        (
            total("DBNE", "Dn,<label>", Word, none, NotExpired, 0),
            exact(10, 2, 0),
        ),
        (
            total("DBRA", "Dn,<label>", Word, none, Expired, 0),
            exact(14, 3, 0),
        ),
        (
            total("JSR", "<ea>", Word, [ind, None], Always, 0),
            exact(16, 2, 2),
        ),
        (
            total("JMP", "<ea>", Word, [ind, None], Always, 0),
            exact(8, 2, 0),
        ),
        (
            total("LEA", "<ea>,An", Long, [ind, None], Always, 0),
            exact(4, 1, 0),
        ),
        (
            total("PEA", "<ea>", Long, [ind, None], Always, 0),
            exact(12, 1, 2),
        ),
        (
            total("MOVE", "<ea>,<ea>", Word, [ind, ind], Always, 0),
            exact(12, 2, 1),
        ),
        (
            total("MOVE", "<ea>,<ea>", Long, [ind, ind], Always, 0),
            exact(20, 3, 2),
        ),
        (
            total("ADD", "<ea>,Dn", Word, [dn, None], Always, 0),
            exact(4, 1, 0),
        ),
        (
            total("ADD", "<ea>,Dn", Long, [dn, None], Always, 0),
            exact(8, 1, 0),
        ),
        (
            total("ADD", "<ea>,Dn", Long, [ind, None], Always, 0),
            exact(14, 3, 0),
        ),
        (
            total("LSL", "#<1..8>,Dn", Long, none, Always, 3),
            exact(14, 1, 0),
        ),
        (
            total("MULU", "<ea>,Dn", Word, [dn, None], Always, 16),
            exact(70, 1, 0),
        ),
    ];
    for (index, (found, expected)) in checks.into_iter().enumerate() {
        assert_eq!(found, expected, "spot check {index}");
    }
}

#[test]
fn further_cycles_from_the_manual() {
    use Outcome::{Always, NoTrap, Trap};
    use Size::{Byte, Long, Word};
    let dn = Some(Mode::Dn);
    let an = Some(Mode::An);
    let ind = Some(Mode::Ind);
    let imm = Some(Mode::Imm);
    let pre = Some(Mode::PreDec);
    // DIVU/DIVS: worst case with Bound::Max.
    let divu = total("DIVU", "<ea>,Dn", Word, [dn, None], Always, 0);
    assert_eq!(divu, (cost(140, 1, 0), Bound::Max));
    let divs = total("DIVS", "<ea>,Dn", Word, [ind, None], Always, 0);
    assert_eq!(divs, (cost(162, 2, 0), Bound::Max));
    // Footnote **: ADDA.L and AND.L with a register or immediate source.
    let adda = total("ADDA", "<ea>,An", Long, [imm, None], Always, 0);
    assert_eq!(adda.0, cost(16, 3, 0));
    assert_eq!(
        total("ADDA", "<ea>,An", Word, [an, None], Always, 0).0,
        cost(8, 1, 0)
    );
    assert_eq!(
        total("AND", "<ea>,Dn", Long, [imm, None], Always, 0).0,
        cost(16, 3, 0)
    );
    assert_eq!(
        total("AND", "<ea>,Dn", Long, [ind, None], Always, 0).0,
        cost(14, 3, 0)
    );
    // Immediate forms (table 8-5).
    assert_eq!(
        total("ANDI", "#<data>,<ea>", Long, [None, dn], Always, 0).0,
        cost(14, 3, 0)
    );
    assert_eq!(
        total("ORI", "#<data>,<ea>", Long, [None, dn], Always, 0).0,
        cost(16, 3, 0)
    );
    assert_eq!(
        total("CMPI", "#<data>,<ea>", Long, [None, dn], Always, 0).0,
        cost(14, 3, 0)
    );
    let addi = total("ADDI", "#<data>,<ea>", Word, [None, ind], Always, 0);
    assert_eq!(addi.0, cost(16, 3, 1));
    assert_eq!(
        total("ADDQ", "#<1..8>,An", Word, [None, an], Always, 0).0,
        cost(8, 1, 0)
    );
    assert_eq!(
        total("SUBQ", "#<1..8>,<ea>", Long, [None, dn], Always, 0).0,
        cost(8, 1, 0)
    );
    // Multi-precision (table 8-11).
    assert_eq!(
        total("ADDX", "-(An),-(An)", Long, [pre, pre], Always, 0).0,
        cost(30, 5, 2)
    );
    assert_eq!(
        total("CMPM", "(An)+,(An)+", Byte, [None, None], Always, 0).0,
        cost(12, 3, 0)
    );
    assert_eq!(
        total("ABCD", "-(An),-(An)", Byte, [pre, pre], Always, 0).0,
        cost(18, 3, 1)
    );
    // MOVEM: base plus a cost per register.
    let movem = total("MOVEM", "<ea>,<list>", Long, [ind, None], Always, 2);
    assert_eq!(movem.0, cost(28, 7, 0));
    let movem = total("MOVEM", "<list>,<ea>", Word, [None, pre], Always, 3);
    assert_eq!(movem.0, cost(20, 2, 3));
    // Bit operations: register forms are an upper bound.
    let bclr = total("BCLR", "#<data>,Dn", Long, [None, dn], Always, 0);
    assert_eq!(bclr, (cost(14, 2, 0), Bound::Max));
    assert_eq!(
        total("BTST", "Dn,<ea>", Byte, [dn, imm], Always, 0).0,
        cost(8, 2, 0)
    );
    // CHK and TRAPV outcomes.
    assert_eq!(
        total("CHK", "<ea>,Dn", Word, [dn, None], NoTrap, 0).0,
        cost(10, 1, 0)
    );
    assert_eq!(
        total("CHK", "<ea>,Dn", Word, [dn, None], Trap, 0).0,
        cost(40, 4, 3)
    );
    let trapv = form_of("TRAPV", "", None);
    assert_eq!(
        cycles(trapv, Word, [None, None], Trap, 0).unwrap().0,
        cost(34, 5, 3)
    );
    // MOVE from SR, MOVE to CCR, MOVEP, shifts in memory.
    assert_eq!(
        total("MOVE", "SR,<ea>", Word, [None, dn], Always, 0).0,
        cost(6, 1, 0)
    );
    assert_eq!(
        total("MOVE", "<ea>,CCR", Word, [imm, None], Always, 0).0,
        cost(16, 3, 0)
    );
    assert_eq!(
        total("MOVEP", "Dn,d16(An)", Long, [None, None], Always, 0).0,
        cost(24, 2, 4)
    );
    assert_eq!(
        total("ASR", "<ea>", Word, [ind, None], Always, 0).0,
        cost(12, 2, 1)
    );
    assert_eq!(
        total("ROXL", "Dn,Dn", Byte, [None, None], Always, 5).0,
        cost(16, 1, 0)
    );
}

#[test]
fn cycles_rejects_what_the_form_does_not_accept() {
    let moveq = form_of("MOVEQ", "#<-128..127>,Dn", None);
    assert_eq!(
        cycles(moveq, Size::Byte, [None, None], Outcome::Always, 0),
        None
    );
    let nop = form_of("NOP", "", None);
    assert_eq!(
        cycles(nop, Size::Word, [None, None], Outcome::Taken, 0),
        None
    );
    let jmp = form_of("JMP", "<ea>", None);
    let dn = Some(Mode::Dn);
    assert_eq!(
        cycles(jmp, Size::Word, [dn, None], Outcome::Always, 0),
        None
    );
    let lsl = form_of("LSL", "#<1..8>,Dn", None);
    assert_eq!(
        cycles(lsl, Size::Long, [None, None], Outcome::Always, u32::MAX),
        None,
        "overflow is None"
    );
}
