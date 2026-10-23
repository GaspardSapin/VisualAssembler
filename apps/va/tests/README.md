# apps/va/tests

## Purpose
End-to-end tests of the `va` binary. They spawn the real executable
(`env!("CARGO_BIN_EXE_va")`) and assert exit codes, stdout, stderr and the JSON schemas. No test lives
in `src/` (architecture §8).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `binary.rs` | test | `--version` and `--help`, the OS-independent usage line, usage errors (exit 2, empty stdout), non-Unicode arguments, usage errors never write terminal controls raw, `va info` human and `--json` (`architectures` = `m68k`, or none without `arch-m68k`), `--lang` is global and `info` ignores it, `lex`/`isa` without architecture exit 3 (only without default features), I/O errors (exit 3) on closed stdout and stderr pipes and on `/dev/full` (Linux only) |
| `lex.rs` | test | `va lex`: exit 0/1/3, text output, `va.lex/1` against every golden fixture, stdin `-`, CRLF and CR, BOM, non-UTF-8 and the 16 MiB limit (exit 3), unreadable paths, `--lang fr` changes only the messages, the `dc.b 1, 2, 3` non-blocking error with its fix, closed pipes, the 64 KiB line limit, column and offset rules, terminal-safe text output and file names, the `VA_LANG` warning |
| `isa.rs` | test | `va isa`: `MOVE`, `move.l`, `beq.s` (separate `.S`/`.W` forms), `DBRA` (canonical `DBF`), `ADD` (`also_accepts`), `--list` (126 spellings), unknown spelling (exit 1, suggestions, `va.error/1`), `moveq.b` (exit 1), `--lang fr`, `VA_LANG` selection and its warning, usage errors, closed stdout pipe, the empty spelling, terminal-safe echoes |

`lex.rs` and `isa.rs` need a CPU, so they compile to nothing without the `arch-m68k` feature.

## Entry points
- `run_va` and `va_command` in `binary.rs`, and the matching helpers of `lex.rs` and `isa.rs`. They set
  `NO_COLOR=1` so clap's output is stable, and control the language variables.

## Dependencies
- Internal: the `va` binary of this package; `arch/m68k/fixtures/lex/` (read by `lex.rs`).
- External: `serde_json`, to parse the JSON documents.

## Tests / usage
```bash
cargo test -p va
cargo test -p va --test lex
cargo test -p va --test isa
cargo test -p va --test binary
cargo test -p va --no-default-features
```
The last command builds `va` without any CPU and runs the `no-architecture` test of `binary.rs`;
`cargo xtask ci` runs it too (step 5b). The `/dev/full` test (`unwritable_stdout_is_io_error`) is
compiled on Linux only.
