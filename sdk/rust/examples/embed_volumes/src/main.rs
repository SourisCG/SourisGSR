//! Hot-volume example: start with volumes, then live-update (recording only).
use souris_gsr_sdk::{Recorder, Source};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eng = Recorder::builder()
        .source(Source::monitor("screen"))
        .fps(60)
        .output("/tmp/sgsr-volumes.mp4")
        .audio_inputs(vec![
            "default_output|vol=80".to_string(),
            "default_input".to_string(),
        ])
        .mic_volume(90)
        .system_volume(80)
        .app_volume(100)
        .on_event(|e| println!("[vol] {e:?}"))
        .start_engine()?;
    for (mic, sys, app) in [(90, 80, 100), (50, 100, 70), (100, 100, 100)] {
        println!("[vol] set {mic}/{sys}/{app}");
        eng.set_volumes(mic, sys, app)?;
        std::thread::sleep(Duration::from_secs(2));
    }
    eng.stop()?;
    Ok(())
}
