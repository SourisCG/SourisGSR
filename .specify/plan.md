# Plan — gsr-rs (frozen reference v5.10.2)

## Phase 0 — Reference freeze (DONE T01)

Cloned `https://codeberg.org/Gnu1/gpu-screen-recorder` to
`reference/gpu-screen-recorder` (v5.10.2, commit `4363f8b`).
Verified `meson.build` src list, `meson_options.txt`, full `src/`, `kms/`,
`include/`, `extra/`, `protocol/`, `plugin/`, `scripts/` trees and
`args_parser.c` flag/enum tables. Result: `reference/MAPA.md` FINAL for v5.10.2.

Key freeze facts: NO `v4l2.c`, NO `wayland_host_bridge.c`, NO `-ipc`,
NO `-auth`/WHIP, NO vulkan-suffixed codecs in v5.10.2. Those are
`DEVIATION-FUTURE` (v2 after upstream re-sync), NOT v1 scope.
X11-only (`xcomposite`, `ximage`, `nvfbc`, `window/x11`, `overclock`,
`xnvctrl`, `screen-direct`, `focused`) is excluded permanently
(`DEVIATION-X11-NEVER`) per user decision.

## Phase 1 — Workspace skeleton (T03, gated by this plan)

`Cargo.toml` workspace + `crates/{core,capture,encode,audio,i18n,kms-server,plugin}`
with `src/` flat + `tests/` + structure-mapping test +
`extra/systemd|udev` + `locales` + CI skeleton. No encoder logic yet.
Gate: `cargo test --workspace` + `clippy -D warnings` + `fmt --check` green.

## Phase 2 — Core (spec-01)

`cli.rs` -> `config.rs` -> `replay*.rs` -> `muxer.rs` -> `screenshot.rs`
with full unit + integration mocks. C-parity `--help`/`ffprobe` harness
(requires C binary side-by-side, else marked ignored).

## Phase 3 — Linux

KMS (`drm` crate + `kms-server` `SCM_RIGHTS` + EGLImage to VAAPI/CUDA,
protocol version 5) then Portal (`zbus` + PipeWire DMA-BUF, `dbus.c`,
`pipewire_video.c`) + Pulse (`sound.cpp`) + PipeWire app-audio
(`pipewire_audio.c`, `-a app:`). Wayland-only.

## Phase 4 — Windows (DEVIATION-WIN-NEW, no C equivalent)

DXGI (`IDXGIOutputDuplication`) then WGC (HWND), shared `ID3D11Device` to
NVENC/AMF/QSV, WASAPI loopback. No `Present()` hooking (anticheat).
Same CLI ideals; X11-only values remain `err_unsupported_x11`.

## Phase 5 — Encode/Audio/i18n hardening

`traits.rs` + `video.rs` + backends (`nvenc`/`vaapi`/`vulkan`/`software`) +
`query_*` + `cuda.rs`/`library.rs` (runtime negotiation), zero-copy test
(fail on CPU `Map`), 1080p144 criterion, 30min soak, security test
(no-root main, minimal helper with `cap_sys_admin+ep`), i18n completeness.

## Phase 6 — Release

`extra/systemd`, `extra/udev`, `docs/` (EN), `--info` parity,
`setcap` install notes, Flatpak FFmpeg baseline (n6.1.1 + ffnvcodec
n11.1.5.3 per user requirement; system FFmpeg otherwise).

## Gates

Each phase ends with `cargo test --workspace` + `clippy -D warnings` +
`fmt --check`. No phase is marked done with red.
