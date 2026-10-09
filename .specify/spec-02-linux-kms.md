# Spec-02 — Linux KMS capture + privileged helper

Refs (frozen reference v5.10.2):
`src/capture/capture.c`, `include/capture/capture.h`,
`src/capture/kms.c`, `include/capture/kms.h`,
`kms/client/kms_client.c`, `kms/client/kms_client.h`,
`kms/kms_shared.h`, `kms/server/kms_server.c`,
`src/egl.c`, `include/egl.h`, `src/window_texture.c`,
`src/shader.c`, `src/color_conversion.c`, `src/cursor.c`, `src/damage.c`,
`meson.build` (`gsr-kms-server` target), `extra/meson_post_install.sh`,
`gsr-kms-server.1`.

## 1. Scope

Monitor capture on Wayland via KMS/DRM (`-w screen|<monitor>`, `-w region`
on the KMS path). `screen-direct`, `focused`, X11 window ids are
`DEVIATION-X11-NEVER` rejections (see spec-01).

## 2. Helper protocol (MUST)

- Binary `gsr-kms-server <domain_socket_path> <card_path>`
  (e.g. `gsr-kms-server /tmp/gsr-kms.sock /dev/dri/card0`), per `gsr-kms-server.1`.
- Single-file helper: only DRM + UNIX socket + `SCM_RIGHTS` fd passing.
  Installed with `cap_sys_admin+ep` (`extra/udev/setcap-kms-server.sh`,
  mirrors `extra/meson_post_install.sh`). Main process keeps zero privileges.
- Protocol pinned to `kms/kms_shared.h`: `GSR_KMS_PROTOCOL_VERSION 5`,
  `GSR_KMS_MAX_ITEMS 8`, `GSR_KMS_MAX_DMA_BUFS 4`, requests
  `REPLACE_CONNECTION`/`GET_KMS`, per-item DMA-BUF `fd/pitch/offset`,
  `width/height/pixel_format/modifier/connector_id/is_cursor/rotation`
  (`KMS_ROT_0/90/180/270`), `x/y/src_w/src_h`, `has_hdr_metadata` +
  `hdr_output_metadata`.
- Client side (`kms_client.rs`, mirrors `kms/client/kms_client.c`) connects,
  replaces connection fd, receives the fixed-size response + ancillary fds.
- Modules: `crates/kms-server/src/main.rs`, `protocol.rs`;
  `crates/capture/src/kms_client.rs`, `protocol.rs`.

## 3. Capture path (MUST)

`src/capture/kms.c` behavior: connector/monitor lookup (KMS plane via
`drmModeGetFB2`, PRIME `drmPrimeHandleToFD` in helper), region crop
(`region_size/region_position` from `-region`), cursor composition
(`cursor.c`, `-cursor`), damage gating (`damage.c`, `-fm content`, `-v`
reporting), EGLImage import (`egl.c`) into the encoder without CPU `Map`.

- Modules: `kms.rs` (orchestration), `kms_client.rs` (fd transport),
  `egl.rs`, `compose.rs` (`window_texture`+`shader`), `color.rs`,
  `cursor.rs`, `damage.rs`, `wayland.rs` (output sizing via `protocol.rs`
  bindings of `protocol/xdg-output-unstable-v1.xml`).
- HDR items (`has_hdr_metadata`) map to `hevc_hdr`/`av1_hdr` encoder paths;
  rotation `90/270` swaps region width/height.

## 4. Tests (MUST)

- `socket_scm.rs`: helper round-trip passes a memfd/DMA-BUF stand-in via
  `SCM_RIGHTS` with protocol version assertion.
- `kms.rs` mock: fake two-connector response, region crop math, rotation swap.
- `zero_copy_guard.rs`: fail if the KMS path calls any CPU `Map`.
- Security test: main binary has no ambient caps; helper source stays under
  the minimal-surface budget (single-purpose file, no CLI parsing beyond
  two argv entries).
