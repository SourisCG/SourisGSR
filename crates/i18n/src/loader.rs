//! Locale loader. Reads `crates/i18n/locales/{en,es}.toml` with identical keys.

use std::collections::HashMap;
use std::path::PathBuf;

/// Directory containing `en.toml` and `es.toml`.
pub fn locales_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("locales")
}

/// Parse a minimal `key = "value"` TOML subset (no external deps in skeleton).
pub fn parse_simple_toml(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim().trim_matches('"').to_string();
            let value = v.trim().trim_matches('"').to_string();
            if !key.is_empty() {
                map.insert(key, value);
            }
        }
    }
    map
}
