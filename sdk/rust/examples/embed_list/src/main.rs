//! Discovery example: video sources + audio devices via C ABI (ffi).
use souris_gsr_sdk::{list_audio_devices, list_video_sources};

fn main() {
    println!("video sources:");
    for s in list_video_sources() {
        println!("  {s:?}");
    }
    println!("audio devices:");
    for (name, desc) in list_audio_devices() {
        println!("  {name} | {desc}");
    }
}
