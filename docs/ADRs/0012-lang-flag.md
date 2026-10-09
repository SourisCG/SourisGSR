# ADR-0012 — `--lang` flag (`DEVIATION-ADD`)

Status: accepted.

## Context

The C binary has no language selection; SourisGSR is bilingual by
requirement (English default + Spanish, `LANG` environment honored).

## Decision

New `--lang en|es|auto` flag (validated like every enum, default `auto`
→ `LANG` → English). The parser pre-scans it so even usage errors are
localized. Newer-upstream-only spellings stay reserved (ADR-0002).

## Consequences

- One extra row in `--help` snapshots (both languages, frozen).
- `resolve_lang` matrix tests: flag beats env, last occurrence wins,
  unknown codes fall back to English.
