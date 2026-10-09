# ADR-0013 — Owned replay save snapshots (`DEVIATION-SNAPSHOT`)

Status: accepted.

## Context

C clones share packets by refcount, so a save pins buffer memory without
copying while the live ring keeps recording.

## Decision

Save snapshots own their packets (`snapshot_range` deep-copies the slice
being saved). One bounded extra copy per save, far simpler lifetimes,
no `Arc` bookkeeping across the mux boundary.

## Consequences

- Same observable save contents (index math identical, tested).
- Revisit only if profiling shows the copy matters (save already re-reads
  everything for the mux write).
