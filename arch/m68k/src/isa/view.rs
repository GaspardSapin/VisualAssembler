//! The CPU-agnostic view of the 68000 tables: [`M68kIsa`] implements
//! `va_core::isa::InstructionSet` (m1-core-isa-lexer.md §3.5), for `va isa` (M1) and hover (M5).
//!
//! Every string is notation or an id. A form's timing rows become [`TimingInfo`]s one to one; a
//! row that covers every mode of its operand has an empty `when`. The MOVE matrices are shown
//! the way they are built: per size and destination column, the cost with a `Dn` source plus
//! the source's EA calculation time (the tests prove this equals every matrix cell).

use std::sync::LazyLock;

use va_core::isa::{
    Bound, ConditionInfo, Cycles, EaTiming, FlagInfo, FormInfo, InstructionInfo, InstructionSet,
    LookupError, ModeCondition, OperandInfo, PerUnit, TimingInfo,
};

use super::model::{Form, Instruction, Operand, Size, SizeSet};
use super::modes::{Mode, ModeSet};
use super::timing::{EA_TIME, MOVE_BW, MOVE_L, Outcome, Timing, TimingRow, move_column};
use super::{SPELLINGS, resolve, spellings_of};

/// The 68000 instruction set, seen through `va_core::isa::InstructionSet`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct M68kIsa;

/// [`EA_TIME`] as `EaTiming`s: per mode, one entry for `.B`/`.W` and one for `.L`.
static EA_TIMING: LazyLock<Vec<EaTiming>> = LazyLock::new(|| {
    Mode::ALL
        .into_iter()
        .flat_map(|mode| {
            let [byte_word, long] = EA_TIME[mode.index()];
            [
                EaTiming {
                    mode: mode.notation(),
                    sizes: vec!["B", "W"],
                    cycles: byte_word,
                },
                EaTiming {
                    mode: mode.notation(),
                    sizes: vec!["L"],
                    cycles: long,
                },
            ]
        })
        .collect()
});

impl InstructionSet for M68kIsa {
    fn spellings(&self) -> &[&'static str] {
        &SPELLINGS
    }

    fn describe(&self, spelling: &str, size: Option<&str>) -> Result<InstructionInfo, LookupError> {
        let resolved = resolve(spelling).ok_or(LookupError::Unknown)?;
        let instruction = resolved.instruction;
        let size = match size {
            None => None,
            Some(letter) => {
                Some(normalize_size(instruction, letter).ok_or(LookupError::SizeNotAccepted)?)
            }
        };
        let forms: Vec<FormInfo> = instruction
            .forms
            .iter()
            .filter(|form| size.is_none_or(|size| form.sizes.contains(size)))
            .map(|form| form_info(resolved.spelling, form))
            .collect();
        if size.is_some() && forms.is_empty() {
            return Err(LookupError::SizeNotAccepted);
        }
        Ok(InstructionInfo {
            spelling: resolved.spelling,
            canonical: resolved.canonical,
            family: instruction.family.map(|family| family.name()),
            condition: resolved.condition.map(|condition| ConditionInfo {
                name: condition.name,
                test: condition.test,
                doc_id: condition.doc_id,
            }),
            aliases: spellings_of(resolved.canonical)
                .filter(|&other| other != resolved.spelling)
                .collect(),
            also_accepts: instruction.also_accepts.to_vec(),
            size: size.map(Size::letter),
            doc_id: instruction.doc_id,
            forms,
        })
    }

    fn ea_timing(&self) -> &[EaTiming] {
        &EA_TIMING
    }
}

/// Parses a suffix letter; on a branch entry `.B` means `.S` and `.L` means `.W` (A68k).
fn normalize_size(instruction: &Instruction, letter: &str) -> Option<Size> {
    let size = Size::from_letter(letter)?;
    let is_branch = instruction
        .forms
        .iter()
        .any(|form| form.sizes.contains(Size::Short));
    Some(match (is_branch, size) {
        (true, Size::Byte) => Size::Short,
        (true, Size::Long) => Size::Word,
        _ => size,
    })
}

fn letters(sizes: SizeSet) -> Vec<&'static str> {
    sizes.iter().map(Size::letter).collect()
}

fn notations(modes: ModeSet) -> Vec<&'static str> {
    modes.iter().map(Mode::notation).collect()
}

fn operand_modes(form: &Form, index: u8) -> ModeSet {
    form.operands
        .get(usize::from(index))
        .map_or(ModeSet::NONE, |operand| operand.modes())
}

fn form_info(spelling: &str, form: &Form) -> FormInfo {
    let mut syntax = String::from(spelling);
    if !form.sizes.is_empty() {
        syntax.push('.');
        syntax.push_str(&letters(form.sizes).join("/"));
    }
    if !form.operands.is_empty() {
        syntax.push(' ');
        let operands: Vec<&str> = form.operands.iter().map(|op| op.notation()).collect();
        syntax.push_str(&operands.join(","));
    }
    FormInfo {
        syntax,
        sizes: letters(form.sizes),
        default_size: form.default_size.map(Size::letter),
        privileged: form.privileged,
        operands: form
            .operands
            .iter()
            .map(|&operand| OperandInfo {
                notation: operand.notation(),
                modes: match operand {
                    Operand::Ea(set) => notations(set),
                    _ => Vec::new(),
                },
            })
            .collect(),
        flags: form
            .flags
            .named()
            .into_iter()
            .map(|(flag, effect)| FlagInfo { flag, effect })
            .collect(),
        timing: match form.timing {
            Timing::Rows(rows) => rows.iter().map(|row| row_info(form, row)).collect(),
            Timing::MoveMatrix => move_info(form),
        },
    }
}

fn row_info(form: &Form, row: &TimingRow) -> TimingInfo {
    let all = operand_modes(form, row.operand);
    let when = if row.modes.is_empty() || all.is_subset(row.modes) {
        Vec::new()
    } else {
        vec![ModeCondition {
            operand: row.operand,
            modes: notations(row.modes),
        }]
    };
    TimingInfo {
        sizes: letters(row.sizes),
        when,
        outcome: (row.outcome != Outcome::Always).then(|| row.outcome.id()),
        base: row.base,
        plus_ea: if row.plus_ea {
            vec![row.operand]
        } else {
            Vec::new()
        },
        per_unit: row.per.map(|(cycles, unit)| PerUnit {
            cycles,
            unit: unit.id(),
        }),
        bound: row.bound,
    }
}

/// Rows of a MOVE or MOVEA form: per size and group of destination modes with the same cost,
/// the cost with a `Dn` source, plus the source's EA calculation time.
fn move_info(form: &Form) -> Vec<TimingInfo> {
    let destinations = operand_modes(form, 1);
    let mut out = Vec::new();
    for size in form.sizes.iter() {
        let matrix = if size == Size::Long {
            &MOVE_L
        } else {
            &MOVE_BW
        };
        let mut groups: Vec<(Cycles, ModeSet)> = Vec::new();
        for mode in destinations.iter() {
            let Some(column) = move_column(mode) else {
                continue;
            };
            let base = matrix[Mode::Dn.index()][column];
            match groups.iter_mut().find(|(cost, _)| *cost == base) {
                Some((_, modes)) => *modes = modes.with(mode),
                None => groups.push((base, ModeSet::only(mode))),
            }
        }
        for (base, modes) in groups {
            let when = if destinations.is_subset(modes) {
                Vec::new()
            } else {
                vec![ModeCondition {
                    operand: 1,
                    modes: notations(modes),
                }]
            };
            out.push(TimingInfo {
                sizes: vec![size.letter()],
                when,
                outcome: None,
                base,
                plus_ea: vec![0],
                per_unit: None,
                bound: Bound::Exact,
            });
        }
    }
    out
}
