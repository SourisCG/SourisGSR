// SKELETON (T03): crate-level allow, remove as T10+ implementation lands.
#![allow(dead_code)]
//! Core crate: CLI, config, replay ring, muxer, screenshots.
//! Mirrors `src/main.cpp`, `src/args_parser.c`, `src/defs.c`,
//! `src/replay_buffer/*.c`, `src/encoder/encoder.c`, `src/image_writer.c`,
//! `src/library_loader.c`, `src/utils.c` (frozen reference v5.10.2).

pub mod cli;
pub mod config;
pub mod library;
pub mod muxer;
pub mod plugins;
pub mod replay;
pub mod replay_disk;
pub mod replay_ram;
pub mod screenshot;
pub mod util;
