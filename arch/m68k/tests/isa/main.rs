//! Consistency tests of the 68000 instruction tables (`src/isa/`, m1-core-isa-lexer.md §4.7,
//! exit criterion E4), run by `cargo test -p va-arch-m68k --test isa`.
#![allow(clippy::unwrap_used)]

mod encoding;
mod forms;
mod spellings;
mod timing;
mod view;

use va_arch_m68k::isa::{Form, INSTRUCTIONS, Instruction, Size, instruction_of};

/// Every (entry, form index, form) of the table.
pub(crate) fn all_forms() -> impl Iterator<Item = (&'static Instruction, usize, &'static Form)> {
    INSTRUCTIONS.iter().flat_map(|entry| {
        entry
            .forms
            .iter()
            .enumerate()
            .map(move |(index, form)| (entry, index, form))
    })
}

/// The operands of a form in notation, comma-separated (`""` without operand).
pub(crate) fn operands(form: &Form) -> String {
    let notations: Vec<&str> = form.operands.iter().map(|op| op.notation()).collect();
    notations.join(",")
}

/// The form of `spelling` whose operands read `operands` and which accepts `size` (any size
/// when `None`). Panics when there is not exactly one.
pub(crate) fn form_of(
    spelling: &str,
    operands_notation: &str,
    size: Option<Size>,
) -> &'static Form {
    let (entry, _) = instruction_of(spelling).unwrap();
    let found: Vec<&Form> = entry
        .forms
        .iter()
        .filter(|form| operands(form) == operands_notation)
        .filter(|form| size.is_none_or(|size| form.sizes.contains(size)))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "{spelling} {operands_notation} {size:?}: {} forms",
        found.len()
    );
    found[0]
}
