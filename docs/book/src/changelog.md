# Changelog

## Unreleased (overlay per-track-only + slider + tray)

- Overlay Audio is per-input-only: global Mic/App/Screen controls removed from
  the UI and no longer emitted (engine keeps them as deprecated fallbacks).
- New real `Slider` widget (drag, arrows, wheel, double-click reset) on every
  audio input row; combo text truncated so the remove button always fits.
- System tray integrated in `sgsr-ui` only (`org.kde.StatusNotifierItem`,
  toggle on click, `show_tray_icon` setting); no right-click menu in v1
  (libdbus aborts on nested dbusmenu layouts); SDK stays headless.
- Tests: `test-volparse` meson suite, `tools/test_volumes_e2e.sh` (proves -6dB
  per halving on real recordings), `tools/test_tray_sni.sh` (bus checks).
- Docs: per-track-only usage, tray section, `sgsr-ui.1` additions.

## 0.3.0-souris (ABI v2)

- `libsouris_gsr.a/so` + `meson test gain` + `tools/archive_libsouris.sh` + CI ffi link.
- Gain formats `u8/s64/dbl(+p)`, word-start mic heuristic, VU peak + `LEVEL_METER`.
- App sink honors single custom `vol=`, warns on mixed.
- IPC `set-volumes` + `sgsr-cli set-volumes`.
- C ABI v2: `REGION x/y`, `FOCUSED`, portal token/restore, real monitor enumerate.
- SDK `Engine::start_engine`, streaming URLs, portal token, `embed_stream/volumes/list`.
- Overlay per-input `vol` rows, no silent app-audio drop.
- Docs: man examples, `docs/book` mdBook, fixed volume FAQ.
- Fix: `sgsr_screenshot` NULL `running` segfault.
