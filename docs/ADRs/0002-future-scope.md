# ADR-0002 — v2 scope for features absent from v5.10.2 (`DEVIATION-FUTURE`)

Status: accepted.

## Context

The frozen mirror is v5.10.2. Newer upstream added V4L2 capture,
`-ipc`, `-auth`/WHIP, `_vulkan` codec aliases, `wayland_host_bridge`,
`--list-monitors`, and the `-ffmpeg-*` option family — none exist here.

## Decision

Those spellings are reserved and rejected with localized
`err_unsupported_future` (see spec-06 §2). They form the v2 backlog (T60)
after an upstream re-sync, never silent scope creep in v1.

## Consequences

- Parsers keep a reserved-name list (tested).
- v1 help text honestly documents the v5.10.2 surface only.
