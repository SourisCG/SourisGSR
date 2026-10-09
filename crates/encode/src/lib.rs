// SKELETON (T03): crate-level allow, remove as T10+ implementation lands.
#![allow(dead_code)]
//! Encode crate. Mirrors `src/encoder/video/*`, `src/codec_query/*`,
//! `src/cuda.c` (v5.10.2). `unsafe` FFmpeg interop stays in this crate.

pub mod cuda;
pub mod nvenc;
pub mod query_nvenc;
pub mod query_vaapi;
pub mod query_vulkan;
pub mod software;
pub mod traits;
pub mod vaapi;
pub mod video;
pub mod vulkan;

#[cfg(target_os = "windows")]
pub mod amf;
#[cfg(target_os = "windows")]
pub mod d3d11va;
