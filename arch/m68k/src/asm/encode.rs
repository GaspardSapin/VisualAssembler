//! Form selection, instruction length and encoding (m2-assembler.md §4.2, frozen; §4.10).
//!
//! [`select`] picks the form of a spelling that its operands and size fit (the spelling's
//! `also_accepts` entries first, then its own, in table order); [`length`] gives its byte length
//! from the operand shapes only; [`encode`] writes its words with final values, through
//! `isa::fields::places` and the EA fields of M1's `Encoding`.
//!
//! **Selection.** A form fits when the operand count, every operand kind and mode, and the size
//! fit. A spelling without suffix takes the form's default size; a branch (Bcc, BRA, BSR)
//! without suffix takes its `.W` form, and on a branch `.B` means `.S` and `.L` means `.W` (A68k).
//! A single `Dn`/`An` is a one-register list where a form wants a list; EXG is symmetric
//! (`exg a0,d0` is the `Dn,An` form). When nothing fits, the closest failure is reported, from
//! the closest down: the operands fit but the size does not (`asm-size-not-allowed`, with the
//! sizes of the forms the operands fit, or `none` when none of them takes a size); only
//! addressing modes do not fit (`asm-mode-not-allowed`, at the first such operand, with every
//! mode accepted there); the operand count fits but a kind does not (`asm-no-matching-form`);
//! the count does not fit (`asm-operand-count`, with the count nearest to the one written).
//! A problem tied to one operand covers that operand; the others cover the operand field
//! written (empty at 0 without operands), and the driver may point them at the operation
//! instead.
//!
//! **Values.** The leaves of an expression (number, character, `*`, a symbol that resolves) are
//! evaluated here; every other expression goes to `expr::eval`. Ranges (§4.10): immediates `.B`
//! −128…255, `.W` −32768…65535, `.L` any 32-bit value; quick values 1…8; MOVEQ −128…127; TRAP
//! 0…15; bit numbers 0…255; CCR immediates as `.B`, STOP and SR immediates as `.W`; LINK, `d16`
//! and `(e).W` −32768…32767; `d8` −128…127; absolute and target addresses below `$1000000`.
//! Formatted arguments (m2 §16.3): addresses as `$` and at least 6 uppercase hexadecimal digits
//! (`$001001`, `$FFFFFF`, `$1000000`), other values in decimal. The `distance` of
//! `asm-branch-out-of-range` is an `Int`, the magnitude in bytes of
//! `target − (opword address + 2)`, and its `size` the branch size used (`S` or `W`).
//!
//! **Encoding protocol.** On success the words are appended to `out`. An error appends
//! nothing, with one exception: the warning `asm-odd-branch-target` is returned as `Err` after
//! the complete words were appended (the frozen signature has no other way to carry a warning
//! with a result), so the driver keeps them.

use va_core::diag::{DiagArg, DiagCode};
use va_core::source::TextRange;

use super::codes::{
    ASM_BRANCH_OUT_OF_RANGE, ASM_MODE_NOT_ALLOWED, ASM_NO_MATCHING_FORM, ASM_ODD_BRANCH_TARGET,
    ASM_OPERAND_COUNT, ASM_SHORT_BRANCH_ZERO, ASM_SIZE_NOT_ALLOWED, ASM_UNKNOWN_OPERATION,
    ASM_VALUE_OUT_OF_RANGE,
};
use super::expr::{self, Expr, Lookup, Scope, Value};
use super::operand::{Ea, Operand, Spanned, Xn};
use super::parser::Problem;
use crate::isa::{
    EaField, Form, Instruction, Mode, ModeSet, Operand as Slot, Place, Size, SizeField, SizeSet,
    instruction_of, places,
};

/// The form chosen for an instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Choice {
    /// The table entry.
    pub entry: &'static Instruction,
    /// The index of the form in `entry.forms`.
    pub form: usize,
    /// The condition code (0–15) of a Bcc, DBcc or Scc spelling.
    pub condition: Option<u8>,
    /// The operation size; `None` for an unsized form.
    pub size: Option<Size>,
}

/// Selects the form of `spelling` (uppercase, without suffix) that `size` and `ops` fit.
///
/// # Errors
///
/// The closest failure (§4.10): `asm-operand-count`, `asm-size-not-allowed`,
/// `asm-mode-not-allowed`, else `asm-no-matching-form` (see the module documentation for the
/// ranking); `asm-unknown-operation` for a spelling that is no 68000 instruction.
pub fn select(
    spelling: &str,
    size: Option<Size>,
    ops: &[Spanned<Operand>],
) -> Result<Choice, Problem> {
    let mnemonic = spelling.to_ascii_uppercase();
    let Some((own, condition)) = instruction_of(spelling) else {
        return Err(problem(
            ASM_UNKNOWN_OPERATION,
            field_range(ops),
            vec![
                ("name", DiagArg::Str(mnemonic)),
                ("count", DiagArg::Int(0)),
                ("suggestions", DiagArg::Str(String::new())),
            ],
        ));
    };
    let size = suffix_size(own, size);
    let mut closest = Closest::default();
    for entry in candidates(own) {
        for (index, form) in entry.forms.iter().enumerate() {
            match fit_form(entry, form, size, ops) {
                Ok(size) => {
                    return Ok(Choice {
                        entry,
                        form: index,
                        condition: entry.family.and(condition).map(|condition| condition.code),
                        size,
                    });
                }
                Err(miss) => closest.add(miss, ops.len()),
            }
        }
    }
    Err(closest.into_problem(&mnemonic, own, size, ops))
}

/// Returns the length in bytes of `choice` with `ops`; it depends on the operand shapes only.
///
/// An absolute operand without size counts as absolute long (§4.10, pass-1 rule).
pub fn length(choice: &Choice, ops: &[Spanned<Operand>]) -> u32 {
    let extension: u32 = places(choice.entry, choice.form)
        .iter()
        .zip(ops)
        .map(|(&place, op)| extension_bytes(place, &op.item, choice.size))
        .sum();
    2 + extension
}

/// Encodes `choice` with `ops` at `address`, evaluating the operands in `scope`, and appends
/// its words to `out`.
///
/// # Errors
///
/// The [`Problem`] of a value that cannot be encoded (`asm-value-out-of-range`,
/// `asm-branch-out-of-range`, `asm-short-branch-zero`, …), and nothing is appended; or the
/// warning `asm-odd-branch-target`, returned after the complete words were appended.
pub fn encode(
    choice: &Choice,
    ops: &[Spanned<Operand>],
    scope: &dyn Scope,
    address: u64,
    out: &mut Vec<u16>,
) -> Result<(), Problem> {
    let start = out.len();
    match Encoder::new(choice, ops, scope, address, out).run() {
        Ok(None) => Ok(()),
        Ok(Some(warning)) => Err(warning),
        Err(error) => {
            out.truncate(start);
            Err(error)
        }
    }
}

/// Evaluates `expr` in `scope`: the leaves here, everything else through `expr::eval`.
///
/// A number, a character or `*` is known; a symbol is known iff it is `Known`. An `Unknown`
/// symbol (pass 1) gives 0, not known. An undefined or poisoned symbol, and every operator, go
/// to `expr::eval`, which owns their diagnostics.
pub(crate) fn evaluate(expr: &Expr, scope: &dyn Scope) -> Result<Value, Problem> {
    let leaf = |value: i64, known: bool| {
        Ok(Value {
            value: wrap(value),
            known,
        })
    };
    match expr {
        Expr::Num(value) | Expr::Char(value) => leaf(*value, true),
        Expr::Here => leaf(scope.here(), true),
        Expr::Sym(sym) => match scope.lookup(sym) {
            Lookup::Known(value) => leaf(value, true),
            Lookup::Later(value) => leaf(value, false),
            Lookup::Unknown => leaf(0, false),
            Lookup::Undefined | Lookup::Poisoned => expr::eval(expr, scope),
        },
        Expr::Unary(..) | Expr::Binary(..) => expr::eval(expr, scope),
    }
}

/// Wraps `value` to 32 bits and sign-extends it.
pub(crate) fn wrap(value: i64) -> i64 {
    i64::from(value as i32)
}

/// Returns the addressing mode of a written effective address (an absolute address without
/// size is absolute long); `None` for an absolute address with a size other than `.W`/`.L`.
pub(crate) fn mode_of(ea: &Ea) -> Option<Mode> {
    Some(match ea {
        Ea::Dn(_) => Mode::Dn,
        Ea::An(_) => Mode::An,
        Ea::Ind(_) => Mode::Ind,
        Ea::PostInc(_) => Mode::PostInc,
        Ea::PreDec(_) => Mode::PreDec,
        Ea::Disp(..) => Mode::Disp,
        Ea::Index(..) => Mode::Index,
        Ea::PcDisp(_) => Mode::PcDisp,
        Ea::PcIndex(..) => Mode::PcIndex,
        Ea::Abs(_, Some(Size::Word)) => Mode::AbsW,
        Ea::Abs(_, None | Some(Size::Long)) => Mode::AbsL,
        Ea::Abs(_, Some(Size::Byte | Size::Short)) => return None,
        Ea::Imm(_) => Mode::Imm,
    })
}

/// Returns the bytes of extension words an operand at `place` takes (see `isa::fields`).
pub(crate) fn extension_bytes(place: Place, op: &Operand, size: Option<Size>) -> u32 {
    match place {
        Place::Ea => match op {
            Operand::Ea(ea) => ea_extension_bytes(ea, size),
            _ => 0,
        },
        Place::Reg9 | Place::Reg0 => match op {
            Operand::Ea(Ea::Disp(..)) => 2,
            _ => 0,
        },
        Place::Disp16 | Place::ImmWord | Place::ImmByte | Place::RegMask => 2,
        Place::Imm => immediate_bytes(size),
        Place::Quick9 | Place::Data8 | Place::Vector | Place::Disp8 | Place::Implied => 0,
    }
}

/// Returns the offset in bytes, from the opword, of the first extension word of operand
/// `position` (the MOVEM mask word comes first, then the operands in order).
pub(crate) fn extension_offset(
    places: &[Place],
    ops: &[Spanned<Operand>],
    size: Option<Size>,
    position: usize,
) -> u32 {
    let mask = places.iter().position(|&place| place == Place::RegMask);
    if mask == Some(position) {
        return 2;
    }
    let mut offset = 2 + mask.map_or(0, |_| 2);
    for (index, (&place, op)) in places.iter().zip(ops).enumerate() {
        if index == position {
            break;
        }
        if Some(index) != mask {
            offset += extension_bytes(place, &op.item, size);
        }
    }
    offset
}

fn ea_extension_bytes(ea: &Ea, size: Option<Size>) -> u32 {
    match ea {
        Ea::Dn(_) | Ea::An(_) | Ea::Ind(_) | Ea::PostInc(_) | Ea::PreDec(_) => 0,
        Ea::Disp(..) | Ea::Index(..) | Ea::PcDisp(_) | Ea::PcIndex(..) => 2,
        Ea::Abs(_, Some(Size::Word)) => 2,
        Ea::Abs(..) => 4,
        Ea::Imm(_) => immediate_bytes(size),
    }
}

fn immediate_bytes(size: Option<Size>) -> u32 {
    if size == Some(Size::Long) { 4 } else { 2 }
}

/// Builds a problem.
fn problem(code: DiagCode, range: TextRange, args: Vec<(&'static str, DiagArg)>) -> Problem {
    Problem {
        code,
        range,
        args,
        fix: None,
    }
}

/// The range from the first operand to the last (empty at 0 without operands).
fn field_range(ops: &[Spanned<Operand>]) -> TextRange {
    match (ops.first(), ops.last()) {
        (Some(first), Some(last)) => TextRange::new(first.range.start, last.range.end),
        _ => TextRange::new(0, 0),
    }
}

/// Whether `entry` is a branch with a `.S` and a `.W` form (Bcc, BRA, BSR).
fn is_branch(entry: &Instruction) -> bool {
    entry
        .forms
        .iter()
        .any(|form| form.sizes.contains(Size::Short))
}

/// The size a suffix stands for on `entry`: on a branch `.B` is `.S` and `.L` is `.W` (A68k).
fn suffix_size(entry: &Instruction, size: Option<Size>) -> Option<Size> {
    match size {
        Some(Size::Byte) if is_branch(entry) => Some(Size::Short),
        Some(Size::Long) if is_branch(entry) => Some(Size::Word),
        other => other,
    }
}

/// The entries a spelling may select: its `also_accepts` entries in order, then its own.
fn candidates(own: &'static Instruction) -> impl Iterator<Item = &'static Instruction> {
    own.also_accepts
        .iter()
        .filter_map(|name| instruction_of(name).map(|(entry, _)| entry))
        .chain(std::iter::once(own))
}

/// Why one form does not fit, from the farthest to the closest.
enum Miss {
    /// The operand count differs.
    Count { expected: usize },
    /// An operand kind does not fit.
    Kind,
    /// Only addressing modes do not fit; the first such operand and what its slot accepts.
    Mode {
        position: usize,
        modes: ModeSet,
        special: Option<&'static str>,
    },
    /// The operands fit; the size does not (the form's sizes).
    Size { sizes: SizeSet },
}

impl Miss {
    fn rank(&self) -> u8 {
        match self {
            Miss::Count { .. } => 0,
            Miss::Kind => 1,
            Miss::Mode { .. } => 2,
            Miss::Size { .. } => 3,
        }
    }
}

/// The closest failure over the forms tried so far.
#[derive(Default)]
struct Closest {
    rank: Option<u8>,
    expected: Option<usize>,
    position: usize,
    modes: ModeSet,
    specials: Vec<&'static str>,
    sizes: SizeSet,
}

impl Closest {
    fn add(&mut self, miss: Miss, found: usize) {
        let rank = miss.rank();
        if self.rank.is_some_and(|best| best > rank) {
            return;
        }
        let first = self.rank != Some(rank);
        if first {
            *self = Closest {
                rank: Some(rank),
                ..Closest::default()
            };
        }
        match miss {
            Miss::Count { expected } => {
                let nearer = self.expected.is_none_or(|best| {
                    let (new, old) = (expected.abs_diff(found), best.abs_diff(found));
                    new < old || (new == old && expected < best)
                });
                if nearer {
                    self.expected = Some(expected);
                }
            }
            Miss::Kind => {}
            Miss::Mode {
                position,
                modes,
                special,
            } => {
                if first {
                    self.position = position;
                }
                if position == self.position {
                    self.modes = self.modes.union(modes);
                    if let Some(special) = special
                        && !self.specials.contains(&special)
                    {
                        self.specials.push(special);
                    }
                }
            }
            Miss::Size { sizes } => {
                self.sizes = SizeSet::of(
                    &Size::ALL
                        .into_iter()
                        .filter(|&size| sizes.contains(size) || self.sizes.contains(size))
                        .collect::<Vec<_>>(),
                );
            }
        }
    }

    fn into_problem(
        self,
        mnemonic: &str,
        own: &'static Instruction,
        size: Option<Size>,
        ops: &[Spanned<Operand>],
    ) -> Problem {
        let name = ("mnemonic", DiagArg::Str(mnemonic.to_owned()));
        match self.rank {
            Some(3) => {
                let allowed: Vec<String> = self
                    .sizes
                    .iter()
                    .map(|size| format!(".{}", size.letter()))
                    .collect();
                let allowed = if allowed.is_empty() {
                    "none".to_owned()
                } else {
                    allowed.join(", ")
                };
                problem(
                    ASM_SIZE_NOT_ALLOWED,
                    field_range(ops),
                    vec![
                        name,
                        (
                            "size",
                            DiagArg::Str(size.map_or("", Size::letter).to_owned()),
                        ),
                        ("allowed", DiagArg::Str(allowed)),
                    ],
                )
            }
            Some(2) => {
                let op = ops.get(self.position);
                let mode = op
                    .and_then(|op| match &op.item {
                        Operand::Ea(ea) => mode_of(ea),
                        _ => None,
                    })
                    .map_or("", Mode::notation);
                let mut allowed: Vec<&str> = self.modes.iter().map(Mode::notation).collect();
                allowed.extend(self.specials.iter().copied());
                problem(
                    ASM_MODE_NOT_ALLOWED,
                    op.map_or_else(|| field_range(ops), |op| op.range),
                    vec![
                        name,
                        ("mode", DiagArg::Str(mode.to_owned())),
                        ("allowed", DiagArg::Str(allowed.join(", "))),
                    ],
                )
            }
            Some(0) => problem(
                ASM_OPERAND_COUNT,
                field_range(ops),
                vec![
                    name,
                    (
                        "expected",
                        DiagArg::Int(self.expected.map_or(0, |count| count as i64)),
                    ),
                    ("found", DiagArg::Int(ops.len() as i64)),
                ],
            ),
            _ => {
                let mut forms: Vec<String> = Vec::new();
                for entry in candidates(own) {
                    for form in entry.forms {
                        let notation: Vec<&str> =
                            form.operands.iter().map(|slot| slot.notation()).collect();
                        let notation = notation.join(",");
                        if !forms.contains(&notation) {
                            forms.push(notation);
                        }
                    }
                }
                problem(
                    ASM_NO_MATCHING_FORM,
                    field_range(ops),
                    vec![name, ("forms", DiagArg::Str(forms.join("; ")))],
                )
            }
        }
    }
}

/// How one written operand fits one slot of a form.
enum Fit {
    Yes,
    /// The kind fits but not the addressing mode; what the slot accepts.
    Mode(ModeSet, Option<&'static str>),
    No,
}

fn fit_operand(slot: Slot, op: &Operand) -> Fit {
    match (slot, op) {
        (Slot::RegList, Operand::RegList(mask)) => {
            if *mask == 0 {
                Fit::No
            } else {
                Fit::Yes
            }
        }
        (Slot::RegList, Operand::Ea(Ea::Dn(_) | Ea::An(_))) => Fit::Yes,
        (Slot::RegList, Operand::Ea(_)) => Fit::Mode(ModeSet::NONE, Some(slot.notation())),
        (Slot::Label, Operand::Ea(Ea::Abs(_, None))) => Fit::Yes,
        (Slot::Label, Operand::Ea(_)) => Fit::Mode(ModeSet::NONE, Some(slot.notation())),
        (Slot::Ccr, Operand::Ccr) | (Slot::Sr, Operand::Sr) | (Slot::Usp, Operand::Usp) => Fit::Yes,
        (Slot::RegList | Slot::Label | Slot::Ccr | Slot::Sr | Slot::Usp, _) => Fit::No,
        (_, Operand::Ea(ea)) => match mode_of(ea) {
            Some(mode) if slot.modes().contains(mode) => Fit::Yes,
            Some(_) => Fit::Mode(slot.modes(), None),
            None => Fit::No,
        },
        (
            _,
            Operand::RegList(_) | Operand::Ccr | Operand::Sr | Operand::Usp | Operand::Bytes(_),
        ) => Fit::No,
    }
}

/// Whether `ops` are the `An,Dn` spelling of EXG's `Dn,An` form (EXG is symmetric).
fn is_swapped_exg(entry: &Instruction, form: &Form, ops: &[Spanned<Operand>]) -> bool {
    entry.name == "EXG"
        && form.operands == [Slot::Dn, Slot::An]
        && matches!(
            ops,
            [
                Spanned {
                    item: Operand::Ea(Ea::An(_)),
                    ..
                },
                Spanned {
                    item: Operand::Ea(Ea::Dn(_)),
                    ..
                }
            ]
        )
}

/// Returns the size `form` takes with `ops` and `size`, or why it does not fit.
fn fit_form(
    entry: &Instruction,
    form: &Form,
    size: Option<Size>,
    ops: &[Spanned<Operand>],
) -> Result<Option<Size>, Miss> {
    if ops.len() != form.operands.len() {
        return Err(Miss::Count {
            expected: form.operands.len(),
        });
    }
    if !is_swapped_exg(entry, form, ops) {
        let mut mode_miss = None;
        for (position, (&slot, op)) in form.operands.iter().zip(ops).enumerate() {
            match fit_operand(slot, &op.item) {
                Fit::Yes => {}
                Fit::Mode(modes, special) => {
                    mode_miss.get_or_insert(Miss::Mode {
                        position,
                        modes,
                        special,
                    });
                }
                Fit::No => return Err(Miss::Kind),
            }
        }
        if let Some(miss) = mode_miss {
            return Err(miss);
        }
    }
    let sizes = form.sizes;
    if sizes.is_empty() {
        return match size {
            None => Ok(None),
            Some(_) => Err(Miss::Size { sizes }),
        };
    }
    match size {
        Some(size) if sizes.contains(size) => Ok(Some(size)),
        Some(_) => Err(Miss::Size { sizes }),
        None => match form.default_size {
            Some(default) => Ok(Some(default)),
            None if is_branch(entry) && sizes.contains(Size::Word) => Ok(Some(Size::Word)),
            // A suffix-only form (`.B` of ADD, CMP, MOVE): its sibling form has the default.
            None => Err(Miss::Kind),
        },
    }
}

/// Formats an address argument: `$` and the uppercase hexadecimal of the 32-bit value, at least
/// 6 digits (`$001001`, `$FFFFFF`, `$1000000`; m2 §16.3).
fn address_text(value: i64) -> String {
    format!("${:06X}", value & 0xFFFF_FFFF)
}

/// The highest absolute or target address (24-bit address bus).
const MAX_ADDRESS: i64 = 0xFF_FFFF;

/// One encoding in progress.
struct Encoder<'a> {
    choice: &'a Choice,
    ops: &'a [Spanned<Operand>],
    scope: &'a dyn Scope,
    address: u64,
    out: &'a mut Vec<u16>,
    /// Index of the opword in `out`.
    start: usize,
    /// Whether the operands are EXG's `An,Dn` spelling of its `Dn,An` form.
    exg_swapped: bool,
}

impl<'a> Encoder<'a> {
    fn new(
        choice: &'a Choice,
        ops: &'a [Spanned<Operand>],
        scope: &'a dyn Scope,
        address: u64,
        out: &'a mut Vec<u16>,
    ) -> Self {
        let start = out.len();
        Encoder {
            choice,
            ops,
            scope,
            address,
            out,
            start,
            exg_swapped: false,
        }
    }

    /// Writes the words; returns the warning to report, if any.
    fn run(mut self) -> Result<Option<Problem>, Problem> {
        let choice = self.choice;
        let entry = choice.entry;
        let places = places(entry, choice.form);
        let form = entry.forms.get(choice.form);
        let Some(form) = form
            .filter(|form| form.operands.len() == self.ops.len() && places.len() == self.ops.len())
        else {
            return Err(problem(
                ASM_OPERAND_COUNT,
                field_range(self.ops),
                vec![
                    ("mnemonic", DiagArg::Str(entry.name.to_ascii_uppercase())),
                    (
                        "expected",
                        DiagArg::Int(
                            entry
                                .forms
                                .get(choice.form)
                                .map_or(0, |f| f.operands.len() as i64),
                        ),
                    ),
                    ("found", DiagArg::Int(self.ops.len() as i64)),
                ],
            ));
        };
        self.exg_swapped = is_swapped_exg(entry, form, self.ops);
        let mut opword = form.encoding.opword;
        if entry.family.is_some()
            && let Some(code) = choice.condition
        {
            opword |= u16::from(code & 0xF) << 8;
        }
        opword |= size_bits(form.encoding.size, choice.size);
        opword |= self.ea_fields(form, places)?;
        self.out.push(0);
        let mut warning = None;
        for (index, &place) in places.iter().enumerate() {
            let (bits, odd) = self.opword_bits(place, index)?;
            opword |= bits;
            warning = warning.or(odd);
        }
        if let Some(mask) = places.iter().position(|&place| place == Place::RegMask) {
            self.extension(Place::RegMask, mask)?;
        }
        for (index, &place) in places.iter().enumerate() {
            if place != Place::RegMask
                && let Some(odd) = self.extension(place, index)?
            {
                warning = Some(odd);
            }
        }
        if let Some(slot) = self.out.get_mut(self.start) {
            *slot = opword;
        }
        Ok(warning)
    }

    fn op(&self, index: usize) -> Result<&'a Spanned<Operand>, Problem> {
        self.ops.get(index).ok_or_else(|| self.no_form())
    }

    fn no_form(&self) -> Problem {
        problem(
            ASM_NO_MATCHING_FORM,
            field_range(self.ops),
            vec![
                (
                    "mnemonic",
                    DiagArg::Str(self.choice.entry.name.to_ascii_uppercase()),
                ),
                ("forms", DiagArg::Str(String::new())),
            ],
        )
    }

    /// The value of `expr`, 32-bit and sign-extended.
    fn value(&self, expr: &Expr) -> Result<i64, Problem> {
        evaluate(expr, self.scope).map(|value| value.value)
    }

    /// The address of the next word to be appended.
    fn next_address(&self) -> i64 {
        let offset = 2 * (self.out.len() - self.start) as u64;
        self.address.wrapping_add(offset) as i64
    }

    /// The EA field bits of the opword.
    fn ea_fields(&self, form: &Form, places: &[Place]) -> Result<u16, Problem> {
        let mut eas = places
            .iter()
            .enumerate()
            .filter(|&(_, &place)| place == Place::Ea)
            .map(|(index, _)| index);
        let low = |index: Option<usize>| -> Result<u16, Problem> {
            let (mode, register) = self.ea_mode_register(index)?;
            Ok(mode << 3 | register)
        };
        let dest = |index: Option<usize>| -> Result<u16, Problem> {
            let (mode, register) = self.ea_mode_register(index)?;
            Ok(register << 9 | mode << 6)
        };
        match form.encoding.ea {
            EaField::None => Ok(0),
            EaField::Low => low(eas.next()),
            EaField::MoveDest => dest(eas.next_back()),
            EaField::Both => {
                let source = low(eas.next())?;
                Ok(source | dest(eas.next())?)
            }
        }
    }

    /// The mode and register fields of the EA operand at `index`.
    fn ea_mode_register(&self, index: Option<usize>) -> Result<(u16, u16), Problem> {
        let op = self.op(index.ok_or_else(|| self.no_form())?)?;
        let Operand::Ea(ea) = &op.item else {
            return Err(self.no_form());
        };
        let mode = mode_of(ea).ok_or_else(|| self.no_form())?;
        let (field, fixed) = mode.fields();
        let register = match (fixed, ea) {
            (Some(register), _) => register,
            (
                None,
                Ea::Dn(r)
                | Ea::An(r)
                | Ea::Ind(r)
                | Ea::PostInc(r)
                | Ea::PreDec(r)
                | Ea::Disp(_, r)
                | Ea::Index(_, r, _),
            ) => *r,
            (None, _) => return Err(self.no_form()),
        };
        Ok((u16::from(field & 7), u16::from(register & 7)))
    }

    /// The register number of the register operand placed at `index` (EXG's `An,Dn` spelling
    /// puts its second operand in the first place).
    fn register(&self, index: usize) -> Result<u16, Problem> {
        let index = if self.exg_swapped { index ^ 1 } else { index };
        match &self.op(index)?.item {
            Operand::Ea(
                Ea::Dn(r) | Ea::An(r) | Ea::PostInc(r) | Ea::PreDec(r) | Ea::Disp(_, r),
            ) => Ok(u16::from(r & 7)),
            _ => Err(self.no_form()),
        }
    }

    /// The expression of an immediate (or label) operand.
    fn immediate_expr(&self, index: usize) -> Result<&'a Expr, Problem> {
        match &self.op(index)?.item {
            Operand::Ea(Ea::Imm(expr) | Ea::Abs(expr, None)) => Ok(expr),
            _ => Err(self.no_form()),
        }
    }

    /// Checks `value` against `min..=max` for the operand at `index`.
    fn in_range(
        &self,
        index: usize,
        value: i64,
        min: i64,
        max: i64,
        address: bool,
    ) -> Result<i64, Problem> {
        if (min..=max).contains(&value) {
            return Ok(value);
        }
        let text = |value: i64| {
            if address {
                address_text(value)
            } else {
                value.to_string()
            }
        };
        Err(problem(
            ASM_VALUE_OUT_OF_RANGE,
            self.op(index)?.range,
            vec![
                ("value", DiagArg::Str(text(value))),
                ("min", DiagArg::Str(text(min))),
                ("max", DiagArg::Str(text(max))),
            ],
        ))
    }

    /// A branch target and its displacement from the opword address + 2; also the odd-target
    /// warning.
    fn branch(&self, index: usize) -> Result<(i64, Option<Problem>), Problem> {
        let target = self.value(self.immediate_expr(index)?)?;
        let target = self.in_range(index, target, 0, MAX_ADDRESS, true)?;
        let base = self.address.wrapping_add(2) as i64;
        let odd = (target & 1 == 1).then(|| {
            problem(
                ASM_ODD_BRANCH_TARGET,
                self.ops
                    .get(index)
                    .map_or(TextRange::new(0, 0), |op| op.range),
                vec![("target", DiagArg::Str(address_text(target)))],
            )
        });
        Ok((target - base, odd))
    }

    fn branch_out_of_range(&self, index: usize, distance: i64, size: Size) -> Problem {
        problem(
            ASM_BRANCH_OUT_OF_RANGE,
            self.ops
                .get(index)
                .map_or(TextRange::new(0, 0), |op| op.range),
            vec![
                ("distance", DiagArg::Int(distance.abs())),
                ("size", DiagArg::Str(size.letter().to_owned())),
            ],
        )
    }

    /// The opword bits of the operand at `index` (and the odd-target warning of a short branch).
    fn opword_bits(&self, place: Place, index: usize) -> Result<(u16, Option<Problem>), Problem> {
        let bits = match place {
            Place::Reg9 => self.register(index)? << 9,
            Place::Reg0 => self.register(index)?,
            Place::Quick9 => {
                let value = self.value(self.immediate_expr(index)?)?;
                let value = self.in_range(index, value, 1, 8, false)?;
                ((value & 7) as u16) << 9
            }
            Place::Data8 => {
                let value = self.value(self.immediate_expr(index)?)?;
                (self.in_range(index, value, -128, 127, false)? & 0xFF) as u16
            }
            Place::Vector => {
                let value = self.value(self.immediate_expr(index)?)?;
                self.in_range(index, value, 0, 15, false)? as u16
            }
            Place::Disp8 => {
                let (distance, odd) = self.branch(index)?;
                if distance == 0 {
                    return Err(problem(
                        ASM_SHORT_BRANCH_ZERO,
                        self.op(index)?.range,
                        Vec::new(),
                    ));
                }
                if !(-128..=127).contains(&distance) {
                    return Err(self.branch_out_of_range(index, distance, Size::Short));
                }
                return Ok(((distance & 0xFF) as u16, odd));
            }
            Place::Ea
            | Place::Disp16
            | Place::Imm
            | Place::ImmWord
            | Place::ImmByte
            | Place::RegMask
            | Place::Implied => 0,
        };
        Ok((bits, None))
    }

    /// Appends the extension words of the operand at `index`; returns the odd-target warning of
    /// a word branch.
    fn extension(&mut self, place: Place, index: usize) -> Result<Option<Problem>, Problem> {
        let size = self.choice.size;
        match place {
            Place::Ea => {
                let op = self.op(index)?;
                if let Operand::Ea(ea) = &op.item {
                    self.ea_extension(ea, index)?;
                }
            }
            Place::Reg9 | Place::Reg0 => {
                if let Operand::Ea(Ea::Disp(expr, _)) = &self.op(index)?.item {
                    let value = self.value(expr)?;
                    let value = self.in_range(index, value, -32768, 32767, false)?;
                    self.out.push((value & 0xFFFF) as u16);
                }
            }
            Place::Disp16 => {
                let (distance, odd) = self.branch(index)?;
                if !(-32768..=32767).contains(&distance) {
                    return Err(self.branch_out_of_range(index, distance, Size::Word));
                }
                self.out.push((distance & 0xFFFF) as u16);
                return Ok(odd);
            }
            Place::Imm => {
                let value = self.value(self.immediate_expr(index)?)?;
                self.immediate(index, value, size)?;
            }
            Place::ImmWord => {
                let value = self.value(self.immediate_expr(index)?)?;
                // LINK takes a signed displacement; STOP and the SR forms a `.W` immediate.
                let max = if self.choice.entry.name == "LINK" {
                    32767
                } else {
                    65535
                };
                let value = self.in_range(index, value, -32768, max, false)?;
                self.out.push((value & 0xFFFF) as u16);
            }
            Place::ImmByte => {
                let value = self.value(self.immediate_expr(index)?)?;
                let min = if is_bit_operation(self.choice.entry) {
                    0
                } else {
                    -128
                };
                let value = self.in_range(index, value, min, 255, false)?;
                self.out.push((value & 0xFF) as u16);
            }
            Place::RegMask => {
                let mask = self.register_mask(index)?;
                self.out.push(mask);
            }
            Place::Quick9 | Place::Data8 | Place::Vector | Place::Disp8 | Place::Implied => {}
        }
        Ok(None)
    }

    /// Appends an immediate of the operation size.
    fn immediate(&mut self, index: usize, value: i64, size: Option<Size>) -> Result<(), Problem> {
        match size {
            Some(Size::Byte) => {
                let value = self.in_range(index, value, -128, 255, false)?;
                self.out.push((value & 0xFF) as u16);
            }
            Some(Size::Long) => {
                self.out.push(((value >> 16) & 0xFFFF) as u16);
                self.out.push((value & 0xFFFF) as u16);
            }
            Some(Size::Word | Size::Short) | None => {
                let value = self.in_range(index, value, -32768, 65535, false)?;
                self.out.push((value & 0xFFFF) as u16);
            }
        }
        Ok(())
    }

    /// Appends the extension words of an effective address.
    fn ea_extension(&mut self, ea: &Ea, index: usize) -> Result<(), Problem> {
        match ea {
            Ea::Dn(_) | Ea::An(_) | Ea::Ind(_) | Ea::PostInc(_) | Ea::PreDec(_) => {}
            Ea::Disp(expr, _) => {
                let value = self.value(expr)?;
                let value = self.in_range(index, value, -32768, 32767, false)?;
                self.out.push((value & 0xFFFF) as u16);
            }
            Ea::Index(expr, _, xn) => {
                let value = self.value(expr)?;
                let value = self.in_range(index, value, -128, 127, false)?;
                self.out.push(brief_word(*xn, value));
            }
            Ea::PcDisp(expr) => {
                let target = self.value(expr)?;
                let target = self.in_range(index, target, 0, MAX_ADDRESS, true)?;
                let distance = target - self.next_address();
                let distance = self.in_range(index, distance, -32768, 32767, false)?;
                self.out.push((distance & 0xFFFF) as u16);
            }
            Ea::PcIndex(expr, xn) => {
                let target = self.value(expr)?;
                let target = self.in_range(index, target, 0, MAX_ADDRESS, true)?;
                let distance = target - self.next_address();
                let distance = self.in_range(index, distance, -128, 127, false)?;
                self.out.push(brief_word(*xn, distance));
            }
            Ea::Abs(expr, Some(Size::Word)) => {
                let value = self.value(expr)?;
                let value = self.in_range(index, value, -32768, 32767, false)?;
                self.out.push((value & 0xFFFF) as u16);
            }
            Ea::Abs(expr, _) => {
                let value = self.value(expr)?;
                let value = self.in_range(index, value, 0, MAX_ADDRESS, true)?;
                self.out.push(((value >> 16) & 0xFFFF) as u16);
                self.out.push((value & 0xFFFF) as u16);
            }
            Ea::Imm(expr) => {
                let value = self.value(expr)?;
                self.immediate(index, value, self.choice.size)?;
            }
        }
        Ok(())
    }

    /// The MOVEM mask of the list operand at `index`, reversed for a `-(An)` destination.
    fn register_mask(&self, index: usize) -> Result<u16, Problem> {
        let mask = match &self.op(index)?.item {
            Operand::RegList(mask) => *mask,
            Operand::Ea(Ea::Dn(r)) => 1 << (r & 7),
            Operand::Ea(Ea::An(r)) => 1 << (8 + (r & 7)),
            _ => return Err(self.no_form()),
        };
        let predecrement = self
            .ops
            .iter()
            .any(|op| matches!(op.item, Operand::Ea(Ea::PreDec(_))));
        Ok(if predecrement {
            mask.reverse_bits()
        } else {
            mask
        })
    }
}

/// Whether `entry` is BTST, BCHG, BCLR or BSET (bit numbers 0…255).
fn is_bit_operation(entry: &Instruction) -> bool {
    matches!(entry.name, "BTST" | "BCHG" | "BCLR" | "BSET")
}

/// The brief extension word of `d8(An,Xn)` and `d8(PC,Xn)`.
fn brief_word(xn: Xn, displacement: i64) -> u16 {
    let kind = if xn.reg >= 8 { 0x8000 } else { 0 };
    let long = if xn.long { 0x0800 } else { 0 };
    kind | u16::from(xn.reg & 7) << 12 | long | (displacement & 0xFF) as u16
}

/// The size field bits of the opword.
fn size_bits(field: SizeField, size: Option<Size>) -> u16 {
    let Some(size) = size else {
        return 0;
    };
    match (field, size) {
        (SizeField::Bits7_6, Size::Byte) => 0x0000,
        (SizeField::Bits7_6, Size::Word) => 0x0040,
        (SizeField::Bits7_6, Size::Long) => 0x0080,
        (SizeField::Move13_12, Size::Byte) => 0x1000,
        (SizeField::Move13_12, Size::Word) => 0x3000,
        (SizeField::Move13_12, Size::Long) => 0x2000,
        (SizeField::Bit8, Size::Long) => 0x0100,
        (SizeField::Bit6, Size::Long) => 0x0040,
        (SizeField::Ext8_6, Size::Word) => 0x0080,
        (SizeField::Ext8_6, Size::Long) => 0x00C0,
        _ => 0,
    }
}
