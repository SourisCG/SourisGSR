# ADR-0016 — Detached script spawn (`DEVIATION-SPAWN`)

Status: accepted.

## Context

C double-forks with `setsid` (+ `SIGHUP` ignore) to run the `-sc` script,
going through `flatpak-spawn --host` inside Flatpak.

## Decision

`Command::spawn` detached (no wait), same `(video_file, kind)` argv with
`kind ∈ {regular, replay, screenshot}`, same Flatpak branch, same
early-false return when the script path doesn't resolve.

## Consequences

- No `setsid` detachment semantics; the child reparents to init on our
  exit, which is adequate for notification scripts. Tested with a marker
  script (path + kind asserted).
