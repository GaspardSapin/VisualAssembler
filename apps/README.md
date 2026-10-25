# apps

## Purpose
The shipped binaries. Each app wires library crates together and adds an interface: a window or a
command line. Apps are leaves: no package may depend on them (rule R7). They hold no reusable
logic; it goes to `crates/` or `arch/`.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `visual-assembler/` | folder | GUI binary + library: eframe window, renderer selection with one automatic fallback, bench instrumentation. The only package allowed to use GUI crates. Unchanged in M1; M4 WP0 declared its new dependencies, and its code changes in wave 2 (WP-G). See its README. |
| `va/` | folder | Headless CLI `va`: `info`, `lex` and `isa` in M1, with a global `--json` and `--lang en\|fr`; `asm` and `disasm` are M2 shells that exit 1 until WP-G. Never depends on a GUI crate. See its README. |

## Entry points
- `visual-assembler/src/lib.rs` (`run`) and `va/src/main.rs`.

## Dependencies
- Internal (allowed): `visual-assembler` may use every `crates/*` and `arch/*` package; since M4
  WP0 its manifest declares `va-core`, `va-editor`, `va-ui`, `va-workspace` and, through its
  default feature `arch-m68k`, `va-arch-m68k`, which its code uses from WP-G. `va` may use them all
  except `va-ui` (architecture §2, m0-foundations.md §17.2); it uses `va-core`, `va-workspace` and,
  through its default feature `arch-m68k`, `va-arch-m68k`.
- External: see `DEPENDANCE.md`.

## Tests / usage
```bash
cargo test -p visual-assembler -p va --all-features
make build        # release builds of both binaries
```
