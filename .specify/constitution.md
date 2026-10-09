# Constitution — gsr-rs (GPU Screen Recorder Rust clone)

## 1. Purpose

`gsr-rs` is a Rust clone of GPU Screen Recorder (C, GPL-3.0-only).
Frozen reference: `reference/gpu-screen-recorder` (Gnu1 mirror, v5.10.2).
Goal: functional and performance parity (~0% CPU, zero-copy VRAM to encoder),
NOT literal translation.

Targets: Linux Wayland-only + Windows 10/11.
Modes: record + replay (last N seconds) + monitor + window.
Containers: MP4/MKV. Video: H264/H265 (+AV1/VP8/VP9 as in reference).
Audio: AAC/Opus (FLAC parses but maps to Opus with warning, as in C).
Multi-source composition with `|`, per-source `;k=v` options.

## 2. Spec-Driven Development (STRICT)

- No code without an approved spec in `.specify/`.
- Every spec cites original C files (`src/...`, `kms/...`, `include/...`,
  `extra/...`, `meson.build`, `gpu-screen-recorder.1`).
- Every deviation from the C implementation is documented as `DEVIATION:`
  with rationale + parity test.
- `reference/gpu-screen-recorder` is read-only. Never copy-paste C;
  re-implement in safe Rust with `unsafe` isolated in `crates/encode`
  (ffmpeg-sys-next) and `crates/capture` (drm/EGL) only.

## 3. Language rules

- User communication: SPANISH.
- Specs, ADRs, docs, code, comments, commits: ENGLISH.
- App runtime: ENGLISH + SPANISH, default ENGLISH.

## 4. Architecture rules (MANDATORY mapping)

One crate per domain, `src/` small and flat like C, `tests/` per crate,
`benches/` for perf. NO generic `utils/`/`helpers/` folders.
If GSR splits files, we split modules. If GSR merges, we merge.

Frozen reference v5.10.2 mapping (verified in `reference/MAPA.md`):

- `src/main.cpp` -> `crates/core/src/main.rs`
- `src/args_parser.c` (`include/args_parser.h`) -> `crates/core/src/cli.rs`
- `src/defs.c` (`include/defs.h`) -> `crates/core/src/config.rs`
- `src/replay_buffer/replay_buffer.c` -> `crates/core/src/replay.rs`
- `src/replay_buffer/replay_buffer_ram.c` -> `crates/core/src/replay_ram.rs`
- `src/replay_buffer/replay_buffer_disk.c` -> `crates/core/src/replay_disk.rs`
- `src/encoder/encoder.c` (mux via FFmpeg) -> `crates/core/src/muxer.rs`
- `src/image_writer.c` -> `crates/core/src/screenshot.rs`
- `src/plugins.c` + `plugin/plugin.h` -> `crates/plugin/src/lib.rs`
- `src/library_loader.c` + `src/cuda.c` -> `crates/core/src/library.rs`
  + `crates/encode/src/cuda.rs`
- `src/utils.c` -> `crates/core/src/util.rs` (mirrors C, not a generic dump)
- `src/capture/capture.c` -> `crates/capture/src/traits.rs`
- `src/capture/kms.c` -> `crates/capture/src/kms.rs`
- `kms/client/kms_client.c` + `kms/kms_shared.h` -> `kms_client.rs` + protocol
- `kms/server/kms_server.c` -> `crates/kms-server/src/main.rs`
- `src/capture/portal.c` + `src/dbus.c` + `src/pipewire_video.c`
  -> `portal.rs` + `dbus.rs`
- `src/pipewire_audio.c` -> `crates/audio/src/pipewire_app.rs`
- `src/sound.cpp` (`include/sound.hpp`) -> `crates/audio/src/pulse.rs`
- `src/window/window.c` + `src/window/wayland.c` -> `wayland.rs`
- `protocol/xdg-output-unstable-v1.xml` -> `protocol.rs` (generated bindings)
- `src/egl.c` -> `egl.rs`
- `src/window_texture.c` + `src/shader.c` -> `compose.rs`
- `src/color_conversion.c` -> `color.rs`
- `src/cursor.c` + `src/damage.c` -> `cursor.rs` + `damage.rs`
- `src/encoder/video/video.c` -> `crates/encode/src/video.rs`
- `src/encoder/video/nvenc.c` + `vaapi.c` + `vulkan.c` + `software.c`
  -> `nvenc.rs` + `vaapi.rs` + `vulkan.rs` + `software.rs`
- `src/codec_query/nvenc.c` + `vaapi.c` + `vulkan.c`
  -> `query_nvenc.rs` + `query_vaapi.rs` + `query_vulkan.rs`
- `include/*` -> public Rust traits + docs, no headers.
- `extra/gpu-screen-recorder.service` + `.env` -> `extra/systemd/`
- `extra/gsr-nvidia.conf` + `extra/meson_post_install.sh` -> `extra/udev/`
- NEW bilingual requirement: `crates/i18n/src/` + `locales/en.toml + es.toml`.
- NEW Windows-only (no C equivalent): `dxgi.rs`, `wgc.rs`, `d3d11va.rs`,
  `amf.rs`, `wasapi.rs` (`DEVIATION-WIN-NEW`).

## 5. Zero-copy and performance (NON-NEGOTIABLE)

- VRAM to encoder must stay on GPU: EGLImage to VAAPI/CUDA (Linux),
  shared `ID3D11Device` to NVENC/AMF/QSV (Windows),
  DMA-BUF via PipeWire (Portal), DRM PRIME fd via `kms-server`.
- Any CPU `Map` in the hot path FAILS the `zero-copy` test.
- 1080p144 criterion bench + 30min soak without leaks required.
- NVIDIA quirks mandatory: P2-state note (vulkan alternative),
  `NVreg_PreserveVideoMemoryAllocations=1`, runtime NVENC negotiation
  via `library_loader`/`cuda` equivalents.

## 6. Security

- Main binary NEVER runs as root. Only `gsr-kms-server` is privileged,
  minimal (single file, only DRM + UNIX socket + `SCM_RIGHTS`),
  `cap_sys_admin+ep` via setcap script, installed to `bin/`.
- Protocol version pinned to `GSR_KMS_PROTOCOL_VERSION` from `kms_shared.h`.
- Windows: hooking `Present()` / DLL injection is FORBIDDEN (anticheat).
  Only `IDXGIOutputDuplication` + `Windows.Graphics.Capture` on HWND.

## 7. I18N

- Zero hardcoded user-facing strings. All keys in `crates/i18n`,
  identical key sets in `en.toml`/`es.toml`.
- `--lang en|es|auto` + `LANG` env. `--help`/logs/errors 100% in both.
- Missing key or hardcoded string FAILS CI.

## 8. Quality gates — NO TASK IS DONE WITHOUT GREEN

- `cargo test --workspace` on Linux + Windows, `clippy -D warnings`,
  `cargo fmt --check`, coverage >=80% on `core`+`i18n`.
- Required suites: unit + integration (mocks, no GPU) + C-parity
  (`--help` diff + `ffprobe`) + zero-copy + perf + soak + security + i18n +
  structure-mapping test.
