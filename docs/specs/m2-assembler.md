# M2 — Assembler, disassembler, `va asm` / `va disasm`: file-level specification

| | |
|---|---|
| Status | Ready for development. User decisions Q1–Q2 of 2026-10-24 are recorded in §13.3; the architecture amendments of §3.1 are ratified in `architecture.md` (2026-10-24); the merged wave order with M4 is §14 |
| Owner | Architect role |
| Date | 2026-10-24 |
| Depends on | `architecture.md` (R1–R8, §3, §4 T3, §5, §6 atomic writes, §8, §9.5); `m1-core-isa-lexer.md` (§3 frozen `va-core`, §3.7 M2 signatures as amended by §3.1 below, §4 ISA tables, §5 lexer and §5.7, §6.2 Localizer, §7 CLI, §12) |
| Parallel milestone | M4 (`docs/specs/m4-editor-shell.md`). M2 never edits `crates/va-editor`, `crates/va-workspace`, `crates/va-ui`, `apps/visual-assembler`, `xtask`, `Cargo.toml` or `Cargo.lock`; the only shared files are the Makefile and the closing documents (§14). |
| Milestone goal | A strict-A68k 68000 assembler (byte-identical to a68k 2.71.F3w `-s` where a68k succeeds), its disassembler on the same tables, S-record/binary/listing/source-map outputs, localized diagnostics with recovery, `va asm` and `va disasm` |

Normative words as in m1: **must**, *rejected*. Blocks marked *frozen* are binding signatures
(implementations may add private items, never remove or rename a public one). "a68k" means
`a68k.exe` 2.71.F3w as observed in §12.

## 0. Exit criteria (all required)

| # | Criterion | How it is checked |
|---|---|---|
| E1 | `make test`, `make lint`, `make docs`, `make security` exit 0 on Windows and in WSL Ubuntu 26.04 | QA, both hosts |
| E2 | CI green on the 4 runners (`cargo xtask ci`, unchanged step list) | `gh run list --workflow ci.yml` |
| E3 | Golden fixtures of §10.2 match; every `asm-*` code is produced by at least one fixture or test, with its documented severity and blocking flag | `cargo test -p va-arch-m68k --test asm` |
| E4 | Disassembler golden files match; all 65,536 opwords decode without panic, exactly the opwords some form matches are valid; the exhaustive and fixture round trips of §10.3 hold | `cargo test -p va-arch-m68k --test disasm --test roundtrip` |
| E5 | Every new diagnostic id and every directive doc id exists in `en` and `fr` with identical id/attribute sets (m1 §6.3 extended) | `cargo test -p va-arch-m68k --test locales` |
| E6 | The acceptance commands of §10.6 give the documented exit codes, files and schemas | QA |
| E7 | Local a68k comparison: 0 mismatches over the compared fixtures and the generated corpus, with and without optimizations (§10.4); report with the a68k version and date | QA, `VA_A68K_PATH=… make compare-a68k` on Windows |
| E8 | Assembler throughput within §11 on the reference machine, with a `docs/perf/` report | Performance agent |
| E9 | Security review: include path policy and limits of §6 hold, no new `unsafe`/network/allow-list entry, M1-S5 re-rated, `SECURITY.md` updated | Security agent |
| E10 | Legal: EN/FR messages and directive docs in our own words; the listing format is ours (no a68k text, §4.12) | QA spot check of 10 entries |

## 1. Work packages and order

Interfaces are frozen here (§3, §4.2), so wave 2 runs in parallel on disjoint files. Developers
write no README; each WP hands the Documentarian its new files with a one-line purpose.

| WP | Wave | Write scope | Needs |
|---|---|---|---|
| WP-A core | 1 (∥ M4 WP0) | `crates/va-core/src/{lib,arch,asm,disasm}.rs`, `crates/va-core/tests/{asm,disasm}.rs` (§3) | — |
| WP0 scaffold | 1b (after WP-A and M4 WP0, §14) | `arch/m68k/src/{lib,arch}.rs` (module declarations, `assembler()`/`disassembler()`, `diagnostic_codes()`), `arch/m68k/src/isa/mod.rs` (the `pub mod fields;` line only), compiling stubs with the §4.2 signatures for every new file of §2 under `arch/m68k/src/{asm,disasm}/` and `isa/fields.rs`; `arch/m68k/tests/asm/main.rs` and `arch/m68k/tests/disasm/main.rs` with every `mod` line of §10.1 plus a `//!`-only file per module (wave 2 then owns each module file, never `main.rs`); `apps/va/src/{cli.rs, commands/mod.rs}` (subcommand shells), Makefile (§9) | WP-A; M4 WP0 (Makefile) |
| WP-B front end | 2 | `arch/m68k/src/asm/{parser,operand,expr}.rs`, `arch/m68k/src/syntax/lexer.rs` (§4.3 lexer row only), `arch/m68k/tests/asm/{parser,expr}.rs`, `arch/m68k/tests/syntax/fields.rs` (new rows), `arch/m68k/fixtures/lex/macros.{asm,tokens}` (re-bless) | WP0 |
| WP-C encoder | 2 | `arch/m68k/src/isa/{fields,mod}.rs` (**day-1 deliverable**, §4.9), `arch/m68k/src/asm/{encode,optimize}.rs`, `arch/m68k/tests/isa/{fields,main}.rs`, `arch/m68k/tests/asm/{encode,optimize}.rs`, `arch/m68k/tests/asm/corpus.rs` (generated forms corpus, shared with §10.4) | WP0 |
| WP-D driver | 2 | `arch/m68k/src/asm/{mod,codes,symbols,directives,macros,layout,listing,srec}.rs`, `arch/m68k/tests/asm/{directives,macros,relax,output,listing,fixtures,diagnostics}.rs`, `arch/m68k/fixtures/{asm,asm-opt,asm-inc,asm-diag}/**` | B and C APIs (frozen); integrates as they land |
| WP-E disassembler | 2 | `arch/m68k/src/disasm/{mod,decode,format}.rs`, `arch/m68k/tests/disasm/{decode,format,flow}.rs`, `arch/m68k/tests/roundtrip.rs`, `arch/m68k/fixtures/disasm/**` | `isa/fields.rs` (C, day 1); `roundtrip.rs` finishes after D |
| WP-F texts | 2 | `arch/m68k/locales/{en,fr}/{diagnostics,directives}.ftl`, `arch/m68k/src/locales.rs`, `arch/m68k/src/syntax/keywords.rs` (directive `doc_id`s only), `arch/m68k/tests/{arch,locales}.rs` (EN and FR writing) | code list of §7 |
| WP-G CLI | 2 (finishes after D, E) | `apps/va/src/**` (new `commands/{asm,disasm}.rs`, `sources.rs`; edits to `cli.rs`, `report.rs`, `commands/{mod,lex}.rs`), `apps/va/tests/{asm,disasm}.rs`, `apps/va/tests/lex.rs` (cap rows) | WP0 shells; D, E APIs |
| WP-H compare + bench | 2 (after D) | `arch/m68k/tests/{a68k,asm_bench}.rs` | D, C corpus |
| — | 3 | Security, QA | `SECURITY.md`; QA reports |
| — | 4 | Performance | `docs/perf/<date>-m2-assembler.md` |
| WP-I docs | 5 | Every README (new folders included), `CLAUDE.md`, `AGENCEMENT.md`, `DEPENDANCE.md`/`IMPORT.md` (no new crate: unchanged), `apps/va/README.md` (path behaviour, §6) | all |

Parallelism: WP-A runs beside M4's WP0; WP0 follows both (§14). In wave 2, B, C, E, F and G
start at once; D starts with stubs of B/C behind the frozen signatures; H starts when D assembles
`basic.asm`. `tests/asm/main.rs` and `tests/disasm/main.rs` stay as WP0 wrote them, so B, C, D and
E never edit the same file; `isa/mod.rs` passes from WP0 (1b) to C (2) in sequence.

## 2. File tree (new or changed)

```
Makefile                                                            # §9 (bench-asm, compare-a68k)
crates/va-core/src/{asm,disasm}.rs; lib.rs, arch.rs (edits)         # §3
crates/va-core/tests/{asm,disasm}.rs
arch/m68k/src/{lib,arch,locales}.rs (edits)
arch/m68k/src/isa/fields.rs; isa/mod.rs (export)                    # §4.9
arch/m68k/src/syntax/{lexer,keywords}.rs (edits)                    # §4.3, §7.1
arch/m68k/src/asm/{mod,codes,parser,operand,expr,symbols,directives,
                   macros,encode,optimize,layout,listing,srec}.rs   # §4 (13 files + README)
arch/m68k/src/disasm/{mod,decode,format}.rs                         # §5
arch/m68k/locales/{en,fr}/{diagnostics.ftl (edit), directives.ftl}  # §7
arch/m68k/fixtures/{asm,asm-opt,asm-inc,asm-diag,disasm}/           # §10.2 (each ≤ 15 files)
arch/m68k/tests/asm/{main,parser,expr,encode,optimize,corpus,directives,macros,
                     relax,output,listing,fixtures,diagnostics}.rs
arch/m68k/tests/disasm/{main,decode,format,flow}.rs
arch/m68k/tests/isa/fields.rs; arch/m68k/tests/{roundtrip,a68k,asm_bench}.rs
apps/va/src/commands/{asm,disasm}.rs, apps/va/src/sources.rs; cli.rs, report.rs, commands/{mod,lex}.rs (edits)
apps/va/tests/{asm,disasm}.rs; apps/va/tests/lex.rs (edit)
```

Depth stays ≤ 4 (fixtures are flat in their area folder: an included file sits next to its
includer). `arch/m68k/tests/` holds 11 entries, `arch/m68k/src/asm/` 14, `va-core/src/` 12.

## 3. `va-core` contracts (WP-A)

### 3.1 Decisions against m1 §3.7

- `Architecture::assembler` and `disassembler` return **`Option<Box<dyn …>>` with a default body
  returning `None`**. Reason: three types outside `arch/m68k` implement `Architecture` today or in
  M4: `crates/va-core/tests/regs.rs` (`TestCpu`), `crates/va-workspace/tests/i18n_localizer.rs`
  (`FakeCpu`, an M4 file) and M4's `crates/va-ui/tests/support/mod.rs`; a required method would
  break the M4 files, which M2 may not touch. A CPU may also ship without either. *Rejected:* the
  m1 signature `Box<dyn …>` (forces every fake to implement it); a "null assembler" default (fake
  diagnostics, and a disassembler that calls every word illegal).
- **Frozen-contract check (Architect, 2026-10-24).** This changes the return type that m1 §3.7
  fixed for M2. That signature was never coded, so no public item of `va-core` is renamed, removed
  or re-typed: in code, M2 only adds items (two provided trait methods, two modules). No pure
  addition with the m1 type works: a required method breaks the fakes above, and a provided method
  needs a value of type `Box<dyn Assembler>`, which only a fabricating null object can give.
  Ratified in architecture §3, which supersedes the M2 row of m1 §3.7 on this point.
- M3 keeps the m1 §3.7 signatures of `emulators`/`io_profiles` and gives them default bodies
  returning an empty `Vec` (architecture §3), for the same reason.
- `SourceProvider` keeps its two m1 methods and gains two **provided** methods for `INCBIN`.
- `AssembleOutput` gains `cancelled`; `DecodedInsn` carries `address`, `target`, `flow`.
- va-core keeps its rules: no I/O, no threads, no dependency, no prose. Derives as in m1 §3.1.

### 3.2 `asm.rs` (frozen)

```rust
pub struct CancelToken(/* private: Arc<AtomicBool> */);        // Clone, Debug, Default
impl CancelToken { pub fn new() -> Self; pub fn cancel(&self); pub fn is_cancelled(&self) -> bool; }

pub enum PathRefusal { Empty, Absolute, Prefix, Colon, DeviceName, Escapes, Nul }   // Copy; id(): "empty", "absolute", "drive", "colon", "device", "escapes", "nul"
pub enum IncludeError {                                         // Copy
    NotFound, Refused(PathRefusal), TooLarge { bytes: usize }, NotUtf8 { offset: usize },
    Unreadable, TooManyFiles, Unsupported,
}
pub trait SourceProvider: Sync {
    fn text(&self, file: FileId) -> Option<&str>;
    fn resolve_include(&self, from: FileId, path: &str) -> Result<FileId, IncludeError>;
    fn resolve_binary(&self, from: FileId, path: &str) -> Result<FileId, IncludeError> { Err(IncludeError::Unsupported) }
    fn binary(&self, file: FileId) -> Option<&[u8]> { None }
}
pub struct AsmLimits {                                          // Copy; DEFAULT values below
    pub max_files: u32,               // 256 files loaded (root included)
    pub max_include_depth: u32,       // 16
    pub max_macro_depth: u32,         // 64 nested expansions
    pub max_statements: u32,          // 1_000_000 statements after expansion
    pub max_diagnostics: u32,         // 1_000 (errors + warnings); then one note
    pub max_listing_bytes: u64,       // 64 MiB of rendered listing text
    pub max_relaxation_passes: u32,   // 16
}
impl AsmLimits { pub const DEFAULT: Self; }                     // also `Default`
pub struct AsmOptions { pub optimize: bool, pub listing: bool, pub limits: AsmLimits }  // Default: true, false, DEFAULT
pub struct AssembleInput<'a> { pub root: FileId, pub sources: &'a dyn SourceProvider, pub options: &'a AsmOptions }
pub enum SymbolKind { Label, LocalLabel, Equ, Set, Macro }      // id(): "label", "local-label", "equ", "set", "macro"
pub struct Symbol {
    pub name: String, pub kind: SymbolKind,
    pub value: Option<i64>,           // None for macros; SET: last value; CPU word, sign-extended (m68k: 32 bits)
    pub scope: Option<String>,        // owner of a local label (the symbol that opened its block)
    pub definition: SourceSpan, pub references: Vec<SourceSpan>,   // references in source order
}
pub struct SymbolTable { /* private: sorted by (name, scope) */ }
impl SymbolTable {
    pub fn new(symbols: Vec<Symbol>) -> Self;  pub fn symbols(&self) -> &[Symbol];
    pub fn get(&self, name: &str) -> Option<&Symbol>;            // non-local symbols and macros only
}
pub struct ListingLine {
    pub file: FileId, pub line: u32,  // source line; for a macro expansion, the call site
    pub depth: u8,                    // 0 = a file line, n = expansion depth
    pub text: String,                 // the line as assembled (expanded text for depth > 0), at most 256 chars
    pub address: Option<u64>, pub value: Option<i64>,           // value: EQU, SET, `=` lines
    pub bytes: Vec<u8>, pub byte_count: u64,                    // first ≤ 64 bytes; total produced
    pub skipped: bool,                // inside a false IFxx block
    pub optimization: Option<&'static str>,                     // id of §4.10, e.g. "addq"
    pub errors: u16, pub warnings: u16,
}
pub struct Listing { pub lines: Vec<ListingLine>, pub truncated: bool }
pub struct AssembleOutput {
    pub image: Option<ProgramImage>,  // Some iff no blocking diagnostic and not cancelled
    pub diagnostics: Vec<Diagnostic>, pub symbols: SymbolTable, pub source_map: SourceMap,
    pub listing: Option<Listing>,     // Some iff options.listing
    pub cancelled: bool,
}
pub trait Assembler: Send {
    fn assemble(&mut self, input: &AssembleInput<'_>, cancel: &CancelToken) -> AssembleOutput;
}
pub struct MemorySources { /* private */ }                      // in-memory provider: tests, bench, round trip
impl MemorySources {
    pub fn new(root_name: &str, root_text: String) -> Self;     // root = FileId(0)
    pub fn with_text(self, name: &str, text: String) -> Self;   // FileId(1), (2), … in call order
    pub fn with_binary(self, name: &str, bytes: Vec<u8>) -> Self;
    pub fn name(&self, file: FileId) -> Option<&str>;
}
```

`MemorySources::resolve_include` reads `\` as `/` and matches names exactly; anything else is
`NotFound`. `ListingLine.bytes` keeps the first 64 bytes so DS/DCB lines stay small.

### 3.3 `disasm.rs` (frozen)

```rust
pub enum FlowKind { Sequential, Jump, Branch, Call { return_address: u64 }, Return, Trap }  // id(): kebab
pub struct DecodedInsn {
    pub address: u64, pub len: u32,
    pub text: String,                 // "move.l d0,d1": operation, one space, operands; reassembles (§5.2)
    pub tokens: Vec<Token>,           // covers `text` exactly (check_line_tokens holds)
    pub target: Option<u64>,          // branch, jump, call or PC-relative target
    pub flow: FlowKind,
}
pub enum DecodeError { Truncated { needed: u32 }, Illegal { word: u16 } }
pub trait Disassembler: Send {
    fn decode(&self, bytes: &[u8], address: u64) -> Result<DecodedInsn, DecodeError>;
    fn flow_kind(&self, insn: &DecodedInsn) -> FlowKind;        // m68k: insn.flow
}
```

### 3.4 `arch.rs` additions (frozen)

```rust
fn assembler(&self) -> Option<Box<dyn Assembler>> { None }
fn disassembler(&self) -> Option<Box<dyn Disassembler>> { None }
```

Tests (`va-core/tests/asm.rs`, `disasm.rs`): token clones share state; `DEFAULT` values;
`SymbolTable::get` ignores local labels and finds macros; sort order; `MemorySources` ids and
`\` handling; `id()` strings; the default methods of a fake architecture return `None`.

## 4. Assembler (`arch/m68k/src/asm/`)

### 4.1 Pipeline

1. **Expand** (pass 1, source order): lex each line (`syntax::lex_line`), parse it, follow
   `INCLUDE`, record and expand macros, evaluate `IFxx`, stop at `END`. Every resulting statement
   is sized with the knowledge available at that point (§4.10) and placed by the location counter.
   Labels get pass-1 values. Cancellation is polled every 256 statements in every pass.
2. **Relax** (only with `optimize`): shorten branches (§4.10) and recompute addresses.
3. **Encode** (pass 2): evaluate every expression with final values, encode, range-check,
   build segments, source map, symbols and listing.

The statement list is built once: conditionals, macros and includes are never re-read in pass 2.
`M68kAssembler` (unit struct, `impl Assembler`) and `pub fn assemble(…)` are the entry points;
`M68K.assembler()` returns `Some(Box::new(M68kAssembler))`.

### 4.2 Internal interfaces between WP-B, WP-C and WP-D (frozen)

```rust
// expr.rs (B)
pub enum Expr { Num(i64), Char(i64), Here, Sym(SymRef), Unary(UnOp, Box<Expr>), Binary(BinOp, Box<Expr>, Box<Expr>) }
pub struct SymRef { pub name: String, pub local: bool, pub range: TextRange }   // range: line-relative
pub enum UnOp { Neg, Plus, Not }  pub enum BinOp { Shl, Shr, And, Or, Mul, Div, Add, Sub }
pub enum Lookup { Known(i64), Later(i64), Unknown, Undefined, Poisoned }
pub trait Scope { fn lookup(&self, sym: &SymRef) -> Lookup; fn here(&self) -> i64; }
pub struct Value { pub value: i64, pub known: bool }     // known: no Later/Unknown symbol involved
pub fn parse_expr(line: &str, tokens: &[Token]) -> Result<(Expr, usize), Problem>;  // usize: tokens used
pub fn eval(expr: &Expr, scope: &dyn Scope) -> Result<Value, Problem>;
// operand.rs (B)
pub struct Xn { pub reg: u8, pub long: bool }            // 0–7 = D0–D7, 8–15 = A0–A7
pub enum Ea { Dn(u8), An(u8), Ind(u8), PostInc(u8), PreDec(u8), Disp(Expr, u8), Index(Expr, u8, Xn),
              PcDisp(Expr), PcIndex(Expr, Xn), Abs(Expr, Option<Size>), Imm(Expr) }
pub enum Operand { Ea(Ea), RegList(u16), Ccr, Sr, Usp, Bytes(Vec<u8>) }  // RegList bit n = Dn, bit 8+n = An
pub struct Spanned<T> { pub item: T, pub range: TextRange }
pub fn parse_operand(line: &str, tokens: &[Token]) -> Result<(Spanned<Operand>, usize), Problem>;
// parser.rs (B)
pub enum OpField { Mnemonic { spelling: &'static str, size: Option<Size> }, Directive { name: &'static str, size: Option<Size> },
                   Macro { name: String, suffix: Option<String> } }
pub struct Line { pub label: Option<Spanned<String>>, pub local_label: bool, pub op: Option<Spanned<OpField>>,
                  pub operand_field: TextRange, pub problems: Vec<Problem> }
pub fn parse_line(line: &str, tokens: &[Token]) -> Line;   // fields only; operands are parsed by the user of the field
pub struct Problem { pub code: DiagCode, pub range: TextRange, pub args: Vec<(&'static str, DiagArg)>, pub fix: Option<(TextRange, String)> }
// encode.rs, optimize.rs (C)
pub struct Choice { pub entry: &'static Instruction, pub form: usize, pub condition: Option<u8>, pub size: Option<Size> }
pub fn select(spelling: &str, size: Option<Size>, ops: &[Spanned<Operand>]) -> Result<Choice, Problem>;
pub fn length(choice: &Choice, ops: &[Spanned<Operand>]) -> u32;            // bytes; depends on operand shapes only
pub fn encode(choice: &Choice, ops: &[Spanned<Operand>], scope: &dyn Scope, address: u64, out: &mut Vec<u16>) -> Result<(), Problem>;
pub fn optimize(choice: &Choice, ops: &[Spanned<Operand>], scope: &dyn Scope, generic: bool)
    -> Option<(Choice, Vec<Spanned<Operand>>, &'static str)>;               // generic: written as ADD/SUB/MOVE/…, not ADDI/ADDA/…
```

`Lookup::Known` = defined on an earlier statement (in expansion order); `Later(v)` = defined
further down, value `v` (pass 2 only); `Unknown` = defined further down, pass 1; `Poisoned` = its
definition failed (uses report nothing, §4.11).

### 4.3 Statements and lexing

- The M1 lexer is the only tokenizer; the parser never re-scans text except macro arguments and
  directive strings. Field rules are m1 §5.1–§5.3; the assembler reports every lexer diagnostic
  of every assembled line (`SyntaxProvider::line_diagnostics`), once per file line, and once per
  expanded line at the call site. `lex-operands-in-comment` is replaced by `asm-operands-in-comment`
  (§7.2) on lines that are instructions or `DC`/`DCB`.
- **Lexer amendment (WP-B, `syntax/lexer.rs`):** in the operand field of a `MacroName` operation,
  a `<` at the start of an argument (field start or right after `,`) opens a bracketed argument
  that ends at the next `>`; the group, brackets included and whitespace allowed inside, is one
  `String` token. Without `>` on the line: `Invalid(lex-unterminated-string)` to the end of line.
  Reason: a68k accepts `name <4(a0,d0)>,d1` and `name <1 2>` (observed). New `fields.rs` rows and a
  re-blessed `macros.tokens`. `TokenKind` is unchanged.
- Case: mnemonics, directives, registers and suffixes are case-insensitive; **symbols and macro
  names are case-sensitive** (a68k: `Foo` ≠ `FOO`, `mac` does not call `Mac`).
- A column-1 identifier is always a label (`nop` alone in column 1 defines `nop`). When a column-1
  label equals a mnemonic or directive spelling (any case) and nothing but a comment follows, the
  warning `asm-label-like-keyword` is added with a fix that inserts a tab (m1 §5.4 promise).
- Operand syntax (strict A68k): `Dn An SP (An) (An)+ -(An) e(An) e(An,Xn[.W|.L]) e(PC)
  e(PC,Xn[.W|.L]) e (e).W (e).L #e`, register lists `d0-d3/a0/a5-a6`, `CCR SR USP`. Index size
  defaults to `.W`. In `e(PC)`, `e` is the **target address** (displacement = e − address of the
  extension word; a68k: `$1010(pc)` at `$100A` gives `0004`). Rejected with a fix: `(a0,d0)`
  (`asm-missing-displacement`, fix `0(a0,d0)`), `$400.w` (`asm-absolute-size-syntax`, fix
  `($400).w`). Rejected: `(4,a0)` (Motorola-2 syntax, `asm-bad-operand`), `d0.w` outside an index
  (`asm-register-size-suffix`; m1 §12.4).

### 4.4 Expressions (`expr.rs`)

- Values are 32-bit two's complement; every operation wraps to 32 bits; `Value.value` is the
  result sign-extended to `i64`. `/` truncates toward zero; by zero → `asm-division-by-zero`.
  `>>` is arithmetic (`-1>>1` = `$FFFFFFFF`, observed); shifts by ≥ 32 give 0 (`<<`) or the sign
  fill (`>>`).
- **Precedence (observed, highest first), left-associative within a level:** unary `-` `+` `~`;
  `<<` `>>`; `&` `!`; `*` `/`; `+` `-`. Examples (a68k): `1+2<<1` = 5, `2*3&1` = 2, `1!2&0` = 0,
  `2<<1*3` = 12, `1!1<<1` = 3, `~1+1` = `$FFFF` (word), `6/2*3` = 9. A unary operator directly
  after another unary operator is an error (`--2`, a68k "Error in operand format").
- Terms: numbers (`$` hex, `%` binary, `@` octal, decimal), symbols, local labels (`1$`, `\name`),
  `*` (the location counter of the statement, after its alignment), quoted characters (1–4 chars,
  right-justified: `'ab'` = `$6162`), parentheses.
- In `DC`, an operand that is exactly one quoted string is a byte string (§4.5). A quoted string
  combined with an operator in `DC.B` is `asm-string-in-expression` (a68k emits the string, then
  the rest: `'A'+1` → `41 01`; deliberately stricter, §4.11).

### 4.5 Directives (`directives.rs`)

| Directive | Behaviour (strict A68k) |
|---|---|
| `ORG e` | `e` known at this line, ≤ `$FFFFFF`; sets the location counter (a new segment if not contiguous). Default origin 0. |
| `lab EQU e`, `lab = e` | Defines a constant; `e` may use later symbols (value fixed in pass 2; uses before the line see `Unknown`). Redefinition → `asm-duplicate-symbol`. |
| `lab SET e` | `e` known at this line; redefinable by `SET` only; uses see the value current at their line. |
| `DC.B/.W/.L list` (no suffix: `.W`) | Each operand an expression or, alone, a quoted string. `.B` string = its bytes (UTF-8 bytes, as a68k); `.W`/`.L` string of ≤ 2/4 chars = a right-justified value (`dc.w 'a'` = `0061`), longer → `asm-string-too-long`. Range per item: `.B` −128…255, `.W` −32768…65535, `.L` any 32-bit value. `.W`/`.L` align to even first. |
| `DS.B/.W/.L n` | `n` known, ≥ 0; reserves `n × size` zero bytes (in the image and the S-record, as a68k); `.W`/`.L` align to even first. |
| `DCB.B/.W/.L n,v` | `n` known, ≥ 0 copies of `v`; `.W`/`.L` align. |
| `EVEN` | Pads one zero byte if the counter is odd. |
| `CNOP o,a` | `a` known, a power of two ≥ 2, `0 ≤ o < a`: pads zeros to the next address ≡ `o` (mod `a`). |
| `INCLUDE path`, `INCBIN path` | §4.8. `INCBIN` appends the file's bytes, no alignment. |
| `lab MACRO` … `ENDM`, `MEXIT` | §4.6. |
| `IFEQ IFNE IFGT IFGE IFLT IFLE e`, `IFD IFND sym`, `IFC IFNC 's1','s2'`, `ENDC`/`ENDIF` | §4.7. |
| `END [e]` | Stops assembly (the rest of every file is ignored); `e` = entry point (S8 record). Non-blank, non-comment text after it → warning `asm-text-after-end`. END is optional. |
| `PAGE SPC LIST NOLIST NOL TTL TITLE IDNT` | Accepted, no effect (listing control). |
| `SECTION CODE DATA BSS CSEG DSEG XDEF XREF PUBLIC` | **`asm-not-in-s-format`** (blocking), with the `.help` pointing to ORG: a68k refuses each of them when it writes S-records (observed for all nine, §12.1). User decision Q1 (a), 2026-10-24 (§13.3). |
| `EQUR REG RORG NEAR FAR` | `asm-unsupported-directive` (blocking). |
| `ELSE REPT OPT` (lexed as `MacroName`) | `asm-not-a68k-directive` (blocking) unless a macro of that exact name exists. |

- **Automatic alignment** (observed): every instruction and every `.W`/`.L` `DC`/`DS`/`DCB`
  starts at an even address; a pad byte `00` is inserted (part of the image, not of any
  source-map entry). A label on such a line takes the aligned address; a label alone on a line
  takes the current address.
- **Labels and local labels.** A statement's label is defined before its operands are evaluated
  in pass 2 but counts as `Unknown` for its own pass-1 sizing (a68k: `self: jmp self` is absolute
  long). Local labels (`n$`, `\name`) live in the block opened by the last symbol defined in a
  label field: an ordinary label, an `EQU`, a `SET` or a `=` (observed: an `EQU` ends the block of
  `1$`); macro names do not open blocks. An undefined local label is `asm-undefined-local-label`
  with a related note at the block start (a68k silently uses 0; §4.11).
- Labels on `INCLUDE`, `IFxx`, `ENDC`, `MEXIT`, `END` lines are ordinary labels at the current
  address; `EQU`/`SET`/`=`/`MACRO` without a label → `asm-label-required`.
- `ORG`, `DS`, `DCB`, `CNOP`, `SET` and `IFxx` need values known at their line, else
  `asm-value-not-known` (with the later symbol named).

### 4.6 Macros (`macros.rs`)

- `name MACRO` stores the raw body lines up to the matching `ENDM`. A `MACRO` inside a body, a
  macro named like a mnemonic or directive (`asm-reserved-name`), or a second definition of a name
  (`asm-duplicate-symbol`) is an error. A macro must be defined before its first call.
- **Arguments:** the operand field (m1 §5.1: it ends at the first unquoted whitespace) split at
  commas outside quotes and `<…>`; the brackets are removed (`<4(a0,d0)>` → `4(a0,d0)`,
  `<1 2>` → `1 2`). Any number of arguments; `NARG` is their count inside the expansion.
- **Substitution (textual, everywhere in the body line):** `\1`–`\9` the arguments (missing → empty);
  `\0` the call's size suffix uppercased, `W` when absent (observed `move.\0` → `move.W`);
  `\@` = `.` + the expansion number, 3 digits zero-padded (`.001`, …, `.999`, then `.1000`), one
  number per expansion, counted over the whole assembly (observed `lp\@` → `lp.003`).
- The expanded line is lexed and parsed like a file line (≤ 64 KiB, else `lex-line-too-long`).
  `MEXIT` ends the current expansion. Expansion depth > `max_macro_depth` → `asm-macro-too-deep`
  (that call produces nothing). Statements over `max_statements` → `asm-expansion-limit`, and
  assembly stops (a68k dies with "Secondary heap overflow" on a recursive macro).
- Diagnostics of an expanded line point at the call site (span = call statement) and carry a
  related entry `asm-related-in-macro` at the body line. `\n` outside a macro body →
  `asm-macro-param-outside`.

### 4.7 Conditionals

Evaluated once, in pass 1. `IFxx e` compares `e` with 0; `IFD`/`IFND` test whether the symbol is
defined on an earlier statement; `IFC`/`IFNC` compare two quoted strings byte for byte. Blocks
nest; lines of a false block are not lexed for diagnostics (they still appear in the listing,
`skipped`). Missing `ENDC` at the end of the root or an `ENDC` without `IF` →
`asm-unbalanced-conditional`; the same for `ENDM`/`MEXIT` outside a macro and a `MACRO` without
`ENDM` (`asm-unbalanced-macro`).

### 4.8 INCLUDE and INCBIN

- The path is the operand's `String` token, quotes removed. The assembler calls
  `resolve_include`/`resolve_binary`, then `text`/`binary`. Diagnostics and source map entries of an
  included file use its `FileId`; each such diagnostic carries one `asm-related-included-from`
  entry per include level (innermost INCLUDE statement first).
- A file already on the include stack → `asm-include-cycle`; depth > `max_include_depth` →
  `asm-include-too-deep`; `IncludeError` → `asm-include-not-found`, `asm-include-refused`
  (`reason` = `PathRefusal::id`) or `asm-include-failed` (`too-large`, `not-utf8`, `unreadable`,
  `too-many-files`, `unsupported`). The line produces nothing and assembly continues (a68k stops
  at the first missing file; §4.11).
- **CLI provider (`apps/va/src/sources.rs`, `FsSources`), the a68k rule adapted:** a68k resolves a
  path against the **working directory**, then each `-i` folder, never against the including file
  (observed). `FsSources` uses the **root file's folder** (the working directory for stdin) instead
  of the working directory, then each `-I` folder in order. The first base where the file exists
  wins. Path policy (§6.2) applies before any file-system access.

### 4.9 Operand fields (`isa/fields.rs`, WP-C day 1, frozen)

M1's `Encoding` gives the fixed bits, the size field and the EA fields. The other operand bits are
described by one explicit table, keyed by the entry and the form index:

```rust
pub enum Place {
    Ea,                    // the form's EA field(s) (Encoding::ea), extension words in operand order
    Reg9, Reg0,            // register number in bits 11–9 / 2–0
    Quick9,                // 1–8 in bits 11–9 (8 stored as 0): ADDQ, SUBQ, shift counts
    Data8,                 // MOVEQ, bits 7–0 (signed)
    Vector,                // TRAP, bits 3–0
    Disp8, Disp16,         // branch displacement: opword low byte / one extension word
    Imm,                   // immediate extension by operation size (.B: low byte of one word)
    ImmWord, ImmByte,      // one word (STOP, LINK, SR forms) / one word, value in its low byte (CCR forms, bit numbers)
    RegMask,               // MOVEM mask word, bit-reversed when the EA is -(An)
    Implied,               // CCR, SR, USP: no bits
}
pub fn places(entry: &Instruction, form: usize) -> &'static [Place];   // one per operand
```

Tests (`tests/isa/fields.rs`): every form of every entry has a row with its operand count; for each
form, the free bits (`!mask` minus size, EA and condition bits) are exactly the union of its
`Reg9`/`Reg0`/`Quick9`/`Data8`/`Vector`/`Disp8` bits, each bit claimed once; every register value
0–7 in each register field gives an opword the form `matches`.

### 4.10 Form selection, sizing and optimizations

**Selection (always, also without `optimize`):** candidates are the spelling's `also_accepts`
entries in listed order, then its own entry; the first form (table order) whose operand kinds,
modes and size fit wins. So `add.w #3,d3` is ADDI, `add.w #3,a0` ADDA, `cmp.w #3,a0` CMPA,
`move.w #5,a0` MOVEA, `and.w #3,d0` ANDI, `exg a0,d0` the `Dn,An` form (`C188`), all as a68k `-n`.
When nothing fits, the closest failure is reported: `asm-operand-count`, `asm-size-not-allowed`,
`asm-mode-not-allowed`, else `asm-no-matching-form`. `chk.l` is `asm-size-not-allowed` (68000
has no CHK.L; a68k assembles CHK.W, m1 §12.4). Branch sizes: `.S`/`.B` short, `.W`/`.L` word,
none → chosen below. A single `Dn`/`An` where a form wants `RegList` is a one-register list.

**Value ranges** (else `asm-value-out-of-range`): immediates `.B` −128…255, `.W` −32768…65535,
`.L` any 32-bit value; ADDQ/SUBQ and shift counts 1…8; MOVEQ −128…127; TRAP 0…15; bit numbers
0…255 (the CPU takes them modulo 32 or 8); STOP and SR/CCR immediates as `.W`/`.B`; LINK, `d16`
and `(e).W` −32768…32767 (`(e).W` also `$FFFF8000`…`$FFFFFFFF`); `d8` −128…127; absolute and
target addresses below `$1000000`.

**Pass-1 sizing rule.** A size-changing choice uses only values `known` at the statement (every
symbol `Known`). Unknown absolute operands are absolute long, unknown immediates keep their form,
unknown branches are word branches marked *relaxable*. Same-size choices are made in pass 2.

**Optimizations** (on by default; `optimize = false` is a68k `-n` and disables all of them). Each
applies only to the generic spelling (ADD, SUB, MOVE, LEA, MOVEM, Bcc/BRA/BSR), never to an
explicit ADDI/ADDA/SUBI/SUBA/MOVEA/ADDQ/… (observed). Ids appear in the listing.

| Id | Rule (observed on a68k) |
|---|---|
| `short-branch` | Relaxable branch → `.S` (§ below) |
| `addq`, `subq` | `ADD`/`SUB #n,<ea>` with known `n` in 1…8 → ADDQ/SUBQ (any size, An included); `n ≤ 0` stays ADDI/ADDA |
| `moveq` | `MOVE.L #n,Dn` with known `n` in −128…127 as a 32-bit value (`#$FFFFFFFF` → `moveq #-1`) |
| `lea` | `ADD`/`SUB #n,An` → `LEA ±n(An),An`: `.L` needs `n` known at the line, in 9…32767 (size change); `.W` with `n` known at the line: 9…32767; `.W` with `n` known only in pass 2: 1…32767 (same size, F1) |
| `addq-lea`, `subq-lea` | `LEA d(An),An` (same register), known `d` in 1…8 → `ADDQ.W #d,An`; −8…−1 → `SUBQ.W` |
| `zero-displacement` | `e(An)` with known `e` = 0 → `(An)` (`0(a0,d0)` unchanged) |
| `pc-relative` | Absolute operand without `.W`/`.L` and with a known value, in a form whose operand set includes `d16(PC)`, displacement in −32768…32767 → `d16(PC)` (`move.w $400,d0` → `303A F3E2`; `jmp $400`, `lea lab,a0` for a backward `lab`, `btst #1,$400`). Destinations never qualify. Otherwise absolute long (never absolute short unless written `(e).W`). |
| `movem-move` | One-register MOVEM → MOVE/MOVEA of the same size and EA, with the warning `asm-movem-single` when the effect differs (flags; for `.W` memory → Dn also no sign extension). Memory → An is identical (no warning). User decision Q2 (a), 2026-10-24 (§13.3): byte-identical to a68k. |

**Branch relaxation** (layout.rs). After pass 1, repeat at most `max_relaxation_passes` times
until nothing changes: for each relaxable branch in statement order, with the addresses of the
previous sweep and the branch still counted as 4 bytes, let `d = target − (address + 2)`; it
becomes `.S` iff `−128 ≤ d ≤ 127` and the short displacement (`d` backward, `d − 2` forward) is not
0; then recompute all addresses (alignment and CNOP padding included). A branch shortened earlier
whose short displacement no longer fits after padding grew goes back to `.W` for good. A
relaxable branch whose target is unknown stays `.W`. This reproduces every observation of §12.2
(forward reach is 124 bytes, backward −128; a backward branch gains from a later forward shrink;
`bra` to the next instruction stays `.W`). Explicit `.S` to the next instruction is
`asm-short-branch-zero` (fix: remove `.s`); explicit sizes out of range are
`asm-branch-out-of-range`. Pass 2 re-checks every pass-1 decision against final values;
a decision that no longer holds is `asm-phase-error` (a safety net; not expected).

### 4.11 Deviations from a68k (deliberately stricter; excluded from the comparison)

| a68k behaviour (observed) | VisualAssembler |
|---|---|
| `dc.b` accepts −256…−129 and `dc.w` −65536…−32769, wrapping silently | `asm-value-out-of-range` (−128…255, −32768…65535) |
| `ds.b -1` moves the counter back; negative `DCB` counts | `asm-negative-count` |
| `dc.b 'A'+1` gives `41 01` | `asm-string-in-expression` |
| An undefined local label silently reads 0 | `asm-undefined-local-label` |
| Addresses past `$FFFFFF` wrap to 0 | `asm-address-overflow` |
| `ORG` back over placed bytes writes overlapping records | `asm-overlap` (with the other place) |
| `-n` turns a one-register MOVEM into a broken word (`4880 0000`) | correct MOVEM (`4890 0001`) |
| `chk.l` assembles CHK.W | `asm-size-not-allowed` |
| Includes relative to the working directory; a missing include stops everything | relative to the root's folder; assembly continues |
| Lexer cases of m1 §5.4 | as M1 |

The a68k comparison skips fixtures marked `; a68k: skip <reason>` (§10.2).

### 4.12 Outputs (`srec.rs`, `listing.rs`, `mod.rs`)

- **Image.** Segments are maximal runs of contiguous bytes, kind `Code` (S-record programs have no
  sections, decision Q1 of §13.3). Entry = the `END` operand, else `None`.
- **S-record (default output) — byte-identical to `a68k -s`:** `S0030000FC` (no module name), then
  `S2` records (24-bit address) of 16 data bytes, each segment starting a new record, then `S8`
  with the entry address or `000000`. Uppercase hex, **CRLF after every record on every OS**.
  `pub fn to_srec(image: &ProgramImage) -> String` and `pub fn parse_srec(text: &str) ->
  Result<ProgramImage, SrecError>` (S0–S3, S5, S7–S9; LF or CRLF; checksums, lengths and hex
  checked; ≤ 16 MiB of data; overlapping records rejected; entry from S7–S9). *Rejected:* S1/S9 for
  addresses below `$10000` (a68k always writes S2/S8; byte identity wins).
- **Binary:** `pub fn to_binary(image) -> (u64, Vec<u8>)`: base = lowest address, bytes up to the
  highest, gaps filled with `00`; empty image → `(0, [])`.
- **Listing (our own format, LF):** first line `VisualAssembler listing: m68k, optimizations on|off`,
  then one line per `ListingLine`:
  `LLLLLd AAAAAA  HHHH HHHH HHHH HHHH F  source` — line number (5, right-aligned), `d` = `+` for
  an expansion, else space; address (6 hex) or `=VVVVVVVV` for a value, blank otherwise; up to 8
  bytes in words; flag `E` error, `W` warning, `o` optimized, `-` skipped (first that applies);
  source text (tabs kept, 160 chars then `…`). More bytes continue on following lines with their
  address, up to 64 bytes, then `…`. A file change prints `---- <file name>`. After the lines:
  `symbols:` (name, value as `$` + 8 hex, kind id, sorted by name) and
  `errors <n>, warnings <n>`. No diagnostic text (it is prose; diagnostics go to stderr/JSON).
  `pub fn render_listing(listing: &Listing, symbols: &SymbolTable, name: &dyn Fn(FileId) -> String) -> String`,
  capped by `max_listing_bytes` (then `listing truncated`).
- **Source map** (M3/M6): one entry per byte-producing statement (`address`, `len`, the file and
  line of the statement — the call site for macro output — and its range); a zero-length entry
  for a label-only line; `EVEN`/`CNOP` own their padding; automatic alignment pads are unmapped.

### 4.13 Error recovery

Every problem is reported on its statement and assembly continues with the next one. An
instruction with an error still occupies the length of its selected form, else 2 bytes when the
mnemonic is known, else 0, so later addresses stay plausible. A symbol whose definition failed is
`Poisoned`: its uses report nothing. Each use of an undefined symbol is reported (the Problems
panel underlines every use). After `max_diagnostics` errors and warnings, collection stops with one
`asm-too-many-diagnostics` note; assembly still runs to decide the image (which a blocking error
already prevents).

## 5. Disassembler (`arch/m68k/src/disasm/`)

### 5.1 Decoding

`M68kDisassembler` (unit struct, `impl Disassembler`; `M68K.disassembler()` returns it). A lazily
built 65,536-entry table maps each opword to its (entry, form, condition) via `Form::matches`
(unique by the M1 test). Extension words follow `isa::fields::places` and the EA fields; the brief
index word is read as the 68000 does (bits 10–8 ignored). Too few bytes → `Truncated { needed }`;
no form → `Illegal { word }`. `flow`: `Branch` Bcc/DBcc, `Jump` BRA/JMP, `Call` BSR/JSR
(`return_address = address + len`), `Return` RTS/RTE/RTR, `Trap` TRAP/TRAPV/ILLEGAL, else
`Sequential`; `target` for branches, DBcc, JMP/JSR/LEA/PEA with an absolute or PC-relative operand
and every PC-relative operand.

### 5.2 Text (`format.rs`) — reassembles to the same bytes with `optimize = false`

Lowercase canonical spelling (`dbf` is written `dbra`), explicit size whenever the form is sized
(`bra.s`, `bra.w`, `addq.w`, `move.w`), one space, operands joined by `,`. Numbers: immediates
`#$` + 2/4/8 hex digits by size; MOVEQ, ADDQ/SUBQ, shift counts, TRAP vectors and bit numbers in
signed decimal; displacements `d16`/`d8` signed decimal; absolute long `$` + 8 hex; absolute short
`($XXXX).w`; branch and PC-relative targets as the absolute address `$` + 8 hex (`$00001016(pc)`,
`bra.s $00001000`); register lists in canonical ranges (`d0-d7/a0-a6`). `tokens` are the lexer's
tokens of `"\t" + text` shifted by one, without the leading whitespace token. For an `Illegal` word
the CLI prints `dc.w $XXXX`.

## 6. Limits and security

### 6.1 Resource limits (Security M1-S5, due in M2)

| Resource | Limit | When hit |
|---|---|---|
| Source file, include file, INCBIN file | 16 MiB each (`MAX_SOURCE_BYTES`), UTF-8 for text | `asm-include-failed` / exit 3 for the root |
| Files per assembly | `max_files` = 256 | `asm-include-failed` (`too-many-files`) |
| Include nesting / macro nesting | 16 / 64 | `asm-include-too-deep` / `asm-macro-too-deep` |
| Statements after expansion | 1,000,000 | `asm-expansion-limit`, assembly stops |
| Address space (DS, DCB, ORG, INCBIN) | the 68000's 16 MiB: every byte below `$1000000` | `asm-address-overflow` |
| Diagnostics | 1,000, then one note | `asm-too-many-diagnostics` |
| Listing text | 64 MiB | `listing truncated` |
| Relaxation sweeps | 16 (each O(statements)) | remaining branches stay `.W` |

Resulting output bounds of `va asm`: image ≤ 16 MiB; S-record ≤ 2.9 × image + 30 bytes ≈ 47 MiB;
binary ≤ 16 MiB; listing ≤ 64 MiB; map ≤ ~60 MB (1,000,000 entries); stderr ≤ ~1 MB (1,000
diagnostics; `fix:` lines only for source lines ≤ 256 bytes); `--json` document ≤ ~150 MB
(symbols bounded by statements). Every algorithm is linear in statements per pass. **`va lex` fix
(WP-G, closes the diagnostic part of M1-S5):** at most 1,000 diagnostics per input in both modes,
then `note: <n> more diagnostics not shown` on stderr (text) and `"summary":{…,"errors":<all>,
"shown":1000}` (JSON, additive key; schema stays `va.lex/1`). The token stream stays proportional to
the input (measured worst case ≈ 225× for JSON; accepted, streamed, bounded by the 16 MiB input).

### 6.2 INCLUDE path policy (`FsSources`; first step of the M7 sandbox)

A source file is untrusted (a course file, a classmate's file), and INCLUDE makes **file content
choose paths**: on Windows a UNC path would open an SMB connection and can leak NTLM credentials
(M1-S6). Before touching the file system, with `/` and `\` both separators on every OS, a path is
refused when it is empty or holds NUL (`empty`, `nul`); is absolute or starts with a separator
(`absolute`); has a drive, UNC, `\\?\` or `\\.\` prefix (`drive`); contains `:` anywhere (drive-
relative and alternate data streams; `colon`); has a component that is a Windows device name (`CON
PRN AUX NUL COM1–COM9 LPT1–LPT9`, any case, any extension; `device`) on every OS; or, after
lexical normalization, climbs above its base folder (`escapes`). A candidate is then
canonicalized and must stay under the canonical base (symlink escape → `escapes`), and must be a
regular file. Reads use `Read::take(MAX_SOURCE_BYTES + 1)` and `SourceText::from_bytes`. Each file
is loaded once (same canonical path → same `FileId`). Storage: a fixed `Box<[OnceLock<Loaded>]>` of
`max_files` slots plus a `Mutex<HashMap<PathBuf, FileId>>`, so `&self` methods hand out stable
`&str`/`&[u8]` with no `unsafe`. The root path and `-I` folders come from the command line and are
used as given (the root still follows M1-S6; M7 owns that). M4's `project.toml` already forbids
`..` in `include_dirs` (m4 §4.5).

### 6.3 Other notes

No new `unsafe`, network API, allow-list entry or dependency. Every `va` text line that carries
outside text (file names, include paths, symbol names, source excerpts, fix lines) goes through
`report::terminal_safe` (`terminal_safe_json` for JSON-quoted text); `--json` stays raw. The a68k
test (§10.4) spawns `VA_A68K_PATH` with an argument vector, no shell, a working directory under
`CARGO_TARGET_TMPDIR`, outputs only there, a 30 s timeout; it never runs in CI and nothing of a68k
is committed. `C:\68000` is never written and never named in code.

## 7. Localization

### 7.1 Files and ids (WP-F)

`arch/m68k/locales/{en,fr}/diagnostics.ftl` gains every `diag-asm-*` id of §7.2 (with `.help` or
`.note` as listed). New `directives.ftl` documents the 49 directive spellings of m1 §5.3 as
`isa-m68k-dir-<name>` (`=` → `isa-m68k-dir-equals`): one-line summary and `.details` saying what
VisualAssembler does (including "refused like a68k -s" / "not supported" rows). `keywords()` then
returns these `doc_id`s; `locale_resources` returns three resources; m1 §6.3's completeness test
also collects the directive ids. `diagnostic_codes()` returns the 11 lexer codes followed by
`asm::codes::ASM_DIAG_CODES` (one static array built by a `const fn`). Style as m1 §6.1 and the
existing file: message lowercase without final period, typographic quotes, `.help` full sentences,
FR regular spaces before `: ; ? »`.

**Id namespaces (architecture §5, 2026-10-24).** Every id of `arch/m68k/locales/` stays in the
CPU's namespaces: `diag-lex-*`, `diag-asm-*` and `isa-m68k-*` (directives as `isa-m68k-dir-*`,
which no M1 id uses), and no Fluent term is added. The root `locales/` of M4 owns `ui-*`, `cmd-*`
and `diag-cfg-*`. The GUI loads both sets into one bundle per language (m4 §4.6), where a
duplicate id is a `LocalizerError::Duplicate`; m4 §10.3 `arch_registry.rs` builds that bundle for
the registered 68000 in `en` and `fr`, so a collision fails a test, never a user's start.

### 7.2 Codes, arguments, severity and text (E = blocking error unless noted; W warning; N note)

Each entry: code (args) [severity] — EN message / FR message; then `.help`/`.note` if any.

- `asm-unknown-operation` (name, count, suggestions) [E] — unknown instruction or macro “{ $name }” / instruction ou macro inconnue « { $name } ».
  .note EN `{ $count -> [0] Macro names are case-sensitive and must be defined before their first use. *[other] Did you mean { $suggestions }? }` FR `{ $count -> [0] Les noms de macro distinguent majuscules et minuscules et doivent être définis avant leur premier appel. *[other] Vouliez-vous dire { $suggestions } ? }` (up to 3 spellings or macro names within edit distance 2, as m1 §12.7)
- `asm-not-a68k-directive` (name) [E] — a68k 2.71 has no { $name } directive / a68k 2.71 ne connaît pas la directive { $name }.
  .help EN `{ $name -> [ELSE] Close the block with ENDC and open a second block with the opposite IF test. [REPT] Write the lines out, or define a macro. *[other] Optimizations are switched off with the no-optimization option of the assembler, not in the source. }` FR `{ $name -> [ELSE] Fermez le bloc par ENDC et ouvrez un second bloc avec le test IF contraire. [REPT] Écrivez les lignes, ou définissez une macro. *[other] Les optimisations se désactivent par l’option de l’assembleur, pas dans le source. }`
- `asm-unsupported-directive` (name) [E] — { $name } is an a68k directive that VisualAssembler does not support / { $name } est une directive d’a68k que VisualAssembler ne prend pas en charge.
- `asm-not-in-s-format` (name) [E] — a68k refuses { $name } when it produces S-records / a68k refuse { $name } quand il produit des S-records.
  .help EN Place code and data with ORG: a program loaded from an S-record has no sections and no external symbols. FR Placez le code et les données avec ORG : un programme chargé depuis un S-record n’a ni sections ni symboles externes.
- `asm-bad-operand` (text) [E] — cannot read the operand “{ $text }” / opérande illisible « { $text } ».
- `asm-bad-expression` (text) [E] — invalid expression “{ $text }” / expression invalide « { $text } ».
- `asm-operand-count` (mnemonic, expected, found) [E] — { $mnemonic } takes { $expected -> [0] no operand [one] one operand *[other] { $expected } operands }, not { $found } / { $mnemonic } prend { $expected -> [0] aucun opérande [one] un opérande *[other] { $expected } opérandes }, pas { $found }.
- `asm-mode-not-allowed` (mnemonic, mode, allowed) [E] — { $mnemonic } does not accept the mode { $mode } here / { $mnemonic } n’accepte pas le mode { $mode } ici. .note EN Accepted here: { $allowed }. FR Accepté ici : { $allowed }.
- `asm-size-not-allowed` (mnemonic, size, allowed) [E] — { $mnemonic } has no .{ $size } form / { $mnemonic } n’existe pas en .{ $size }. .note EN Sizes: { $allowed }. FR Tailles : { $allowed }.
- `asm-no-matching-form` (mnemonic, forms) [E] — these operands fit no form of { $mnemonic } / ces opérandes ne correspondent à aucune forme de { $mnemonic }. .note EN Forms: { $forms }. FR Formes : { $forms }.
- `asm-register-size-suffix` (text) [E] — a size after a register (“{ $text }”) is allowed only for the index of d8(An,Xn) / une taille après un registre (« { $text } ») n’est permise que pour l’index de d8(An,Xn).
- `asm-missing-displacement` [E] — a68k needs a displacement before (An,Xn) / a68k exige un déplacement devant (An,Xn). .help EN Write 0 before the parenthesis. FR Écrivez 0 devant la parenthèse. (fix)
- `asm-absolute-size-syntax` (text) [E] — a68k does not read “{ $text }” as a sized address / a68k ne lit pas « { $text } » comme une adresse avec taille. .help EN Put the address in parentheses: (address).W. FR Mettez l’adresse entre parenthèses : (adresse).W. (fix)
- `asm-value-out-of-range` (value, min, max) [E] — { $value } does not fit here (allowed: { $min } to { $max }) / { $value } ne tient pas ici (permis : de { $min } à { $max }).
- `asm-branch-out-of-range` (distance, size) [E] — the target is { $distance } bytes away, too far for a .{ $size } branch / la cible est à { $distance } octets, trop loin pour un branchement .{ $size }. .help EN Remove the size and let the assembler choose. FR Retirez la taille et laissez l’assembleur choisir.
- `asm-short-branch-zero` [E] — a short branch cannot jump to the next instruction (its displacement would be 0) / un branchement court ne peut pas viser l’instruction suivante (son déplacement vaudrait 0). .help EN Remove “.s”. FR Retirez « .s ». (fix)
- `asm-odd-branch-target` (target) [W] — the target { $target } is at an odd address: the 68000 will stop with an address error / la cible { $target } est à une adresse impaire : le 68000 s’arrêtera sur une erreur d’adresse.
- `asm-undefined-symbol` (name, count, suggestions) [E] — “{ $name }” is not defined / « { $name } » n’est pas défini.
  .note EN `{ $count -> [0] Symbols distinguish upper and lower case. *[other] Did you mean { $suggestions }? }` FR `{ $count -> [0] Les symboles distinguent majuscules et minuscules. *[other] Vouliez-vous dire { $suggestions } ? }`
- `asm-undefined-local-label` (name) [E] — the local label “{ $name }” is not defined in this block / l’étiquette locale « { $name } » n’est pas définie dans ce bloc. .note EN A local label is visible only until the next label, EQU or SET. FR Une étiquette locale n’est visible que jusqu’à la prochaine étiquette, EQU ou SET.
- `asm-duplicate-symbol` (name) [E] — “{ $name }” is already defined / « { $name } » est déjà défini.
- `asm-reserved-name` (name) [E] — “{ $name }” is an instruction or directive and cannot name a macro / « { $name } » est une instruction ou une directive et ne peut pas nommer une macro.
- `asm-label-like-keyword` (name) [W] — “{ $name }” in column 1 is a label, not an instruction / « { $name } » en colonne 1 est une étiquette, pas une instruction. .help EN Indent the line to make it an instruction. FR Indentez la ligne pour en faire une instruction. (fix: insert a tab)
- `asm-label-required` (name) [E] — { $name } needs a name in column 1 / { $name } exige un nom en colonne 1.
- `asm-value-not-known` (name, symbol) [E] — { $name } needs a value known at this line, but “{ $symbol }” is defined further down / { $name } exige une valeur connue à cette ligne, mais « { $symbol } » est défini plus bas. .help EN Move the definition of “{ $symbol }” above this line. FR Déplacez la définition de « { $symbol } » au-dessus de cette ligne.
- `asm-division-by-zero` [E] — division by zero / division par zéro.
- `asm-negative-count` (name, count) [E] — { $name } cannot use a negative count ({ $count }) / { $name } n’accepte pas un nombre négatif ({ $count }).
- `asm-address-overflow` (address) [E] — the program goes past { $address }, the end of the 68000 address space / le programme dépasse { $address }, la fin de l’espace d’adressage du 68000.
- `asm-overlap` (address) [E] — these bytes overlap bytes already placed at { $address } / ces octets recouvrent des octets déjà placés en { $address }.
- `asm-string-in-expression` (text) [E] — a quoted string cannot be part of a calculation (“{ $text }”) / une chaîne entre guillemets ne peut pas entrer dans un calcul (« { $text } »). .help EN Write the character code, for example $42. FR Écrivez le code du caractère, par exemple $42.
- `asm-string-too-long` (size, max, length) [E] — a .{ $size } value holds at most { $max } characters, this string has { $length } / une valeur .{ $size } contient au plus { $max } caractères, cette chaîne en a { $length }.
- `asm-text-after-end` [W] — the lines after END are ignored / les lignes après END sont ignorées.
- `asm-unbalanced-conditional` (name, kind) [E] — `{ $kind -> [close] { $name } has no matching IF *[open] this { $name } block is never closed by ENDC }` / `{ $kind -> [close] { $name } n’a pas de IF correspondant *[open] ce bloc { $name } n’est jamais fermé par ENDC }`.
- `asm-unbalanced-macro` (name, kind) [E] — `{ $kind -> [close] { $name } is outside any macro *[open] the macro “{ $name }” has no ENDM }` / `{ $kind -> [close] { $name } est hors de toute macro *[open] la macro « { $name } » n’a pas de ENDM }`.
- `asm-macro-param-outside` (text) [E] — “{ $text }” is a macro parameter, but this line is not in a macro / « { $text } » est un paramètre de macro, mais cette ligne n’est pas dans une macro.
- `asm-macro-too-deep` (limit) [E] — macros call each other more than { $limit } levels deep / les macros s’appellent sur plus de { $limit } niveaux. .help EN Look for a macro that calls itself. FR Cherchez une macro qui s’appelle elle-même.
- `asm-expansion-limit` (limit) [E] — the program exceeds { $limit } lines once macros are expanded; assembly stops / le programme dépasse { $limit } lignes une fois les macros développées ; l’assemblage s’arrête.
- `asm-include-too-deep` (limit) [E] — INCLUDE files are nested more than { $limit } levels deep / les fichiers INCLUDE sont imbriqués sur plus de { $limit } niveaux.
- `asm-include-cycle` (path) [E] — “{ $path }” includes itself / « { $path } » s’inclut lui-même.
- `asm-include-not-found` (path) [E] — cannot find “{ $path }” / fichier « { $path } » introuvable. .note EN Paths are relative to the folder of the main file, then to each include folder. FR Les chemins partent du dossier du fichier principal, puis de chaque dossier d’inclusion.
- `asm-include-refused` (path, reason) [E] — EN the path “{ $path }” is refused: `{ $reason -> [absolute] it is absolute [drive] it names a drive, a network share or a device [colon] it contains “:” [device] it names a Windows device [escapes] it leads out of its folder *[other] it is empty or invalid }` FR le chemin « { $path } » est refusé : `{ $reason -> [absolute] il est absolu [drive] il désigne un lecteur, un partage réseau ou un périphérique [colon] il contient « : » [device] il désigne un périphérique Windows [escapes] il sort de son dossier *[other] il est vide ou invalide }`. .help EN Use a relative path inside the folder of the main file, or add an include folder. FR Utilisez un chemin relatif dans le dossier du fichier principal, ou ajoutez un dossier d’inclusion.
- `asm-include-failed` (path, reason) [E] — EN cannot use “{ $path }”: `{ $reason -> [too-large] it is larger than 16 MB [not-utf8] it is not UTF-8 text [too-many-files] the program already uses too many files [unsupported] this tool cannot read binary files *[other] it cannot be read }` FR impossible d’utiliser « { $path } » : `{ $reason -> [too-large] il dépasse 16 Mo [not-utf8] ce n’est pas du texte UTF-8 [too-many-files] le programme utilise déjà trop de fichiers [unsupported] cet outil ne lit pas les fichiers binaires *[other] il est illisible }`.
- `asm-too-many-diagnostics` (limit) [N] — only the first { $limit } problems are reported / seuls les { $limit } premiers problèmes sont signalés.
- `asm-operands-in-comment` (text, kind, bytes, kept, cause) [E, non-blocking] — the m1 §5.7 messages; `.help` selected on `$cause` exactly as `diag-lex-operands-in-comment`.
- `asm-movem-single` (size, effect) [W] — EN a68k assembles this one-register MOVEM as MOVE.{ $size }: `{ $effect -> [sign] the flags change and the register keeps its upper word instead of being sign-extended *[flags] the flags N and Z change and V and C are cleared }` FR a68k assemble ce MOVEM d’un seul registre en MOVE.{ $size } : `{ $effect -> [sign] les indicateurs changent et le registre garde son mot haut au lieu d’être étendu par le signe *[flags] les indicateurs N et Z changent, V et C sont mis à 0 }`. .help EN Write MOVE yourself, or switch optimizations off to keep MOVEM. FR Écrivez MOVE vous-même, ou désactivez les optimisations pour garder MOVEM.
- `asm-phase-error` (name) [E] — the value of “{ $name }” changed after the layout was fixed / la valeur de « { $name } » a changé après le placement du code.
- Related (rendered as notes): `asm-related-first-definition` (name) first definition of “{ $name }” / première définition de « { $name } »; `asm-related-scope-start` this block starts here / ce bloc commence ici; `asm-related-included-from` included from here / inclus depuis ici; `asm-related-in-macro` (name) line of the macro “{ $name }” / ligne de la macro « { $name } »; `asm-related-other-bytes` the other bytes come from here / les autres octets viennent d’ici.

Numbers in `value`, `min`, `max`, `address`, `target` are passed as formatted `Str` (`$1000`, `-129`,
m1 §3.3); `count`, `expected`, `found`, `limit`, `distance`, `bytes`, `length`, `max` are `Int`.
`text` is truncated to 32 chars plus `…` (m1 §3.4).

## 8. CLI (WP-G, `apps/va`)

### 8.1 Grammar

```
va [--json] [--lang en|fr] asm <FILE|-> [-o <OUT|->] [--format srec|bin] [--listing[=<PATH>]]
                                        [--map[=<PATH>]] [-I <DIR>]... [--no-opt]
va [--json] disasm <FILE|-> [--format srec|bin] [--org <ADDR>]
```

- `-o` defaults to `<FILE stem>.hex` (srec) or `<FILE stem>.bin` next to FILE. **Default
  extension `.hex`:** d68k's open dialog lists only "S-Record (*.hex)" files (plan §1), so the
  student's file shows up there directly; the content is always Motorola S-record (`--format`
  decides, never the extension). *Rejected:* `.s` (a68k's default; collides with assembler sources),
  `.s68` (hidden by d68k's filter). With FILE `-`, `-o` is required (`output-required`, exit 2).
  `-o -` writes the output on stdout (text mode only; with `--json`: `stdout-conflict`, exit 2).
  An output path equal to FILE → `output-is-input`, exit 2.
- `--listing` writes `<OUT stem>.lst` (or PATH); `--map` writes `<OUT stem>.map` (or PATH,
  JSON `va.asm.map/1`). `-I` adds include folders in order. `--no-opt` = a68k `-n`.
- `--org` (bin only, default 0) and addresses accept `$1000`, `0x1000` or decimal.
- Each output is written atomically by the rule of architecture §6 (amended 2026-10-24) and the
  procedure of m4 §4.7: an existing target is resolved with `fs::canonicalize` (a symlink is never
  replaced); the temp file `<name>.<pid>.<n>.tmp` (`n` a per-process counter) is created in the
  target's folder with `create_new(true)`, written, flushed and `sync_all`ed, then renamed over the
  target; on error it is removed. *Rejected:* a fixed `<path>.tmp`, which can clobber a student's
  file of that name and collides between two concurrent runs. The helper is private to
  `apps/va/src/commands/asm.rs`: M4's `va_workspace::persist::write_atomic` lands in the same wave,
  so M2 does not wait for it (convergence is deferred, §13.2). Outputs are written iff an image
  exists (no blocking diagnostic); otherwise nothing is written and existing files are left
  unchanged.

### 8.2 `va asm` results and exit codes

- Exit 0: image produced, no error diagnostic (warnings allowed). Exit 1: at least one error
  diagnostic, blocking or not (a non-blocking error still writes the outputs, as `va lex` counts
  §5.7). Exit 2: usage (clap, `output-required`, `output-is-input`, `stdout-conflict`). Exit 3:
  root unreadable/too large/not UTF-8 (M1 codes), `write-failed`, `no-architecture`,
  `locale-invalid`, stdout/stderr failure. Include problems are diagnostics (exit 1).
- **Text mode:** diagnostics on stderr in the `va lex` format
  (`<file>:<line>:<column>: <severity>[<code>]: <message>`, `  help:`, `  note:`, and `  fix:`
  only for source lines ≤ 256 bytes), each related entry as `<file>:<line>:<column>: note: <message>`; file names are the root as
  given and includes as `base.join(path)` (lexical, so relative stays relative). Then one stdout
  line (stderr when `-o -`): `wrote <OUT> (<n> bytes, <k> segments, entry $XXXXXX)` or
  `no output: <e> errors` — English CLI text.
- **JSON `va.asm/1`** (one document, keys in this order):

```json
{"schema":"va.asm/1","arch":"m68k","lang":"en","file":"prog.asm","optimize":true,
 "files":[{"id":0,"name":"prog.asm"},{"id":1,"name":"inc/defs.i"}],
 "output":{"format":"srec","path":"prog.hex","written":true},"listing":{"path":"prog.lst","written":true},"map":null,
 "image":{"entry":4096,"bytes":44,"segments":[{"address":4096,"len":44,"kind":"code"}]},
 "symbols":[{"name":"start","kind":"label","value":4096,"scope":null,"file":0,"line":2,"column":1,"references":3}],
 "diagnostics":[{"code":"asm-undefined-symbol","severity":"error","blocking":true,"file":0,"line":7,"column":9,
   "start":95,"end":101,"message":"…","help":null,"note":"…","fix":null,
   "related":[{"code":"…","file":0,"line":3,"column":1,"message":"…"}]}],
 "summary":{"statements":18,"errors":1,"warnings":0,"notes":0}}
```

  `image` is `null` without an image; `listing`/`map` are `null` without the flag. JSON keeps
  outside text raw (serde escapes it). `va.asm.map/1`: `{"schema":"va.asm.map/1","arch":"m68k",
  "files":[…],"entries":[{"address":4096,"len":6,"file":0,"line":2,"start":11,"end":33}]}`.

### 8.3 `va disasm`

Reads FILE or stdin (≤ 16 MiB, `Read::take`); `srec` (default) is parsed by `parse_srec`, `bin`
is loaded at `--org`. Bad S-record → `input-not-srecord` (exit 3, message names the line and the
reason). Each segment is decoded from its start; an `Illegal` or `Truncated` word prints as
`dc.w $XXXX` (a trailing odd byte as `dc.b $XX`) and decoding resumes 2 bytes later. Text: one line
per item `AAAAAA  <words, padded to 24>  <text>`. JSON `va.disasm/1`: `{"schema":"va.disasm/1",
"arch":"m68k","file":"…","entry":…|null,"segments":[{"address":…,"items":[{"address":…,"bytes":"4E71",
"text":"nop","flow":"sequential","target":null,"valid":true}]}]}`. Exit 0 (illegal words are not
an error; `valid:false` marks them).

### 8.4 Other CLI changes

`report::ErrorCode` gains `write-failed` (3), `input-not-srecord` (3), `output-required`,
`output-is-input`, `stdout-conflict` (2). `Session::open` is reused; `va disasm` needs no localizer
(it calls `default_architecture()` directly). The `va lex` cap of §6.1. `va info` is unchanged.

## 9. Manifests, Makefile, dependencies (shared files)

- **No new dependency.** `Cargo.toml`, `Cargo.lock`, `.cargo/*`, `ci.yml` and the xtask layering
  table are unchanged by M2 (no new package; `va-arch-m68k → va-core` and `va → va-arch-m68k,
  va-core, va-workspace` are already allowed). *Rejected:* `memchr`/`smallvec` for speed (measure
  first, M7); a hex or S-record crate (40 lines each).
- **The lock belongs to M4.** M4's WP0 changes `Cargo.toml` and `Cargo.lock` in wave 1 (§14); M2's
  WP0 and wave 2 start from that lock and never commit a change to it. Because the `xtask` alias
  has no `--locked` (CLAUDE.md pitfall), a `Cargo.lock` diff in an M2 work package is a defect to
  drop, not to commit. After M4's WP-B, `va` links `toml` and, on Windows, `sys-locale` through
  `va-workspace` (m4 §8.2); M2 uses neither, and M2's `make security` runs on the merged lock.
- **Makefile (normative, WP0, applied on top of M4's WP0 diff, §14):** add `bench-asm
  compare-a68k` to `.PHONY`; after the `bench-lexer` help line add `$(info   bench-asm    measure assembler throughput on 5000 and 100000 generated lines in release)`
  and `$(info   compare-a68k compare with a68k byte for byte - needs VA_A68K_PATH - never in CI)`;
  the header comment (which M4's WP0 has already extended) gets its final text:

```make
# Normative content: docs/specs/m0-foundations.md section 7, amended by sections 17.5 and 18.7
# (security target) and 19.5 (RENDERER default), and amended by m1-core-isa-lexer.md section 8
# (bench-lexer target), m4-editor-shell.md section 8 (bench help line) and m2-assembler.md
# section 9 (bench-asm and compare-a68k targets). Comments may be added; values must not change.
```

  Targets:

```make
bench-asm:
	$(CARGO) test -p va-arch-m68k --release --test asm_bench -- --ignored --nocapture

compare-a68k:
	$(CARGO) test -p va-arch-m68k --test a68k -- --ignored --nocapture
```

  M4 also edits the Makefile (m4 §8.2: the header comment and the `bench` help line). Both diffs
  touch the header comment, so they are applied in sequence: M4's WP0 in wave 1, then this one in
  wave 1b (§14). The `bench` recipe and every other line stay as they are.

## 10. Tests and acceptance

### 10.1 Unit and integration tests (each file starts with `//!` and `#![allow(clippy::unwrap_used)]`)

| File | Covers |
|---|---|
| `tests/isa/fields.rs` | §4.9 |
| `tests/asm/parser.rs`, `expr.rs` | every operand form and rejection of §4.3 with its fix; the `<…>` lexer rows; precedence examples of §4.4, wrap, `/` toward zero, `>>` sign, shifts ≥ 32, chars, `*`, `--2` |
| `tests/asm/encode.rs` | the golden opwords of m1 §4.7 reproduced by assembling their source text; the selection examples of §4.10; every `Place`; MOVEM mask reversal; error codes of selection |
| `tests/asm/corpus.rs` | generates the forms corpus: one line per (entry, form, size, mode of each `Ea` operand) with representative operands (`d1 a2 (a3) (a4)+ -(a5) $10(a6) $10(a0,d1.w) ($1234).w ($00012345).l lab(pc) lab(pc,d2.l) #$12`); asserts it assembles without diagnostics (no-opt) and that every encoded instruction decodes back to its form; exported for §10.4 |
| `tests/asm/optimize.rs` | every row of §4.10 on and off, with the forward-reference cases of §12.2 |
| `tests/asm/relax.rs` | the relaxation probes of §12.2 (`r118`–`r126`, `f124`–`f126`, `b1`, next-instruction branch), the CNOP regrowth rule, the sweep cap |
| `tests/asm/directives.rs`, `macros.rs` | every row of §4.5–§4.7; `\0`, `\@` numbering, NARG, `<…>`, MEXIT, depth and statement limits (lowered `AsmLimits`) |
| `tests/asm/output.rs`, `listing.rs` | exact S-record text of §12.1 probes (`t1`, `t2`, `y2`, `y5`), CRLF, empty image, `parse_srec` errors and round trip `parse_srec(to_srec(i)) == i`; binary base and gap fill; listing layout, truncation, cap |
| `tests/asm/fixtures.rs` | §10.2 golden files, `VA_BLESS=1` rewrites |
| `tests/asm/diagnostics.rs` | programmatic cases (16 MiB + 1 include, unreadable, too many files, cancellation mid-run gives `cancelled` and no image); **coverage**: the union of codes over fixtures and this file equals `ASM_DIAG_CODES`; severity and `blocking` per §7.2 |
| `tests/disasm/{decode,format,flow}.rs` | golden decodes per mode and family, `Truncated`/`Illegal`, all 65,536 words: no panic, valid ⇔ some form matches, `tokens` cover `text`; `flow`/`target` per §5.1 |
| `tests/roundtrip.rs` | §10.3 |
| `tests/locales.rs`, `arch.rs` | m1 §6.3 extended to `diag-asm-*` and `isa-m68k-dir-*`; `assembler()`/`disassembler()` are `Some`; `diagnostic_codes()` = lexer codes + `ASM_DIAG_CODES` |
| `apps/va/tests/asm.rs` | exit codes 0/1/2/3; default `.hex` path; `--format bin`; `--listing`, `--map` files; `-o -`; stdin with `-o`; `-I`; the refused paths `\\host\s\x.i`, `//host/s/x.i`, `C:x.i`, `C:\x.i`, `/etc/x`, `..\x.i` escaping, `CON`, `aux.txt`, `a:b` → `asm-include-refused` with their reason ids; JSON schema and key order; `--lang fr` changes only messages; terminal safety of include names and excerpts; outputs untouched on failure; closed stdout → 3 |
| `apps/va/tests/disasm.rs`, `lex.rs` | srec and bin input, illegal words, `input-not-srecord`; the `va lex` cap (1,001 bad lines → 1,000 shown + note, JSON `shown`) |

### 10.2 Golden fixtures (`arch/m68k/fixtures/<area>/`, LF-only)

| Folder | Cases (`<name>.asm` + `<name>.expect`) |
|---|---|
| `asm/` | `basic`, `modes` (every addressing mode and form family), `data` (DC/DS/DCB/EVEN/CNOP, strings, alignment), `symbols` (EQU/SET/`=`, local blocks, case), `macros`, `conditionals` |
| `asm-opt/` | `optimize` (every §4.10 row), `optimize-n` (`; options: no-opt` + `INCLUDE optimize.asm`), `relax`, `relax-n`, `movem` |
| `asm-inc/` | `main` (INCLUDE nested, INCBIN of `blob.txt`, include folder) with `defs.i`, `nested.i`, `sub-*.i`; `cycle` with `self.i` |
| `asm-diag/` | `syntax`, `operands`, `symbols`, `ranges`, `directives`, `macros` (`; limits:` header lowers limits), `strict` (the §4.11 rows, `; a68k: skip`) |
| `disasm/` | `modes.s68`/`modes.dis`, `flow.s68`/`flow.dis`, `illegal.s68`/`illegal.dis` |

Header comments on the first lines of a fixture: `; options: no-opt`, `; limits: key=n,…`,
`; a68k: skip <reason>`. `.expect` format (language-independent; sections in this order):

```
files: 0 main.asm | 1 defs.i
entry: $001000                       (or: none)
segment $001000 code 44
  001000 203C 1234 5678 7205 D081 5242 5643 5184
srec:
  S0030000FC
map:
  001000+6 0:2 11..33
symbols:
  label start $00001000 0:2:1 refs=3
diagnostics:
  0:7:9 error asm-undefined-symbol blocking name="undef1" count=0 suggestions=""
    related 0:3:1 asm-related-first-definition name="dup"
    fix 0:12:5..0:12:6 "0("
listing:
  <the rendered listing lines>
```

S-record lines are stored without CR; `output.rs` asserts the CRLF separately.

### 10.3 Round trip (asm → bin → disasm → asm)

1. **Exhaustive:** for every opword some form matches, append the extension words of a fixed
   pattern (d16 `$0102`, index word `D1.W`/disp 2, imm `.B $0012` `.W $1234` `.L $12345678`,
   abs.W `$1234`, abs.L `$00012345`, branch word `$0102`, MOVEM mask `$0102`, STOP/LINK `$2700`),
   decode at `$1000`, assemble the text (`ORG $1000`, `optimize = false`), compare words; odd
   short-branch targets are accepted with `asm-odd-branch-target`.
2. **Fixtures:** every code-only fixture (`asm/modes`, `asm-opt/*`) with both option sets: its
   image disassembles and reassembles (no-opt) to the same image.

### 10.4 Local a68k comparison (`tests/a68k.rs`, `#[ignore]`, never in CI)

Reads `VA_A68K_PATH` (unset → the test fails with "set VA_A68K_PATH to a68k.exe"; it runs only when
asked, `make compare-a68k`). For every fixture of `asm/`, `asm-opt/`, `asm-inc/` without `; a68k:
skip`, and for the generated corpus of §10.1 (plus a variant with the generic mnemonics and
optimizations), in both option sets: copy the inputs into a new folder under
`CARGO_TARGET_TMPDIR`, run `a68k <file> -s -o<dir>/out.s68 [-n]` there (argument vector, working
directory = that folder, 30 s timeout), and compare: a68k exit 0 ⇒ our image exists and
`parse_srec` of both files gives equal images and entries; a68k exit 10 or 20 ⇒ we have a blocking
error. Every mismatch is printed (fixture, address, both bytes); QA attaches the run to E7.

### 10.5 Benchmark (`tests/asm_bench.rs`, `#[ignore]`, `make bench-asm`)

Generates 5,000 and 100,000 lines by cycling 25 representative lines (labels and local loops,
forward and backward branches, every addressing mode, DC/DS, EQU, one macro call, comments),
assembles them from `MemorySources` with default options and `listing = false`, release build,
1 warm-up then 10 runs each, asserts the statement count, and prints one line per size
`{"schema":"va.bench.asm/1","lines":…,"bytes":…,"statements":…,"image_bytes":…,"runs":10,"median_ms":…,"min_ms":…,"max_ms":…}`.

### 10.6 Acceptance commands (QA, Windows and WSL)

```bash
make test && make lint && make docs && make security && make ci
make build
target/release/va asm arch/m68k/fixtures/asm/basic.asm -o target/m2/basic.hex --listing   # exit 0, .hex and .lst
target/release/va --json asm arch/m68k/fixtures/asm-diag/symbols.asm -o target/m2/s.hex   # exit 1, va.asm/1, no file
target/release/va --lang fr asm arch/m68k/fixtures/asm-diag/operands.asm -o target/m2/o.hex  # exit 1, French
target/release/va asm arch/m68k/fixtures/asm-opt/optimize.asm --no-opt -o target/m2/n.hex --map  # exit 0
target/release/va disasm target/m2/basic.hex                                             # exit 0
target/release/va --json disasm --format bin --org '$1000' target/m2/basic.bin            # after --format bin
make bench-asm                                                    # va.bench.asm/1 lines (Performance)
VA_A68K_PATH=<path to a68k.exe> make compare-a68k                 # Windows, local only (E7)
```

## 11. Performance budget

Measured by `make bench-asm` on the reference machine (on mains, "Best performance"): median
**≤ 20 ms for 5,000 lines** and **≤ 300 ms for 100,000 lines** (about 3.3 MB). Reason: background
assembly runs after the 300 ms debounce (architecture T3); 100,000 lines then show diagnostics in
well under a second, and a 5,000-line student file in about one frame after the debounce. The lexer
alone takes 7.2 ms per 100,000 lines (`docs/perf/2026-10-23-m1-lexer.md`). A miss blocks M2
acceptance (E8), never CI. The Performance agent writes `docs/perf/<date>-m2-assembler.md`.

## 12. a68k observations (2026-10-24)

**How:** `C:\68000\a68k.exe` 2.71.F3w was run read-only, once without argument (help) and then on
about 45 probe sources written to the scratch folder
`C:\Users\sapin\AppData\Local\Temp\claude\C--Users-sapin-Documents-Code-Info-VisualAssembler\e303d630-d549-41dd-b29b-e41604c5a88f\scratchpad\m2-a68k\`,
always with that folder as working directory and `-s -l<file> -o<file>` (and `-n`) pointing into it.
Probe names below are the scratch file names. Nothing was written to `C:\68000`.

### 12.1 Output, errors, structure

- `-s` writes S-records; default name `<stem>.s`, listing `<stem>.lst`, CRLF. Records:
  `S0030000FC`, `S2` with 16 data bytes, a new record at each ORG discontinuity, `S8` with the END
  address or `000000` (t1: `S804001000EB`; t2, y5). Addresses above 24 bits wrap (y2).
- Errors: exit 10 (any number), fatal 20 (missing include, heap overflow); messages on stdout as
  `<file> line <n>`, the source line, a caret line with an English message; no output file is
  written when an error occurs (e3).
- With `-s`, SECTION, CODE, DATA, BSS, XDEF and XREF each give "Not supported in S-format." (s1–s3).
  So do the aliases CSEG, DSEG and PUBLIC (s5, exit 10, probed the same way on 2026-10-24 by the
  integrating architect in the sibling scratch folder `…\scratchpad\m2-arch-check\`).
- INCLUDE resolves against the working directory then `-i` folders, not the includer's folder (i1).
  INCBIN works. A self-including file overflows the heap (i2). Lines after END are ignored (x1).
- Symbols and macro names are case-sensitive; an unknown operation is "No such op-code." (y9).

### 12.2 Encoding and optimization facts

- Defaults (t3, t4, y8): `move.l #5,d0` → `7005`; `add.l #4,a0` → `5888`; `add.w #100,a0` →
  `41E8 0064`; `sub.l #100,a0` → `41E8 FF9C`; `add.w #-3,d0` → `0640 FFFD`; `add.w #0,d0` →
  `0640 0000`; `lea 4(a0),a0` → `5848`; `lea -8(a1),a1` → `5149`; `lea 0(a1),a1` → `43D1`;
  `move.w 0(a0),d0` → `3010`; `move.w $400,d0` → `303A F3E2`; `move.w d0,$400` → `33C0 00000400`;
  `jmp $400` → `4EFA F3B0`; `tst.w $400` → `4A79 00000400`; `move.w ($400).w,d0` → `3038 0400`;
  `move.w $400.w,d0` → error; `move.w (a0,d0),d1` → error; explicit ADDI/ADDA/SUBI/SUBA/MOVEA are
  never rewritten; `move.l #5,a0` → `207C 00000005`; `cmp.w #3,a0` → `B0FC 0003`; `exg a0,d0` →
  `C188`; `movem.w d0,(a0)` → `3080`, `movem.w (a0),d0` → `3010`, `movem.l a2,(a0)` → `208A`.
- `-n` (t1n, t3n, y1n): no rewrite at all; absolute operands are absolute long; branches without
  size are `.W`; one-register MOVEM gives broken words (`4880 0000`, `48C0 0000`).
- Pass-1 knowledge (t5, t6, y6): backward symbols allow the size-changing rewrites, forward ones do
  not (`move.l #F,d0` forward → `203C`, backward → `7003`); `self: jmp self` is absolute long;
  forward `add.w #G,a0` (G = 100) still becomes LEA (same size).
- Relaxation (r118–r126, f124–f126, b1): forward branches shorten up to a short displacement of
  124, backward to −128; two nested forward branches both shorten (r120) but not when the outer one
  needs exactly 126 (r122); a backward branch shortens after a forward one inside it did (b1); a
  branch to the next instruction stays `.W` and `bra.s` to it is "Short displacement can't be zero".
- Data (d1, x1, x2): DC/DS/DCB of `.W`/`.L` and instructions align to even with `00`; `dc.w 'a'` =
  `0061`; `cnop 2,8`; `-7/2` = −3; range leniencies of §4.11; `dc.b 'é'` = `C3 A9`.
- Expressions (e1, e2): the precedence of §4.4; `$ffff+1` in `dc.w` → "Operand size error".
- Macros (m1–m4): `\0` uppercased, `W` by default; `\@` = `.NNN`; NARG; missing argument empty;
  commas split even inside parentheses; `<…>` groups and protects spaces; 11 arguments accepted;
  MEXIT; IFC/IFNC/IFD/IFND/IFEQ/IFNE/IFGT/IFLT nest. Local blocks end at an EQU (y7).

## 13. Fragile points, deferred items, user decisions

### 13.1 Fragile points

| # | Risk | Mitigation |
|---|---|---|
| F1 | a68k's sizing details beyond the probes (e.g. forward `n` in 1…8 for `ADD.W #n,An`, MOVEM on unusual EAs) | The generated corpus comparison (§10.4) in both option sets; a mismatch is fixed in the code toward a68k and ratified in a later § of this spec |
| F2 | `pc-relative` surprises students (`move.w $400,d0` encodes PC-relative) | Listing flag `o` with the rule id; `--no-opt`; M5 hover explains the id |
| F3 | Relaxation edge cases with CNOP/EVEN regrowth | Rule of §4.10, sweep cap, `relax.rs` |
| F4 | Placement table mistakes | Free-bit partition test, exhaustive round trip, a68k corpus |
| F5 | Output amplification (M1-S5) | Limits of §6.1, `va lex` cap |
| F6 | Content-driven file access through INCLUDE | §6.2 before any file-system call; tests with UNC and device paths |
| F7 | `Architecture` implementors outside M2 (M4 fakes) | Provided methods with defaults (§3.1) |
| F8 | Byte vs char columns and CRLF in diagnostics | M1 `LineIndex`; CRLF fixture built in test code |

### 13.2 Deferred

| Item | When |
|---|---|
| Sections, linking, Amiga hunks (Q1 decided (a): refused like `a68k -s`) | non-goal (architecture §1) |
| `EQUR`/`REG` register equates (the lexer is stateless) | later dialect work |
| Quick-fix application in the editor, hover text for optimization ids | M5 |
| A `va-workspace` renderer for `Related` (the CLI renders them with `message`) | M5 |
| Assembler diagnostics in the GUI (background assembly, Problems panel, underline); the GUI's `SourceProvider` over `project.toml` `include_dirs`, which must apply the §6.2 policy: its lexical part then moves out of `apps/va/src/sources.rs` into a crate both apps may use | M5 |
| `va asm` writing through `va_workspace::persist::write_atomic` (M4) instead of its private helper (§8.1) | M5 or M7 cleanup |
| `SyntaxProvider::line_comment` (m4 §13 candidate): not added by WP-A, whose scope is §3 | M5 decides |
| Full path sandbox (root argument, UNC for the CLI argument, device paths) | M7 |
| criterion benches, fuzzing of parser/encoder (M1-S7 class) | M7 |

### 13.3 User decisions (2026-10-24, in chat)

Both questions are closed; the body implements the chosen options and nothing remains open.

- **Q1 — SECTION, CODE, DATA, BSS, XDEF, XREF: user decision (a).** Refused as `a68k -s` refuses
  them, with the blocking error `asm-not-in-s-format` whose `.help` says to place code and data
  with ORG (§4.5, §7.2). The aliases CSEG, DSEG and PUBLIC get the same verdict, as a68k gives
  them (§12.1, s5). Reason: identical verdicts with the school's a68k + d68k workflow, nothing to
  invent for a single-file program, linking is a non-goal (architecture §1). *Rejected:* (b)
  accepting SECTION/CODE/DATA/BSS as consecutive segments, XDEF with no effect and XREF refused,
  each with a warning.
- **Q2 — one-register MOVEM: user decision (a).** With optimizations on, a one-register MOVEM is
  assembled as MOVE/MOVEA, byte-identical to a68k (`movem.l d0,-(sp)` → `move.l d0,-(sp)`,
  `movem.w (a0),d0` → `move.w (a0),d0`), with the warning `asm-movem-single` explaining the flag
  change and, for `.W` memory → Dn, the missing sign extension (§4.10, §7.2). With `--no-opt` the
  MOVEM is kept (§4.11). Reason: programs behave in the debugger exactly as in d68k with a68k's
  output, and the warning teaches the trap. *Rejected:* (b) always encoding MOVEM (textbook
  behaviour, bytes differ from a68k, excluded from the comparison).

## 14. Coordination with M4 (2026-10-24)

M2 ∥ M4 run at the same time. This section is identical in substance to m4 §14; if they ever
differ, the Architect fixes both.

### 14.1 Merged wave order

No two work packages of the same wave edit the same file. "Owns" lists the files a package may
write; everything else is read-only for it.

| Wave | WP | Milestone | Owns | Depends on |
|---|---|---|---|---|
| 1 | WP-A core | M2 | `crates/va-core/src/{lib,arch,asm,disasm}.rs`, `crates/va-core/tests/{asm,disasm}.rs` | — |
| 1 | WP0 scaffold | M4 | `Cargo.toml`, `Cargo.lock`, Makefile (header comment and `bench` help line, m4 §8.2), the manifests of `va-editor`, `va-workspace`, `va-ui`, `visual-assembler`, the stubs of m4 §1, `crates/va-ui/tests/support/mod.rs`, `locales/{en,fr}/*.ftl` (headers), `assets/fonts/*` | user approvals (given 2026-10-24) |
| 1b | WP0 scaffold | M2 | `arch/m68k/src/{lib,arch}.rs`, `arch/m68k/src/isa/mod.rs` (one line), stubs under `arch/m68k/src/{asm,disasm}/` and `isa/fields.rs`, `arch/m68k/tests/{asm,disasm}/main.rs` and their empty module files, `apps/va/src/{cli.rs,commands/mod.rs}`, Makefile (§9 diff) | M2 WP-A (trait), M4 WP0 (Makefile, lock) |
| 1c | Docs checkpoint | both | READMEs of the folders created in waves 1–1b, `AGENCEMENT.md` (annotations) | 1, 1b |
| 2 | WP-B … WP-H | M2 | the §1 scopes | §1 |
| 2 | WP-A … WP-H | M4 | the m4 §1 scopes | m4 §1 |
| 3 | Security, then QA | M2, M4 | `SECURITY.md` (one milestone at a time); QA reports outside the repository | the milestone's wave 2 |
| 4 | Performance | M2, M4 | `docs/perf/<date>-m2-assembler.md`, `docs/perf/<date>-m4-editor.md`, `docs/perf/README.md` (one milestone at a time) | the milestone's QA |
| 5 | WP-I docs | M2, M4 | READMEs, `CLAUDE.md`, `AGENCEMENT.md`, `DEPENDANCE.md`, `IMPORT.md` (one milestone at a time) | the milestone's waves 3–4 |

- Waves 3–5 run per milestone: the first milestone to finish wave 2 may enter wave 3 while the
  other is still in wave 2, because wave-2 packages never write the files of waves 3–5. The same
  phase of the two milestones never runs at the same time: `SECURITY.md`, `docs/perf/README.md`,
  `CLAUDE.md`, `AGENCEMENT.md`, the root and `crates/`/`apps/` READMEs are shared.
- Wave-2 developers write no README and never commit `AGENCEMENT.md`; a local `make docs` rewrite
  of it is discarded. Each work package uses its own worktree and cargo target folder (two
  `cargo xtask ci` runs on one Windows target folder fail, CLAUDE.md). Performance measures with
  no build or test running on the machine.

### 14.2 Contracts between the two milestones

| Topic | Rule |
|---|---|
| `va-core` | Only M2 edits it (WP-A, wave 1). Every trait method it adds is provided (§3.4, `SourceProvider` §3.2), so M4's fakes compile before and after it; M4 relies on no M2 item. |
| `Architecture` in the GUI | M4's registry (m4 §6.7) stores `&'static dyn Architecture` and calls only M1 methods; `assembler()`/`disassembler()` are first called by M5. |
| Fluent | M2 adds `diag-asm-*` and `isa-m68k-dir-*` to `arch/m68k/locales/` (§7.1); M4 adds `ui-*`, `cmd-*`, `diag-cfg-*` in the root `locales/`. M4's `Localizer::for_ui` loads the CPU's resources with its own, so the assembler texts are in the GUI's bundle once both land, but M4 renders no assembler diagnostic: the GUI shows them from M5 (§13.2). |
| `va-workspace` | Only M4 edits it. `apps/va` uses `Lang`, `LangChoice`, `resolve_lang`, `Localizer::{for_architecture, message, render}`, `RenderedDiagnostic`; M4 keeps their signatures and only adds items (new `LangSource` variants are safe: `va` never matches on it). |
| Project files | `va asm` does not read `project.toml`; `include_dirs` (m4 §4.5, relative, no `..`) is for the M5 GUI provider. `va asm -I` folders come from the command line and are used as given (§6.2). |
| Writes | `va asm` and the GUI follow the same atomic-write rule (architecture §6; §8.1). |
| Bench inputs | M2's assembler bench generates its lines in memory (§10.5); M4 owns `bench/generated/` (m4 §7.1). Neither reads the other's inputs. |
| Shared files | Makefile (waves 1 then 1b), the closing documents (waves 3–5, one milestone at a time). M2 never edits `Cargo.toml`, `Cargo.lock`, `xtask` or the layering table. |
