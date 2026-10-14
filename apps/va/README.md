# apps/va

## Purpose
`va`, the headless VisualAssembler command line (package `va`). It is how a script, CI or an agent
reaches every feature without a window (architecture goal G5). In M0 it has one subcommand, `info`.
From M1 on: `lex`, `isa`, `asm`, `run`, `debug --script`, `symbols`, `hover`. It never depends on a
GUI crate (rule R4), so it builds and runs on headless machines.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Package `va`, binary only. Dependencies `clap`, `serde`, `serde_json`, all with `workspace = true`. |
| `src/` | folder | Grammar, exit codes and subcommands. See its README. |
| `tests/` | folder | End-to-end tests of the binary. See its README. |

## Entry points
- `src/main.rs`, then `src/cli.rs` and `src/commands/`.

Conventions fixed in M0 for every future subcommand (m0-foundations.md §5):
- Exit codes: 0 success · 1 the operation ran and found problems · 2 usage · 3 I/O or environment
  error · 101 panic. The table is in `src/README.md`.
- Human errors go to stderr, prefixed `error: `.
- `--json` is a global flag. stdout then carries exactly one JSON document (or JSON Lines for
  streaming commands such as `run --trace`). Every document has `"schema": "va.<command>/<major>"`.
  A fatal error in JSON mode prints `{"schema":"va.error/1","code":"<kebab>","message":"…"}` on
  stdout plus a human line on stderr; the first subcommand that needs it implements that printer
  (M0 does not). clap's parse errors stay human-only.
- `--lang en|fr` becomes a global flag in M2.

## Dependencies
- Internal: none in M0 (see `src/README.md` for what is allowed).
- External: `clap`, `serde`, `serde_json`. See `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va
cargo run -p va -- info --json
make build        # release binary: target/release/va (va.exe on Windows)
```
