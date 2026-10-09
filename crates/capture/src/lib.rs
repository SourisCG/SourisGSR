// SKELETON (T03): crate-level allow, remove as T10+ implementation lands.
#![allow(dead_code)]
//! Capture crate. Mirrors `src/capture/*`, `src/window/wayland.c`,
//! `src/egl.c`, `src/window_texture.c`, `src/shader.c`,
//! `src/color_conversion.c`, `src/cursor.c`, `src/damage.c`,
//! `src/dbus.c`, `kms/client/*` (v5.10.2). X11-only C files are excluded.

pub mod color;
pub mod compose;
pub mod cursor;
pub mod damage;
pub mod dbus;
pub mod egl;
pub mod kms;
pub mod kms_client;
pub mod portal;
pub mod protocol;
pub mod traits;
pub mod wayland;

#[cfg(target_os = "windows")]
pub mod dxgi;
#[cfg(target_os = "windows")]
pub mod wgc;
