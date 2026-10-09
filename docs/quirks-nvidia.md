# NVIDIA quirks — gsr-rs

Mirrors `extra/gsr-nvidia.conf` and the `-oc`/`-tune` behavior of v5.10.2.

- Install `extra/udev/gsr-nvidia.conf` so the driver keeps
  `NVreg_PreserveVideoMemoryAllocations=1` across suspend (CUDA would
  otherwise break until driver reload).
- CUDA/NVENC use lowers the memory-transfer performance level on some
  drivers (P2 state). v5.10.2 works around it with `-oc` via `libXNVCtrl`
  on X11 only. On Wayland/Windows there is no overclock path: prefer the
  Vulkan encoder (`vulkan.rs`) or accept the P2 downclock.
- Runtime loader (`library.rs` + `cuda.rs`) `dlopen`s CUDA/NVENC and
  negotiates older (Kepler-era) GPUs instead of hard-failing.
- Flatpak baseline per requirements: FFmpeg n6.1.1 + ffnvcodec n11.1.5.3.
