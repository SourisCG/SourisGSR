# Contributing to SourisGSR

Talk to the maintainer in Spanish or English. Everything else (specs, docs, code,
comments, commits) is in English. The app itself is English + Spanish.

## Spec-Driven Development

1. Pick the next unchecked task in `.specify/tasks.md` (strict order).
2. If it has no approved spec in `.specify/`, write one first: cite the
   C files, list every deviation as `DEVIATION:` with rationale + test.
   Big deviations also get an ADR in `docs/ADRs/` (see DOC4 pattern).
3. Implement with mocks only — no GPU, daemon, or network in tests.
4. New user-facing text goes in **both** `crates/i18n/locales/en.toml`
   and `es.toml` in the same commit (tests enforce identical key sets).
5. Freeze `--help` wording changes with `UPDATE_SNAPSHOTS=1` only together
   with a Spanish review.
6. Mark the task done in `tasks.md` and commit in English (one commit per
   task, never push unless asked).

Deviations need user approval first when they touch: new dependencies,
scope, X11 in any form (always rejected), or anything a spec doesn't cover.

## Mandatory gates

```sh
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo tarpaulin -p gsr-core -p gsr-i18n --fail-under 80   # coverage gate
```

A task is done only with all green on Linux; Windows-only code paths are
`#[cfg]`-gated so the workspace compiles on both (see `docs/build.md`).

## Tests without hardware

- Tests must pass with no GPU, no daemons, no root: skip when hardware is
  absent, accept every graceful failure mode, never hang (bounded waits,
  cleanup on all paths, kill + reap spawned helpers).
- Nested `cargo build` inside tests uses an isolated `--target-dir`
  (the outer cargo holds the workspace target lock).
- Env knobs: `KMS_TEST_CARD` (real DRM card or skip),
  `UPDATE_SNAPSHOTS=1` (regenerate `--help` snapshots),
  `FLATPAK_ID` must not leak into tests (spawn paths branch on it).
- `#[ignore]`d tests carry a reason pointing at the task that un-ignores
  them (e.g. the `ffprobe` parity test waits for T40 encoder output).

## New-machine checklist

1. Stable Rust toolchain (`cargo`, `rustfmt`, `clippy` components).
2. System libs — Linux: `libdrm` dev files now; later `pipewire`/`dbus`,
   `pulse`, `ffmpeg` (see `docs/build.md`). Windows: nothing yet.
3. Optional: `cargo-tarpaulin` for the coverage gate.
4. `git clone` + `git submodule`? None — but the C mirror is required:
   `git clone https://codeberg.org/Gnu1/gpu-screen-recorder reference/gpu-screen-recorder`
   (git-ignored; only `reference/MAPA.md` is tracked).
5. Run the four gate commands above; all green before touching code.

## Citing the original

`reference/gpu-screen-recorder` is read-only: never copy-paste C, cite
`file:line` instead, re-implement in safe Rust. `unsafe` lives only in
hardware/FFI boundaries with `SAFETY:` comments. Credits and license terms
are in the README (GPL-3.0-only, like upstream).
