# apps/va/src

## Purpose
Sources of the `va` binary: the command-line grammar, the exit-code policy and the subcommands.
There is no library target: the CLI is tested end to end by spawning the binary.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `main.rs` | binary root | Parses with `Cli::try_parse`, runs the subcommand, maps every outcome to an exit code. Defines `EXIT_USAGE = 2` and `EXIT_IO = 3`. |
| `cli.rs` | module | clap derive grammar: `Cli` (global `--json`, `bin_name = "va"`) and the `Command` enum (`Info` in M0) |
| `commands/` | folder | One module per subcommand. See its README. |

## Entry points
- `main.rs`: start here; it shows the whole control flow in 70 lines.
- `cli.rs`: add a subcommand to `Command`, then a file in `commands/`.

Exit codes (m0-foundations.md §5, amended by §17.10):

| Code | Meaning |
|------|---------|
| 0 | Success, including `--help` and `--version` (printed on stdout) |
| 1 | Reserved: the operation ran and found problems (for example assembly errors). Not used before M1. |
| 2 | Usage error: missing subcommand, unknown flag, bad value. clap's message goes to stderr. |
| 3 | I/O or environment error. In M0: stdout (or clap's message) cannot be written. stderr gets `error: cannot write to stdout: …`, and no `va.error/1` document is printed. |
| 101 | Panic. Always a bug. |

The usage line is `Usage: va [OPTIONS] <COMMAND>` on every OS, not `va.exe` (§17.11 D13).

## Dependencies
- Internal: none in M0. Allowed later (rule R4 and §17.2): `va-core`, `va-editor`, `va-lang`,
  `va-debug`, `va-workspace` and `va-arch-*`. Never `va-ui` or any GUI crate.
- External: `clap`, `serde`, `serde_json`. See `DEPENDANCE.md`.

## Tests / usage
```bash
cargo run -p va -- --help
cargo test -p va
```
