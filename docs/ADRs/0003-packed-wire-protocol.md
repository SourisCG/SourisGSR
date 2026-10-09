# ADR-0003 — Packed little-endian KMS wire protocol (`DEVIATION-WIRE`)

Status: accepted.

## Context

`kms/kms_shared.h` ships raw C structs over the socket (native layout,
`int`/`bool` ABI coupling, unchecked peer counts).

## Decision

Both ends speak a packed little-endian encoding with identical field
order and limits (fixed 8-byte requests, 1036-byte responses); decoding
is strict and fail-closed (bad version, over-long counts, truncation and
trailing bytes are errors; received fds drop with the message). Protocol
concept version stays 5.

## Consequences

- No interop with the C helper — always fine, we ship both ends.
- Layout is asserted by constants (`ITEM_LEN == 112`) and byte-offset
  roundtrip tests; the field-order bug in the first Python fixture proved
  the tests bite.
