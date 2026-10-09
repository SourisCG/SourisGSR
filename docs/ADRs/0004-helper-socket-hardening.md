# ADR-0004 — Helper socket hardening (`DEVIATION-SEC`, `DEVIATION-SEC-PATH`)

Status: accepted.

## Context

The C helper dance uses a predictable `/tmp` socket with `umask(0000)`,
reads only the first `SCM_RIGHTS` message, never checks peer credentials,
trusts peer counts as loop bounds, leaks fds on error paths, waits
forever, and leaves zombies (`SIGKILL` without `waitpid`).

## Decision

Keep the reversed roles (required by `pkexec`, see ADR-0005) but harden
everything compatible with them: socket `0700` under `$XDG_RUNTIME_DIR`
(`$HOME/.cache`, `/tmp` fallbacks), bounded 10 s accept wait with
liveness checks, `waitpid` reaping, `MSG_CMSG_CLOEXEC`, all control
messages consumed and validated, fail-closed count checks, fd-count
verification per response, per-request receive timeouts.

## Consequences

- New deps `libc` (unix-only) for sockets/credentials/capabilities.
- Security tests: perms, peercred, clamps, fd-leak accounting via
  `/proc/self/fd`, bounded-failure timing.
