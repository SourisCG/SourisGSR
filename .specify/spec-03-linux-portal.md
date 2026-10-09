# Spec-03 — Linux Portal capture + audio

Refs (frozen reference v5.10.2):
`src/capture/portal.c`, `include/capture/portal.h`,
`src/dbus.c`, `include/dbus.h`,
`src/pipewire_video.c`, `include/pipewire_video.h`,
`src/pipewire_audio.c`, `include/pipewire_audio.h`,
`src/sound.cpp`, `include/sound.hpp`,
`meson.build` (`-DGSR_PORTAL`, `-DGSR_APP_AUDIO`), `meson_options.txt`.

## 1. Portal video (MUST)

`-w portal` on Wayland: xdg-desktop-portal ScreenCast session over D-Bus
(`dbus.rs`, mirrors `src/dbus.c`; session token honored via
`-restore-portal-session` + `-portal-session-token-filepath`, default
`~/.config/gpu-screen-recorder/restore_token`), PipeWire stream
(`portal.rs` +jack on `src/pipewire_video.c`): DMA-BUF buffers,
`SPA_META_VideoCrop` region, video-damage metadata, cursor
`cursor_region`, rotation swap. `-fm content` syncs encode to content
updates (portal + KMS only). No root required.

## 2. Audio (MUST)

- Pulse (`pulse.rs`, mirrors `src/sound.cpp`): `default_output`,
  `default_input`, `device:<name>`, repeatable `-a`, `|` multitrack,
  `--list-audio-devices` source of truth.
- PipeWire app-audio (`pipewire_app.rs`, mirrors `src/pipewire_audio.c`,
  behind the `app_audio` meson option): `app:<name>` (case-insensitive),
  `app-inverse:<name>`, `--list-application-audio` source of truth.
- Codec/bitrate: `-ac opus|aac|flac` (flac warns + opus fallback),
  `-ab` kbps (`0` = automatic, x1000 internally).
- Modules: `crates/audio/src/traits.rs`, `pulse.rs`, `pipewire_app.rs`.

## 3. Tests (MUST)

- Portal mock: fake PipeWire node emitting crop+damage+rotation frames,
  assert region/cursor math, no GPU required.
- Audio mocks: `default_output|app:firefox` multitrack graph builds;
  unknown app name still accepted (as in C).
- `--list-*` snapshot tests against C output format (one item per line).
