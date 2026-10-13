# Build Windows

> Windows work happens on Windows. This page tracks the plan so Linux work stays compatible.

- Toolchain: Meson + MSVC + vcpkg (`vcpkg.json`: ffmpeg + cppwinrt).
- Configure: `meson setup C:/sgsr-build --vsenv -Dportal=false -Dapp_audio=false ...`
- Backends: WGC default (window+monitor) + DXGI monitor fallback + WASAPI loopback/mic/per-app.
- Encode order: NVENC > AMF > QSV > MediaFoundation. Never switch silently.
- See `core/docs/WINDOWS_PORT.md` for the detailed port plan.
