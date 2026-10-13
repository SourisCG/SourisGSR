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
  include/souris_gain.h / src/souris_gain.c        0-100% recording-only gain + VU peak
  include/capture/win_capture.h                    WGC (default) + DXGI (fallback)
  include/souris_win_audio.h                       WASAPI loopback/mic/per-app
  include/encoder/video/souris_win_encode.h        NVENC/AMF/QSV/MF mapping
  src/souris_gsr.c                                 embeddable C ABI (v2)
  src/cli/ipc.c + tools/gsr-cli/                   set-volumes live + get-level ready
  tests/test_gain.c                                meson test (offline)
  docs/WINDOWS_PORT.md                             detailed port plan
overlay/              ShadowPlay overlay (same options Linux/Win)
  src/gui/SettingsPage.cpp                         globals + per-input vol= rows
  src/Overlay.cpp                                  no silent app-audio drop
docs/book/            English mdBook (build-linux/windows, usage, arch, sdk)
  ref-gtk/            gpu-screen-recorder-gtk (settings reference only)
include/souris_gsr.h  stable C ABI v2 (sole surface for the SDK)
sdk/rust/             souris-gsr-sdk (mock + Engine ffi) + embed_* examples
tools/archive_libsouris.sh  libsouris_gsr.a helper for SDK link
```

Docs: `docs/book/src/` (`mdbook build docs/book`). Man: `core/gpu-screen-recorder.1`, `core/gsr-cli.1`.

## What's new in SourisGSR (v2 in this commit)

### 1. OBS-style 0-100 volume modifier — recording only
Never touches the system volume. Software gain applied post-capture.
- CLI: `-mic-volume 0-100 -system-volume 0-100 -app-volume 0-100`
  (defaults 100) + per-input `-a "default_output|vol=80|default_input|vol=90"`.
  ```sh
  gpu-screen-recorder -w portal -f 60 -a default_output -a default_input \
    -mic-volume 90 -system-volume 80 -o video.mp4
  gsr-cli -ipc "$XDG_RUNTIME_DIR/gsr.sock" set-volumes 80 100 90
  ```
- Overlay UI: globals + per-row `vol` entries emitting `|vol=`, no silent drops.
- SDK: `.mic_volume(90).system_volume(80)` + hot `set_volumes()` + `LevelMeter` VU.

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
// Real engine (ffi): .start_engine()? + .output_url(rtmp://) + .replay() + list_*()
```
See `sdk/rust/` + `docs/book/src/sdk-rust.md`. C ABI v2: `include/souris_gsr.h`
(`sgsr_start/stop/pause/resume/save_replay/screenshot/set_volumes`, enumeration, `LEVEL_METER`).

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

Rust SDK (mock offline, engine with link):
```sh
cargo test -p souris-gsr-sdk --manifest-path sdk/rust/Cargo.toml
./tools/archive_libsouris.sh /tmp/sgsr-build /tmp/sgsr-native
SOURIS_GSR_LIB_DIR=/tmp/sgsr-native cargo test --features ffi -p souris-gsr-sdk --manifest-path sdk/rust/Cargo.toml
cargo run -p embed_list --manifest-path sdk/rust/Cargo.toml
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
