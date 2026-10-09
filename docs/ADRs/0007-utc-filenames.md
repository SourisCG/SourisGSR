# ADR-0007 — Recording filenames use UTC (`DEVIATION-UTC-TIME`)

Status: accepted (interim).

## Context

The C code formats `Replay_*/Video_*` timestamps with `localtime`; Rust
`std` has no localtime, and no timezone crate was approved.

## Decision

Civil-date math (Hinnant's algorithm) over UTC seconds, injected as a
parameter so filename tests are deterministic. Documented in `replay.rs`
and covered by fixed-timestamp tests.

## Consequences

- Filenames may differ by timezone offset from the C original.
- Revisit with `chrono`/`time` if the user approves the dependency; the
  injection point makes the swap mechanical.
