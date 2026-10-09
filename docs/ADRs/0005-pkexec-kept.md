# ADR-0005 — `pkexec` fallback kept

Status: accepted (user decision).

## Context

The C client launches the helper directly as root/with file caps, via a
Flatpak host proxy, or via `pkexec`. Dropping `pkexec` would simplify the
launcher to setcap-only with a clear error.

## Decision

Keep all three paths in the same precedence order. Without an auth agent,
`pkexec` blocks, so the client bounds the wait (10 s), kills and reaps on
every error path (single-exit cleanup — an early version of this code
leaked orphaned helpers, caught during T20 testing).

## Consequences

- `LaunchMode::{Direct, FlatpakProxy, Pkexec}` with pure selection and
  argv-builder functions, all unit-tested.
- The host-side `kms-server-proxy` ships with packaging (T51), same argv
  shape as C.
