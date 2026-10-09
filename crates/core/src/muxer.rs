//! FFmpeg muxer. Mirrors the muxing half of `src/encoder/encoder.c` (v5.10.2).
//! `unsafe` FFmpeg interop stays inside this module (see constitution).

pub struct Muxer;
