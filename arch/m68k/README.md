# arch/m68k

## Purpose
Package `va-arch-m68k`: Motorola 68000 support. ISA tables, syntax and lexer, A68k-dialect
assembler, disassembler, emulator adapter over the `m68k` crate, I/O profiles, and its own `.ftl`
resources. It implements the `va-core` traits and is registered by the apps; nothing in `crates/`
may depend on it (rule R2).

**M0 skeleton:** no logic yet; the first content arrives in M1.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `Cargo.toml` | manifest | Package template of m0-foundations.md §3.6; no dependency in M0 |
| `src/` | folder | Crate sources. See its README. |

Projected (m0-foundations.md §2.2): `locales/{en,fr}/` (diagnostics and instruction docs, `.ftl`),
`tests/{isa,syntax,asm,disasm,emu,io,fixtures}/`, `benches/` (criterion).

## Entry points
- `src/lib.rs`.

## Dependencies
- Internal (allowed): `va-core`.
- External: none in M0. Planned (m0-foundations.md §13.3): `m68k` (MIT), M3, without its `jit`
  feature.

The external `a68k.exe` and `d68k.exe` tools are never shipped or redistributed (no licence). The
user configures their paths; nothing hardcodes them. A comparison with `a68k` runs only when
`VA_A68K_PATH` is set (M2), never in CI.

## Tests / usage
```bash
cargo test -p va-arch-m68k
```
In M0 this runs no test.
