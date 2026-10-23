# apps/va

## Purpose
`va`, the headless VisualAssembler command line (package `va`). It is how a script, CI or an agent
reaches every feature without a window (architecture goal G5). It never depends on a GUI crate
(rule R4), so it builds and runs on headless machines.

**M1 (implemented):** `info`, `lex` and `isa` (m1-core-isa-lexer.md §7), with the global flags
`--json` and `--lang en|fr`. **Not implemented yet:** `asm`, `run`, `debug --script` (M2–M3),
`symbols` and `hover` (M5).

```text
va [--json] [--lang en|fr] info | lex <FILE|-> | isa <MNEMONIC[.SIZE]> | isa --list
```

- `va lex <FILE|->` lexes a 68000 source (`-` = stdin) with the A68k-dialect lexer. Text mode prints
  one line per non-whitespace token on stdout and the diagnostics on stderr, GNU style
  (`<file>:<line>:<column>: error[<code>]: <message>`, then `  help:`, `  note:` and `  fix:`
  lines). JSON mode prints one `va.lex/1` document, whitespace tokens included. Exit 0 without an
  error diagnostic, 1 with one, **blocking or not**. Operand text after a space
  (`dc.b 1, 2, 3`) is the non-blocking `lex-operands-in-comment` error, so it exits 1 with a fix.
- `va isa <MNEMONIC[.SIZE]>` describes an instruction: summary, details, forms, addressing modes,
  flags and cycle counts (`va.isa/1`). `va isa --list` lists the 126 spellings (`va.isa.list/1`).
  An unknown spelling exits 1 with up to three suggestions; a size the instruction rejects exits 1.
- `va info` prints the version, target and registered CPUs (`va.info/1`).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Package `va`, binary only. Dependencies `clap`, `serde`, `serde_json`, `va-core`, `va-workspace`, optional `va-arch-m68k`; feature `arch-m68k` (default) |
| `src/` | folder | Grammar, input, reporting and subcommands. See its README. |
| `tests/` | folder | End-to-end tests of the binary. See its README. |

## Entry points
- `src/main.rs`, then `src/cli.rs` and `src/commands/`.

Conventions for every subcommand (m0-foundations.md §5, m1 §7.5):
- Exit codes: 0 success · 1 the command ran and found problems · 2 usage · 3 I/O or environment
  error · 101 panic. The table is in `src/README.md`.
- Human errors go to stderr, prefixed `error: `; warnings are prefixed `warning: `. CLI messages
  are English, whatever `--lang` says; only diagnostics and documentation are localized.
- `--json` is global. stdout then carries exactly one JSON document, with
  `"schema": "va.<command>/<major>"`. A failure prints `{"schema":"va.error/1","code":"<kebab>",
  "message":"…"}` on stdout plus the human line on stderr, unless stdout itself failed.
- `--lang` takes `en` or `fr` (`--lang auto` is a usage error). Without it, `VA_LANG` (`en`, `fr`,
  `auto`) applies, then `LC_ALL`, `LC_MESSAGES`, `LANG`. An invalid `VA_LANG` prints
  `warning: ignoring VA_LANG=<value> (expected en, fr or auto)` and is ignored. The Windows UI
  language is not read before M4.
- **Text output escapes terminal controls; `--json` output keeps them raw.** Every human line that
  carries outside text (source text, file names, the query, `VA_LANG`, clap's usage errors) passes
  through `report::terminal_safe`, and token texts through `report::terminal_safe_json`. C0 and C1
  controls, DEL, and bidi and line-separator characters become `\u{…}` or `\uXXXX` escapes. A JSON
  document is a faithful machine format: serde_json escapes C0 controls but leaves DEL, C1 and bidi
  characters as they are, so a consumer that prints JSON fields to a terminal must escape them
  itself.

**Input** (`va lex`, m1 §7.2): UTF-8 only (one BOM is stripped), at most 16 MiB per input and 64 KiB
per line (a longer line is one `lex-line-too-long` token). Invalid UTF-8 (`input-not-utf8`, with the
byte offset), a larger input (`input-too-large`), an unreadable path or a directory (`read-failed`)
exit 3. No lossy decoding: a Windows-1252 file must be converted first.

**Only pass local files** (Security M1-S6, open). `va lex` opens the path as given and its reads
have **no timeout**: a FIFO without a writer, `CON` or a terminal device (`/dev/tty`) blocks forever.
On Windows, a UNC path (`\\host\share\f.asm`) makes the OS connect to that host over SMB and can
send the user's NTLM credentials. A script must not pass an untrusted path to `va lex`. The path
sandbox that will reject such paths is planned for M7 and **not implemented**.

## Dependencies
- Internal: `va-core`, `va-workspace` (`i18n`), `va-arch-m68k` (feature `arch-m68k`, default).
- External: `clap`, `serde`, `serde_json`. See `DEPENDANCE.md` and `IMPORT.md`.

## Tests / usage
```bash
cargo test -p va
cargo test -p va --no-default-features
cargo run -p va -- info --json
cargo run -p va -- lex arch/m68k/fixtures/lex/basic.asm
printf ' dc.b 1, 2, 3\n' | cargo run -p va -- lex -
cargo run -p va -- --lang fr isa move.l
cargo run -p va -- isa --list
make build        # release binary: target/release/va (va.exe on Windows)
```
The `--no-default-features` run builds `va` without any CPU: `va lex` and `va isa` then exit 3 with
`no-architecture` (`cargo xtask ci` runs it, m1 §12.7). The `printf` line prints
`-:1:10: error[lex-operands-in-comment]: a68k will ignore “2, 3”: the operands end at the first
space.`, then `  help: Remove the spaces after the commas.` and `  fix:  dc.b 1,2,3`, and exits 1.
