# Support — SourisGSR

## Officially supported platforms
- **Linux Wayland** (GNOME, KDE Plasma, Hyprland) — portal + PipeWire + KMS/DRM.
- **Windows 11 22H2+** — WGC + NVENC/AMF/QuickSync + WASAPI.

## Best-effort without SLA (may end up WONTFIX)
- **X11** (code frozen, PRs only).
- **Windows 10** (WGC exists but is not tested; NVFBC is deprecated by NVIDIA).

If you report an X11/Win10 bug, state the OS, compositor/driver and whether it
reproduces on Wayland/Win11. Without that info it will be closed as
`needs-info`, and even if confirmed it may be tagged `wontfix-x11-win10`
given project priorities.

## Audio/volume
The 0-100% modifier is **recording only** (post-capture gain). If your report
is "the system doesn't sound louder", that is expected behavior: the system
mixer is never touched by design.

## Where the code comes from
Fork of `dec05eba/gpu-screen-recorder` + `-ui` (GPL-3.0-only). Bugs that
reproduce on pure upstream should also be reported upstream when possible.
