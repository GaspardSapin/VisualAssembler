//! The optimizations of a68k (m2-assembler.md §4.2, frozen; §4.10): `addq`, `subq`, `moveq`,
//! `lea`, `addq-lea`, `subq-lea`, `zero-displacement`, `pc-relative` and `movem-move`, each
//! applied only to the generic spelling and only with `AsmOptions::optimize`. Branch shortening
//! (`short-branch`) is the relaxation of `asm::layout`.
//!
//! **Rules** (observed on a68k, §12.2):
//!
//! | Id | Rewrite |
//! |---|---|
//! | `moveq` | `MOVE.L #n,Dn`, `n` known in −128…127 → MOVEQ |
//! | `addq`, `subq` | `ADD`/`SUB #n,<ea>` (ADDI/SUBI or ADDA/SUBA selected), `n` known in 1…8 → ADDQ/SUBQ, same size |
//! | `lea` | `ADD`/`SUB #n,An` (ADDA/SUBA) → `LEA ±n(An),An`: `.L` with `n` known in 9…32767; `.W` with `n` known in 9…32767, or not known but valued (pass 2) in 1…32767 |
//! | `addq-lea`, `subq-lea` | `LEA d(An),An` (same register), `d` known in 1…8 → `ADDQ.W #d,An`; −8…−1 → `SUBQ.W #-d,An` |
//! | `zero-displacement` | `e(An)` with `e` known 0, where the form accepts `(An)` → `(An)` (`0(An,Xn)` and MOVEP's `d16(An)` stay) |
//! | `pc-relative` | absolute address without `.W`/`.L`, known, where the form accepts `d16(PC)` (never a destination: those sets are alterable), displacement in −32768…32767 → `d16(PC)` |
//! | `movem-move` | MOVEM of one register → MOVE (or MOVEA for memory → `An`) of the same size and EA |
//!
//! "Known" is [`Value::known`](super::expr::Value): every symbol defined on an earlier
//! statement. A rule that changes the length uses known values only (the pass-1 sizing rule);
//! the only rule that may use a value not known yet is the same-length `.W` case of `lea`.
//!
//! **One call does all.** The instruction rule (at most one applies) runs first, then the
//! operand rules (`zero-displacement`, `pc-relative`) on its result, so the displacement of a
//! `pc-relative` operand is computed for the final instruction. The id returned is the
//! instruction rule's, else the first operand rule's.
//!
//! **`asm-movem-single`.** The driver reports this warning for a `movem-move` result whose entry
//! is MOVE (never for MOVEA, whose effect is identical): `effect` = `sign` when the MOVEM was
//! memory → `Dn` at `.W`, else `flags`; `size` = the size letter.

use super::encode::{Choice, evaluate, extension_offset};
use super::expr::{Expr, Scope};
use super::operand::{Ea, Operand, Spanned};
use crate::isa::{Instruction, Mode, Operand as Slot, Size, instruction_of, places};

/// Rewrites `choice` with `ops` into a shorter or equivalent instruction, if a §4.10 rule
/// applies; returns the new choice, its operands and the rule id (e.g. `"addq"`).
///
/// `generic` tells whether the instruction was written with the generic spelling (ADD, SUB,
/// MOVE, …) rather than an explicit ADDI, ADDA, MOVEA, ADDQ, …, which is never rewritten.
pub fn optimize(
    choice: &Choice,
    ops: &[Spanned<Operand>],
    scope: &dyn Scope,
    generic: bool,
) -> Option<(Choice, Vec<Spanned<Operand>>, &'static str)> {
    if !generic {
        return None;
    }
    match rewrite_instruction(choice, ops, scope) {
        Some((choice, ops, id)) => {
            let ops = rewrite_operands(&choice, &ops, scope).map_or(ops, |(ops, _)| ops);
            Some((choice, ops, id))
        }
        None => rewrite_operands(choice, ops, scope).map(|(ops, id)| (*choice, ops, id)),
    }
}

/// The value of an expression and whether it is known; `None` when it has no value here.
fn value(expr: &Expr, scope: &dyn Scope) -> Option<(i64, bool)> {
    evaluate(expr, scope)
        .ok()
        .map(|value| (value.value, value.known))
}

/// The known value of an expression.
fn known(expr: &Expr, scope: &dyn Scope) -> Option<i64> {
    value(expr, scope).and_then(|(value, known)| known.then_some(value))
}

/// The table entry named `name`.
fn entry(name: &str) -> Option<&'static Instruction> {
    instruction_of(name).map(|(entry, _)| entry)
}

fn choice(name: &str, form: usize, size: Option<Size>) -> Option<Choice> {
    Some(Choice {
        entry: entry(name)?,
        form,
        condition: None,
        size,
    })
}

fn spanned(item: Operand, like: &Spanned<Operand>) -> Spanned<Operand> {
    Spanned {
        item,
        range: like.range,
    }
}

/// The instruction rule that applies, if any.
fn rewrite_instruction(
    original: &Choice,
    ops: &[Spanned<Operand>],
    scope: &dyn Scope,
) -> Option<(Choice, Vec<Spanned<Operand>>, &'static str)> {
    let size = original.size;
    match (original.entry.name, ops) {
        ("MOVE", [source, destination]) if original.form == 1 && size == Some(Size::Long) => {
            let (Operand::Ea(Ea::Imm(expr)), Operand::Ea(Ea::Dn(_))) =
                (&source.item, &destination.item)
            else {
                return None;
            };
            let n = known(expr, scope)?;
            (-128..=127).contains(&n).then_some(())?;
            Some((choice("MOVEQ", 0, size)?, ops.to_vec(), "moveq"))
        }
        (name @ ("ADDI" | "SUBI"), [source, _]) => {
            let Operand::Ea(Ea::Imm(expr)) = &source.item else {
                return None;
            };
            let n = known(expr, scope)?;
            (1..=8).contains(&n).then_some(())?;
            let (quick, id) = if name == "ADDI" {
                ("ADDQ", "addq")
            } else {
                ("SUBQ", "subq")
            };
            Some((choice(quick, 0, size)?, ops.to_vec(), id))
        }
        (name @ ("ADDA" | "SUBA"), [source, destination]) => {
            let (Operand::Ea(Ea::Imm(expr)), Operand::Ea(Ea::An(register))) =
                (&source.item, &destination.item)
            else {
                return None;
            };
            let add = name == "ADDA";
            let (n, is_known) = value(expr, scope)?;
            if is_known && (1..=8).contains(&n) {
                let (quick, id) = if add {
                    ("ADDQ", "addq")
                } else {
                    ("SUBQ", "subq")
                };
                return Some((choice(quick, 1, size)?, ops.to_vec(), id));
            }
            let lea = match size {
                Some(Size::Long) => is_known && (9..=32767).contains(&n),
                Some(Size::Word) => {
                    (is_known && (9..=32767).contains(&n))
                        || (!is_known && (1..=32767).contains(&n))
                }
                _ => false,
            };
            if !lea {
                return None;
            }
            let displacement = if add { n } else { -n };
            let new_ops = vec![
                spanned(
                    Operand::Ea(Ea::Disp(Expr::Num(displacement), *register)),
                    source,
                ),
                destination.clone(),
            ];
            Some((choice("LEA", 0, Some(Size::Long))?, new_ops, "lea"))
        }
        ("LEA", [source, destination]) => {
            let (Operand::Ea(Ea::Disp(expr, from)), Operand::Ea(Ea::An(to))) =
                (&source.item, &destination.item)
            else {
                return None;
            };
            if from != to {
                return None;
            }
            let d = known(expr, scope)?;
            let (quick, n, id) = match d {
                1..=8 => ("ADDQ", d, "addq-lea"),
                -8..=-1 => ("SUBQ", -d, "subq-lea"),
                _ => return None,
            };
            let new_ops = vec![
                spanned(Operand::Ea(Ea::Imm(Expr::Num(n))), source),
                destination.clone(),
            ];
            Some((choice(quick, 1, Some(Size::Word))?, new_ops, id))
        }
        ("MOVEM", [first, second]) => {
            if original.form == 0 {
                // Register to memory: MOVE <register>,<ea>.
                let register = single_register(&first.item)?;
                let new_ops = vec![spanned(Operand::Ea(register), first), second.clone()];
                Some((choice("MOVE", 1, size)?, new_ops, "movem-move"))
            } else {
                // Memory to register: MOVE <ea>,Dn or MOVEA <ea>,An.
                let register = single_register(&second.item)?;
                let target = match register {
                    Ea::An(_) => choice("MOVEA", 0, size)?,
                    _ => choice("MOVE", 1, size)?,
                };
                let new_ops = vec![first.clone(), spanned(Operand::Ea(register), second)];
                Some((target, new_ops, "movem-move"))
            }
        }
        _ => None,
    }
}

/// The register of a one-register list (`Dn`, `An`, or a list with one bit).
fn single_register(op: &Operand) -> Option<Ea> {
    match op {
        Operand::Ea(Ea::Dn(r)) => Some(Ea::Dn(*r)),
        Operand::Ea(Ea::An(r)) => Some(Ea::An(*r)),
        Operand::RegList(mask) if mask.count_ones() == 1 => {
            let bit = mask.trailing_zeros() as u8;
            Some(if bit < 8 {
                Ea::Dn(bit)
            } else {
                Ea::An(bit - 8)
            })
        }
        _ => None,
    }
}

/// Applies `zero-displacement` and `pc-relative` to every operand that qualifies, in operand
/// order; returns the new operands and the first id applied, or `None` (without copying the
/// operands) when no operand qualifies.
fn rewrite_operands(
    choice: &Choice,
    ops: &[Spanned<Operand>],
    scope: &dyn Scope,
) -> Option<(Vec<Spanned<Operand>>, &'static str)> {
    let form = choice.entry.forms.get(choice.form)?;
    let mut rewritten_ops: Option<Vec<Spanned<Operand>>> = None;
    let mut first = None;
    for position in 0..ops.len() {
        let Some(&Slot::Ea(set)) = form.operands.get(position) else {
            continue;
        };
        let current = rewritten_ops.as_deref().unwrap_or(ops);
        let rewritten = match current.get(position).map(|op| &op.item) {
            Some(Operand::Ea(Ea::Disp(expr, register)))
                if set.contains(Mode::Ind) && known(expr, scope) == Some(0) =>
            {
                Some((Ea::Ind(*register), "zero-displacement"))
            }
            Some(Operand::Ea(Ea::Abs(expr, None))) if set.contains(Mode::PcDisp) => {
                pc_relative(choice, current, position, expr, scope).map(|ea| (ea, "pc-relative"))
            }
            _ => None,
        };
        if let Some((ea, id)) = rewritten
            && let Some(op) = rewritten_ops
                .get_or_insert_with(|| ops.to_vec())
                .get_mut(position)
        {
            op.item = Operand::Ea(ea);
            first = first.or(Some(id));
        }
    }
    Some((rewritten_ops?, first?))
}

/// The `d16(PC)` form of the absolute operand at `position`, if its known target is within
/// reach of the extension word.
fn pc_relative(
    choice: &Choice,
    ops: &[Spanned<Operand>],
    position: usize,
    expr: &Expr,
    scope: &dyn Scope,
) -> Option<Ea> {
    let target = known(expr, scope)?;
    if !(0..=0xFF_FFFF).contains(&target) {
        return None;
    }
    let row = places(choice.entry, choice.form);
    let offset = extension_offset(row, ops, choice.size, position);
    let distance = target - (scope.here() + i64::from(offset));
    (-32768..=32767)
        .contains(&distance)
        .then(|| Ea::PcDisp(expr.clone()))
}
