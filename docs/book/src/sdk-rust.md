# SDK Rust

Crate `souris-gsr-sdk` (GPL-3.0-only, like core). Apps linking it must be GPL-3.0.

## Mock (offline, no link)

```rust
use souris_gsr_sdk::{Recorder, Source};
let rec = Recorder::builder()
    .source(Source::portal())
    .fps(60).output("video.mp4")
    .audio_inputs(vec!["default_output|vol=80".into()])
    .mic_volume(90).system_volume(80)
    .on_event(|e| println!("{e:?}"))
    .start()?;
rec.stop()?;
```

`cargo test -p souris-gsr-sdk` covers mock logic.

## Engine (real, ffi)

```sh
./tools/archive_libsouris.sh /tmp/sgsr-build /tmp/sgsr-native
SOURIS_GSR_LIB_DIR=/tmp/sgsr-native cargo test --features ffi -p souris-gsr-sdk
```

```rust
let eng = Recorder::builder()
    .source(Source::monitor("screen"))
    .output_url("rtmp://live/test") // streaming auto-detect
    .replay(std::time::Duration::from_secs(300))
    .start_engine()?;
eng.set_volumes(80, 90, 100)?;
eng.screenshot(std::path::Path::new("shot.jpg"))?;
eng.save_replay(std::path::Path::new("clip.mp4"))?;
eng.stop()?;
```

- `Source::{monitor,window,window_hwnd,portal,region,focused}` → `RawSource` v2.
- `list_video_sources()/list_audio_devices()` query C ABI with ffi, else stub.
- Events via trampoline (`Started/Stopped/Paused/Resumed/ReplaySaved/LevelMeter/Error`).
- Examples: `embed_basic`, `embed_replay`, `embed_stream`, `embed_volumes`, `embed_list`.
