# Security model

## Privilege separation

Only `gsr-kms-server` is privileged. Everything else — CLI, capture
orchestration, encoders, muxing, file writing — runs as the invoking
user. The helper does exactly three things: open one DRM card read-only,
answer `GET_KMS` with DMA-BUF fds, and replace its connection on request.

## Helper transport threats and mitigations

| C behavior | SourisGSR |
|------------|-----------|
| Predictable `/tmp` socket, `umask(0000)` | `0700` socket under `$XDG_RUNTIME_DIR` (+`$HOME/.cache`, `/tmp` fallbacks), restrictive umask only around `bind` |
| No peer check | `SO_PEERCRED` allowlist (uid 0 or euid) on accept; the 0700 directory is the real control |
| Peer counts used as loop bounds | Strict fixed-size decoding; over-long counts fail closed |
| Only first `SCM_RIGHTS` message read | Every control message consumed and validated |
| Fds leaked on error paths | `OwnedFd` everywhere; `/proc/self/fd` leak test on rejection |
| Unbounded waits | 10 s accept bound with liveness checks, per-request receive timeouts |
| `SIGKILL` without reap (zombies) | Kill + `waitpid` on every exit path, including `Drop` |

## Launch modes (in precedence order)

1. **Direct** as root or with file capabilities.
2. **Flatpak host proxy** (`flatpak-spawn --host kms-server-proxy …`,
   shipped with packaging).
3. **`pkexec`** (kept by design decision, ADR-0005). Without an auth
   agent it blocks, so the client bounds the wait and reaps.

Install the capability instead of living on `pkexec`:

```sh
sudo setcap cap_sys_admin+ep "$(command -v gsr-kms-server)"  # see extra/udev
```

## Windows

No helper, no privileges: DXGI duplication + `Windows.Graphics.Capture`
only. `Present()` hooking / DLL injection is forbidden (anticheat) and
audited by test. The KMS/portal/Pulse modules don't compile there.

## Known residual risks

- Same-user socket squatting is out of scope (same threat model as C);
  random 10-char suffixes + 0700 dirs mitigate casually.
- Daemon stderr stays English (ADR-0006); journal consumers see codes.
- Real-DRM capture paths execute only on hardware (manual QA per
  `docs/qa-manual.md`); CI covers protocol, transport, client and mocks.
