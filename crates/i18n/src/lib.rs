// SKELETON (T03): crate-level allow, remove as T10+ implementation lands.
#![allow(dead_code)]
//! I18N crate. `--lang en|es|auto` + `LANG`. No hardcoded strings elsewhere.

pub mod loader;

/// Supported locales.
pub const LOCALES: [&str; 2] = ["en", "es"];
