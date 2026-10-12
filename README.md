# SourisGSR — fork multiplataforma de GPU Screen Recorder

Fork de [GPU Screen Recorder](https://git.dec05eba.com/gpu-screen-recorder/about/)
(`dec05eba`, GPL-3.0-only) + [overlay UI estilo ShadowPlay](https://git.dec05eba.com/gpu-screen-recorder-ui/about/)
para **Linux + Windows** con máximo reuso del core C existente y SDK Rust embebible.

> **Soporte oficial: Wayland + Windows 11.**
> X11 y Windows 10 son *best-effort*: si algo falla ahí, puede tardar mucho
> en arreglarse o quedar como `WONTFIX` directamente. Ver `SUPPORT.md`.

## Estructura del monorepo

```
core/                 gpu-screen-recorder upstream (CLI + pipeline AV) + cambios SourisGSR
  include/souris_gain.h / src/souris_gain.c        gain 0-100% solo-grabación
  include/capture/win_capture.h                    WGC (defecto) + DXGI (fallback)
  include/souris_win_audio.h                       WASAPI loopback/mic/por-app
  include/encoder/video/souris_win_encode.h        NVENC/AMF/QSV/MF mapping
  src/souris_gsr.c                                 C ABI embebible (v1)
  docs/WINDOWS_PORT.md                             plan detallado del port
overlay/              gpu-screen-recorder-ui (overlay ShadowPlay, a portar backend a Win)
  include/souris/OverlayBackend.hpp                abstracción LinuxEGL vs WinD3D11
docs/ref-gtk/         gpu-screen-recorder-gtk (solo referencia de settings)
include/souris_gsr.h  C ABI estable (única superficie para el SDK)
sdk/rust/             crate souris-gsr-sdk + ejemplos embed_basic/embed_replay
```

## Novedades SourisGSR (v1 en este commit)

### 1. Modificador de volumen 0-100 tipo OBS — solo grabación
Nunca toca el volumen del sistema. Gain software post-captura.
- CLI: `-mic-volume 0-100 -system-volume 0-100 -app-volume 0-100`
  (defaults 100) + por-input `-a "default_output|vol=80|default_input|vol=90"`.
  ```sh
  gpu-screen-recorder -w portal -f 60 -a default_output -a default_input \
    -mic-volume 90 -system-volume 80 -o video.mp4
  ```
- UI overlay: pendiente de cablear sliders a estos mismos flags/`vol=` (siguiente paso).
- SDK: `.mic_volume(90).system_volume(80)` + `set_volumes()` en caliente.

### 2. Ventanas individuales
- Linux: ya existe (`-w window|focused|portal`). En Wayland el picker es el
  portal (`GNOME ≥43 / KDE ≥5.27 / Hyprland` sí listan ventanas;
  `xdg-desktop-portal-wlr` solo monitores — limitación del backend, no nuestra).
- Windows 11: `WGC` con `HWND` (`-w window`, scaffold en `win_capture.h`).
  `DXGI` solo monitores, queda como fallback.

### 3. SDK Rust embebible (control total)
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
Ver `sdk/rust/`. C ABI: `include/souris_gsr.h` (`sgsr_start/stop/pause/resume/
save_replay/screenshot/set_volumes`, enumerate, eventos).

## Build

Linux (core CLI):
```sh
# deps: wayland x11 ffmpeg libva libdrm libpulse libcap vulkan-headers
#       libdbus libpipewire (portal) meson ninja gcc pkg-config
meson setup build && ninja -C build
```

Windows 11 (cuando esté el backend real): `Meson + MSVC + vcpkg`
(ffmpeg, C++/WinRT para WGC). Ver `core/docs/WINDOWS_PORT.md`.

SDK Rust (sin dependencias externas, funciona offline):
```sh
cargo test -p souris-gsr-sdk
cargo run -p embed_basic
```

## Sync con upstream (sin `clone` dentro de esta carpeta)
Esta carpeta ya tiene `origin`; upstream se trae por remotos + `read-tree`:
```sh
git remote -v  # origin + upstream-core/upstream-ui/upstream-gtk
git fetch --all
# importar de nuevo (solo ejemplo, ya hecho):
git merge -s ours --no-commit --allow-unrelated-histories upstream-core/master
git read-tree --prefix=core/ -u upstream-core/master
git commit
```

## Licencia
GPL-3.0-only (heredada del upstream). Todo lo nuevo (gain, stubs Win, C ABI,
SDK) también es GPL-3.0-only. Apps que linken el SDK deben ser GPL-3.0.
