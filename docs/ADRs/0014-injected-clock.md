# ADR-0014 — Injected clock in replay search (`DEVIATION-NOW-PARAM`)

Status: accepted.

## Context

C `find_packet_index_by_time_passed` reads the monotonic clock internally,
which makes the binary search untestable deterministically.

## Decision

`now` is an explicit `f64` parameter; production passes monotonic time
(T20+ wiring). The search loop itself is a line-for-line mirror, verified
by hand-traced index tests (several initial test expectations were wrong
and corrected against the algorithm, not the other way round).

## Consequences

- Deterministic time tests, zero behavior change in production.
