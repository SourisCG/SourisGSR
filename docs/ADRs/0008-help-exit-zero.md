# ADR-0008 — `--help` exits 0 (`DEVIATION-HELP-EXIT`)

Status: accepted.

## Context

The C `args_parser_parse` returns false for `-h`/`--help`, so
`gpu-screen-recorder --help` exits 1.

## Decision

`--help`/`-h` is a successful `Action::Help` (exit 0). Parse errors keep
exit 1 with the message on stderr and short usage on stdout, mirroring C
streams exactly (verified: usage → stdout, error → stderr).

## Consequences

- One-line behavioral difference, snapshot-tested per language.
