# Tasks — gsr-rs (frozen reference v5.10.2)

Conventions: each task lists spec ref + test command. DONE = green.
`C` = `reference/gpu-screen-recorder`.

## DONE

- [x] T00 Approve constitution + spec-01 + plan + tasks + MAPA + tree (user gate v2).
- [x] T01 Clone reference + verify MAPA vs real `src/|kms/|include/|extra/|meson.build`.
  Verified: v5.10.2, commit `4363f8b`, 38-entry `meson.build` src list, no `v4l2.c`.

## DONE (skeleton + specs materialized, gates green at the time)

- [x] T02 Materialize `.specify/` + `docs/` index + `reference/MAPA.md`.
- [x] T03 Workspace skeleton + structure-mapping test + locales/extra/CI.
- [x] T04 Specs 02..06 with exact C-file citations + DEVIATIONs.

## NEXT (strict order after skeleton is green)

- [x] T13a i18n Catalog API (`Lang`, `resolve_lang`, `Catalog`, `fill`) + 48 help/error
  keys EN/ES + `catalog.rs` tests (enables T10 localized errors).
  Test: `cargo test -p gsr-i18n` (8 passed).
- [x] T10 `core/cli+config` unit tests (valid/invalid, en/es `--help`).
  Test: `cargo test -p gsr-core cli` (21 passed). Exit 1 on parse errors
  (spec-01 corrected: C `_exit(1)`). Deviations: HELP-EXIT, STRICT-INT,
  TYPO, WINDOW-LEN, ADD(--lang), X11-NEVER, FUTURE.
- [x] T11 `core/replay` ring tests (`replay*.rs` vs `replay_buffer/*.c`).
  Test: `cargo test -p gsr-core replay` (18 passed). Deviations: NOW-PARAM,
  NO-KEYFRAME, SNAPSHOT, UTC-TIME, SPAWN.
- [x] T12 `core/muxer+screenshot` timestamp tests + 10s mock integration + `ffprobe` parity.
  Test: `cargo test -p gsr-core mux` (10 passed, 1 ffprobe harness ignored for
  T40). Screenshot via `image` crate (approved dep). Env: FFmpeg 8.1 present.
- [x] T13 i18n keys + fallback + no-hardcode test.
  Test: `cargo test -p gsr-i18n` (10 passed: fallback, catalog, no-hardcode
  lint with allowlist). Prefixes `gsr error/warning/info` + script errors
  moved into locales. Coverage: 90.49% overall, core-src 86.8%,
  i18n-src 89% (tarpaulin, gate >=80%).
- [x] T14 plugins (`-p` load + triangle example vs `plugin/plugin.h`).
  Test: `cargo test -p gsr-core plugin` (6 passed: ABI layout, real `.so`
  load/draw/unload, missing-file/symbol errors, init validation, ES).
  Manager in `core/src/plugins.rs`, C ABI in `crates/plugin/src/abi.rs`,
  fixtures in `crates/test-plugin{,-nosymbols}` (approved libloading dep).
- [x] T20 `kms-server` (socket + `SCM_RIGHTS`, protocol v5, caps) + security test.
  Done: packed-LE v5 types + roundtrips (14 tests), fd transport with
  CLOEXEC + fail-closed validation (7), helper binary (CLI/exit codes,
  reverse-connect, DRM grab via `drm` 0.15), client (spawn modes incl.
  pkexec, REPLACE dance, timeouts, reaping, fd-count check), fake-helper
  full-dance test (0.03 s), fd-leak + perms + peercred + clamp tests,
  no-hardcode allowlist triage. Deviations in spec-02 §5 (WIRE, SEC,
  SEC-PATH, HELPER-LOGS). HW serve path is manual-QA only (no privileges
  in CI); new deps `drm`, `drm-fourcc`, `libc` (unix-only).
- [ ] T21 KMS capturer mock + zero-copy guard (vs `src/capture/kms.c`).
  Test: `cargo test -p gsr-capture kms`.
- [ ] T22 Portal capturer mock incl. `dbus.c` + `pipewire_video.c` (DMA-BUF, no GPU in CI).
  Test: `cargo test -p gsr-capture portal`.
- [ ] T30 DXGI mock + `d3d11va` shared-device test (`DEVIATION-WIN-NEW`).
- [ ] T31 WGC HWND mock, no-hooking audit (`DEVIATION-WIN-NEW`).
- [ ] T40 `nvenc`/`vaapi`/`vulkan`/`software` traits + `query_*` + `cuda`/`library`
  runtime negotiation + P2-quirk note. Test: `cargo test -p gsr-encode`.
- [ ] T41 `pulse`/`pipewire_app`/`wasapi` loopback mocks + multitrack `|` test.
  Test: `cargo test -p gsr-audio`.
- [ ] T50 criterion 1080p144 + soak 30min (manual) + coverage >=80% `core`+`i18n`.
- [ ] T51 CI linux+windows (`test`, `clippy`, `fmt`) + `extra/systemd|udev` install check.

## v2 (DEVIATION-FUTURE, needs upstream re-sync, NOT v1)

- T60 V4L2 camera, vulkan-suffixed codecs, `-ipc` JSON socket, `-auth`/WHIP,
  `--list-monitors`, `wayland_host_bridge` (flatpak bridge).
