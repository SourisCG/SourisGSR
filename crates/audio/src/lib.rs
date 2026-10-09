// SKELETON (T03): crate-level allow, remove as T10+ implementation lands.
#![allow(dead_code)]
//! Audio crate. Mirrors `src/sound.cpp` + `src/pipewire_audio.c` (v5.10.2).

pub mod pipewire_app;
pub mod pulse;
pub mod traits;

#[cfg(target_os = "windows")]
pub mod wasapi;
