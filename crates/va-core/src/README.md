# crates/va-core/src

## Purpose
Sources of `va-core`: one flat module per contract (m1-core-isa-lexer.md §2.1, §3). Each public
signature here is frozen: the M1 modules since M1 (§3), `asm.rs` and `disasm.rs` since M2 WP-A
(m2-assembler.md §3). M3 adds `emu.rs` and `io.rs`.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate doc (rules: no I/O, no threads, no dependency, no prose) and the module list (`arch`, `asm`, `diag`, `disasm`, `image`, `isa`, `regs`, `source`, `syntax`) |
| `source.rs` | module | `SourceText` (UTF-8 check, BOM strip, `MAX_SOURCE_BYTES` = 16 MiB), `MAX_LINE_BYTES` = 64 KiB, `TextRange`, `FileId`, `SourceSpan`, `LineCol` (columns in chars, a tab counts one), `lines` and `LineIndex` (LF, CRLF and lone CR, ropey's `cr_lines` counting) |
| `diag.rs` | module | `DiagCode` (kebab-case, Fluent id `diag-<code>`), `Severity`, `DiagArg`, `Diagnostic` (severity separate from `blocking`), `Related`, `Suggestion` (quick-fix data) |
| `syntax.rs` | module | `TokenKind` (16 kinds, final for M1–M6), `Token`, `LineState`, `KeywordInfo`, the `SyntaxProvider` trait and its contract, `check_line_tokens`, `token_diagnostics` |
| `isa.rs` | module | The `InstructionSet` trait and its view types (`InstructionInfo`, `FormInfo`, `FlagEffect`, `Cycles`, `Bound`, `TimingInfo`, `EaTiming`, …): ids and notation only |
| `regs.rs` | module | `RegId`, `RegisterGroup`, `FlagField`, `RegisterSpec` |
| `image.rs` | module | `ProgramImage` of `Segment`s (overlap and overflow rejected), `SourceMap`; produced by M2, loaded by M3 |
| `asm.rs` | module | M2 (§3.2): the `Assembler` trait (`Send`), `AssembleInput` and `AssembleOutput` (image iff no blocking diagnostic and not cancelled, diagnostics, `SymbolTable` of `Symbol`s, source map, `Listing` of `ListingLine`s), `AsmOptions` with `AsmLimits` (the m2 §6.1 caps), `CancelToken` (shared flag, polled every 256 statements), the `SourceProvider` trait through which INCLUDE and INCBIN reach files (`IncludeError`, `PathRefusal`: ids only), and `MemorySources`, the in-memory provider of tests and benches |
| `disasm.rs` | module | M2 (§3.3): the `Disassembler` trait (`decode`, `flow_kind`), `DecodedInsn` (address, length, text, tokens, target, flow), `DecodeError` (`Truncated`, `Illegal`), `FlowKind` (sequential, jump, branch, call with its return address, return, trap) |
| `arch.rs` | module | The `Architecture` trait: `id`, `syntax`, `instruction_set`, `registers`, `diagnostic_codes`, `locale_resources`; since M2 the provided `assembler` and `disassembler`, `None` by default so a CPU or a fake without them still compiles |

## Entry points
- `arch.rs`: everything the apps and CPU-agnostic crates know about a CPU goes through a
  `&dyn Architecture`.
- `syntax.rs`: the lexer contract (tokens cover the line exactly, no panic, linear time, no
  allocation besides `out`).

## Dependencies
- Internal: none.
- External: none.

## Tests / usage
```bash
cargo test -p va-core
```
Tests live in `crates/va-core/tests/`, never in `src/` (architecture §8).
