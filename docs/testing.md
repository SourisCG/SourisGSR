# Testing guide

## Gates (every task)

```sh
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Coverage (`core` + `i18n` only, >= 80%):

```sh
cargo tarpaulin -p gsr-core -p gsr-i18n --fail-under 80
```

## What runs where

- **CI (no hardware):** everything, with mocks. Tests that need the
  outside world adapt: skip without hardware, accept every graceful
  failure mode, never hang. Currently passing: 95+ tests, 0 failed.
- **Manual QA (hardware):** `docs/qa-manual.md` checklist per phase
  (real DRM card + privileges, real Portal session, real encoders).

## Environment knobs

| Variable | Effect |
|----------|--------|
| `KMS_TEST_CARD` | DRM card path for the live helper dance test (default `/dev/dri/card0`); skips gracefully when absent, accepts the permission hint when unprivileged |
| `UPDATE_SNAPSHOTS=1` | Regenerate `--help` snapshots under `crates/core/tests/snapshots/` — only with a Spanish review |
| `FLATPAK_ID` | Switches helper spawn to the host-proxy argv shape; tests must not depend on it being set or unset |
| `LANG` / `--lang` | i18n resolution under test (`resolve_lang` matrix in `catalog.rs`) |

## Test-only fixtures

- `crates/test-plugin`, `crates/test-plugin-nosymbols`: cdylibs built
  into **isolated** `--target-dir`s by the tests that load them (never a
  nested plain `cargo build` — the outer cargo holds the target lock and
  would deadlock).
- Same pattern for the `gsr-kms-server` binary in client tests
  (`ensure_helper`), plus a Python fake helper speaking the wire protocol
  for the deterministic full dance (0.03 s, no DRM/pkexec).

## Conventions for new tests

- Mirror the C file/line in the test module docs.
- Hand-trace index math expectations against the algorithm (several
  initial replay expectations were wrong and fixed against the C loop).
- `#[ignore]` requires a reason naming the un-ignoring task.
- Never `pkill -f` with a pattern matching your own shell (lesson learned
  in T20: the pattern matched the invoking shell and killed it).
