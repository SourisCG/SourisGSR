# AGENTS.md — working agreement for AI agents on SourisGSR

This repo is **SourisGSR**: a Rust clone of GPU Screen Recorder (C,
GPL-3.0-only). CLI and crate names keep the `gsr-*` / `gpu-screen-recorder`
spelling for parity; the project itself is SourisGSR. Talk to the user in
**Spanish**. Specs, docs, code, comments and commits are in **English**.
The app runtime is English + Spanish (default English).

## Spec-Driven Development (strict)

- No code without an approved spec in `.specify/`. Specs cite original C
  files under `reference/gpu-screen-recorder` (read-only mirror, v5.10.2).
- Every deviation from the C implementation is a `DEVIATION:` with
  rationale + test. Big ones also get an ADR in `docs/ADRs/`.
- Frozen reference facts live in `reference/MAPA.md`. Newer upstream
  features absent from v5.10.2 are `DEVIATION-FUTURE`, never silent scope
  creep. X11 support is permanently out (`DEVIATION-X11-NEVER`, user decision).

## Mandatory gates (no task is done without green)

```sh
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Coverage gate for `core` + `i18n` (tarpaulin, >= 80%). CI runs the same on
Linux and Windows; platform-specific code is `#[cfg]`-gated so every target
compiles everywhere (Linux-only daemons get non-unix stubs).

## Crate map (one crate per domain, flat `src/` like the C files)

- `core`: CLI/parser (`cli.rs`), validated config, replay ring, muxer
  timing, screenshots, plugin manager, binary entry point.
- `capture`: capturer trait + Linux KMS/Portal/Wayland/EGL/color/cursor/
  damage/dbus + helper client; Windows DXGI/WGC (`cfg(windows)`).
- `encode`: video backends (`nvenc`/`vaapi`/`vulkan`/`software`), codec
  queries, CUDA loader; `d3d11va`/`amf` on Windows.
- `audio`: `pulse`, `pipewire_app`, `wasapi` (Windows).
- `i18n`: `Lang`, `Catalog`, embedded `locales/en.toml` + `es.toml`.
- `kms-server`: privileged helper lib (`protocol`, `transport`, `grab`)
  plus the `gsr-kms-server` binary (Linux-only logic, portable stubs).
- `plugin`: C ABI types (`plugin.h` mirror); `test-plugin*`: test-only
  cdylibs built into isolated target dirs by the tests that load them.

No generic `utils/`/`helpers` folders. `unsafe` stays inside
hardware/FFI boundaries with `SAFETY:` comments.

## i18n rules

- Zero hardcoded user-facing strings outside `crates/i18n`. Same key set
  in `en.toml` and `es.toml` (enforced by tests).
- New keys go in **both** locales at once. The `no_hardcode` lint fails on
  prose literals; genuine diagnostics go in `allowlist-technical.txt` with
  a reason, never silently.
- `--help` snapshots (`crates/core/tests/snapshots/`) freeze wording;
  regenerate with `UPDATE_SNAPSHOTS=1` only alongside a Spanish review.

## Test rules

- Unit + integration tests per task, mocks only (no GPU/daemon in CI).
- Tests needing the outside world adapt: skip without hardware, accept
  every graceful failure mode, never hang (bounded waits, cleanup on all
  paths). Never `pkill -f` with a pattern matching your own shell.
- Nested `cargo build` inside tests must use an isolated `--target-dir`
  (the outer cargo holds the workspace target lock).
- `#[ignore]`d tests need a reason pointing at the task that un-ignores them.

## Commits and approval

- Commits in English, one per finished task, only as the user configured
  (currently: commit per task). Never push unless asked.
- Stop and ask (in Spanish) on: new dependencies, scope changes, new
  deviations, and anything the spec does not cover.
