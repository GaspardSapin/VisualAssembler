//! `va isa <MNEMONIC[.SIZE]>` and `va isa --list`: the instruction documentation of the default
//! architecture, from its CPU-agnostic `InstructionSet` view and its Fluent resources
//! (m1-core-isa-lexer.md §7.4).
//!
//! The argument splits at its first `.` into spelling and size; `describe` does the rest. Text
//! mode prints the summary line `<SPELLING>: <summary>`, the details as written in the Fluent file
//! (newlines kept), the condition, aliases, borrowed forms and flags note when present, then one
//! block per form. JSON mode prints one [`ISA_SCHEMA`] or [`ISA_LIST_SCHEMA`] line. An unknown
//! spelling (with up to 3 suggestions within edit distance 2) or a size that no form accepts
//! exits 1, reported like any failure (a `va.error/1` document in JSON mode). The labels of the
//! text layout are English CLI chrome; summaries, details and outcome and unit names are
//! localized.

use std::cmp::Reverse;
use std::io::{self, Write};

use serde::Serialize;
use va_core::isa::{
    Bound, Cycles, EaTiming, FlagEffect, FormInfo, InstructionInfo, InstructionSet, LookupError,
    TimingInfo,
};

use crate::commands::Session;
use crate::report::{CommandError, ErrorCode, Outcome};

/// Schema identifier of the JSON document printed by `va isa <MNEMONIC> --json`.
pub const ISA_SCHEMA: &str = "va.isa/1";

/// Schema identifier of the JSON document printed by `va isa --list --json`.
pub const ISA_LIST_SCHEMA: &str = "va.isa.list/1";

/// At most this many spellings are suggested for an unknown mnemonic.
const MAX_SUGGESTIONS: usize = 3;

/// Spellings within this edit distance of an unknown mnemonic are suggested.
const MAX_SUGGESTION_DISTANCE: usize = 2;

/// The Fluent attribute holding the long description of an instruction.
const DETAILS: &str = "details";

/// The Fluent attribute explaining the `Special` flag effects of an instruction.
const FLAGS: &str = "flags";

/// Runs `va isa <query>`: describes one instruction.
///
/// # Errors
///
/// `isa-unknown-mnemonic` or `isa-size-not-accepted` (exit 1), or a stdout failure (exit 3).
pub fn run(session: &Session, query: &str, json: bool) -> Result<Outcome, CommandError> {
    let (spelling, size) = match query.split_once('.') {
        Some((spelling, size)) => (spelling, Some(size)),
        None => (query, None),
    };
    let isa = session.arch.instruction_set();
    let info = isa
        .describe(spelling, size)
        .map_err(|error| lookup_failure(isa, spelling, size, error))?;
    let texts = Texts::new(session, &info);
    let written = if json {
        write_line(&IsaDocument::new(session, query, &info, &texts))
    } else {
        write_text(&describe_text(session, &info, &texts))
    };
    written
        .map(|()| Outcome::Clean)
        .map_err(CommandError::Stdout)
}

/// Runs `va isa --list`: every spelling with its canonical spelling and summary, in the order of
/// `InstructionSet::spellings`.
///
/// # Errors
///
/// A stdout failure (exit 3).
pub fn run_list(session: &Session, json: bool) -> Result<Outcome, CommandError> {
    let isa = session.arch.instruction_set();
    let mnemonics: Vec<ListEntry> = isa
        .spellings()
        .iter()
        .filter_map(|&spelling| isa.describe(spelling, None).ok())
        .map(|info| ListEntry {
            spelling: info.spelling,
            canonical: info.canonical,
            summary: text(session, info.doc_id, None),
        })
        .collect();
    let written = if json {
        write_line(&ListDocument {
            schema: ISA_LIST_SCHEMA,
            arch: session.arch.id(),
            lang: session.localizer.lang().code(),
            mnemonics: &mnemonics,
        })
    } else {
        let mut out = String::new();
        for entry in &mnemonics {
            out.push_str(entry.spelling);
            out.push_str("  ");
            out.push_str(&entry.summary);
            out.push('\n');
        }
        write_text(&out)
    };
    written
        .map(|()| Outcome::Clean)
        .map_err(CommandError::Stdout)
}

/// Turns a failed lookup into its reported failure (exit 1).
fn lookup_failure(
    isa: &dyn InstructionSet,
    spelling: &str,
    size: Option<&str>,
    error: LookupError,
) -> CommandError {
    match error {
        LookupError::Unknown => {
            let suggestions = suggestions(isa.spellings(), spelling);
            let message = if suggestions.is_empty() {
                format!("unknown mnemonic '{spelling}'")
            } else {
                format!(
                    "unknown mnemonic '{spelling}' (did you mean {}?)",
                    suggestions.join(", ")
                )
            };
            CommandError::reported(ErrorCode::IsaUnknownMnemonic, message)
        }
        LookupError::SizeNotAccepted => {
            let size = size.unwrap_or("");
            let message = match isa.describe(spelling, None) {
                Ok(info) => {
                    let accepted = accepted_sizes(&info.forms);
                    if accepted.is_empty() {
                        format!(
                            "{} does not accept the size suffix '.{size}' (it takes no size suffix)",
                            info.spelling
                        )
                    } else {
                        let accepted: Vec<String> =
                            accepted.iter().map(|size| format!(".{size}")).collect();
                        format!(
                            "{} does not accept the size suffix '.{size}' (accepted: {})",
                            info.spelling,
                            accepted.join(", ")
                        )
                    }
                }
                Err(_) => format!("'{spelling}' does not accept the size suffix '.{size}'"),
            };
            CommandError::reported(ErrorCode::IsaSizeNotAccepted, message)
        }
    }
}

/// The size letters of `forms`, each once, in order of first appearance.
fn accepted_sizes(forms: &[FormInfo]) -> Vec<&'static str> {
    let mut sizes = Vec::new();
    for size in forms.iter().flat_map(|form| form.sizes.iter().copied()) {
        if !sizes.contains(&size) {
            sizes.push(size);
        }
    }
    sizes
}

/// Up to [`MAX_SUGGESTIONS`] spellings within [`MAX_SUGGESTION_DISTANCE`] of `query` (compared
/// in uppercase): the closest first, then the longest common prefix with the query, then the
/// order of `spellings`.
fn suggestions(spellings: &[&'static str], query: &str) -> Vec<&'static str> {
    let query = query.to_ascii_uppercase();
    let mut candidates: Vec<(usize, Reverse<usize>, usize, &'static str)> = spellings
        .iter()
        .enumerate()
        .filter_map(|(index, &spelling)| {
            let distance = edit_distance(query.as_bytes(), spelling.as_bytes());
            (distance <= MAX_SUGGESTION_DISTANCE).then(|| {
                let prefix = query
                    .bytes()
                    .zip(spelling.bytes())
                    .take_while(|(a, b)| a == b)
                    .count();
                (distance, Reverse(prefix), index, spelling)
            })
        })
        .collect();
    candidates.sort_unstable();
    candidates
        .into_iter()
        .take(MAX_SUGGESTIONS)
        .map(|(_, _, _, spelling)| spelling)
        .collect()
}

/// The Levenshtein distance between two byte strings (insertions, deletions, substitutions).
fn edit_distance(a: &[u8], b: &[u8]) -> usize {
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0; b.len() + 1];
    for (i, &left) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, &right) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(left != right);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

/// Formats the message `id` (or its `attribute`) in the session's language; falls back to the id
/// itself (`id.attribute`) when no language defines it, as `Localizer::render` does for codes.
fn text(session: &Session, id: &str, attribute: Option<&str>) -> String {
    session
        .localizer
        .message(id, attribute, &[])
        .unwrap_or_else(|| match attribute {
            Some(attribute) => format!("{id}.{attribute}"),
            None => id.to_owned(),
        })
}

/// The Fluent id of an outcome or unit name: `isa-<arch>-<kind>-<id>` (m1 §3.5).
fn named_id(session: &Session, kind: &str, id: &str) -> String {
    format!("isa-{}-{kind}-{id}", session.arch.id())
}

/// The localized texts of one instruction.
struct Texts {
    /// The one-line summary (`<doc_id>`).
    summary: String,
    /// The long description (`<doc_id>.details`), newlines kept.
    details: String,
    /// The explanation of `Special` flag effects (`<doc_id>.flags`), if the entry has one.
    flags_note: Option<String>,
    /// The meaning of the condition of a conditional spelling.
    condition: Option<String>,
}

impl Texts {
    /// Looks up the texts of `info` in the session's language.
    fn new(session: &Session, info: &InstructionInfo) -> Self {
        Self {
            summary: text(session, info.doc_id, None),
            details: text(session, info.doc_id, Some(DETAILS)),
            flags_note: session.localizer.message(info.doc_id, Some(FLAGS), &[]),
            condition: info
                .condition
                .map(|condition| text(session, condition.doc_id, None)),
        }
    }
}

/// Writes `value` as one JSON line on stdout and flushes it.
fn write_line<T: Serialize>(value: &T) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    // serde_json converts its error into io::Error, keeping the original I/O error if any.
    serde_json::to_writer(&mut stdout, value)?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

/// Writes `text` on stdout and flushes it.
fn write_text(text: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()
}

/// The one-character symbol of a flag effect in the text form.
fn flag_symbol(effect: FlagEffect) -> char {
    match effect {
        FlagEffect::Unaffected => '-',
        FlagEffect::Result => '*',
        FlagEffect::Cleared => '0',
        FlagEffect::Set => '1',
        FlagEffect::Undefined => 'U',
        FlagEffect::Special => '!',
    }
}

/// Motorola's `n(r/w)` notation.
fn cycles_notation(cycles: Cycles) -> String {
    format!("{}({}/{})", cycles.clocks, cycles.reads, cycles.writes)
}

/// The notation of operand `index` of `form`, or `operand <index + 1>` if there is none.
fn operand_notation(form: &FormInfo, index: u8) -> String {
    form.operands.get(usize::from(index)).map_or_else(
        || format!("operand {}", u16::from(index) + 1),
        |operand| operand.notation.to_owned(),
    )
}

/// The text form of one instruction.
fn describe_text(session: &Session, info: &InstructionInfo, texts: &Texts) -> String {
    let mut out = format!("{}: {}\n{}\n", info.spelling, texts.summary, texts.details);
    if let (Some(condition), Some(meaning)) = (info.condition, &texts.condition) {
        out.push_str(&format!(
            "condition: {} ({}) {meaning}\n",
            condition.name, condition.test
        ));
    }
    if !info.aliases.is_empty() {
        out.push_str(&format!("aliases: {}\n", info.aliases.join(", ")));
    }
    if !info.also_accepts.is_empty() {
        out.push_str(&format!("also accepts: {}\n", info.also_accepts.join(", ")));
    }
    if let Some(note) = &texts.flags_note {
        out.push_str(&format!("flags: {note}\n"));
    }
    for form in &info.forms {
        out.push('\n');
        push_form(&mut out, session, form);
    }
    out
}

/// Appends the text block of one form.
fn push_form(out: &mut String, session: &Session, form: &FormInfo) {
    out.push_str(&form.syntax);
    out.push('\n');
    if let Some(size) = form.default_size {
        out.push_str(&format!("  default .{size}\n"));
    }
    if form.privileged {
        out.push_str("  privileged\n");
    }
    for (index, operand) in form.operands.iter().enumerate() {
        if !operand.modes.is_empty() {
            out.push_str(&format!(
                "  operand {}: {}\n",
                index + 1,
                operand.modes.join(", ")
            ));
        }
    }
    if !form.flags.is_empty() {
        let flags: Vec<String> = form
            .flags
            .iter()
            .map(|flag| format!("{}{}", flag.flag, flag_symbol(flag.effect)))
            .collect();
        out.push_str(&format!("  flags: {}\n", flags.join(" ")));
    }
    for row in &form.timing {
        out.push_str(&cycles_line(session, form, row));
        out.push('\n');
    }
}

/// One `  cycles` line: `  cycles[ .<sizes>][ <operand>=<modes>][ (<outcome>)]: n(r/w)[ + ea]
/// [ + n(r/w) × <unit>][ (max)]`.
fn cycles_line(session: &Session, form: &FormInfo, row: &TimingInfo) -> String {
    let mut line = String::from("  cycles");
    if !row.sizes.is_empty() {
        line.push_str(" .");
        line.push_str(&row.sizes.join("/"));
    }
    for condition in &row.when {
        line.push(' ');
        line.push_str(&operand_notation(form, condition.operand));
        line.push('=');
        line.push_str(&condition.modes.join(","));
    }
    if let Some(outcome) = row.outcome {
        let name = text(session, &named_id(session, "outcome", outcome), None);
        line.push_str(&format!(" ({name})"));
    }
    line.push_str(": ");
    line.push_str(&cycles_notation(row.base));
    for &operand in &row.plus_ea {
        if row.plus_ea.len() == 1 {
            line.push_str(" + ea");
        } else {
            line.push_str(&format!(" + ea({})", operand_notation(form, operand)));
        }
    }
    if let Some(per_unit) = row.per_unit {
        let unit = text(session, &named_id(session, "unit", per_unit.unit), None);
        line.push_str(&format!(" + {} × {unit}", cycles_notation(per_unit.cycles)));
    }
    if row.bound == Bound::Max {
        line.push_str(" (max)");
    }
    line
}

/// One entry of `va.isa.list/1`. Field order is the JSON key order.
#[derive(Serialize)]
struct ListEntry {
    /// The spelling, uppercase.
    spelling: &'static str,
    /// Its canonical spelling.
    canonical: &'static str,
    /// The localized one-line summary.
    summary: String,
}

/// The `va.isa.list/1` document. Field order is the JSON key order.
#[derive(Serialize)]
struct ListDocument<'a> {
    /// Always [`ISA_LIST_SCHEMA`].
    schema: &'static str,
    /// The architecture id.
    arch: &'static str,
    /// The language code of the texts.
    lang: &'static str,
    /// Every spelling, in order.
    mnemonics: &'a [ListEntry],
}

/// The `condition` object of `va.isa/1`.
#[derive(Serialize)]
struct ConditionJson<'a> {
    /// Condition name, uppercase.
    name: &'static str,
    /// The flag test, in notation.
    test: &'static str,
    /// The localized meaning.
    summary: &'a str,
}

/// One operand of a form.
#[derive(Serialize)]
struct OperandJson<'a> {
    /// The operand in notation.
    notation: &'static str,
    /// The accepted modes, in notation.
    modes: &'a [&'static str],
}

/// The effect of a form on one flag.
#[derive(Serialize)]
struct FlagJson {
    /// Flag name.
    flag: &'static str,
    /// `FlagEffect::id`.
    effect: &'static str,
}

/// A restriction of a timing row to some modes of one operand.
#[derive(Serialize)]
struct WhenJson<'a> {
    /// 0-based operand index.
    operand: u8,
    /// The modes, in notation.
    modes: &'a [&'static str],
}

/// The per-unit cost of a timing row.
#[derive(Serialize)]
struct PerUnitJson {
    /// Clock periods per unit.
    clocks: u16,
    /// Bus reads per unit.
    reads: u8,
    /// Bus writes per unit.
    writes: u8,
    /// Unit id.
    unit: &'static str,
}

/// One timing row. Field order is the JSON key order.
#[derive(Serialize)]
struct TimingJson<'a> {
    /// Size letters the row applies to.
    sizes: &'a [&'static str],
    /// Mode restrictions; empty means any operands.
    when: Vec<WhenJson<'a>>,
    /// Outcome id, or `null` for a row that does not depend on an outcome.
    outcome: Option<&'static str>,
    /// Base clock periods.
    clocks: u16,
    /// Base bus reads.
    reads: u8,
    /// Base bus writes.
    writes: u8,
    /// 0-based indexes of the operands whose EA time is added.
    plus_ea: &'a [u8],
    /// The per-unit cost, or `null`.
    per_unit: Option<PerUnitJson>,
    /// `Bound::id`.
    bound: &'static str,
}

/// One form. Field order is the JSON key order.
#[derive(Serialize)]
struct FormJson<'a> {
    /// The form in notation.
    syntax: &'a str,
    /// Accepted size letters.
    sizes: &'a [&'static str],
    /// The size used without a suffix, or `null`.
    default_size: Option<&'static str>,
    /// Whether the form needs supervisor mode.
    privileged: bool,
    /// The operands, in source order.
    operands: Vec<OperandJson<'a>>,
    /// Effect on each condition flag.
    flags: Vec<FlagJson>,
    /// Cycle counts.
    timing: Vec<TimingJson<'a>>,
}

/// One EA calculation time.
#[derive(Serialize)]
struct EaTimingJson<'a> {
    /// The mode, in notation.
    mode: &'static str,
    /// Size letters.
    sizes: &'a [&'static str],
    /// Clock periods.
    clocks: u16,
    /// Bus reads.
    reads: u8,
    /// Bus writes.
    writes: u8,
}

/// The `va.isa/1` document. Field order is the JSON key order (m1 §7.4).
#[derive(Serialize)]
struct IsaDocument<'a> {
    /// Always [`ISA_SCHEMA`].
    schema: &'static str,
    /// The architecture id.
    arch: &'static str,
    /// The language code of the texts.
    lang: &'static str,
    /// The argument as given.
    query: &'a str,
    /// The spelling, uppercase.
    spelling: &'static str,
    /// Its canonical spelling.
    canonical: &'static str,
    /// `Bcc`, `DBcc`, `Scc`, or `null`.
    family: Option<&'static str>,
    /// The condition of a conditional spelling, or `null`.
    condition: Option<ConditionJson<'a>>,
    /// The other spellings of the canonical one.
    aliases: &'a [&'static str],
    /// The table entries whose forms this spelling also accepts.
    also_accepts: &'a [&'static str],
    /// The normalized size filter, or `null`.
    size: Option<&'static str>,
    /// The raw Fluent summary.
    summary: &'a str,
    /// The raw Fluent `.details` (may contain newlines).
    details: &'a str,
    /// The raw Fluent `.flags`, or `null`.
    flags_note: Option<&'a str>,
    /// The forms that accept `size`.
    forms: Vec<FormJson<'a>>,
    /// The EA calculation times; non-empty iff a timing row has `plus_ea`.
    ea_timing: Vec<EaTimingJson<'a>>,
}

impl<'a> IsaDocument<'a> {
    /// Builds the document of `info` for the argument `query`.
    fn new(
        session: &'a Session,
        query: &'a str,
        info: &'a InstructionInfo,
        texts: &'a Texts,
    ) -> Self {
        let isa = session.arch.instruction_set();
        let adds_ea = info
            .forms
            .iter()
            .flat_map(|form| &form.timing)
            .any(|row| !row.plus_ea.is_empty());
        let ea_timing: &'a [EaTiming] = if adds_ea { isa.ea_timing() } else { &[] };
        Self {
            schema: ISA_SCHEMA,
            arch: session.arch.id(),
            lang: session.localizer.lang().code(),
            query,
            spelling: info.spelling,
            canonical: info.canonical,
            family: info.family,
            condition: info.condition.zip(texts.condition.as_deref()).map(
                |(condition, summary)| ConditionJson {
                    name: condition.name,
                    test: condition.test,
                    summary,
                },
            ),
            aliases: &info.aliases,
            also_accepts: &info.also_accepts,
            size: info.size,
            summary: &texts.summary,
            details: &texts.details,
            flags_note: texts.flags_note.as_deref(),
            forms: info.forms.iter().map(form_json).collect(),
            ea_timing: ea_timing
                .iter()
                .map(|entry| EaTimingJson {
                    mode: entry.mode,
                    sizes: &entry.sizes,
                    clocks: entry.cycles.clocks,
                    reads: entry.cycles.reads,
                    writes: entry.cycles.writes,
                })
                .collect(),
        }
    }
}

/// The JSON form of one [`FormInfo`].
fn form_json(form: &FormInfo) -> FormJson<'_> {
    FormJson {
        syntax: &form.syntax,
        sizes: &form.sizes,
        default_size: form.default_size,
        privileged: form.privileged,
        operands: form
            .operands
            .iter()
            .map(|operand| OperandJson {
                notation: operand.notation,
                modes: &operand.modes,
            })
            .collect(),
        flags: form
            .flags
            .iter()
            .map(|flag| FlagJson {
                flag: flag.flag,
                effect: flag.effect.id(),
            })
            .collect(),
        timing: form
            .timing
            .iter()
            .map(|row| TimingJson {
                sizes: &row.sizes,
                when: row
                    .when
                    .iter()
                    .map(|condition| WhenJson {
                        operand: condition.operand,
                        modes: &condition.modes,
                    })
                    .collect(),
                outcome: row.outcome,
                clocks: row.base.clocks,
                reads: row.base.reads,
                writes: row.base.writes,
                plus_ea: &row.plus_ea,
                per_unit: row.per_unit.map(|per_unit| PerUnitJson {
                    clocks: per_unit.cycles.clocks,
                    reads: per_unit.cycles.reads,
                    writes: per_unit.cycles.writes,
                    unit: per_unit.unit,
                }),
                bound: row.bound.id(),
            })
            .collect(),
    }
}
