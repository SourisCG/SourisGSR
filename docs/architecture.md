# Architecture — gsr-rs

See `../.specify/constitution.md` for rules and `../reference/MAPA.md` for
the verified C-to-Rust mapping (frozen reference v5.10.2).

## Pipeline

```text
[KMS fd | Portal DMA-BUF] -> EGLImage/CUDA -> composed texture
  -> VAAPI/NVENC/Vulkan/Software -> FFmpeg mux (MP4/MKV)
  + Pulse/PipeWire-app audio -> replay ring (RAM/disk) or file
```

- Linux Wayland-only. No X11 code paths exist in this workspace.
- Windows uses DXGI/WGC into a shared `ID3D11Device` (no `Present` hooking).
- `gsr-kms-server` is the only privileged process (`cap_sys_admin+ep`).
- All user-facing strings resolve through `gsr-i18n` (`en` default, `es` full).
