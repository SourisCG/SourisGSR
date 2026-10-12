# Soporte — SourisGSR

## Plataformas oficiales
- **Linux Wayland** (GNOME, KDE Plasma, Hyprland) — portal + PipeWire + KMS/DRM.
- **Windows 11 22H2+** — WGC + NVENC/AMF/QuickSync + WASAPI.

## Best-effort sin SLA (puede quedar WONTFIX)
- **X11** (código congelado, solo se aceptan PRs).
- **Windows 10** (WGC existe pero no se testea; NVFBC está deprecado por NVIDIA).

Si reportas un bug de X11/Win10, indica OS, compositor/driver y si reproduce
en Wayland/Win11. Sin esa info se cerrará como `needs-info`, y aunque se
confirme puede marcarse `wontfix-x11-win10` por prioridad del proyecto.

## Audio/volumen
El modificador 0-100% es **solo grabación** (gain post-captura). Si tu reporte
es "no se escucha más alto el sistema", es comportamiento esperado: el sistema
no se toca por diseño.

## De dónde viene el código
Fork de `dec05eba/gpu-screen-recorder` + `-ui` (GPL-3.0-only). Los bugs que
reproduzcan en upstream puro deben reportarse también arriba si es posible.
