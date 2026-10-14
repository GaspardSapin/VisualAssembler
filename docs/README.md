# docs

## Purpose
Project documentation that is longer than a README: normative specifications and measurement
reports. User-facing guides (French and English) arrive in M8 under `user/`.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `specs/` | folder | Architecture and milestone specifications (Architect role). See its README. |
| `perf/` | folder | Measurement reports; every performance claim cites one. See its README. |

## Entry points
- `specs/architecture.md`, then `specs/m0-foundations.md`.
- `perf/M0-backend.md` for the measured renderer numbers. The renderer decision itself is the
  user's: `specs/m0-foundations.md` §19 and `specs/architecture.md` §9.8.

## Dependencies
- Internal: none.
- External: none.

## Tests / usage
Not executable. `make docs` checks the README cascade of this folder like every other one.
