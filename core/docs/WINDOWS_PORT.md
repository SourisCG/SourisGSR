# Windows port (SourisGSR) — plan concreto, sin reescribir el core

Objetivo: mismo CLI/core en Windows 11, captura ventana individual.

## Captura

- Defecto `WGC` (`Windows.Graphics.Capture`):
  - Ventana: `GraphicsCapturePicker` / `GraphicsCaptureItem::TryCreateFromWindowHandle(HWND)`.
  - Monitor: `TryCreateFromDisplayId`.
  - Frames en D3D11 (`ID3D11Texture2D`) vía `Direct3D11CaptureFramePool`.
  - Limitación conocida: el cursor pasa a software-rendered en la sesión WGC.
- Fallback `DXGI` (`IDXGIOutputDuplication`):
  - Solo monitores. Requiere correr en la misma GPU que el display
    (a diferencia de WGC que es cross-GPU).
  - Usar cuando WGC no disponible o para diagnóstico de cursor HW.
- Integración al `gsr_capture` vtable existente (`capture/capture.h`):
  `wgc.c` implementa `start/capture/destroy` convirtiendo D3D11 -> EGL/CUDA
  o alimentando FFmpeg `d3d11va` directo (cero copias CPU).

## Encode

FFmpeg con `hwupload` D3D11. Orden de prueba:
1. `h264_nvenc/hevc_nvenc/av1_nvenc` (NVIDIA)
2. `h264_amf/hevc_amf` (AMD)
3. `h264_qsv/hevc_qsv/av1_qsv` (Intel, deriva contexto del mismo device D3D11)
4. `h264_mf` (MediaFoundation, fallback SDR/NV12, sin HDR)

Nunca cambiar de encoder en silencio: si el pedido no existe, fallar con log
claro (como hace `capture-engine`).

## Audio (WASAPI)

- Sistema: `IAudioClient` loopback. Mic: `eCapture`. Por-app: process-loopback
  (Win10 20348+, ok en Win11).
- Gain 0-100% idéntico a Linux: `souris_gain.h` post-captura. Prohibido
  `IAudioEndpointVolume` (no tocar volumen del sistema).

## Overlay (reuso, no de 0)

`overlay/` ya es estilo ShadowPlay (hotkeys, replay, selección). Solo se
abstrae el backend:
- `OverlayLinux (EGL/X11/Wayland/Pango, existente, congelar)` vs
  `OverlayWin (Win32 layered + D3D11 + DirectWrite/Direct2D, nuevo, misma
  interfaz DrawText/DrawRect/Present)`.
- `GlobalHotkeysLinux (evdev/X11)` vs `GlobalHotkeysWin (RegisterHotKey)`.
- `FileChooser` -> `IFileDialog`, config `config.json` compartido.
- Fase 1 alternativa si Direct2D se atasca: CLI + SDK + mini-UI WebView2 que
  llama al mismo CLI; overlay nativo después. Captura/encode no se bloquean.

## Build Windows

- `Meson + MSVC + vcpkg` (ffmpeg, WinRT/C++/WinRT para WGC).
- `Cargo` solo para `sdk/rust` (linkea `souris_gsr.dll` vía C ABI).
- CI: `windows-11` runner, compile-check + tests unitarios gain/SDK.
