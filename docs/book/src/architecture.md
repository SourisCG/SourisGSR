# Architecture

```text
core/  CLI + AV pipeline (FFmpeg) + gsr_capture vtable
  portal.c / pipewire_video.c   Wayland picker + zero-copy
  pipewire_audio.c              per-app combined sink
  audio_capture.c               gain (souris_gain.h) + VU peak + amix
  recorder.c                    set_volumes (atomic) + get_audio_level
  cli/ipc.c + sgsr-cli           stop/pause/save-replay/set-volumes
  souris_gsr.c                  C ABI engine (worker thread + level thread)
include/souris_gsr.h            stable ABI v2 (sources x/y, portal token)
overlay/ ShadowPlay UI (drives CLI today, SDK tomorrow)
sdk/rust  souris-gsr-sdk (mock offline + Engine ffi real)
```

- `souris_gain`: `percent_to_gain`, `apply_u8/s16/s32/s64/flt/dbl(+p)`, `resolve`, `is_mic`.
- `sgsr_start` mirrors `cli/main.c run()/record()` (same parsers, windowing+EGL).
- `sgsr_enumerate_video_sources` tries real DRM/X11 monitors, falls back to screen+portal headless.
- `REGION` via `WxH+X+Y`, `FOCUSED` needs `width/height`, portal restore via token path.
- VU: audio thread peak → `audio_capture.level` → `recorder_get_audio_level` → `sgsr_level_thread` 10Hz `LEVEL_METER`.
