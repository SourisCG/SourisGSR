# ADR-0010 — Fixed C message typo (`DEVIATION-TYPO`)

Status: accepted.

## Context

`args_parser.c` prints `is not an floating-point number` for `-keyint`.

## Decision

We print grammatically correct `is not a floating-point number` (and the
Spanish equivalent). Cosmetic only; the key (`err_double_invalid`) and
the branch are identical.

## Consequences

- Byte-level `--help`/error diffs against C must normalize this line
  (noted in `docs/parity.md`).
