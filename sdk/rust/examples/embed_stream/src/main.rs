//! Streaming example: RTMP/SRT via the real engine (ffi).
//! Requires libsouris_gsr linked (SOURIS_GSR_LIB_DIR). Run headless to see
//! clean NoDevice error mapping; on Wayland+NVIDIA it streams for 10s.
use souris_gsr_sdk::{Recorder, Source};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "rtmp://localhost/live/test".to_string());
    println!("[stream] url={url}");
    let eng = Recorder::builder()
        .source(Source::portal())
        .fps(60)
        .output_url(url)
        .audio_inputs(vec!["default_output".to_string()])
        .mic_volume(90)
        .system_volume(80)
        .on_event(|e| println!("[stream] {e:?}"))
        .start_engine()?;
    std::thread::sleep(std::time::Duration::from_secs(10));
    eng.stop()?;
    Ok(())
}
