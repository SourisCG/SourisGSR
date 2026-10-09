# ADR-0006 — Helper diagnostics stay English (`DEVIATION-HELPER-LOGS`)

Status: accepted.

## Context

The constitution requires app logs/errors 100% in English + Spanish, but
the privileged daemon writes to the journal, has no locale context, and
mirrors C `fprintf` diagnostics.

## Decision

Daemon diagnostics stay English on stderr (like C). Every user-facing CLI
message — including helper failures — surfaces localized through the
client (`err_kms_*` keys; server free text travels only as detail inside
the localized wrapper). The `no_hardcode` lint allowlists daemon strings
under this ADR's name.

## Consequences

- `spec-02 §5` records the deviation; allowlist entries cite it.
- A future journal-aware catalog could revisit this without wire changes.
