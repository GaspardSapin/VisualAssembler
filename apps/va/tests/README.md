# apps/va/tests

## Purpose
End-to-end tests of the `va` binary. They spawn the real executable
(`env!("CARGO_BIN_EXE_va")`) and assert exit codes, stdout, stderr and the JSON schema. No test lives
in `src/` (architecture §8).

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `binary.rs` | test | `--version` and `--help` (exit 0, stdout), the OS-independent usage line, usage errors (exit 2, empty stdout, message on stderr), non-Unicode arguments, `va info` human and `--json` output (flag before or after the subcommand), and I/O errors (exit 3) on a closed stdout pipe, on a closed stderr pipe, and on `/dev/full` (Linux only) |

## Entry points
- `run_va` and `va_command`: the helpers every test starts from. They set `NO_COLOR=1` so clap's
  output is stable.

## Dependencies
- Internal: the `va` binary of this package.
- External: `serde_json`, to parse the `va info --json` line.

## Tests / usage
```bash
cargo test -p va --test binary
```
The `/dev/full` test (`unwritable_stdout_is_io_error`) is compiled on Linux only.
