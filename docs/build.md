# Build guide

## Toolchain

Stable Rust (see `rustc --version` in CI logs), plus `clippy` and `rustfmt`
components. No pinned toolchain file yet; MSRV is whatever CI's stable is.

## System dependencies

### Linux

| Phase | Package (Debian/Ubuntu) | Package (Fedora) | Used by |
|-------|------------------------|------------------|---------|
| T20 (done) | `libdrm-dev` | `libdrm-devel` | `drm`, `drm-fourcc` in `kms-server` |
| T22 | `libpipewire-0.3-dev`, `libdbus-1-dev`, `libspa-0.2-dev` | `pipewire-devel`, `dbus-devel` | portal capture + app audio |
| T40 | `libavcodec-dev libavformat-dev libavutil-dev libswresample-dev libavfilter-dev`, `libva-dev`, headers for ffnvcodec | `ffmpeg-devel`, `libva-devel` | encoders (FFmpeg n6.1.1 + ffnvcodec n11.1.5.3 baseline, see quirks) |
| T41 | `libpulse-dev` | `pulseaudio-libs-devel` | audio devices |

Rust-only crates (`image`, `libloading`, `zbus` pure parts) need nothing.

Windows: no system deps yet; DXGI/WGC come with the OS SDK via crates.
The KMS helper, `drm`, portal and audio-Pulse modules are `#[cfg(unix)]`
gated — the workspace must compile on Windows without them.

### CI (`.github/workflows/ci.yml`)

CI installs the system packages for the current phase before
`cargo test --workspace --all-features`. If you add a phase needing a new
system library, extend the workflow's install step in the same commit.

## Feature policy

No cargo features yet; platform selection is purely `#[cfg]`. Linux-only
binaries (like `gsr-kms-server`) compile to a stub `main` elsewhere that
exits 1 with a message — never `compile_error!`, so `--workspace` stays
green on every target.
