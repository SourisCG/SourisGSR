# Architecture — SourisGSR

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

## Process model

```text
gpu-screen-recorder (user)
 ├─ captures via helper fd / portal DMA-BUF (zero-copy, GPU-resident)
 ├─ encodes on the same GPU, muxes to file / replay ring
 └─ spawns: gsr-kms-server (KMS only) · save scripts (-sc) · flatpak-spawn (Flatpak)
gsr-kms-server (cap_sys_admin+ep, or root/pkexec):
  open card → reverse-connect to client socket → serve GET_KMS → exit
  with the client (reaped, never orphaned)
```

Control flow is synchronous request/response everywhere (CLI parse →
capture → encode → mux); the only background work is replay-save threads
and detached `-sc` scripts. No async runtime: D-Bus/portal calls block
like the C original.

## Crate responsibilities

Parser (`core::cli`) produces a validated `Config` or exits 1; the replay
ring owns encoded packets (RAM ring or 256 MB disk segments); the mux
timing engine (`gop_size`, `rescale_ts`, CFR pacing) is pure and unit
tested; `encode`/`audio` backends plug in behind traits in T40/T41. The
KMS helper speaks a packed protocol (version 5, see `docs/ADRs/0003-*`)
over `SCM_RIGHTS`; see `docs/security.md` for the threat model.

