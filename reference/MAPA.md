# MAPA — GSR C -> gsr-rs (FINAL for frozen reference v5.10.2)

> Status: FINAL against `reference/gpu-screen-recorder` (Gnu1 mirror,
> `meson.build` version `5.10.2`, commit `4363f8b`, verified 2026-10-09).
> Newer upstream features seen in other mirrors (V4L2 capture,
> vulkan-suffixed codecs, `-ipc`, `-auth`/WHIP, `wayland_host_bridge`,
> `--list-monitors`) do NOT exist here and are `DEVIATION-FUTURE` (v2).

## 1. Top-level (verified)

`external/ extra/ include/ install.sh kms/ LICENSE meson.build
meson_options.txt plugin/ project.conf protocol/ README.md scripts/
src/ study/ TODO uninstall.sh`
(`gpu-screen-recorder.1`, `gsr-kms-server.1` are installed man pages.)

`meson_options.txt`: `systemd`, `capabilities` (setcap helper),
`nvidia_suspend_fix`, `portal` (`-DGSR_PORTAL`), `app_audio`
(`-DGSR_APP_AUDIO`), `plugin_examples`.

## 2. Build units (`meson.build` src list, 38 entries)

```text
kms/client/kms_client.c
src/capture/capture.c, nvfbc.c, xcomposite.c, ximage.c, kms.c
src/encoder/encoder.c
src/encoder/video/video.c, nvenc.c, vaapi.c, vulkan.c, software.c
src/codec_query/nvenc.c, vaapi.c, vulkan.c
src/window/window.c, x11.c, wayland.c
src/replay_buffer/replay_buffer.c, replay_buffer_ram.c, replay_buffer_disk.c
src/egl.c, cuda.c, xnvctrl.c, overclock.c, window_texture.c, shader.c,
color_conversion.c, utils.c, library_loader.c, cursor.c, damage.c,
image_writer.c, args_parser.c, defs.c, plugins.c, sound.cpp, main.cpp
+ conditional: src/capture/portal.c, src/dbus.c, src/pipewire_video.c (portal=true)
+ conditional: src/pipewire_audio.c (app_audio=true)
+ protocol/: xdg-output-unstable-v1.xml (wayland-scanner code+client-header)
binaries: gsr-kms-server (kms/server/kms_server.c + libdrm, -fstack-protector-all),
gpu-screen-recorder (all above)
```

System deps: `threads m libavcodec libavformat libavutil libswresample
libavfilter x11 xcomposite xrandr xfixes xdamage libpulse libva libva-drm
libdrm wayland-egl wayland-client [libcap on linux]
[libpipewire-0.3 libspa-0.2 dbus-1 when portal/app_audio]`.
Runtime: `libglvnd` (GLES 3.0+), vendor stacks (mesa/vaapi, cuda/nvenc),
`libXNVCtrl.so.0` (only `-oc` path).

## 3. Canonical mapping (C -> Rust)

| C (v5.10.2) | Rust | Notes |
|---|---|---|
| `src/main.cpp` | `crates/core/src/main.rs` | CLI dispatch, signals, `--info/--list-*`, screenshot path |
| `src/args_parser.c`, `include/args_parser.h` | `crates/core/src/cli.rs` | 31 flags, enums in §4, yes/no bools, exit-2 errors |
| `src/defs.c`, `include/defs.h` | `crates/core/src/config.rs` | Validated `Config`, `-q/-s/-region/-o` rules |
| `src/replay_buffer/replay_buffer.c` | `crates/core/src/replay.rs` | Facade |
| `src/replay_buffer/replay_buffer_ram.c` | `crates/core/src/replay_ram.rs` | RAM packet ring |
| `src/replay_buffer/replay_buffer_disk.c` | `crates/core/src/replay_disk.rs` | Disk ring |
| `src/encoder/encoder.c` (+FFmpeg mux) | `crates/core/src/muxer.rs` + `crates/encode/src/traits.rs` | MP4/MKV, ts, `-keyint/-pixfmt/-cr` |
| `src/image_writer.c` | `crates/core/src/screenshot.rs` | JPEG/PNG single frame |
| `src/plugins.c`, `plugin/plugin.h`, `plugin/examples/hello_triangle/triangle.c` | `crates/plugin/` | `-p` repeatable overlay ABI |
| `src/library_loader.c`, `src/cuda.c` | `crates/core/src/library.rs`, `crates/encode/src/cuda.rs` | `dlopen` negotiation |
| `src/utils.c` | `crates/core/src/util.rs` | 1:1 mirror, keep small |
| `src/capture/capture.c`, `include/capture/capture.h` | `crates/capture/src/traits.rs` | Capturer trait + per-source `;k=v` + `\|` |
| `src/capture/kms.c`, `include/capture/kms.h` | `crates/capture/src/kms.rs` | Monitor/region/cursor/composition |
| `kms/client/kms_client.c/.h`, `kms/kms_shared.h` | `crates/capture/src/kms_client.rs` + `protocol.rs` | Protocol v5, `items[8]`, `dma_buf[4]`, rotation, HDR |
| `kms/server/kms_server.c` | `crates/kms-server/src/main.rs` | `gsr-kms-server <socket> <card>`, `SCM_RIGHTS` |
| `src/capture/portal.c`, `include/capture/portal.h`, `src/dbus.c`, `src/pipewire_video.c` | `crates/capture/src/portal.rs`, `dbus.rs` | ScreenCast + DMA-BUF + crop/damage |
| `src/pipewire_audio.c`, `include/pipewire_audio.h` | `crates/audio/src/pipewire_app.rs` | `-a app:` / `app-inverse:` |
| `src/sound.cpp`, `include/sound.hpp` | `crates/audio/src/pulse.rs` | Pulse devices, `default_output/input`, `\|` |
| `src/window/window.c`, `window/wayland.c` | `crates/capture/src/wayland.rs` | Wayland window/region path |
| `protocol/xdg-output-unstable-v1.xml` | `crates/capture/src/protocol.rs` | Generated bindings |
| `src/egl.c`, `include/egl.h` | `crates/capture/src/egl.rs` | EGLImage context |
| `src/window_texture.c`, `src/shader.c` | `crates/capture/src/compose.rs` | GL composition |
| `src/color_conversion.c` | `crates/capture/src/color.rs` | RGB->NV12/YUV444 shaders |
| `src/cursor.c`, `src/damage.c` | `cursor.rs`, `damage.rs` | `-cursor`, `-fm content`, `-v` |
| `src/encoder/video/video.c` | `crates/encode/src/video.rs` | Video pipeline facade |
| `src/encoder/video/nvenc.c` | `nvenc.rs` | NVIDIA path |
| `src/encoder/video/vaapi.c` | `vaapi.rs` | AMD/Intel path |
| `src/encoder/video/vulkan.c` | `vulkan.rs` | Vulkan encode path (base, no suffixed aliases in v5.10.2) |
| `src/encoder/video/software.c` | `software.rs` | CPU H264 fallback |
| `src/codec_query/nvenc.c` | `query_nvenc.rs` | Capability query |
| `src/codec_query/vaapi.c` | `query_vaapi.rs` | Capability query |
| `src/codec_query/vulkan.c` | `query_vulkan.rs` | Capability query |
| `extra/gpu-screen-recorder.service`, `.env` | `extra/systemd/` | User unit, `KillSignal=SIGINT`, replay env |
| `extra/gsr-nvidia.conf` (`NVreg_PreserveVideoMemoryAllocations=1`), `extra/meson_post_install.sh` (`setcap cap_sys_admin+ep`) | `extra/udev/` + docs | Suspend fix + helper caps |
| `scripts/start-replay.sh save-replay.sh stop-replay.sh start-recording.sh twitch-stream.sh ...` | `docs/` examples | Not ported as code |
| `include/encoder/*`, `include/window/*`, `include/replay_buffer/*`, `include/capture/*` | Public traits + docs | No headers in Rust |

## 4. Exclusions and deviations

- `DEVIATION-X11-NEVER` (permanent, user decision): `src/capture/xcomposite.c`,
  `src/capture/ximage.c`, `src/capture/nvfbc.c`, `src/window/x11.c`,
  `src/overclock.c`, `src/xnvctrl.c`, `-w screen-direct/-force`, `-w focused`,
  raw X11 window ids, `-oc` effect. `-oc` still parses but warns
  `warn_oc_requires_x11`. `damage.c` X11 damage backend is replaced by
  KMS/portal damage metadata.
- `DEVIATION-WIN-NEW` (no C equivalent, required): `dxgi.rs`, `wgc.rs`,
  `d3d11va.rs`, `amf.rs`, `wasapi.rs`; signals mapped to Ctrl-C + named pipe;
  `Present()` hooking forbidden.
- `DEVIATION-FUTURE` (v2, absent in v5.10.2): V4L2 camera, `_vulkan`-suffixed
  `-k` aliases, `-ipc` JSON socket, `-auth`/WHIP, `-ffmpeg-opts` family,
  `-low-power`, `-exclude-metadata`, `-write-first-frame-ts`,
  `--list-monitors`/`--list-v4l2-devices`, `wayland_host_bridge`.
  Parsers must reserve these names and return `err_unsupported_future`,
  never silently accept them.

## 5. CLI/enum freeze (v5.10.2, `src/args_parser.c`)

Flags: `-w -c -f -s -region -a -q -o -ro -r -restart-replay-on-save -k -ac
-ab -oc -fm -bm -pixfmt -v -gl-debug -df -sc -cr -tune -cursor -keyint
-restore-portal-session -portal-session-token-filepath -encoder
-fallback-cpu-encoding -replay-storage -p` +
`--info --list-capture-options [--list-capture-options card]
--list-audio-devices --list-application-audio --version -h/--help`.

`-k`: `auto h264 h265 hevc hevc_hdr hevc_10bit av1 av1_hdr av1_10bit vp8 vp9`.
`-ac`: `opus aac flac` (flac warns + falls back to opus).
`-pixfmt`: `yuv420 yuv444`. `-encoder`: `gpu cpu`.
`-w`: `screen | screen-direct[*] | focused[*] | portal | region | <monitor>`
([*] = X11-only, rejected).

## 6. KMS protocol freeze (`kms/kms_shared.h`)

`GSR_KMS_PROTOCOL_VERSION 5`, `MAX_ITEMS 8`, `MAX_DMA_BUFS 4`,
requests `REPLACE_CONNECTION`/`GET_KMS`, results incl.
`FAILED_TO_GET_PLANE(S)`/`FAILED_TO_SEND`, per-item `fd/pitch/offset`,
`width/height/format/modifier/connector_id/is_cursor/rotation/x/y/src_w/src_h`
+ `hdr_output_metadata`. Helper CLI: `gsr-kms-server <socket> <card>`.
