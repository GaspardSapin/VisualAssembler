# apps/va/src/commands

## Purpose
One module per `va` subcommand. A subcommand does its work, writes its output on stdout and returns
`Result<(), CommandError>`. Parsing lives in `../cli.rs`; turning an error into an `error: ` line and
an exit code lives in `../main.rs`. M0 has a single subcommand, `info`. The next ones (`lex`, `isa`,
`asm`, `run`, `debug`, `symbols`, `hover`) arrive from M1 on, one file each (architecture §2).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | `run(&Cli)` dispatcher and `CommandError`. Its only variant in M0 is `Stdout`: stdout cannot be written or flushed, exit code 3. |
| `info.rs` | module | `va info`: `InfoReport` (version, target OS, target arch, registered CPU architectures, always empty in M0), its human text and its `va.info/1` JSON line |

## Entry points
- `commands::run` in `mod.rs`: the `match` over `Command` that every new subcommand extends.
- `info::InfoReport::current`, `to_human_text`, `write_json_line`.

Output of `va info` (human, then `--json`):

```text
va 0.1.0
os: windows
arch: x86_64
architectures: (none)
{"schema":"va.info/1","version":"0.1.0","os":"windows","arch":"x86_64","architectures":[]}
```

## Dependencies
- Internal: `../cli.rs` (the `Command` enum), `../main.rs` (`EXIT_IO`).
- External: `serde` (derive) and `serde_json` (the JSON line). See `DEPENDANCE.md`.

## Tests / usage
```bash
cargo run -p va -- info
cargo run -p va -- info --json
```
The tests live in `apps/va/tests/binary.rs` (see `apps/va/tests/README.md`).
