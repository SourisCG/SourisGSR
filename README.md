# SourisGSR — multiplatform fork of GPU Screen Recorder

Fork of [GPU Screen Recorder](https://git.dec05eba.com/gpu-screen-recorder/about/)
(`dec05eba`, GPL-3.0-only) + [ShadowPlay-style overlay UI](https://git.dec05eba.com/gpu-screen-recorder-ui/about/)
for **Linux + Windows** with maximum reuse of the existing C core and an embeddable Rust SDK.

> **Officially supported: Wayland + Windows 11.**
> X11 and Windows 10 are *best-effort*: if something breaks there, it may take
> a long time to fix or be closed as `WONTFIX` outright. See `SUPPORT.md`.

## Monorepo layout

```
core/                 gpu-screen-recorder upstream (CLI + AV pipeline) + SourisGSR changes
  include/souris_gain.h / src/souris_gain.c        0-100% recording-only gain
  include/capture/win_capture.h                    WGC (default) + DXGI (fallback)
  include/souris_win_audio.h                       WASAPI loopback/mic/per-app
  include/encoder/video/souris_win_encode.h        NVENC/AMF/QSV/MF mapping
  src/souris_gsr.c                                 embeddable C ABI (v1)
  docs/WINDOWS_PORT.md                             detailed port plan
overlay/              gpu-screen-recorder-ui (ShadowPlay overlay, backend to be ported to Win)
  include/souris/OverlayBackend.hpp                LinuxEGL vs WinD3D11 abstraction
docs/ref-gtk/         gpu-screen-recorder-gtk (settings reference only)
include/souris_gsr.h  stable C ABI (sole surface for the SDK)
sdk/rust/             souris-gsr-sdk crate + embed_basic/embed_replay examples
```

## What's new in SourisGSR (v1 in this commit)

### 1. OBS-style 0-100 volume modifier — recording only
Never touches the system volume. Software gain applied post-capture.
- CLI: `-mic-volume 0-100 -system-volume 0-100 -app-volume 0-100`
  (defaults 100) + per-input `-a "default_output|vol=80|default_input|vol=90"`.
  ```sh
  gpu-screen-recorder -w portal -f 60 -a default_output -a default_input \
    -mic-volume 90 -system-volume 80 -o video.mp4
  ```
- Overlay UI: sliders still to be wired to these same flags/`vol=` (next step).
- SDK: `.mic_volume(90).system_volume(80)` + hot `set_volumes()`.

### 2. Individual windows
- Linux: already exists (`-w window|focused|portal`). On Wayland the picker is
  the portal (`GNOME >=43 / KDE >=5.27 / Hyprland` do list windows;
  `xdg-desktop-portal-wlr` is monitors-only — a backend limitation, not ours).
- Windows 11: `WGC` with `HWND` (`-w window`, scaffold in `win_capture.h`).
  `DXGI` is monitors-only, kept as fallback.

### 3. Embeddable Rust SDK (full control)
```rust
use souris_gsr_sdk::{Recorder, Source, Codec};
let rec = Recorder::builder()
    .source(Source::portal())
    .fps(60).codec(Codec::Auto)
    .mic_volume(80).system_volume(100)
    .on_event(|e| println!("{e:?}"))
    .start()?;
rec.stop()?;
```
See `sdk/rust/`. C ABI: `include/souris_gsr.h` (`sgsr_start/stop/pause/resume/
save_replay/screenshot/set_volumes`, enumeration, events).

## Build

Linux (core CLI):
```sh
# deps: wayland x11 ffmpeg libva libdrm libpulse libcap vulkan-headers
#       libdbus libpipewire (portal) meson ninja gcc pkg-config
meson setup build && ninja -C build
```

Overlay UI (Linux):
```sh
# upstream submodule (same as upstream, not vendored here):
git clone https://repo.dec05eba.com/mglpp overlay/depends/mglpp
meson setup overlay-build overlay && meson compile -C overlay-build
```

Windows 11 (once the real backend lands): `Meson + MSVC + vcpkg`
(ffmpeg, C++/WinRT for WGC). See `core/docs/WINDOWS_PORT.md`.

Rust SDK (no external dependencies, works offline):
```sh
cargo test -p souris-gsr-sdk
cargo run -p embed_basic
```

## Syncing with upstream (no `clone` inside this folder)
This folder already has `origin`; upstream comes via remotes + `read-tree`:
```sh
git remote -v  # origin + upstream-core/upstream-ui/upstream-gtk
git fetch --all
# re-import example (already done):
git merge -s ours --no-commit --allow-unrelated-histories upstream-core/master
git read-tree --prefix=core/ -u upstream-core/master
git commit
```

## License
GPL-3.0-only (inherited from upstream). Everything new (gain, Win stubs, C ABI,
SDK) is also GPL-3.0-only. Apps linking the SDK must be GPL-3.0.
