# apps/va/src/commands

## Purpose
One module per `va` subcommand. A subcommand does its work, writes its output on stdout and returns
`Result<Outcome, CommandError>`. Parsing lives in `../cli.rs`; turning an outcome or an error into
an exit code and an `error: ` line lives in `../main.rs` and `../report.rs`. M1 has `info`, `lex`
and `isa`; `asm`, `run`, `debug`, `symbols` and `hover` arrive in M2–M5, one file each.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `mod.rs` | module | `run(&Cli)` dispatcher; `Session::open`, which resolves the language (with the `VA_LANG` warning), picks the default architecture (`no-architecture` otherwise) and builds the `Localizer` (`locale-invalid` otherwise) |
| `info.rs` | module | `va info`: version, target OS and arch, registered CPU architectures; human text and the `va.info/1` line. Reads no language variable. |
| `lex.rs` | module | `va lex <FILE\|->`: lexes every line, renders the diagnostics; text mode (tokens on stdout, GNU-style diagnostics on stderr) or the `va.lex/1` document, streamed, with the diagnostics from a second identical lexing pass |
| `isa.rs` | module | `va isa <MNEMONIC[.SIZE]>` (`va.isa/1`) and `va isa --list` (`va.isa.list/1`); suggestions within edit distance 2 for an unknown spelling |

## Entry points
- `commands::run` in `mod.rs`: the `match` over `Command` that every new subcommand extends.
- `Session::open` for any subcommand that renders localized text.

Outputs (text, then `--json`), run on Windows x64:

```text
$ va info
va 0.1.0
os: windows
arch: x86_64
architectures: m68k
$ va info --json
{"schema":"va.info/1","version":"0.1.0","os":"windows","arch":"x86_64","architectures":["m68k"]}
$ va lex arch/m68k/fixtures/lex/basic.asm        (first lines; exit 0)
1:1 comment "* basic.asm: a small complete program, every common field"
2:1 comment "; a semicolon comment line"
3:1 label "start"
3:7 mnemonic "move"
3:11 size-suffix ".l"
$ va isa MOV                                     (exit 1)
error: unknown mnemonic 'MOV' (did you mean MOVE, MOVEA, MOVEM?)
$ va isa moveq.b                                 (exit 1)
error: MOVEQ does not accept the size suffix '.b' (accepted: .L)
```

`va isa` text layout is for humans, not a contract (m1 §12.7): the first line
`<SPELLING>: <summary>`, the details, then one block per form with its default size, privilege,
operand modes, `flags: X- N* Z* V0 C0` (`-` unaffected, `*` result, `0` cleared, `1` set, `U`
undefined, `!` special) and one `cycles` line per timing row. Cycle values are provisional until
M3 (`arch/m68k/src/isa/README.md`).

## Dependencies
- Internal: `../cli.rs`, `../report.rs`, `../input.rs`, `../arch_registry.rs`; `va-core`,
  `va-workspace` (`i18n`).
- External: `serde` (derive) and `serde_json` (the JSON documents). See `DEPENDANCE.md`.

## Tests / usage
```bash
cargo run -p va -- info
cargo run -p va -- info --json
cargo run -p va -- lex arch/m68k/fixtures/lex/basic.asm
cargo run -p va -- isa MOV
cargo run -p va -- --json isa MOVE
```
The tests live in `apps/va/tests/` (see `apps/va/tests/README.md`).
