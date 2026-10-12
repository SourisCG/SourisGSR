//! Ejemplo mínimo: app Rust con SourisGSR embebido como motor.
use souris_gsr_sdk::{Codec, Recorder, Source};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rec = Recorder::builder()
        .source(Source::portal()) // Wayland: picker portal. Win: WGC.
        .fps(60)
        .codec(Codec::Auto)
        .output("video.mp4")
        .audio_inputs(vec!["default_output|vol=80".to_string(), "default_input".to_string()])
        .mic_volume(90)
        .system_volume(80)
        .on_event(|e| println!("[gsr] {e:?}"))
        .start()?;

    std::thread::sleep(Duration::from_secs(5));
    rec.stop()?;
    Ok(())
}
