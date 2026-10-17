# M1 — Core contracts, 68000 ISA tables, shared lexer: file-level specification

| | |
|---|---|
| Status | Ready for development; amended by §12 (ratified wave 1–2 implementation, 2026-10-17) |
| Owner | Architect role |
| Date | 2026-10-17 |
| Depends on | `architecture.md` (crate map R1–R8, contracts §3, i18n §5, perf §7, tests §8, policies §9); `m0-foundations.md` §3 (workspace), §5 (CLI conventions), §17.4/§18.5 (source scan) |
| Milestone goal | Frozen `va-core` contracts; complete 68000 ISA tables; the A68k-dialect lexer shared by the highlighter (M4) and the assembler (M2); EN/FR instruction docs; `va lex` and `va isa` |

Normative words: **must** = required; *rejected* = considered and refused, with the reason. Code
blocks marked *frozen* are binding signatures: implementations may add private items and derive
traits, never remove or rename a public item. A change goes back to the Architect.

## 0. Exit criteria (all required)

| # | Criterion | How it is checked |
|---|---|---|
| E1 | `make test`, `make lint`, `make docs`, `make security` exit 0 on Windows and in WSL Ubuntu 26.04 | QA, both hosts |
| E2 | CI green on the 4 runners (jobs of m0 §8, unchanged) | `gh run list --workflow ci.yml` |
| E3 | Lexer tests produce every `TokenKind` and every `lex-*` code, the §5.7 table holds, and the coverage, ordering and no-panic properties hold (§5, §9) | `cargo test -p va-arch-m68k --test syntax` |
| E4 | ISA consistency tests pass: spelling list, form/size/mode rules, opword uniqueness over all 65,536 words, timing completeness, flag and privilege facts, golden opwords and cycles (§4.7) | `cargo test -p va-arch-m68k --test isa` |
| E5 | Every mnemonic, condition, outcome, unit and diagnostic is documented in `en` and `fr`, with identical id/attribute sets and no orphan (§6.3) | `cargo test -p va-arch-m68k --test locales` |
| E6 | The acceptance commands of §9.2 give the documented exit codes and schemas | QA |
| E7 | Lexer throughput within the §5.6 budget on the reference machine, with a `docs/perf/` report | Performance agent |
| E8 | Security review: no new `unsafe`, network or allow-list entry; input limits of §7.2 hold; `SECURITY.md` updated | Security agent |
| E9 | Legal: factual sources recorded (§4.6); a QA spot check of 10 doc entries finds no text copied from the manuals | QA |

## 1. Work packages and order

Interfaces are frozen in this spec, so wave 2 runs fully in parallel. Write scopes are disjoint.

| WP | Wave | Owner | Write scope |
|---|---|---|---|
| WP0 scaffold | 1 | Developer | Root `Cargo.toml` and `Cargo.lock` (§8), `Makefile` (§8), member manifests `arch/m68k/Cargo.toml`, `crates/va-workspace/Cargo.toml`, `apps/va/Cargo.toml` (§8). Compiling stub files that wave 2 then owns: `arch/m68k/src/{lib.rs,isa/mod.rs,syntax/mod.rs}`, `crates/va-workspace/src/{lib.rs,i18n/mod.rs}`. |
| WP-A core | 1 | Developer | `crates/va-core/src/**`, `crates/va-core/tests/**` (§3) |
| WP-B isa | 2 | Developer | `arch/m68k/src/isa/**`, `arch/m68k/tests/isa/**` (§4). First deliverable, on day 1: `isa::SPELLINGS` and `isa::spelling_info` (§4.3), which WP-C needs. |
| WP-C syntax | 2 | Developer | `arch/m68k/src/syntax/**`, `arch/m68k/tests/syntax/**`, `arch/m68k/tests/syntax_bench.rs`, `arch/m68k/fixtures/lex/**` (§5) |
| WP-D m68k glue + docs | 2 | Developer (EN and FR writing) | `arch/m68k/src/{lib.rs,arch.rs,registers.rs,locales.rs}`, `arch/m68k/locales/**`, `arch/m68k/tests/{arch.rs,locales.rs}` (§3.6, §6) |
| WP-E i18n | 2 | Developer | `crates/va-workspace/src/**`, `crates/va-workspace/tests/**` (§6.2) |
| WP-F cli | 2 (finishes after B–E land) | Developer | `apps/va/src/**`, `apps/va/tests/**` (§7) |
| — | 3 | Security, QA | `SECURITY.md`; QA reports |
| — | 4 | Performance | `docs/perf/<date>-m1-lexer.md` (§5.6) |
| WP-G docs | 5 | Documentarian | Every `README.md` (new folders included), `CLAUDE.md`, `AGENCEMENT.md`, `DEPENDANCE.md`, `IMPORT.md` (two new crates, §8) |

Developers write no README (as in M0). Each WP hands the Documentarian a list of its new files with
a one-line purpose. Wave 1 is one developer, in order: WP0 manifests, WP-A (all of §3 with tests),
then the WP0 stubs, which implement the WP-A traits.

## 2. File trees

### 2.1 New or changed at the end of M1

```
Cargo.toml, Cargo.lock, Makefile                                    # §8
crates/va-core/src/{lib,source,diag,syntax,isa,regs,image,arch}.rs  # §3 (one file per contract)
crates/va-core/tests/{source,diag,syntax,isa,regs,image}.rs
crates/va-workspace/src/{lib.rs, i18n/{mod,lang,localizer}.rs}      # §6.2
crates/va-workspace/tests/{i18n_lang,i18n_localizer}.rs
arch/m68k/src/{lib,arch,registers,locales}.rs                       # §3.6, §6.1
arch/m68k/src/isa/{mod,model,modes,conditions,encoding,timing,view,
                   table_arith,table_logic,table_move,table_flow}.rs  # §4
arch/m68k/src/syntax/{mod,lexer,literals,keywords,codes,spacing}.rs # §5 (spacing: §5.7)
arch/m68k/locales/{en,fr}/{isa,diagnostics}.ftl                     # §6.1
arch/m68k/fixtures/lex/{basic,labels,literals,comments,macros,errors}.{asm,tokens}
arch/m68k/tests/isa/{main,spellings,forms,encoding,timing,view}.rs
arch/m68k/tests/syntax/{main,kinds,fields,coverage,robustness,fixtures}.rs
arch/m68k/tests/{arch,locales,syntax_bench}.rs
apps/va/src/{main,cli,arch_registry,input,report}.rs, apps/va/src/commands/{mod,info,lex,isa}.rs
apps/va/tests/{binary,lex,isa}.rs
```

- **Fixtures move to `arch/<cpu>/fixtures/<area>/`** (depth 4). *Rejected:* `arch/m68k/tests/fixtures/<area>/`
  (architecture §8 wording), which is depth 5. Architecture §8 gets a cross-reference.
- va-core stays flat. *Rejected:* the folders of m0 §2.2 now; each file stays under ~400 lines, and
  M2/M3 add `asm.rs`, `disasm.rs`, `emu.rs`, `io.rs` (13 files with the README, within 15).
- Fixtures are LF-only UTF-8 (`.gitattributes` normalizes to LF). Byte-level cases (CRLF, CR, BOM,
  invalid UTF-8, NUL) are built in test code, never stored as files.

### 2.2 Projected tree at ~6 months (new folders and files only)

| Path | When | Purpose |
|---|---|---|
| `crates/va-core/src/{asm,disasm}.rs`, `{emu,io}.rs` | M2, M3 | Contracts of §3.7 |
| `arch/m68k/src/asm/{mod,parser,expr,symbols,directives,macros,conditional,encode,optimize,output}.rs` | M2 | A68k assembler: statements, expressions, labels, directives, macros, IFxx, encoding from the ISA tables, ADDQ/MOVEQ/short-branch optimizations, S-record/binary/listing |
| `arch/m68k/src/disasm/{mod,decode,format}.rs` | M2 | Decoder over the same `Form::matches` tables, A68k-syntax output |
| `arch/m68k/src/emu/{mod,adapter,bus,history}.rs`, `src/io/{mod,easy68k,d68k_tp}.rs` | M3 | `m68k` crate adapter, 24-bit bus, write log for undo; TRAP #15 I/O profiles |
| `arch/m68k/fixtures/{asm,disasm}/`, `tests/{asm/,disasm/,emu/,io/,roundtrip.rs,sst.rs}` | M2–M3 | Golden files; tests mirroring `src/`; SingleStepTests subset (vectors in gitignored `xtask/cache/`) |
| `arch/m68k/benches/` | M7 | criterion benches (§5.6) |
| `crates/va-workspace/src/{project,settings,keybindings,session,backup,watch}/`, `locales/{en,fr}/{ui,diagnostics}.ftl` | M4 | Files of architecture §6; architecture-independent strings |
| `crates/va-debug/src/{session,breakpoints,stepping,history}/` | M3 | Debug worker |
| `crates/va-lang/src/{analysis,symbols,navigation,completion,hover,outline}/` | M5 | Language services |
| `apps/va/src/commands/{asm,run,debug,symbols,hover}.rs`; `apps/visual-assembler/src/arch_registry.rs` | M2–M5; M4 | Remaining subcommands; GUI CPU registration |
| `xtask/src/{generate,sst,package}/` | M4, M3, M8 | 5k/100k file generator, SingleStepTests fetch, packaging |

## 3. `va-core` contracts (WP-A)

### 3.1 What lands when

M1 implements and freezes `source`, `diag`, `syntax`, `isa`, `regs`, `image` and `arch` (with the M1
methods). M2 adds `asm`, `disasm` and `Architecture::{assembler, disassembler}`; M3 adds `emu`,
`io` and `Architecture::{emulators, io_profiles}` (signatures in §3.7). `ProgramImage` and
`SourceMap` land in M1 although M2 produces them first, so M3 can start its adapter during M2, as
the plan's parallelism requires. *Rejected:* defining the M2/M3 traits in code now; their
associated types (`SymbolTable`, `StepOutcome`, …) are not designed yet and frozen guesses would churn.

va-core keeps its M0 rules: no I/O, no threads, no dependency, **no user-facing text** (no `Display`
on its error enums; callers render them). Ids returned by `name()`/`id()` methods are identifiers,
not prose. Derives (omitted below): small value types derive `Clone, Copy, Debug, PartialEq, Eq,
Hash` (`FileId`, `LineCol`, `DiagCode`, `RegId` also `PartialOrd, Ord`; `TextRange`, `LineState`,
`Cycles` also `Default`); other types derive `Clone, Debug, PartialEq, Eq` where their fields allow.

### 3.2 `source.rs` (frozen)

```rust
pub const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;   // 16 MiB per file
pub const MAX_LINE_BYTES: usize = 64 * 1024;            // longer lines are one invalid token (§5.2)
pub struct FileId(pub u32);
pub struct TextRange { pub start: u32, pub end: u32 }    // byte offsets, start <= end
impl TextRange {
    pub const fn new(start: u32, end: u32) -> Self;      // end below start is raised to start (no panic)
    pub const fn len(self) -> u32;  pub const fn is_empty(self) -> bool;
    pub const fn shifted(self, by: u32) -> Self;         // saturating; line-relative → file-relative
    pub fn slice(self, text: &str) -> Option<&str>;      // None if out of bounds or not on char boundaries
}
pub struct SourceSpan { pub file: FileId, pub range: TextRange }
pub struct LineCol { pub line: u32, pub column: u32 }    // both 1-based; column counts Unicode scalar values
pub struct SourceText { /* private: text, had_bom */ }
pub enum SourceError { TooLarge { bytes: usize }, NotUtf8 { offset: usize } }
impl SourceText {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, SourceError>; // size, UTF-8, strip one leading BOM
    pub fn from_string(text: String) -> Result<Self, SourceError>;  // size, strip one leading BOM
    pub fn as_str(&self) -> &str;  pub fn had_bom(&self) -> bool;  pub fn into_string(self) -> String;
}
pub fn lines(text: &str) -> Lines<'_>;          // Iterator<Item = (TextRange, &str)>: content without terminator
pub struct LineIndex { /* private: line start offsets */ }
impl LineIndex {
    pub fn new(text: &str) -> Self;  pub fn line_count(&self) -> u32;   // >= 1
    pub fn line_range(&self, line: u32) -> Option<TextRange>;        // 1-based line, content without terminator
    pub fn position(&self, text: &str, offset: u32) -> LineCol;      // offset clamped to text.len()
    pub fn offset(&self, text: &str, at: LineCol) -> Option<u32>;    // None past the line end
}
```

- **Line terminators are LF, CRLF and a lone CR**, identical in `lines` and `LineIndex`. `""` has 1
  line, `"a\n"` has 2 (the last is empty), `"a\r\nb\rc"` has 3. This is ropey's counting with
  `default-features = false, features = ["simd", "cr_lines"]`, which M4 must use. *Rejected:* LF/CRLF
  only, which would make a classic-Mac file one line in the assembler and many in the editor.
- **Spans are bytes; display columns are chars.** Every stored span (`Token`, `Diagnostic`,
  `SourceMap`) is a byte range. `LineCol.column` counts Unicode scalar values, tab = 1, which matches
  egui's char cursor (M4) and the `va hover f.asm:L:C` syntax (M5). *Rejected:* UTF-16 columns; only
  LSP needs them and LSP is a non-goal (architecture §1).
- `SourceText` strips a BOM, so offsets are relative to the text after it (the M4 editor uses the
  same type and restores the BOM on save). `NotUtf8.offset` is the first invalid input byte.

### 3.3 `diag.rs` (frozen)

```rust
pub struct DiagCode(pub &'static str);   // kebab-case [a-z0-9]+(-[a-z0-9]+)*; Fluent id = "diag-" + code
impl DiagCode { pub fn is_valid(self) -> bool; pub fn fluent_id(self) -> String; }
pub enum Severity { Error, Warning, Note }  impl Severity { pub const fn id(self) -> &'static str; }
pub enum DiagArg { Int(i64), Str(String) }
pub struct Diagnostic {
    pub code: DiagCode, pub severity: Severity, pub span: SourceSpan,
    pub blocking: bool,                                             // prevents the program image (§5.7)
    pub args: Vec<(&'static str, DiagArg)>,                         // Fluent variables, e.g. ("text", Str)
    pub related: Vec<Related>, pub suggestion: Option<Suggestion>,
}
pub struct Related { pub span: SourceSpan, pub code: DiagCode, pub args: Vec<(&'static str, DiagArg)> }
pub struct Suggestion { pub span: SourceSpan, pub replacement: String }   // explained by the parent's .help
impl Diagnostic {
    pub fn new(code: DiagCode, severity: Severity, span: SourceSpan) -> Self;  // blocking = severity is Error
    pub fn with_arg(self, name: &'static str, value: DiagArg) -> Self;
    pub fn non_blocking(self) -> Self;
}
```

*Rejected:* `DiagArg::Hex`/address variants. Number notation is CPU-specific (`$1F` vs `0x1F`), so
the producer passes a formatted `Str`. **`blocking`** separates how a problem is shown (severity) from
whether it stops output: M2 produces an image iff no diagnostic is blocking; warnings and notes are
never blocking. *Rejected:* deciding blocking from a list of codes in the assembler, which every
consumer (Problems panel, F5, `va asm`) would have to repeat.

### 3.4 `syntax.rs` (frozen)

```rust
pub struct LineState(pub u32);  impl LineState { pub const INITIAL: Self = Self(0); }
pub enum TokenKind {
    Whitespace, Comment, SuspectComment(DiagCode), Label, LocalLabel, Mnemonic, Directive, MacroName,
    SizeSuffix, Register, Symbol, Number, String, Operator, Punctuation, MacroParam, Invalid(DiagCode),
}
impl TokenKind { pub const fn name(self) -> &'static str; }  // kebab ids, e.g. "local-label", "invalid"
pub struct Token { pub kind: TokenKind, pub range: TextRange }  // line-relative bytes
pub enum KeywordKind { Mnemonic, Directive, Register, SizeSuffix }
pub struct KeywordInfo { pub name: &'static str, pub kind: KeywordKind, pub doc_id: Option<&'static str> }
pub trait SyntaxProvider: Send + Sync {
    /// Lexes one line (content without terminator) and APPENDS its tokens to `out`.
    fn lex_line(&self, line: &str, state: LineState, out: &mut Vec<Token>) -> LineState;
    /// Mnemonics, directives, registers and size suffixes (uppercase names), for completion and hover.
    fn keywords(&self) -> &[KeywordInfo];
    /// Diagnostics of one lexed line (file offsets). Default: `token_diagnostics`.
    fn line_diagnostics(&self, file: FileId, line_start: u32, line: &str, tokens: &[Token],
                        out: &mut Vec<Diagnostic>) { token_diagnostics(file, line_start, line, tokens, out) }
}
pub enum TokenInvariantError { Gap { at: u32 }, Overlap { at: u32 }, Empty { index: usize },
                               NotCharBoundary { at: u32 }, Short { end: u32 }, Long { end: u32 } }
pub fn check_line_tokens(line: &str, tokens: &[Token]) -> Result<(), TokenInvariantError>;
pub fn token_diagnostics(file: FileId, line_start: u32, line: &str, tokens: &[Token],
                         out: &mut Vec<Diagnostic>);  // Invalid → blocking Error; SuspectComment → non-blocking Error; arg "text"
```

**Contract of every `SyntaxProvider`** (tested for m68k in §9, reusable for any CPU through
`check_line_tokens`): the appended tokens are non-empty, contiguous, in order, start and end on char
boundaries, and cover exactly `0..line.len()`; an empty line appends nothing; `lex_line` never
panics, runs in O(len), is deterministic, and allocates nothing besides growing `out`.
`token_diagnostics` truncates the `text` argument to 32 chars plus `…`.

- **`TokenKind` is exhaustive** and final for M1–M6 (the highlighter matches every kind). Versus the
  architecture §3 sketch it adds `Whitespace` (full byte coverage), `MacroName` (unknown word in
  the operation field) and `SuspectComment` (§5.7); `Invalid` and `SuspectComment` carry their code
  so highlighter, assembler and `va lex` report the same problem. *Rejected:* a fieldless `Invalid`
  plus a side query, a second pass that can drift.
- **`state` stays** although A68k is line-oriented (m68k always returns `INITIAL`). *Rejected:*
  dropping it; a GAS-style dialect with `/* */` comments would then need a trait change in `crates/`.
- Quoted literals are always `String`; the M2 parser turns `'AB'` in an expression into a number.
  *Rejected:* `Number` for quoted operands; the stateless lexer cannot tell them from `DC.B 'AB'`.

### 3.5 `isa.rs`: CPU-agnostic instruction view (frozen)

The view feeds `va isa` (M1) and hover (M5, in `va-lang`, which may not depend on `arch/*`, rule R2).

```rust
pub trait InstructionSet: Send + Sync {
    fn spellings(&self) -> &[&'static str];                 // every accepted spelling, uppercase, sorted
    /// `spelling` without suffix, any case; `size` is the suffix letter(s) without the dot, any case.
    fn describe(&self, spelling: &str, size: Option<&str>) -> Result<InstructionInfo, LookupError>;
    fn ea_timing(&self) -> &[EaTiming];                      // referenced by TimingInfo::plus_ea
}
pub enum LookupError { Unknown, SizeNotAccepted }
pub struct InstructionInfo {
    pub spelling: &'static str, pub canonical: &'static str,           // "BHS" / "BCC"
    pub family: Option<&'static str>, pub condition: Option<ConditionInfo>,   // "Bcc" | "DBcc" | "Scc"
    pub aliases: Vec<&'static str>,                                    // other spellings of `canonical`
    pub also_accepts: Vec<&'static str>,                               // forms borrowed by this spelling (§4.1)
    pub size: Option<&'static str>, pub doc_id: &'static str,          // normalized filter "S"; "isa-m68k-bcc"
    pub forms: Vec<FormInfo>,                                          // only forms accepting `size`
}
pub struct ConditionInfo { pub name: &'static str, pub test: &'static str, pub doc_id: &'static str }
pub struct FormInfo {
    pub syntax: String,                                                // "ADD.W/L <ea>,Dn" (notation)
    pub sizes: Vec<&'static str>, pub default_size: Option<&'static str>, pub privileged: bool,
    pub operands: Vec<OperandInfo>, pub flags: Vec<FlagInfo>, pub timing: Vec<TimingInfo>,
}
pub struct OperandInfo { pub notation: &'static str, pub modes: Vec<&'static str> }
pub struct FlagInfo { pub flag: &'static str, pub effect: FlagEffect }
pub enum FlagEffect { Unaffected, Result, Cleared, Set, Undefined, Special }  // id(): "unaffected", …
pub struct Cycles { pub clocks: u16, pub reads: u8, pub writes: u8 }          // Motorola n(r/w)
pub enum Bound { Exact, Max }                                                 // id(): "exact" | "max"
pub struct TimingInfo {
    pub sizes: Vec<&'static str>, pub when: Vec<ModeCondition>,     // empty `when` = any operands
    pub outcome: Option<&'static str>,                              // id, e.g. "taken"
    pub base: Cycles, pub plus_ea: Vec<u8>,                          // operand indexes whose EA time is added
    pub per_unit: Option<PerUnit>, pub bound: Bound,
}
pub struct ModeCondition { pub operand: u8, pub modes: Vec<&'static str> }
pub struct PerUnit { pub cycles: Cycles, pub unit: &'static str }   // unit id, e.g. "shift-count"
pub struct EaTiming { pub mode: &'static str, pub sizes: Vec<&'static str>, pub cycles: Cycles }
```

- Outcome and unit ids are documented as `isa-<arch>-outcome-<id>` and `isa-<arch>-unit-<id>`.
  Mode and operand strings are notation, never translated.
- *Rejected:* letting `va isa` read the m68k tables directly. It would work for the CLI, but hover
  (M5) could not use it, and two renderings of the same facts would drift.

### 3.6 `regs.rs`, `image.rs`, `arch.rs` (frozen)

```rust
pub struct RegId(pub u16);  pub enum RegisterGroup { General, Address, Control, Status }
pub struct FlagField { pub name: &'static str, pub bit: u8, pub width: u8 }
pub struct RegisterSpec { pub id: RegId, pub name: &'static str, pub aliases: &'static [&'static str],
                          pub bits: u8, pub group: RegisterGroup, pub fields: &'static [FlagField] }
pub enum SegmentKind { Code, Data, Bss }
pub struct Segment { pub address: u64, pub kind: SegmentKind, pub bytes: Vec<u8> }  // Bss bytes are zeros
pub struct ProgramImage { /* private: segments sorted by address, entry */ }
pub enum ImageError { Overlap { first: usize, second: usize }, AddressOverflow { segment: usize } }
impl ProgramImage {
    pub fn new(segments: Vec<Segment>, entry: Option<u64>) -> Result<Self, ImageError>; // sorts, rejects overlap
    pub fn segments(&self) -> &[Segment]; pub fn entry(&self) -> Option<u64>;
    pub fn byte_at(&self, address: u64) -> Option<u8>;
}
pub struct SourceMapEntry { pub address: u64, pub len: u32, pub file: FileId, pub line: u32, pub range: TextRange }
pub struct SourceMap { /* private: entries by address, (file, line) index */ }
impl SourceMap {
    pub fn new(entries: Vec<SourceMapEntry>) -> Self;                       // sorts both indexes
    pub fn at_address(&self, address: u64) -> Option<&SourceMapEntry>;     // entry containing address
    pub fn addresses(&self, file: FileId, line: u32) -> impl Iterator<Item = u64> + '_;  // sorted
    pub fn entries(&self) -> &[SourceMapEntry];
}
pub trait Architecture: Send + Sync {
    fn id(&self) -> &'static str;                                   // "m68k"
    fn syntax(&self) -> &dyn SyntaxProvider;
    fn instruction_set(&self) -> &dyn InstructionSet;
    fn registers(&self) -> &'static [RegisterSpec];
    fn diagnostic_codes(&self) -> &'static [DiagCode];              // every code this CPU can emit
    fn locale_resources(&self, lang: &str) -> &'static [&'static str]; // embedded .ftl; [] if unknown
}
```

- Bss bytes are materialized as zeros. *Rejected:* a length-only Bss variant; it saves at most
  16 MiB on the 68000 and every consumer would special-case it. `diagnostic_codes()` is new versus
  architecture §3; it drives the completeness test of §6.3.
- m68k (WP-D): `pub struct M68k; pub static M68K: M68k = M68k;` in `arch.rs`, re-exported by
  `lib.rs` (which declares `pub mod isa; pub mod syntax;` and private `arch`, `registers`,
  `locales`). Registers: D0–D7 (General, 32), A0–A6 and A7 alias `SP` (Address, 32), USP, SSP, PC
  (Control, 32), SR (Status, 16) with fields T(15,1) S(13,1) I(8,3) X(4,1) N(3,1) Z(2,1) V(1,1) C(0,1).

### 3.7 M2/M3 contracts (signatures fixed now, code lands in M2/M3)

They refine architecture §3 and use the M1 types. The M2 and M3 specs finalize only the fields of
the types named in the last column.

| Lands | Signatures | Finalized by the milestone spec |
|---|---|---|
| M2 | `Architecture::assembler(&self) -> Box<dyn Assembler>`; `trait Assembler: Send { fn assemble(&mut self, input: &AssembleInput<'_>, cancel: &CancelToken) -> AssembleOutput; }`; `AssembleInput<'a> { root: FileId, sources: &'a dyn SourceProvider, options: &'a AsmOptions }`; `trait SourceProvider: Sync { fn text(&self, file: FileId) -> Option<&str>; fn resolve_include(&self, from: FileId, path: &str) -> Result<FileId, IncludeError>; }`; `AssembleOutput { image: Option<ProgramImage>, diagnostics: Vec<Diagnostic>, symbols: SymbolTable, source_map: SourceMap, listing: Option<Listing> }`; `CancelToken(Arc<AtomicBool>)` with `new`, `cancel`, `is_cancelled` | `AsmOptions`, `IncludeError`, `SymbolTable`, `Listing` |
| M2 | `Architecture::disassembler(&self) -> Box<dyn Disassembler>`; `trait Disassembler: Send { fn decode(&self, bytes: &[u8], address: u64) -> Result<DecodedInsn, DecodeError>; fn flow_kind(&self, insn: &DecodedInsn) -> FlowKind; }`; `FlowKind` as architecture §3 | `DecodedInsn` (length, text, tokens), `DecodeError` |
| M3 | `Architecture::emulators(&self) -> Vec<Box<dyn EmulatorFactory>>`, `io_profiles(&self) -> Vec<Box<dyn IoProfile>>`; `EmulatorFactory`, `Emulator`, `RunBudget`, `StopReason`, `IoProfile` exactly as architecture §3, with `RegId` from `regs` | `EmulatorConfig`, `StepOutcome`, `RegisterSnapshot`, `EmuError`, `Capabilities`, `ExceptionInfo`, `CpuState`, `Console`, `TrapOutcome`, `MmioRegion` |

## 4. 68000 ISA tables (WP-B, `arch/m68k/src/isa/`)

### 4.1 Scope (facts; A68k 2.71.F3w is the dialect reference)

**126 spellings**, which must equal the opcode table of A68k 2.71.F3w (observed in the binary).
They give **74 table entries** (one doc id each): one per spelling once the conditional spellings
merge into the families Bcc, DBcc and Scc and the aliases below are dropped. The CCR/SR/USP
variants are forms of ANDI, EORI, ORI and MOVE. Table files: `table_arith.rs` (ADD…, SUB…, CMP…,
MULS/MULU, DIVS/DIVU, NEG/NEGX, CLR, EXT, TST, ABCD/SBCD/NBCD, CHK), `table_logic.rs` (AND…, OR…,
EOR…, NOT, shifts and rotates, bit operations, TAS, Scc), `table_move.rs` (MOVE…, EXG, SWAP, LEA,
PEA, LINK, UNLK), `table_flow.rs` (Bcc, BRA, BSR, DBcc, JMP, JSR, RTE, RTR, RTS, TRAP, TRAPV, NOP,
RESET, STOP, ILLEGAL).

```
ABCD ADD ADDA ADDI ADDQ ADDX AND ANDI ASL ASR BCC BCHG BCLR BCS BEQ BGE BGT BHI BHS BLE BLO BLS
BLT BMI BNE BPL BRA BSET BSR BTST BVC BVS CHK CLR CMP CMPA CMPI CMPM DBCC DBCS DBEQ DBF DBGE DBGT
DBHI DBHS DBLE DBLO DBLS DBLT DBMI DBNE DBPL DBRA DBT DBVC DBVS DIVS DIVU EOR EORI EXG EXT ILLEGAL
JMP JSR LEA LINK LSL LSR MOVE MOVEA MOVEM MOVEP MOVEQ MULS MULU NBCD NEG NEGX NOP NOT OR ORI PEA
RESET ROL ROLX ROR RORX ROXL ROXR RTE RTR RTS SBCD SCC SCS SEQ SF SGE SGT SHI SHS SLE SLO SLS SLT
SMI SNE SPL ST STOP SUB SUBA SUBI SUBQ SUBX SVC SVS SWAP TAS TRAP TRAPV TST UNLK
```

- **Conditions** (code, test notation): T 0 `1`, F 1 `0`, HI 2 `!C & !Z`, LS 3 `C | Z`, CC 4 `!C`,
  CS 5 `C`, NE 6 `!Z`, EQ 7 `Z`, VC 8 `!V`, VS 9 `V`, PL 10 `!N`, MI 11 `N`, GE 12 `N == V`,
  LT 13 `N != V`, GT 14 `!Z & N == V`, LE 15 `Z | N != V`. Bcc uses 2–15 (codes 0/1 are BRA/BSR);
  DBcc and Scc use all 16.
- **Aliases** (same instruction): BHS=BCC, BLO=BCS, DBHS=DBCC, DBLO=DBCS, DBRA=DBF, SHS=SCC,
  SLO=SCS, ROLX=ROXL, RORX=ROXR.
- **`also_accepts`** (A68k selects the variant from the operands; observed): ADD→ADDA, ADDI;
  SUB→SUBA, SUBI; CMP→CMPA, CMPI, CMPM; AND→ANDI; OR→ORI; EOR→EORI; MOVE→MOVEA. The size-driven
  optimizations (ADDQ/SUBQ, MOVEQ, `ADD #n,An`→LEA, short branches; disabled by a68k `-n`) are M2
  assembler behaviour, not table data.
- **Sizes.** Without a suffix, a sized form is `.W` (observed on ADD, TST, CLR, EXT, MOVEM, CHK,
  LSL memory, MOVE SR). A single-size form accepts its own suffix (`LEA.L`, `SWAP.W`, `EXG.L`,
  `MOVEQ.L` observed) and rejects others. Unsized forms (NOP, RTS, RTE, RTR, RESET, TRAPV, ILLEGAL,
  STOP, TRAP, JMP, JSR, UNLK) reject any suffix. BTST/BCHG/BCLR/BSET are `.L` on Dn and `.B` on
  memory. Branches (Bcc, BRA, BSR) have two forms, `.S` (non-zero 8-bit displacement) and `.W`
  (low byte zero, extension word); suffix `.B` means S and `.L` means W (A68k compatibility,
  observed); without a suffix M2 chooses. DBcc is W. **Byte forms are separate** from forms whose
  operand may be An, which is never a byte operand (a68k: "Operand size error" on `add.b a0,d0`):
  `ADD.B <ea>,Dn` (data modes) and `ADD.W/L <ea>,Dn` (all modes, default W), likewise SUB, CMP and
  MOVE; ADDQ/SUBQ have a `#<1-8>,An` form (W/L, flags unaffected).
- **68000 only.** No 68010+ form: no CHK.L, no TST/CMPI on An, PC-relative or immediate operands,
  MOVE from SR is not privileged, no BKPT, RTD, MOVEC, EXTB, 32-bit branch displacements.
- **Privileged** forms are exactly: ANDI/EORI/ORI to SR, MOVE to SR, MOVE USP (both directions),
  RESET, RTE, STOP.

### 4.2 Data representation

**Decision: hand-written static Rust tables** (`static`/`const` items in `table_*.rs`), checked by
tests. Zero start-up cost, compile-time typed, readable in review diffs, no new dependency.
*Rejected:* a data file (TOML/CSV) parsed at run time, which needs a parser dependency in M1 and a
run-time error path for a build defect; *rejected:* `build.rs` generation, which needs an
`INCLUDE_ALLOWLIST` entry with a Security review (m0 §17.4 S8) and hides the data from review.

### 4.3 Model (`model.rs`, `modes.rs`, `conditions.rs`; frozen for WP-C, WP-D and M2)

```rust
pub enum Size { Byte, Word, Long, Short }                      // Short = 8-bit branch displacement
pub struct SizeSet(u8);                                        // SizeSet::NONE = unsized
pub enum Mode { Dn, An, Ind, PostInc, PreDec, Disp, Index, AbsW, AbsL, PcDisp, PcIndex, Imm } // Motorola order
pub struct ModeSet(u16);  // ALL, DATA, MEMORY, CONTROL, ALTERABLE, DATA_ALTERABLE, MEMORY_ALTERABLE, CONTROL_ALTERABLE
pub enum QuickRange { OneToEight, Signed8, Vector4 }           // ADDQ/SUBQ/shift #1-8, MOVEQ, TRAP #0-15
pub enum Operand { Ea(ModeSet), Dn, An, PostIncAn, PreDecAn, DispAn, Imm, Quick(QuickRange),
                   RegList, Label, Ccr, Sr, Usp }
pub struct Flags { pub x: FlagEffect, pub n: FlagEffect, pub z: FlagEffect, pub v: FlagEffect, pub c: FlagEffect }
pub struct Form { pub operands: &'static [Operand], pub sizes: SizeSet, pub default_size: Option<Size>,
                  pub flags: Flags, pub privileged: bool, pub outcomes: &'static [Outcome],
                  pub encoding: Encoding, pub timing: Timing }
pub enum Family { Bcc, DBcc, Scc }
pub struct Instruction { pub name: &'static str, pub family: Option<Family>, pub doc_id: &'static str,
                         pub forms: &'static [Form], pub also_accepts: &'static [&'static str] }
pub struct Condition { pub name: &'static str, pub code: u8, pub test: &'static str, pub doc_id: &'static str }
pub static INSTRUCTIONS: [Instruction; 74];
pub static SPELLINGS: [&str; 126];                             // sorted, the §4.1 list
pub struct SpellingInfo { pub canonical: &'static str, pub takes_operands: bool, pub doc_id: &'static str }
pub fn spelling_info(name: &str) -> Option<SpellingInfo>;      // ASCII case-insensitive, no allocation
pub fn instruction_of(name: &str) -> Option<(&'static Instruction, Option<&'static Condition>)>;  // same
pub struct M68kIsa;  // impl va_core::isa::InstructionSet (view.rs)
```

`FlagEffect`, `Cycles` and `Bound` are the `va_core::isa` types. Mode notation (frozen, used in
JSON): `Dn An (An) (An)+ -(An) d16(An) d8(An,Xn) xxx.W xxx.L d16(PC) d8(PC,Xn) #imm`. `Special`
marks an instruction-specific flag rule (ABCD/SBCD/NBCD/ADDX/SUBX/NEGX Z, CHK N, CCR/SR logic
forms); its doc entry must have a `.flags` attribute.

### 4.4 Cycle counts (`timing.rs`)

Facts come from the MC68000 timing tables (User's Manual §8, tables 8-1 to 8-14).

```rust
pub enum Outcome { Always, Taken, NotTaken, True, False, NotExpired, Expired, Trap, NoTrap }
pub enum Unit { ShiftCount, RegisterCount, OneBits, BitPairs }  // n in 6+2n, 8+4n, 38+2n (MULU, MULS)
pub struct TimingRow { pub sizes: SizeSet, pub operand: u8, pub modes: ModeSet, pub outcome: Outcome,
                       pub base: Cycles, pub plus_ea: bool, pub per: Option<(Cycles, Unit)>, pub bound: Bound }
pub enum Timing { Rows(&'static [TimingRow]), MoveMatrix }
pub static EA_TIME: [[Cycles; 2]; 12];        // [mode][0 = .B/.W, 1 = .L], table 8-1; Dn/An = 0(0/0)
pub static MOVE_BW: [[Cycles; 9]; 12];        // table 8-2: [source mode][destination column]
pub static MOVE_L: [[Cycles; 9]; 12];         // table 8-3; columns Dn An (An) (An)+ -(An) d16(An) d8(An,Xn) xxx.W xxx.L
pub fn cycles(form: &Form, size: Size, modes: [Option<Mode>; 2], outcome: Outcome, n: u32)
    -> Option<(Cycles, Bound)>;               // total, EA time and per-unit cost included
```

**How EA-dependent cycles are represented.** A form's rows partition the space
(size × addressing mode of the varying operand `operand` × outcome). A row gives the base cost and
whether the EA calculation time of that operand (`EA_TIME`) is added (the manual's `+`). A
footnote that changes the base for some modes becomes a separate row: `ADD.L <ea>,Dn` is
`8(1/0)+ea` for Dn, An, #imm and `6(1/0)+ea` for the other modes. Tables 8-10 (JMP, JSR, LEA, PEA,
MOVEM) give complete per-mode times, so their rows have `plus_ea = false`. MOVE and MOVEA use the
two full matrices. Count-dependent costs use `per` (shift count, MOVEM register count, MULU one
bits, MULS bit pairs). DIVU/DIVS store the manual's upper bound with `Bound::Max`, as do the bit
operations whose time depends on the bit number. *Rejected:* one precomputed total per
(form, size, mode) cell, which is ~3,000 hand-typed numbers instead of ~400 rows and loses the
manual's structure that students read.

### 4.5 Encodings (`encoding.rs`)

Every form carries its opword facts, so M1 can prove the table has no ambiguity and M2's encoder and
disassembler start from one source.

```rust
pub struct Encoding { pub opword: u16, pub mask: u16, pub size: SizeField, pub ea: EaField, pub rule: FieldRule }
pub enum SizeField { None, Bits7_6, Move13_12, Bit8, Bit6, Ext8_6 }   // 00/01/10, 01/11/10, W/L, W/L, 010/011
pub enum EaField { None, Low, MoveDest, Both }                        // bits 5–0; MOVE dest 11–6 (reg/mode swapped)
pub enum FieldRule { None, Disp8NonZero }                             // Bcc.S/BRA.S/BSR.S
impl Form { pub fn matches(&self, opword: u16, condition: Option<u8>) -> bool; }
```

`matches` is true when the fixed bits equal `opword`, the size field holds a value allowed by
`sizes`, every EA field decodes to a mode of its operand's `ModeSet` (mode 7 accepts registers 0–4
only), and `rule` holds. Free register and data fields accept any value.

### 4.6 Factual sources and legal rule

| Source | Used for |
|---|---|
| Motorola, *M68000 Family Programmer's Reference Manual* (M68000PM/AD rev. 1, 1992) | Instruction list, operand sizes, addressing-mode categories, condition codes, X N Z V C effects, opword bit patterns |
| Motorola, *MC68000 8-/16-/32-Bit Microprocessors User's Manual* (M68000UM/AD rev. 8), §8 | Clock periods and bus read/write counts (tables 8-1 to 8-14) |
| J. Cwik, *68000 Yacht* (cycle analysis) | Cross-check of the timing footnotes only |
| `a68k.exe` 2.71.F3w run locally (2026-10-17, read-only, outputs in a scratch folder) | Spellings, directives, aliases, size defaults, dialect rules, golden opwords of §4.7 |

Numbers, bit patterns and names are facts and may be reproduced. Every description (doc text,
comments) is written in our own words: no sentence, table caption or example is copied from the
manuals, from EASy68K's help, or from another program's tables. The table values are typed from the
manuals, never copied from other software. The Documentarian records this table in
`arch/m68k/src/isa/README.md`.

### 4.7 Consistency tests (`arch/m68k/tests/isa/`)

| File | Assertions |
|---|---|
| `spellings.rs` | `SPELLINGS` equals the §4.1 list exactly (sorted, unique, uppercase ASCII); family expansion plus aliases produce exactly it; every spelling resolves through `instruction_of`; alias targets exist; `also_accepts` targets are entries; each condition appears once per family |
| `forms.rs` | `default_size ∈ sizes`; unsized ⇔ `SizeSet::NONE`; operand counts 0–2; `Ea` sets are non-empty and inside the Motorola category they claim; privileged set equals §4.1; flag spot checks: MOVE `- * * 0 0`, ADD `* * * * *`, ADDA all `-`, CMP `- * * * *`, CLR `- 0 1 0 0`, BTST only Z `*`, ABCD Z `Special`, CHK N `Special`, LEA and Scc all `-` |
| `encoding.rs` | `opword & !mask == 0`; for each of the 65,536 opwords at most one (form, condition) matches; every form matches at least one opword; the number of valid opwords is printed and pinned as a golden constant after QA review; golden opwords (observed in a68k listings): `4E71` NOP, `4E75` RTS, `4AFC` ILLEGAL (not TAS), `747F` MOVEQ, `2200` MOVE.L, `3040` MOVEA.W, `51C8` DBF, `64D8` BCC.S, `6500` BCS.W, `60DC` BRA.S, `61C4` BSR.S, `55C0` SCS, `B308` CMPM.B, `C308` ABCD, `D380` ADDX.L, `C141` EXG, `4840` SWAP, `4880`/`48C0` EXT.W/.L, `43D0` LEA, `4E68` MOVE USP→An, `4E60` MOVE An→USP, `44C0` MOVE to CCR, `40C0` MOVE from SR, `007C` ORI to SR, `023C` ANDI to CCR, `0A3C` EORI to CCR, `4190` CHK, `48E7`/`4C9F` MOVEM.L/.W, `0108` MOVEP.W, `5240` ADDQ.W, `5181` SUBQ.L, `0681` ADDI.L, `D0C0` ADDA.W, `0C50` CMPI.W, `E348`/`E3D0`/`E362` LSL reg/LSL mem/ASL Dn, `0800`/`0310`/`08D0` BTST #/BTST Dn/BSET #, `4E56` LINK, `4E4F` TRAP, `4E72` STOP, `4A40` TST.W, `4250` CLR.W; and **no** match for `A000`, `F000`, `4E74` (RTD), `4E7A` (MOVEC), `49C0` (EXTB), `4AFB` (TAS d8(PC,Xn)), `4C00` |
| `timing.rs` | For every form, size, mode of the varying operand and declared outcome, exactly one row matches; rows use only modes of the operand's set; every `Cycles` has an even `clocks` and `clocks >= 4 × (reads + writes)`; `EA_TIME` spot checks (table 8-1): (An) 4(1/0)/8(2/0), -(An) 6(1/0)/10(2/0), d8(An,Xn) 10(2/0)/14(3/0), xxx.L 12(3/0)/16(4/0), #imm 4(1/0)/8(2/0); `cycles()` spot checks: NOP 4(1/0), RTS 16(4/0), RTE 20(5/0), MOVEQ 4(1/0), EXG 6(1/0), SWAP 4(1/0), LINK 16(2/2), UNLK 12(3/0), TRAP 34(4/3), Bcc.S taken 10(2/0) / not taken 8(1/0), Bcc.W not taken 12(2/0), BSR 18(2/2), DBcc true 12(2/0) / not expired 10(2/0) / expired 14(3/0), JSR (An) 16(2/2), JMP (An) 8(2/0), LEA (An) 4(1/0), PEA (An) 12(1/2), MOVE.W (An),(An) 12(2/1), MOVE.L (An),(An) 20(3/2), ADD.W Dn,Dn 4(1/0), ADD.L Dn,Dn 8(1/0), ADD.L (An),Dn 14(3/0), LSL.L #3,Dn 14(1/0), MULU max 70(1/0) for n = 16 |
| `view.rs` | `M68kIsa::spellings() == SPELLINGS`; `describe` is case-insensitive; `describe("BRA", Some("b"))` gives size `S`; `describe("BRA", Some("l"))` gives `W`; `describe("MOVEQ", Some("b"))` is `SizeNotAccepted`; `describe("MOV", None)` is `Unknown`; flag names equal the SR fields X N Z V C of `registers()`; every mode string is in the frozen notation list |

M3 adds two cross-checks against the `m68k` crate: every opword the table calls valid executes
without an illegal-instruction exception and vice versa, and `cycles()` matches the crate's cycle
count (within `Bound::Max` for DIVU/DIVS).

## 5. Lexer (WP-C, `arch/m68k/src/syntax/`)

`pub fn lex_line(line: &str, out: &mut Vec<Token>)` is the single implementation. `M68kSyntax`
(unit struct, `impl SyntaxProvider`) calls it and returns `LineState::INITIAL`; M2's parser calls
it directly. `pub static DIAG_CODES: [DiagCode; 11]` lists the codes of §5.4 and §5.7.

### 5.1 Line grammar (A68k, verified against `a68k.exe`)

```
line := '*' rest                                        (column 1: whole-line comment)
      | [label-field] [ws op-field [ws operand-field]] [ws trailing-comment]
```

Whitespace (ws) is space or tab. `;` outside a string starts a comment anywhere. **The operand
field ends at the first unquoted whitespace, and the rest of the line is a comment** (A68k:
`move.l d0, d1` is an error and `dc.b 1, 2, 3` assembles one byte). An identifier in column 1 is
always a label (A68k: `nop` in column 1 defines the label `nop`). A `*` after column 1 is never a
comment.

### 5.2 Token rules (normative)

Character classes: ID_START `A–Z a–z _ .`; ID_CONT `ID_START 0–9 @`. Keyword matching is ASCII
case-insensitive and allocation-free (a fixed table searched with `eq_ignore_ascii_case`).

| Field | Input | Token(s) |
|---|---|---|
| column 1 | `*` | `Comment` to end of line |
| any | `;` outside quotes | `Comment` to end of line |
| label (column 1, up to ws or `;`) | ID_START ID_CONT* | `Label` |
| label | DIGIT+ `$`, or `\` ID_START ID_CONT* | `LocalLabel` (A68k `1$`; TIGCC-style `\name`) |
| label | trailing run of `:` (`lab:`, `lab::`, `lab:::`; a68k assembles them identically) | one `Punctuation` |
| label | anything else | `Invalid(lex-bad-label)` to the field end if no label token yet, else `Invalid(lex-label-junk)` (e.g. `lab:nop`) |
| op (first word after ws) | label form followed by a run of `:`, when column 1 was blank | `Label`/`LocalLabel` + `Punctuation`, then the op field follows (A68k accepts `  name: nop`) |
| op | `=` | `Directive` |
| op | ID, split at its first `.` | stem: `Mnemonic` (a §4.1 spelling), `Directive` (§5.3) or `MacroName`; then `.` + one of B W L S → `SizeSuffix` (the token includes the dot); `.` + macro parameter → `Punctuation` + `MacroParam`; any other suffix → `Invalid(lex-bad-size-suffix)` to the field end |
| op | anything else | `Invalid(lex-bad-operation)` to the field end |
| after op | keyword without operands (§5.3, and NOP RESET RTE RTR RTS TRAPV ILLEGAL) | ws + `Comment` for any remaining text |
| operand (INCLUDE, INCBIN) | quoted text, or the unquoted field | `String` (paths may hold `\` and `:`) |
| operand | `'…'` or `"…"`, delimiter doubled inside | `String`; no backslash escapes (A68k: `'a\nb'` is 4 bytes); unterminated → `Invalid(lex-unterminated-string)` to **end of line** |
| operand | `$`HEX+, `%`BIN+, `@`OCT+, DIGIT+ | `Number`; a run glued to ID_CONT chars (`10h`, `0x10`, `$12g`) or a prefix with no digit (`$`, `%2`, `@8`) → `Invalid(lex-bad-number)` over the run |
| operand | DIGIT+ `$` | `LocalLabel` |
| operand | identifier equal to D0–D7, A0–A7, SP, USP, CCR, SR, PC | `Register`; `d0.w` style → `Register` + `SizeSuffix` |
| operand | other identifier (may contain `.` and `@`, A68k) | `Symbol` |
| operand | `\` ID_START ID_CONT* | `LocalLabel` |
| any field | `\0`–`\9`, `\@` (not in strings or comments) | `MacroParam`; the text around it is lexed by the field's rules (`lbl\@:` → `Label`, `MacroParam`, `Punctuation`) |
| operand | `\` + anything else | `Invalid(lex-bad-backslash)`: the backslash and the next char, or the backslash alone before whitespace, `;` or the line end |
| operand | `+ - * / ~ & ! \| << >>` | `Operator` (`*` is also the location counter; M2 decides) |
| operand | `# , ( )` | `Punctuation` |
| operand | `.` + one of B W L S right after `)` or a `Number` | `SizeSuffix` |
| operand | run of non-ASCII chars | `Invalid(lex-non-ascii)` |
| operand | run of other ASCII chars (`^ [ ] { } < > = ? :` backquote, controls, a lone `.`) | `Invalid(lex-unexpected-char)` |
| after operands | ws + rest | `Whitespace` + `Comment`, or `SuspectComment` when the §5.7 rule fires |

Every whitespace run is one `Whitespace` token; tabs are whitespace and count as one column. An
empty line yields no token. A line longer than `MAX_LINE_BYTES` is exactly one
`Invalid(lex-line-too-long)` token; any shorter line is lexed in linear time. A `\r` or `\n` inside
a line (API misuse) is `Invalid(lex-unexpected-char)`, and the text after it is lexed as a new line. Non-ASCII text is valid only in comments and
strings (`DC.B 'é'` is two UTF-8 bytes; M2 may warn); identifiers are ASCII.

### 5.3 Directives and operand policy (A68k 2.71.F3w directive table, observed)

| Policy | Directives |
|---|---|
| no operands (the rest is a comment) | BSS CODE CSEG DATA DSEG ENDC ENDIF ENDM EVEN FAR LIST MACRO MEXIT NOL NOLIST PAGE |
| path | INCBIN INCLUDE |
| operands (required or optional) | CNOP DC DCB DS END EQU EQUR IDNT IFC IFD IFEQ IFGE IFGT IFLE IFLT IFNC IFND IFNE NEAR ORG PUBLIC REG RORG SECTION SET SPC TITLE TTL XDEF XREF `=` |

49 spellings. `ELSE`, `REPT` and `OPT` are not A68k 2.71 directives: they lex as `MacroName` and M2
reports them. `NARG`, `CHIP`, `FAST`, `CODE`/`DATA`/`BSS` in operands are `Symbol`. Register
equates (`EQUR`, `REG`) are not tracked: their uses lex as `Symbol` (the lexer is stateless).
`keywords()` returns the 126 mnemonics (doc id from §4.3), the 49 directives (doc id `None` until
M2 documents them), the 21 registers and the 4 size suffixes.

### 5.4 Error codes (`codes.rs`)

`lex-bad-label`, `lex-label-junk`, `lex-bad-operation`, `lex-bad-size-suffix`, `lex-bad-number`,
`lex-unterminated-string`, `lex-bad-backslash`, `lex-unexpected-char`, `lex-non-ascii`,
`lex-line-too-long`. Each is a blocking `Severity::Error` when turned into a diagnostic, with the
`text` argument. The eleventh code, `lex-operands-in-comment`, is non-blocking (§5.7).

**Deliberately stricter than a68k** (a68k accepts silently, which is almost always a mistake): a
number prefix without digits (`#$` reads 0 in a68k), text glued after a label colon (`lab:nop`
drops the `nop` in a68k), lines over 64 KiB. M2's a68k comparison excludes these cases. M2 adds a
warning for a column-1 label spelled like a mnemonic.

### 5.5 Fixtures

`arch/m68k/fixtures/lex/<name>.asm` with `<name>.tokens`: one token per line,
`<line>:<start>..<end> <kind>[ <code>] <text as a JSON string>`, whitespace tokens included.
`VA_BLESS=1 cargo test -p va-arch-m68k --test syntax` rewrites the `.tokens` files; the diff is
reviewed in git. Six fixtures: `basic`, `labels`, `literals`, `comments` (with every line of the
§5.7 table, tabs included), `macros`, `errors`.

### 5.6 Performance budget and measurement

- **Input:** 100,000 lines generated in the test by cycling 20 fixed representative lines (labels,
  every addressing mode, directives, strings, macro parameters, comments, one error line), ~3.3 MB.
- **Measured:** `lines()` + `lex_line` over the whole text into one reused `Vec<Token>` (cleared per
  line), release build, 1 warm-up then 10 runs.
- **Budget:** median ≤ **50 ms** (≥ 65 MB/s) on the reference machine (Ryzen AI 7 350, on mains,
  "Best performance"). A miss blocks M1 acceptance (E7), never CI.
- **Command:** `make bench-lexer` (§8) runs
  `cargo test -p va-arch-m68k --release --test syntax_bench -- --ignored --nocapture`, which asserts
  the token count and prints one line `{"schema":"va.bench.lexer/1","lines":100000,"bytes":…,"tokens":…,"runs":10,"median_ms":…,"min_ms":…,"max_ms":…,"mb_per_s":…}`.
  The Performance agent writes `docs/perf/<date>-m1-lexer.md`.
- **CI guard against quadratic behaviour:** `robustness.rs` lexes 40 pathological 64 KiB lines
  (`'`, `\`, `$`, `(`, `;`, `a`, `.`, `é`, …) in debug mode and asserts the set takes < 30 s (linear:
  ~0.3 s; quadratic: minutes).
- **Decision: no criterion before M7.** *Rejected:* criterion now (architecture §7 said M1+); ~30
  crates for one benchmark that a 60-line ignored test covers. M7 decides for all benches.

### 5.7 Operand-like text after a space (user decision, 2026-10-17; `syntax/spacing.rs`)

**Semantics stay strict A68k:** the text after the operand field is a comment, for the lexer, the
M2 assembler and the bytes (`dc.b 1, 2, 3` produces one byte, as in a68k). Only its presentation
changes: text that looks like the continuation of the operands becomes a `SuspectComment` token,
greyed like a comment and underlined as an error; real comments stay plain `Comment`.

**Rule** (lexer-level, deterministic). It applies only when the operation takes operands (a
mnemonic with `takes_operands`, a directive with the operands policy, or a `MacroName`), the operand
field is followed by whitespace, and the remaining text T does not start with `;`. Let O be the
operand-field tokens and W the first word of T (up to unquoted whitespace or `;`).

| Rule | Fires when | `cause` | Fix |
|---|---|---|---|
| R1 dangling operand | the last O token is `,`, `#`, `(` or an `Operator`, or O opens more `(` than it closes | `after-comma` if that token is `,`, else `inside-expression` | yes |
| R2 leading joiner | T starts with `,`, or with an operator other than `-` and `*` (`+ / & ! \| << >>`) | `after-comma` for `,`, else `inside-expression` | yes |
| R3 operand list | W, lexed with the operand rules, has no `Invalid` token, contains a `,` outside parentheses, and starts with a `Register`, `Number`, `LocalLabel`, `String`, `#`, `(` or `-(` | `operand-list` | no |

When a rule fires, T up to its first unquoted `;` (trailing whitespace excluded) is one
`SuspectComment(lex-operands-in-comment)`; the `;` part stays a `Comment`. A comment after `;`, a
column-1 `*` line and the text after a no-operand keyword (`rts d0, d1`) never fire.

**False-positive policy.** False negatives are accepted and stay plain comments, exactly as a68k
(`dc.b 1 - 2`, `dc.b 1 * 2`, `dc.b 1 2`): `-` and `*` often start free text, so they are not
joiners. False positives are limited to text that dangles from an incomplete operand field or whose
first word is itself an operand list; writing the comment after `;` always silences the rule, and
the help text says so.

**Diagnostic.** `lex-operands-in-comment`, **severity Error, non-blocking** (`Diagnostic::non_blocking`):
it is drawn as an error underline and counted as an error (Problems panel, exit code 1), but it
never prevents the program image, so the output stays byte-identical to a68k. *Rejected:* a
blocking error, which refuses files a68k assembles; *rejected:* a warning, which is not the error
underline the user asked for. `M68kSyntax::line_diagnostics` overrides the default to add:
- args `text` (Str, the ignored text, truncated as in §3.4) and `cause` (Str, the id above);
- for R1/R2, a `Suggestion` (the quick-fix data): span = from the whitespace that separates O from
  T to the end of the token; replacement = the token text with every unquoted whitespace run
  removed when it touches `,`, `#`, `(`, `)` or an operator. `dc.b 1, 2, 3 bytes` becomes
  `dc.b 1,2,3 bytes`. M1 computes and outputs this data; applying it in the editor is M5's
  quick-fix. R3 gets no fix (joining `3,4` to `1,2` is a guess).

**Messages.** M1 (`diag-lex-operands-in-comment`: fixed value, `.help` selected on `$cause`):
EN "a68k will ignore “{ $text }”: the operands end at the first space." / help "Remove the spaces
after the commas." (`after-comma`), "Remove the spaces inside the expression."
(`inside-expression`), "Join the operands, or start the comment with “;”." (`operand-list`).
FR "a68k ignorera « { $text } » : les opérandes s’arrêtent au premier espace." / "Supprimez les
espaces après les virgules.", "Supprimez les espaces dans l’expression.", "Collez les opérandes, ou
commencez le commentaire par « ; »." **M2 refines the effect** with `asm-operands-in-comment` on the
same span and with the same fix (non-blocking Error; it supersedes the lexer diagnostic wherever
the assembly result is shown): args `text`, `kind` (`data` | `instruction`), `bytes` (Int, bytes
the line produces) and `kept` (Str, the operands a68k uses). EN "a68k will ignore “{ $text }”: only
{ $bytes -> [one] 1 byte *[other] { $bytes } bytes } will be produced." (`data`) or "…: only
“{ $kept }” is assembled." (`instruction`); FR "a68k ignorera « { $text } » : { $bytes -> [one] seul
{ $bytes } octet sera produit *[other] seuls { $bytes } octets seront produits }." or "… : seul
« { $kept } » est assemblé.". The FR/EN files of M2 carry these ids; M1 carries only the lexer one.

**Expected results** (`tests/syntax/fields.rs` and the `comments` fixture; a68k 2.71.F3w column
observed on 2026-10-17). Each line is a statement: the tests put it behind whitespace, since
column 1 is the label field.

| Line (tabs written `⇥`) | Token after the operands | a68k |
|---|---|---|
| `dc.b 1, 2, 3` | `SuspectComment "2, 3"`, after-comma, fix `1,2,3` | 1 byte `01`, no error |
| `move.l d0, d1` | `SuspectComment "d1"`, after-comma, fix `d0,d1` | "Error in operand format" |
| `⇥move.l⇥d0,⇥d1` | `SuspectComment "d1"`, after-comma | same error |
| `dc.b 1 ,2` | `SuspectComment ",2"`, after-comma, fix `1,2` | 1 byte, no error |
| `dc.b 1, 2 ; three` | `SuspectComment "2"`, then `Comment "; three"` | 1 byte, no error |
| `dc.b 'a, b', 2` | `SuspectComment "2"` (the string keeps its space and comma) | 4 bytes `612C2062`, no error |
| `dc.b 1 + 2` | `SuspectComment "+ 2"`, inside-expression, fix `1+2` | 1 byte `01`, no error |
| `move.l # 1,d0` | `SuspectComment "1,d0"`, inside-expression, fix `#1,d0` (`#` joins, §12) | errors ("Undefined Symbol", "Error in operand format") |
| `move.l #1 + 2,d0` | `SuspectComment "+ 2,d0"`, inside-expression, fix `#1+2,d0` | error |
| `lea 4( a0),a1` | `SuspectComment "a0),a1"`, inside-expression, fix `4(a0),a1` | errors |
| `movem.l d0-d7, -(sp)` | `SuspectComment "-(sp)"`, after-comma, fix `d0-d7,-(sp)` | error |
| `dc.b 1,2 3,4 swapped` | `SuspectComment "3,4 swapped"`, operand-list, no fix | 2 bytes `0102`, no error |
| `dc.b 'a b',2` | none (no trailing text) | `61206202` |
| `move.l d0,d1 ; real comment`, `move.l d0,d1 free text`, `move.l d0,d1 * comment`, `⇥move.l⇥d0,d1⇥free text` | `Comment` | `2200`, no error |
| `rts d0, d1`; `* dc.b 1, 2` in column 1; `dc.b 1 - 2` | `Comment` (no-operand keyword; comment line; accepted false negative) | `4E75`; nothing; 1 byte |

## 6. Instruction documentation and localization

### 6.1 Files and ids (WP-D)

`arch/m68k/locales/{en,fr}/isa.ftl` and `diagnostics.ftl`, embedded by `src/locales.rs` with
`include_str!` (allowed by m0 §17.4 S8) and returned by `M68K.locale_resources("en"|"fr")`.

| Id | Value | Attributes |
|---|---|---|
| `isa-m68k-<entry>` (74, e.g. `isa-m68k-add`, `isa-m68k-bcc`, `isa-m68k-dbcc`) | one-line summary | `.details` (required, 2–6 sentences); `.flags` (required iff a form has a `Special` flag) |
| `isa-m68k-cond-<name>` (16, `t`…`le`) | meaning of the condition | — |
| `isa-m68k-outcome-<id>`, `isa-m68k-unit-<id>` | short phrase | — |
| `diag-lex-<…>` (11) | message, may use `{ $text }` | `.help` optional; required for `diag-lex-operands-in-comment`, selected on `$cause` (§5.7) |

Style rules (tested by parsing): no `{` except placeables (write `{"{"}`); no continuation line
starting with `.`, `*` or `[`; plain ASCII apostrophes or typographic ones, consistently per file.
Example (our own words):

```ftl
isa-m68k-add = Adds the source to the destination.
    .details = The result replaces the destination. ADD also accepts an address register
        destination (ADDA) or an immediate source (ADDI); the assembler picks the encoding.
```

*Rejected:* Rust string tables for docs; translators and reviewers would edit code, and the M5
hover would need a second text pipeline next to Fluent.

### 6.2 Fluent in M1 (WP-E, `crates/va-workspace/src/i18n/`)

**Decision: `fluent-bundle` is added in M1** and `va-workspace` gets the localizer of architecture
§5. *Rejected:* only validating the `.ftl` files in M1 with an in-house parser; `va isa --lang fr`
must render real text, and M2 needs the same renderer for diagnostics.

```rust
pub enum Lang { En, Fr }   // Copy, Eq, Hash
impl Lang { pub const ALL: [Lang; 2]; pub const fn code(self) -> &'static str; pub fn parse(code: &str) -> Option<Self>; }
pub enum LangSource { Flag, VaLang, Environment, Default }
pub struct LangChoice { pub lang: Lang, pub source: LangSource, pub ignored_va_lang: Option<String> }
pub fn resolve_lang(flag: Option<Lang>, var: &dyn Fn(&str) -> Option<String>) -> LangChoice;
pub struct Localizer { /* private: concurrent FluentBundle for lang, one for en */ }   // Send + Sync
pub enum LocalizerError { Parse { lang: Lang, resource: usize, errors: Vec<String> }, Duplicate { lang: Lang, id: String } }
impl Localizer {
    pub fn new(lang: Lang, primary: &[&'static str], english: &[&'static str]) -> Result<Self, LocalizerError>;
    pub fn for_architecture(lang: Lang, arch: &dyn Architecture) -> Result<Self, LocalizerError>;
    pub fn lang(&self) -> Lang;
    pub fn message(&self, id: &str, attribute: Option<&str>, args: &[(&str, DiagArg)]) -> Option<String>;
    pub fn render(&self, diagnostic: &Diagnostic) -> RenderedDiagnostic;
}
pub struct RenderedDiagnostic { pub message: String, pub help: Option<String>, pub note: Option<String> }
```

- `resolve_lang`: `flag`, then `VA_LANG` (`en`, `fr` or `auto`; another value is ignored and
  returned in `ignored_va_lang`), then `auto`. **M1 `auto`** reads `LC_ALL`, `LC_MESSAGES`, `LANG`
  (first non-empty; `fr…` → Fr), else En. The Windows UI language and the `ui.language` setting
  arrive in M4 (decided then: `sys-locale` or `windows-sys` under Security review).
- `message` looks up the requested language, then English, and returns `None` if both miss.
  `render` uses `diag-<code>`; if it is missing everywhere, `message` is the code itself (never a
  panic). Bundles use `fluent_bundle::concurrent::FluentBundle` (so `Localizer` is `Send + Sync`
  for M5 workers) with `set_use_isolating(false)`: no U+2068/U+2069 marks in terminals or egui.

### 6.3 Completeness tests (`arch/m68k/tests/locales.rs`, WP-D)

`fluent-bundle` is a dev-dependency of `va-arch-m68k` (external, so rule R1 holds). The test:
1. parses every resource of both languages with `FluentResource::try_new`: no error;
2. lists ids by scanning column-0 `identifier =` lines;
3. collects every id the CPU can reference: for each spelling, `describe(spelling, None)` gives the
   doc, condition, outcome and unit ids; `diagnostic_codes()` gives `diag-<code>`;
4. asserts each id exists in `en` and `fr` with the required attributes, that `en` and `fr` have the
   same id and attribute sets, and that no id is unreferenced;
5. formats every message in both languages with sample arguments and asserts no resolver error.

## 7. CLI (WP-F, `apps/va`)

### 7.1 Grammar

```
va [--json] [--lang en|fr] info | lex <FILE|-> | isa <MNEMONIC[.SIZE]> | isa --list
```

- `isa` takes exactly one of `MNEMONIC` and `--list` (clap: both or neither is a usage error, 2).
- `--lang` becomes a global flag **in M1** (m0 §5 said M2), because `va isa` renders docs.
  `VA_LANG` and `auto` follow §6.2. An ignored `VA_LANG` prints `warning: ignoring VA_LANG=<value>
  (expected en, fr or auto)` on stderr; the `var` closure reads `var_os` lossily, so a non-Unicode
  value is reported too. With `--lang`, no variable is read and nothing is reported.
- `src/arch_registry.rs`: `pub fn architectures() -> Vec<&'static dyn Architecture>` and
  `default_architecture()` (the first one). Feature `arch-m68k` (default) adds `&va_arch_m68k::M68K`.
  No `--arch` flag until a second CPU exists. *Rejected:* `--arch` now (one CPU; M4's
  `project.toml` will choose).
- `va info` now reports `architectures: m68k` / `["m68k"]`; the M0 tests are updated, the
  `va.info/1` schema is unchanged.

### 7.2 Input (`src/input.rs`)

The file (or stdin for `-`) is read through `Read::take(MAX_SOURCE_BYTES + 1)`, then
`SourceText::from_bytes`. Errors: unreadable path or directory → `read-failed`; more than 16 MiB →
`input-too-large`; invalid UTF-8 → `input-not-utf8` with the byte offset. All exit 3.
*Rejected:* lossy decoding of Windows-1252 files; it silently changes bytes and offsets. Encoding
conversion is an M4 editor feature ("reopen as Windows-1252").

### 7.3 `va lex`

- Splits with `lines`, lexes each line, collects `line_diagnostics`, renders them with the
  `Localizer`. Exit 0 without error diagnostic, 1 with at least one, blocking or not (so
  `dc.b 1, 2, 3` gives exit 1).
- **Text:** stdout, one line per non-whitespace token: `<line>:<column> <kind> <text as JSON string>`,
  plus ` <code>` for `invalid` and `suspect-comment` tokens. Diagnostics on stderr, GNU style
  (clickable): `<file>:<line>:<column>: error[<code>]: <message>`, then `  help: <help>` and
  `  fix: <the line with the fix applied>` when present. The `error: ` prefix rule of the CLI
  applies to the CLI's own failures, not to diagnostics.
- **JSON `va.lex/1`** (one line, keys in this order; written incrementally, never held whole):

```json
{"schema":"va.lex/1","arch":"m68k","lang":"en","file":"f.asm",
 "lines":[{"line":1,"offset":0,"tokens":[
   {"kind":"label","start":0,"end":5,"column":1,"text":"start"},
   {"kind":"invalid","start":17,"end":18,"column":18,"text":"$","code":"lex-bad-number"}]}],
 "diagnostics":[{"code":"lex-bad-number","severity":"error","blocking":true,"line":1,"column":18,"start":17,"end":18,
   "message":"…","help":null,"note":null,"fix":null}],
 "summary":{"lines":1,"tokens":2,"errors":1}}
```

  Token `start`/`end` are line-relative bytes; `offset` and diagnostic `start`/`end` are file bytes;
  `line` and `column` are 1-based, columns in chars. `file` is the argument as given (lossy UTF-8),
  `-` for stdin. Every line appears, empty ones with `"tokens":[]`. A §5.7 token is
  `{"kind":"suspect-comment",…,"code":"lex-operands-in-comment"}`; its diagnostic has
  `"blocking":false` and `"fix":{"start":…,"end":…,"replacement":"2,3"}` (file bytes; `null` for
  R3). `summary.errors` counts every error, blocking or not. These keys belong to `va.lex/1` from
  its first release (nothing has shipped), so no version bump.

### 7.4 `va isa`

- The argument splits at its first `.` into spelling and size; `describe` does the rest.
- **Text:** first line `<SPELLING>: <summary>`, then `.details`; then, when present, lines
  `condition: <NAME> (<test>) <condition text>`, `aliases: …`, `also accepts: …`, `flags: <.flags>`;
  then one block per form: `<syntax>` with `  default .<size>` and `  privileged` when relevant,
  `  operand <i>: <modes>`, `  flags: X<s> N<s> Z<s> V<s> C<s>` with `s` ∈ `- * 0 1 U !`
  (unaffected, result, cleared, set, undefined, special), and one `  cycles` line per row, e.g.
  `  cycles .L <ea>=Dn,An,#imm: 8(1/0) + ea`. Only the first line and the flags lines are tested.
- **JSON `va.isa/1`** (one line, keys in this order):

```json
{"schema":"va.isa/1","arch":"m68k","lang":"fr","query":"beq.s","spelling":"BEQ","canonical":"BEQ",
 "family":"Bcc","condition":{"name":"EQ","test":"Z","summary":"…"},"aliases":[],"also_accepts":[],
 "size":"S","summary":"…","details":"…","flags_note":null,
 "forms":[{"syntax":"BEQ.S <label>","sizes":["S"],"default_size":null,"privileged":false,
   "operands":[{"notation":"<label>","modes":[]}],
   "flags":[{"flag":"X","effect":"unaffected"},{"flag":"N","effect":"unaffected"},…],
   "timing":[{"sizes":["S"],"when":[],"outcome":"taken","clocks":10,"reads":2,"writes":0,"plus_ea":[],"per_unit":null,"bound":"exact"}]}],
 "ea_timing":[]}
```

  Without a size, `BEQ` lists two forms, `BEQ.S <label>` (`["S"]`) and `BEQ.W <label>` (`["W"]`).
  `outcome` is `null` for rows that do not depend on an outcome. `summary`, `details` and
  `flags_note` are the raw Fluent values: `details` may contain newlines (a single newline is a
  soft wrap, a blank line a paragraph break); text mode prints it as is, M5 hover reflows it.
  `per_unit` is `{"clocks":2,"reads":0,"writes":0,"unit":"shift-count"}` when present. `ea_timing`
  lists `{"mode","sizes","clocks","reads","writes"}` entries and is non-empty iff a row has
  `plus_ea`.
- **`--list`, JSON `va.isa.list/1`:** `{"schema":"va.isa.list/1","arch":"m68k","lang":"en","mnemonics":[{"spelling":"ABCD","canonical":"ABCD","summary":"…"}, …]}`
  with the 126 spellings in order; text mode prints `<SPELLING>  <summary>` per line.
- Unknown spelling: exit 1 with `error: unknown mnemonic 'MOV' (did you mean MOVE, MOVEA, MOVEQ?)`:
  up to 3 spellings within edit distance 2. Size not accepted: exit 1.

### 7.5 Errors and exit codes (`src/report.rs`)

M1 implements the `va.error/1` printer of m0 §5: in JSON mode a failure prints
`{"schema":"va.error/1","code":"<code>","message":"<english>"}` on stdout and the human line on
stderr; a stdout failure prints no document (unchanged M0 rule). CLI messages stay English, like
`va info` (*rejected:* localizing CLI chrome now; it would create the root `locales/` folder for ten
strings before any GUI exists; M4 does it).

| Exit | Codes |
|---|---|
| 0 | success (`lex` without error diagnostic) |
| 1 | `lex` found an error diagnostic, blocking or not (result document, not `va.error/1`); `isa-unknown-mnemonic`, `isa-size-not-accepted` |
| 2 | clap usage errors (unchanged) |
| 3 | `read-failed`, `input-too-large`, `input-not-utf8`, `no-architecture` (built without `arch-*`), `locale-invalid` (embedded resource fails to load: a build defect), stdout failure |
| 101 | panic (a bug) |

## 8. Manifests, Makefile and dependencies (WP0)

Root `[workspace.dependencies]` additions (normative values, sorted with the existing keys):

```toml
fluent-bundle = "=0.16.0"
unic-langid = "=0.9.6"
va-arch-m68k = { path = "arch/m68k" }
va-core = { path = "crates/va-core" }
va-workspace = { path = "crates/va-workspace" }
```

| Member | `[dependencies]` | Other |
|---|---|---|
| `va-core` | none | — |
| `va-arch-m68k` | `va-core` | `[dev-dependencies] fluent-bundle` |
| `va-workspace` | `va-core`, `fluent-bundle`, `unic-langid` | — |
| `va` | `clap`, `serde`, `serde_json`, `va-core`, `va-workspace`, `va-arch-m68k = { workspace = true, optional = true }` | `[features] default = ["arch-m68k"]`, `arch-m68k = ["dep:va-arch-m68k"]` |

All entries use `{ workspace = true }` (R6). The layering table needs no change: every edge is
already allowed (R1–R8). `Cargo.lock` is committed with the manifests (`make security` is `--locked`).
The normative-content header comments of `Cargo.toml` and `Makefile` gain "amended by
m1-core-isa-lexer.md §8".

| Crate | Version | Licence | Purpose | Why std is not enough |
|---|---|---|---|---|
| `fluent-bundle` | 0.16.0 (latest; MSRV 1.67) | Apache-2.0 OR MIT | Fluent message formatting (architecture §5) | Plurals, selectors and typed arguments of the Fluent syntax |
| `unic-langid` | 0.9.6 (latest), default features | MIT OR Apache-2.0 | `LanguageIdentifier` required by `FluentBundle::new` | Fluent's API takes this type |

New transitive packages (resolved on 2026-10-17): fluent-langneg, fluent-syntax, intl-memoizer,
intl_pluralrules, type-map, unic-langid-impl, tinystr (Unicode-3.0), displaydoc, self_cell
(Apache-2.0 OR GPL-2.0-only, Apache chosen), smallvec, rustc-hash, memchr (Unlicense OR MIT),
thiserror 2, and zerofrom 0.1.8 and zerovec 0.11.8 (Unicode-3.0, through tinystr's default
features; ratified in §12). `cargo deny check licenses bans` with `.cargo/deny.toml` passed on that closure; no
licence or ban change is needed. *Rejected:* `thiserror` as a direct dependency (std `Error` impls
suffice for the few M1 error types); `criterion` (§5.6).

**Makefile addition** (normative; same style as m0 §7): add `bench-lexer` to `.PHONY`, a help line
`$(info   bench-lexer  measure lexer throughput on 100000 generated lines in release)` after the
`bench` line, and the target

```make
bench-lexer:
	$(CARGO) test -p va-arch-m68k --release --test syntax_bench -- --ignored --nocapture
```

## 9. Tests and acceptance

### 9.1 Tests per crate (each file starts with `//!` and `#![allow(clippy::unwrap_used)]`)

| Crate / file | Covers |
|---|---|
| `va-core/tests/source.rs` | `lines`/`LineIndex` on LF, CRLF, CR, mixed, empty, trailing terminator; `position`/`offset` round trip with multi-byte chars and tabs; BOM strip; 16 MiB limit (exactly at and one over); `NotUtf8` offset |
| `va-core/tests/diag.rs`, `syntax.rs` | `DiagCode::is_valid`; builder; `check_line_tokens` accepts a valid cover and reports each `TokenInvariantError`; `token_diagnostics` offsets, truncation and blocking flags (`Invalid` blocking, `SuspectComment` not); `Diagnostic::new`/`non_blocking`; `TokenKind::name` unique kebab ids |
| `va-core/tests/isa.rs`, `regs.rs`, `image.rs` | ids of `FlagEffect`/`Bound`; overlap and overflow rejection; `byte_at`; `SourceMap` lookups at segment edges and `addresses(file, line)` order |
| `arch/m68k/tests/isa/` | §4.7 |
| `arch/m68k/tests/syntax/kinds.rs` | Table of (line, expected kinds): every `TokenKind` and every `lex-*` code appears; the union over the table and fixtures is asserted complete |
| `…/syntax/fields.rs` | The §5.1/§5.2 rules: column-1 label, indented `name:`, `nop` in column 1, comment after operands, every row of the §5.7 table (token, `cause`, fix text), no-operand keywords, INCLUDE paths, `=`, size suffixes, register suffixes, `1$`, `\name`, `\@`, `\0`, every number base, doubled quotes |
| `…/syntax/coverage.rs` | Properties via `check_line_tokens`: all ASCII strings of length ≤ 3 over 24 interesting chars; 100,000 deterministic pseudo-random lines (xorshift seed in the test; printable ASCII, tabs, multi-byte chars); statelessness (lexing lines in shuffled order gives the same tokens); always `LineState::INITIAL` |
| `…/syntax/robustness.rs` | 64 KiB pathological lines (§5.6 guard); 64 KiB + 1 → one `lex-line-too-long`; `\r`, `\n`, NUL inside a line; 4-byte UTF-8 chars in comments and strings; every keyword in mixed case |
| `…/syntax/fixtures.rs` | §5.5 golden files, `VA_BLESS=1` |
| `arch/m68k/tests/arch.rs` | `M68K.id() == "m68k"`; every keyword from `keywords()` lexes as its kind; every spelling lexes as `Mnemonic` (and with each accepted suffix); `diagnostic_codes() == DIAG_CODES`; register specs and SR fields |
| `arch/m68k/tests/locales.rs` | §6.3 |
| `arch/m68k/tests/syntax_bench.rs` | §5.6 (ignored) |
| `va-workspace/tests/i18n_lang.rs` | `resolve_lang` precedence and the `VA_LANG`/`LC_*`/`LANG` cases via a fake `var` |
| `va-workspace/tests/i18n_localizer.rs` | Fallback to English; missing id → `None`/code; arguments; no isolation marks; parse and duplicate errors on synthetic resources |
| `apps/va/tests/lex.rs` | Exit 0/1/3; JSON schema and key order on a fixture; stdin `-`; CRLF file gives the same tokens as LF; BOM; non-UTF-8 → 3 with `input-not-utf8` in JSON mode; 16 MiB + 1 → 3; directory → 3; `--lang fr` changes only the messages; `dc.b 1, 2, 3` → exit 1 with `suspect-comment`, `"blocking":false` and the `fix` object, while `; comment` lines → 0; closed stdout pipe → 3 |
| `apps/va/tests/isa.rs` | `MOVE`, `move.l`, `beq.s`, `DBRA` (canonical `DBF`), `ADD` (`also_accepts`), `--list` (126 entries), unknown → 1 with suggestions and `va.error/1` in JSON mode, `moveq.b` → 1, `--lang fr` summary differs from `en`, invalid `VA_LANG` warning |
| `apps/va/tests/binary.rs` | Updated `info` expectations (`m68k`) |

Temp files follow the M0 rule (`CARGO_TARGET_TMPDIR`, `fs::create_dir`).

### 9.2 Acceptance commands (QA, Windows and WSL)

```bash
make test && make lint && make docs && make security && make ci
make build
target/release/va info --json                                   # "architectures":["m68k"]
target/release/va lex arch/m68k/fixtures/lex/basic.asm          # exit 0
target/release/va --json lex arch/m68k/fixtures/lex/errors.asm  # exit 1, va.lex/1, every lex-* code
target/release/va isa MOVE --json                               # exit 0, va.isa/1
target/release/va --lang fr isa move.l                          # exit 0, French summary
target/release/va isa --list --json                             # 126 spellings
target/release/va isa MOV                                       # exit 1, suggestions
make bench-lexer                                                # va.bench.lexer/1 line (Performance)
```

## 10. Fragile points

| # | Risk | Mitigation |
|---|---|---|
| F1 | A68k ends operands at the first space; students write `d0, d1` and `dc.b 1, 2, 3` | Strict A68k semantics plus the non-blocking §5.7 error with its fix; M2 refines the effect |
| F13 | §5.7 heuristic flags a real comment | Rules limited to dangling operands, leading joiners and operand-list first words; `;` always silences it; the §5.7 table is the regression set |
| F2 | Timing footnotes are easy to mistype | Rows per footnote case, parity/bus-count sanity check, 30 spot checks, M3 cross-check with the `m68k` crate |
| F3 | Overlapping encodings (ILLEGAL vs TAS #, DBcc vs Scc An, MOVEP vs BTST An) and 68010+ leaks | 65,536-opword uniqueness test, negative golden opwords |
| F4 | Fluent syntax traps in prose (`{`, lines starting with `.`/`*`/`[`) and bidi isolation marks | Parse test, `set_use_isolating(false)` test |
| F5 | `.gitattributes` turns CRLF fixtures into LF | Byte-level inputs built in code (§2.1) |
| F6 | Byte vs char columns (multi-byte chars in comments) | `LineIndex` tests; JSON exposes both |
| F7 | ropey line counting differs from `lines` | M4 must enable only `cr_lines` (§3.2); M4 test compares both |
| F8 | Huge `va lex --json` output (~20× the input) | Incremental writer; 16 MiB input limit |
| F9 | Windows console and French accents | Rust std writes UTF-16 to consoles and UTF-8 to pipes; `tests/isa.rs` checks UTF-8 bytes on a pipe |
| F10 | Copied manual text in docs | Own-words rule, QA spot check (E9), sources table (§4.6) |
| F11 | `cargo xtask` has no `--locked` and can rewrite `Cargo.lock` after the manifest change | Commit the lock with WP0; `make security` checks it |
| F12 | Keyword lookup allocating per line, hurting the budget | Allocation-free rule (§5.2), budget measured (§5.6) |

## 11. Deferred items and open questions

- **Permissive dialect profile (user decision, 2026-10-17; not M1).** An EASy68K/vasm-style dialect,
  where spaces may follow commas inside operands, will be a dialect profile chosen per project like
  the I/O profile (`.visualasm/project.toml`, M4+), implemented as a second `SyntaxProvider` of
  `va-arch-m68k` (architecture §10.3). M1 has one dialect, A68k; §5.7 is its only presentation
  layer over the strict rule.
- Open questions: none blocking. The d68k/TP I/O profile still needs the user's lab handouts before
  M3 (plan §6).

## 12. Ratified implementation decisions (waves 1–2, 2026-10-17)

Same precedence as the body it amends; where §12 and an earlier passage differ, §12 wins. The
passages WP-F reads (§3.5, §4.1, §5.2, §5.7, §7.1, §7.4, §8) were also corrected in place.
Source: the wave-2 notes per WP, checked against the code at `66fd916` and a68k where stated.

### 12.1 Fix-now (small, local; before WP-F closes)

| # | Owner | Files | Exact change |
|---|---|---|---|
| R12-1 | WP-C | `arch/m68k/src/syntax/spacing.rs`, `arch/m68k/tests/syntax/fields.rs` | Add `b'#'` to `joins()`. Test row ` move.l # 1,d0` → `SuspectComment "1,d0"`, cause `inside-expression`, replacement `1,d0` → line `move.l #1,d0` (today the fix is a no-op). |
| R12-2 | WP-C | `arch/m68k/src/syntax/lexer.rs`, `tests/syntax/fields.rs`, `fixtures/lex/labels.{asm,tokens}` | A run of `:` after a label (column 1 or indented) is one `Punctuation` token: `lab::`, `lab:::`, `  other:: nop` no longer give `lex-label-junk`; `lab:nop` still does. Reason (strict A68k): a68k 2.71.F3w assembles `lab::` and `lab:::` to an object byte-identical to `lab:` (run 2026-10-17). Re-bless `labels.tokens`. |
| R12-3 | WP-D | `arch/m68k/locales/{en,fr}/isa.ftl` | Remove from `.details` every clock-period count that the timing tables give (Bcc 10/8, BRA 10, BSR 18, DBcc 10, EXG 6, LINK 16, MOVEM per register, MULS/MULU 38 + 2n and 70, NOP 4, RTE 20, "2 more clock periods per bit position", …); keep qualitative wording. Timing facts absent from the tables (RESET holds its output for 124 clock periods) may stay. Reason: one source of truth, and the table values are provisional (§12.3). |

### 12.2 Ratified as implemented

| WP | Decisions (the spec now says what the code does) |
|---|---|
| Wave 1 (WP0, WP-A) | zerofrom 0.1.8 and zerovec 0.11.8 (Unicode-3.0, already allowed) through tinystr (§8). The 16 MiB limit counts input bytes including a BOM. `lines("")` yields one empty line. `position()` inside a terminator gives the column after the content, inside a multi-byte char that char's column. `check_line_tokens` reports Gap/Overlap, then Empty, Long, NotCharBoundary, Short. `ImageError::Overlap` holds input positions, smaller first; overflow is checked before overlap; empty segments never overlap; a segment may end exactly at 2^64. `SourceMap::at_address` skips zero-length entries, `addresses()` includes them. Additions: `Default` for `SourceMap`, `#[must_use]` on `with_arg`/`non_blocking`. The fake `Architecture` lives in `va-core/tests/regs.rs` (no `tests/arch.rs`). |
| WP-B | Bcc/BRA/BSR have separate `.S` (non-zero disp8) and `.W` (zero low byte) forms. Byte forms are split from forms that admit An (a68k "Operand size error"); ADDQ/SUBQ gain `#<1-8>,An` (W/L, flags unaffected). MOVE to CCR/SR, RTE, RTR, STOP flags are `Result`; `Special` is used exactly as §4.3 lists (pinned by a test). `Form::matches(op, Some(c))` places `c` in bits 11–8; rows with `ModeSet::NONE` are mode-independent; forms without outcomes declare `[Outcome::Always]` (view: `outcome` null); `cycles()` ignores the size of unsized forms; DIVU/DIVS have no divide-by-zero outcome (the exception is M3's); the view renders MOVE rows per destination group (a test proves equality with the matrices); `ea_timing` has 24 entries (12 modes × 2 size classes); `EaField::MoveDest` stays frozen but unused (MOVE uses `Both`); `takes_operands` is the §5.2 no-operand list (tested); extra public helpers are allowed. |
| WP-C | §5.7 table lines are statements behind whitespace. `\r`/`\n` inside a line: invalid run, rest lexed as a new line. A bad backslash before whitespace, `;` or the end is the backslash alone. Any other operation field is one `Invalid(lex-bad-operation)`; operand rules then apply, §5.7 does not. `b\1` → `MacroName` + `MacroParam`. INCLUDE/INCBIN paths: `String`, no §5.7. `1.5`, `$12.x` → `lex-bad-number`; `$1000.w` → `Number` + `SizeSuffix`. `errors.asm` holds one 65,542-byte line (`lex-line-too-long`; `errors.tokens` ~67 KB). |
| WP-D | `.flags` exists iff a form has a `Special` flag (MOVE to CCR/SR and STOP explain in `.details`). Outcome ids: taken, not-taken, true, false, not-expired, expired, trap, no-trap (no `always`); 4 unit ids. `locale_resources` matches exactly `"en"`/`"fr"`. `.details` may span lines (§7.4). French uses regular spaces before `:` `;` `»` and after `«` (*rejected:* U+202F, rendered unevenly in terminals). The §5.7 message is a fixed value with `.help` selected on `$cause`. |
| WP-E | An empty or differently-cased `VA_LANG` is "another value" (ignored, reported); `Lang::parse` accepts exactly `en`/`fr`. With `--lang`, no variable is read. Under `auto`, the source is `Environment` when the first non-empty locale variable starts with `fr`/`en`, else `Default`. English fallback works per pattern (a FR message without `.help` gets the EN `.help`). Resolver errors give best-effort text, never a fallback. `DiagArg::Int` beyond ±2^53 is passed as text (default plural). `LocalizerError` has an English `Display` and `std::error::Error` (build defects only), `Parse.errors` as `line <n>: <msg>`, first error wins. English is loaded once when the language is En. `render()` ignores `Related` in M1 (no M1 producer emits one; callers use `message(&related.code.fluent_id(), None, &related.args)`). |

### 12.3 Cycle and opword facts: verification status

The timing values and `VALID_OPWORDS = 45_816` were typed without the manual at hand. They stay as
they are and are **provisional**, in particular: TRAPV trap 34(5/3), CHK trap 40(4/3)+ea, TAS memory
10(1/1)+ea, ANDI.L #,Dn 14(3/0), ANDI.L #,memory 20(3/2) (some printings show 20(3/1)), RESET
132(1/0), the MOVEM per-mode bases. Verification: (a) QA checks this list against the manual only
if the user supplies a copy (no web fetch); (b) **M3 makes it binding**: `cycles()` and the
valid-opword set are cross-checked against the `m68k` crate and the SingleStepTests 680x0 vectors
(on-demand test data in gitignored `xtask/cache/`), every disagreement is settled from the manual or
the vectors, and the pinned count is updated with a recorded reason. Until then
`arch/m68k/src/isa/README.md` (Documentarian) says the values are provisional, citing this section.
*Rejected:* a per-row "verified" flag in the tables and in `va.isa/1`, which would churn the schema
for a state that ends in M3.

### 12.4 Deferred

| Item | When |
|---|---|
| a68k leniencies seen in wave 2: `chk.l d0,d1` assembles as CHK.W (`4380`); `$1000.w` is rejected; `d0.w` outside an index is read as a symbol. The M2 a68k comparison decides; `sizes`/`describe` may then accept such suffixes. Until then `va isa chk.l` exits 1. | M2 |
| `RenderedDiagnostic` with rendered `related` entries | M2 (first producer of `Related`) |
| Reflow of multi-line `.details` in hover | M5 |
| Allocation-free lexer: code review only in M1 (a counting allocator needs `unsafe`, hence an allow-list entry) | M7 decides |
| Binding timing and opword verification | M3 (§12.3) |

### 12.5 QA-check list

1. §12.3 values against the manual, if the user supplies one; otherwise record "deferred to M3".
2. `VALID_OPWORDS = 45_816`: plausibility (per-instruction counts printed by the test, negative
   golden opwords of §4.7); the binding check is M3.
3. WP-D facts: CLR reads before it writes; BTST with a register destination accepts an immediate
   bit number; MOVEP byte order; NBCD ten's/nine's complement wording; STOP wake-up conditions;
   RESET 124 clock periods on the line versus 132 for the instruction; CHK N undefined when no
   exception occurs; MOVEM never stores to PC-relative modes; DIVS/DIVU N and Z undefined after
   overflow; ASR "V is cleared" against the table's `Result`.
4. French terminology of both `fr/*.ftl`, and the E9 legal spot check (10 entries, own words).
5. E7: the formal lexer bench on mains, "Best performance" (wave-2 developer run: median 7.663 ms
   for 100,000 lines on "Balanced"; budget 50 ms).
6. Lexer hot path reviewed for allocations (§3.4 contract).
7. After R12-1 to R12-3: `make test`, `make lint`, re-blessed `labels.tokens` reviewed in the diff.

### 12.6 For WP-F

§7 keeps its content; only these points change or are made explicit:
- §7.1: the `var` closure uses `var_os` lossily (non-Unicode `VA_LANG` reported); no variable is
  read with `--lang`.
- §7.4: `beq.s` gives one form `BEQ.S <label>` with `["S"]`; without a size there are two forms;
  `outcome` is `null` for outcome-independent rows; `details` is the raw multi-line value.
- API (WP-E): `resolve_lang(flag, &var)` → `LangChoice` (`ignored_va_lang` drives the warning,
  `Lang::code()` the JSON `lang`); `Localizer::for_architecture(lang, arch)` built once, its error →
  exit 3, code `locale-invalid`, message = its English `Display`; `render(&diag)` gives
  `message`/`help`/`note` (JSON `null` when absent, text `  help:` line); docs via
  `message(id, None | Some("details") | Some("flags"), &[])`.
- `fix` is present only for R1/R2 diagnostics (§5.7); after R12-1 a dangling `#` has a real fix.
