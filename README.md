# SourisGSR

SourisGSR is a Rust workspace that clones
[GPU Screen Recorder](https://git.dec05eba.com/gpu-screen-recorder/about/)
(C, GPL-3.0-only) with functional and performance parity (~0% CPU,
zero-copy VRAM to encoder) — not a literal translation.

Binary and crate names keep the upstream `gsr-*` / `gpu-screen-recorder`
spelling so the CLI stays familiar; the project itself is SourisGSR.

- Targets: Linux (Wayland only, no X11 — ever) + Windows 10/11.
- Modes: recording, replay of the last N seconds, monitor and window.
- Containers MP4/MKV; video H264/H265 (+AV1/VP8/VP9 as upstream);
  audio AAC/Opus; multi-track with `|`.
- UI languages: English (default) + Spanish, no hardcoded strings.

## Status

Spec-Driven Development against the frozen C mirror v5.10.2
(`reference/gpu-screen-recorder`, see `reference/MAPA.md`).
Done and green: i18n catalog, CLI parser + validated config, replay ring
(RAM/disk), muxer timing, screenshots, plugin ABI, KMS helper protocol +
transport + DRM grab + client. Next: KMS capturer, Portal, Windows
capture, encoders, audio (see `.specify/tasks.md`).

Recording itself is not wired yet: `Run`/`Info`/`List*` actions report
`not implemented` until their backend phases land.

## Build

```sh
# Linux needs at least: libdrm dev files (kms-server), later phases add
# pipewire/dbus (portal), pulse (audio), ffmpeg (encode). See docs/build.md.
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Coverage gate (`core` + `i18n` >= 80%) via tarpaulin; CI runs the same on
Linux and Windows. Details: `docs/build.md`, `docs/testing.md`.

## Layout

```text
.specify/      specs (constitution, spec-01..06, plan, tasks) — English
crates/core        CLI, config, replay, muxer, screenshots, plugins, binary
crates/capture     KMS/Portal/Wayland/EGL capture (+ helper client; DXGI/WGC on Windows)
crates/encode      nvenc/vaapi/vulkan/software + queries (+ d3d11va/amf on Windows)
crates/audio       pulse/pipewire_app (+ wasapi on Windows)
crates/i18n        Lang/Catalog + locales/en.toml + locales/es.toml
crates/kms-server  privileged helper lib + gsr-kms-server binary
crates/plugin      C ABI types + test-only cdylibs
docs/              guides + ADRs (one per deviation from the C code)
```

## Docs

Start with [`AGENTS.md`](AGENTS.md) (workflow for humans and AI),
[`CONTRIBUTING.md`](CONTRIBUTING.md), then `docs/`.
Every deviation from the C implementation is a `DEVIATION:` with rationale
and test; the big ones have an ADR in `docs/ADRs/`.

## Credits

Screen recording design, protocol, CLI surface and performance approach by
dec05eba and the GPU Screen Recorder contributors:
<https://git.dec05eba.com/gpu-screen-recorder/about/>.
This clone re-implements their work in safe Rust; no C code was copied.
Licensed GPL-3.0-only like the original.
