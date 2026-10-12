# Windows port (SourisGSR) — concrete plan, no core rewrite

Goal: same CLI/core on Windows 11, with individual window capture.

## Capture

- Default `WGC` (`Windows.Graphics.Capture`):
  - Window: `GraphicsCapturePicker` / `GraphicsCaptureItem::TryCreateFromWindowHandle(HWND)`.
  - Monitor: `TryCreateFromDisplayId`.
  - D3D11 frames (`ID3D11Texture2D`) via `Direct3D11CaptureFramePool`.
  - Known limitation: the cursor switches to software-rendered inside a WGC session.
- `DXGI` fallback (`IDXGIOutputDuplication`):
  - Monitors only. Must run on the same GPU as the display
    (unlike WGC, which is cross-GPU).
  - Use when WGC is unavailable or for HW-cursor diagnostics.
- Integration into the existing `gsr_capture` vtable (`capture/capture.h`):
  `wgc.c` implements `start/capture/destroy`, converting D3D11 -> EGL/CUDA
  or feeding FFmpeg `d3d11va` directly (zero CPU copies).

## Encode

FFmpeg with D3D11 `hwupload`. Probe order:
1. `h264_nvenc/hevc_nvenc/av1_nvenc` (NVIDIA)
2. `h264_amf/hevc_amf` (AMD)
3. `h264_qsv/hevc_qsv/av1_qsv` (Intel, context derived from the same D3D11 device)
4. `h264_mf` (MediaFoundation, SDR/NV12 fallback, no HDR)

Never switch encoders silently: if the requested one is missing, fail with a
clear log (like `capture-engine` does).

## Audio (WASAPI)

- System: `IAudioClient` loopback. Mic: `eCapture`. Per-app: process-loopback
  (Win10 20348+, fine on Win11).
- Same 0-100% gain as Linux: `souris_gain.h` post-capture. `IAudioEndpointVolume`
  is forbidden (never touch the system volume).

## Overlay (reuse, not from scratch)

`overlay/` is already ShadowPlay-style (hotkeys, replay, selection). Only the
backend is abstracted:
- `OverlayLinux (EGL/X11/Wayland/Pango, existing, frozen)` vs
  `OverlayWin (Win32 layered + D3D11 + DirectWrite/Direct2D, new, same
  DrawText/DrawRect/Present interface)`.
- `GlobalHotkeysLinux (evdev/X11)` vs `GlobalHotkeysWin (RegisterHotKey)`.
- `FileChooser` -> `IFileDialog`, shared `config.json`.
- Phase-1 fallback if Direct2D stalls: CLI + SDK + a minimal WebView2 UI calling
  the same CLI; native overlay later. Capture/encode are never blocked by this.

## Windows build

- `Meson + MSVC + vcpkg` (ffmpeg, WinRT/C++/WinRT for WGC).
- `Cargo` only for `sdk/rust` (links `souris_gsr.dll` via the C ABI).
- CI: `windows-11` runner, compile-check + gain/SDK unit tests.
