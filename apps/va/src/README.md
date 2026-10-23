# apps/va/src

## Purpose
Sources of the `va` binary: the command-line grammar, the CPU registry, input reading, failure
reporting and exit codes, and the subcommands. There is no library target: the CLI is tested end
to end by spawning the binary.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `main.rs` | binary root | Parses with `Cli::try_parse`, runs the subcommand, maps every outcome to an exit code. clap's usage errors go to stderr through `report::terminal_safe`. |
| `cli.rs` | module | clap derive grammar: `Cli` (global `--json` and `--lang en\|fr`, `bin_name = "va"`) and the `Command` enum (`Info`, `Lex`, `Isa`) |
| `arch_registry.rs` | module | `architectures()` and `default_architecture()`: `va_arch_m68k::M68K` when the feature `arch-m68k` is on (default), none otherwise |
| `input.rs` | module | Reads the `va lex` input (file or stdin `-`) through `Read::take(16 MiB + 1)`, then `SourceText::from_bytes`; errors `read-failed`, `input-too-large`, `input-not-utf8` |
| `report.rs` | module | Exit-code constants, `Outcome`, `ErrorCode`, `CommandError`, `report_failure` (the `error:` line and the `va.error/1` document), `english_io`, `terminal_safe`, `terminal_safe_json` |
| `commands/` | folder | One module per subcommand, and the shared `Session` (language, CPU, localizer). See its README. |

## Entry points
- `main.rs`: start here; it shows the whole control flow.
- `cli.rs`: add a subcommand to `Command`, then a file in `commands/`.
- `report.rs`: every new failure gets an `ErrorCode`; every new human line that carries outside
  text goes through `terminal_safe` (or `terminal_safe_json` for a JSON string on a text line).

Exit codes (m0-foundations.md §5, amended by §17.10; m1-core-isa-lexer.md §7.5):

| Code | Meaning |
|------|---------|
| 0 | Success, including `--help` and `--version` (printed on stdout); `va lex` without error diagnostic |
| 1 | The command ran and found problems: `va lex` found an error diagnostic, blocking or not (the result document is printed, not `va.error/1`); `isa-unknown-mnemonic`, `isa-size-not-accepted` |
| 2 | Usage error: missing subcommand, unknown flag, bad value (`--lang auto` included), `isa` with both or neither of `MNEMONIC` and `--list`. clap's message goes to stderr. |
| 3 | I/O or environment error: `read-failed`, `input-too-large`, `input-not-utf8`, `no-architecture` (built without `arch-*`), `locale-invalid` (embedded resources fail to load: a build defect); stdout cannot be written (no `va.error/1` document then); stderr cannot be written while `va lex` prints diagnostics in text mode |
| 101 | Panic. Always a bug. |

I/O errors are printed in English as `<kind> (os error <n>)`, e.g.
`error: cannot write to stdout: broken pipe (os error 232)` on Windows, because Windows localizes OS
messages (m1 §12.8 F-1). The usage line is `Usage: va [OPTIONS] <COMMAND>` on every OS, not `va.exe`
(m0 §17.11 D13).

## Dependencies
- Internal: `va-core` (`source`, `syntax`, `diag`, `isa`, `arch`), `va-workspace` (`i18n`),
  `va-arch-m68k` (feature `arch-m68k`). Never `va-ui` or any GUI crate (rule R4).
- External: `clap`, `serde`, `serde_json`. See `DEPENDANCE.md`.

## Tests / usage
```bash
cargo run -p va -- --help
cargo test -p va
```
