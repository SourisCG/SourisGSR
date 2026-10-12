//! ShadowPlay-style replay example: 5 min buffer + hotkey saves a clip.
use souris_gsr_sdk::{Recorder, Source};
use std::path::Path;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rec = Recorder::builder()
        .source(Source::monitor("screen"))
        .fps(60)
        .replay(Duration::from_secs(300))
        .mic_volume(80)
        .system_volume(100)
        .on_event(|e| println!("[replay] {e:?}"))
        .start()?;

    println!("recording replay... press Enter to save a clip");
    let mut s = String::new();
    std::io::stdin().read_line(&mut s)?;
    rec.save_replay(Path::new("clip.mp4"))?;
    rec.stop()?;
    Ok(())
}
