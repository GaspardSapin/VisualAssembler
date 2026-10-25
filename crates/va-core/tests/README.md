# crates/va-core/tests

## Purpose
Tests of the CPU-agnostic contracts of `va-core` (m1-core-isa-lexer.md §3, §9.1; m2-assembler.md
§3 for `asm` and `disasm`), one file per source module (architecture §8). They need no CPU crate. Where a trait must be exercised, they
define a small fake implementation in the test file (rule R1 forbids a dev-dependency on `arch/*`).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `source.rs` | test | `TextRange`; `SourceText` validation (16 MiB limit exactly at and one over, invalid UTF-8 with its offset, BOM strip); `lines` and `LineIndex` on LF, CRLF, CR, mixed, empty and trailing terminators; position/offset round trips with multi-byte chars and tabs |
| `diag.rs` | test | `DiagCode` validation and Fluent ids, `Severity` ids, the `Diagnostic` builder and its blocking rule |
| `syntax.rs` | test | `TokenKind` ids (unique kebab-case), `LineState`, `check_line_tokens` on a valid cover and on each `TokenInvariantError`, `token_diagnostics` (offsets, truncation, blocking flags), the default `line_diagnostics` |
| `isa.rs` | test | Ids of `FlagEffect` and `Bound`, `Cycles` defaults, an `InstructionSet` used as a trait object |
| `regs.rs` | test | Static register descriptions and `RegId` order, read through a fake `Architecture` |
| `image.rs` | test | `ProgramImage` sorting, overlap and overflow rejection, `byte_at`; `SourceMap` lookups at entry edges and the order of `addresses(file, line)` |
| `asm.rs` | test | M2: `CancelToken` sharing across clones and threads, the `PathRefusal` and `SymbolKind` ids, the `AsmLimits`/`AsmOptions` defaults, the provided `SourceProvider` methods, `SymbolTable` order and lookup, `MemorySources` ids and name matching, an `Assembler` reached through a fake `Architecture`, and the default `Architecture::assembler` (`None`) |
| `disasm.rs` | test | M2: `FlowKind` ids, `DecodeError` values, a fake `Disassembler` as a trait object (text covered by its tokens, targets, flow, truncated and illegal input), reached through a fake `Architecture`, and the default `Architecture::disassembler` (`None`) |

There is no `arch.rs` test file: `src/arch.rs` has no logic of its own beyond the two `None`
defaults, which `asm.rs` and `disasm.rs` cover.

## Entry points
- `source.rs` and `syntax.rs`: the contracts the lexer and `va lex` rely on.

## Dependencies
- Internal: `va-core` only.
- External: none.

## Tests / usage
```bash
cargo test -p va-core
cargo test -p va-core --test source
cargo test -p va-core --test asm
cargo test -p va-core --test disasm
```
On 2026-10-25 the `asm` target ran 36 tests and the `disasm` target 11, all passing.
