# Introduction

SourisGSR is a multiplatform fork of GPU Screen Recorder (`dec05eba`, GPL-3.0-only)
for **Linux Wayland + Windows 11**, with maximum reuse of the C core and an
embeddable Rust SDK.

Officially supported: Wayland (GNOME, KDE Plasma, Hyprland) + Windows 11 22H2+.
X11 and Windows 10 are best-effort and may be closed as `WONTFIX`. See Support.

New in SourisGSR:

- OBS-style 0-100% recording-only volumes (never the system mixer).
- Individual window/app capture on Wayland portal + Windows WGC.
- Stable C ABI (`include/souris_gsr.h`) + Rust `souris-gsr-sdk` crate.
- ShadowPlay-style overlay UI with the same options on both platforms.
