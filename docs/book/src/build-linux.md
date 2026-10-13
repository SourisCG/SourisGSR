# Build Linux

## Core CLI

```sh
sudo apt-get install -y ninja-build pkg-config gcc \
  libx11-dev libxcomposite-dev libxrandr-dev libxfixes-dev \
  libxdamage-dev libpulse-dev libva-dev libdrm-dev \
  libcap-dev libwayland-dev wayland-protocols \
  libdbus-1-dev libpipewire-0.3-dev libspa-0.2-dev \
  libavcodec-dev libavformat-dev libavutil-dev \
  libswresample-dev libavfilter-dev \
  libglvnd-dev linux-libc-dev
meson setup /tmp/sgsr-build core -Dcapabilities=false -Dsystemd=false
meson compile -C /tmp/sgsr-build
meson test -C /tmp/sgsr-build
```

This also builds `libsouris_gsr.a/so` for the Rust SDK.

## Overlay UI

```sh
git clone --depth 1 https://repo.dec05eba.com/mglpp overlay/depends/mglpp
meson setup /tmp/build-overlay overlay
meson compile -C /tmp/build-overlay
```

## SDK archive helper

```sh
./tools/archive_libsouris.sh /tmp/sgsr-build /tmp/sgsr-native
SOURIS_GSR_LIB_DIR=/tmp/sgsr-native cargo test --features ffi -p souris-gsr-sdk --manifest-path sdk/rust/Cargo.toml
```
