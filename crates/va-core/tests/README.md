# crates/va-core/tests

## Purpose
Tests of the CPU-agnostic contracts of `va-core` (m1-core-isa-lexer.md §3, §9.1), one file per
source module (architecture §8). They need no CPU crate. Where a trait must be exercised, they
define a small fake implementation in the test file (rule R1 forbids a dev-dependency on `arch/*`).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `source.rs` | test | `TextRange`; `SourceText` validation (16 MiB limit exactly at and one over, invalid UTF-8 with its offset, BOM strip); `lines` and `LineIndex` on LF, CRLF, CR, mixed, empty and trailing terminators; position/offset round trips with multi-byte chars and tabs |
| `diag.rs` | test | `DiagCode` validation and Fluent ids, `Severity` ids, the `Diagnostic` builder and its blocking rule |
| `syntax.rs` | test | `TokenKind` ids (unique kebab-case), `LineState`, `check_line_tokens` on a valid cover and on each `TokenInvariantError`, `token_diagnostics` (offsets, truncation, blocking flags), the default `line_diagnostics` |
| `isa.rs` | test | Ids of `FlagEffect` and `Bound`, `Cycles` defaults, an `InstructionSet` used as a trait object |
| `regs.rs` | test | Static register descriptions and `RegId` order, read through a fake `Architecture` (there is no `arch.rs` test: `src/arch.rs` has no logic of its own) |
| `image.rs` | test | `ProgramImage` sorting, overlap and overflow rejection, `byte_at`; `SourceMap` lookups at entry edges and the order of `addresses(file, line)` |

## Entry points
- `source.rs` and `syntax.rs`: the contracts the lexer and `va lex` rely on.

## Dependencies
- Internal: `va-core` only.
- External: none.

## Tests / usage
```bash
cargo test -p va-core
cargo test -p va-core --test source
```
