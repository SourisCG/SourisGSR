//! I18N crate. `--lang en|es|auto` + `LANG`. No hardcoded strings elsewhere.
//!
//! Locales are embedded with `include_str!` so the binary is self-contained.
//! `crates/i18n/locales/*.toml` remain the source of truth; a test guards
//! that the embedded copy matches the files on disk.

pub mod loader;

pub use loader::{fill, locales_dir, parse_simple_toml, resolve_lang, Catalog, Lang};

/// Supported locales.
pub const LOCALES: [&str; 2] = ["en", "es"];
