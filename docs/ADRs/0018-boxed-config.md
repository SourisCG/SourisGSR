# ADR-0018 — Boxed `Config` in the `Action` enum

Status: accepted.

## Context

`clippy::large_enum_variant` (`-D warnings`) fired: `Action::Run(Config)`
is 352 bytes next to unit variants. Config grows with every option and
lives as long as the recording.

## Decision

`Action::Run(Box<Config>)`. One pointer-sized variant, heap allocation
once per parse — irrelevant next to a recording session.

## Consequences

- Tests unbox via `*config`; no behavior change.
