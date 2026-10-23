# crates/va-core/src

## Purpose
Sources of `va-core`: one flat module per contract (m1-core-isa-lexer.md §2.1, §3). Each public
signature here is frozen for M1 (§3); M2 and M3 add `asm.rs`, `disasm.rs`, `emu.rs` and `io.rs`.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `lib.rs` | library root | Crate doc (rules: no I/O, no threads, no dependency, no prose) and the module list |
| `source.rs` | module | `SourceText` (UTF-8 check, BOM strip, `MAX_SOURCE_BYTES` = 16 MiB), `MAX_LINE_BYTES` = 64 KiB, `TextRange`, `FileId`, `SourceSpan`, `LineCol` (columns in chars, a tab counts one), `lines` and `LineIndex` (LF, CRLF and lone CR, ropey's `cr_lines` counting) |
| `diag.rs` | module | `DiagCode` (kebab-case, Fluent id `diag-<code>`), `Severity`, `DiagArg`, `Diagnostic` (severity separate from `blocking`), `Related`, `Suggestion` (quick-fix data) |
| `syntax.rs` | module | `TokenKind` (16 kinds, final for M1–M6), `Token`, `LineState`, `KeywordInfo`, the `SyntaxProvider` trait and its contract, `check_line_tokens`, `token_diagnostics` |
| `isa.rs` | module | The `InstructionSet` trait and its view types (`InstructionInfo`, `FormInfo`, `FlagEffect`, `Cycles`, `Bound`, `TimingInfo`, `EaTiming`, …): ids and notation only |
| `regs.rs` | module | `RegId`, `RegisterGroup`, `FlagField`, `RegisterSpec` |
| `image.rs` | module | `ProgramImage` of `Segment`s (overlap and overflow rejected), `SourceMap`; produced by M2, loaded by M3 |
| `arch.rs` | module | The `Architecture` trait: `id`, `syntax`, `instruction_set`, `registers`, `diagnostic_codes`, `locale_resources` |

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
