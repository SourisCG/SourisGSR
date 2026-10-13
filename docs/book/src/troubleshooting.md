# Troubleshooting

- **No system volume change?** Expected: volumes are recording-only by design.
- **`app:` unsupported?** Need PipeWire (`pulseaudio_server_is_pipewire`). Pure Pulse → `UNSUPPORTED`.
- **`vol= without prior input?** Put `vol=` after an input: `"default_output|vol=80"`.
- **Portal lists monitors only?** `xdg-desktop-portal-wlr` is monitors-only (backend limit, not ours). Use GNOME/KDE/Hyprland for windows.
- **Screenshot crash fixed:** `sgsr_screenshot` now passes a live `running` atomic (was NULL).
- **Headless CI:** enumerate falls back to screen+portal; engine start returns `NoDevice` cleanly.
- **NVIDIA suspend:** install `gsr-nvidia.conf` + reboot (see upstream README).
