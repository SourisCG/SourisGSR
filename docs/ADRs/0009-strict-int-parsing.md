# ADR-0009 — Strict integer parsing (`DEVIATION-STRICT-INT`)

Status: accepted.

## Context

C `sscanf` accepts trailing garbage (`-f 60x` parses as 60; same for
`-s` `WxH` and `-region` `WxH+X+Y`).

## Decision

Full-string parses everywhere; trailing garbage is a localized usage
error. Negative `-s`/`-region` dimensions are validated after parsing,
same order as C.

## Consequences

- Stricter than C on malformed input, identical on valid input; every
  branch has a parity test documenting the C-order validation sequence.
