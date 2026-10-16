# apps

## Purpose
The shipped binaries. Each app wires library crates together and adds an interface: a window or a
command line. Apps are leaves: no package may depend on them (rule R7). They hold no reusable
logic; it goes to `crates/` or `arch/`.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `visual-assembler/` | folder | GUI binary + library: eframe window, renderer selection with one automatic fallback, bench instrumentation. The only package allowed to use GUI crates. See its README. |
| `va/` | folder | Headless CLI `va` (`info` in M0). Never depends on a GUI crate. See its README. |

## Entry points
- `visual-assembler/src/lib.rs` (`run`) and `va/src/main.rs`.

## Dependencies
- Internal (allowed, none wired in M0): `visual-assembler` may use every `crates/*` and `arch/*`
  package; `va` may use them all except `va-ui` (architecture §2, m0-foundations.md §17.2).
- External: see `DEPENDANCE.md`.

## Tests / usage
```bash
cargo test -p visual-assembler -p va --all-features
make build        # release builds of both binaries
```
