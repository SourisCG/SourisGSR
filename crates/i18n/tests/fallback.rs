//! I18N fallback test: every EN key resolves in ES (no missing keys).

use gsr_i18n::loader::{locales_dir, parse_simple_toml};

#[test]
fn es_covers_en_keys_with_fallback() {
    let dir = locales_dir();
    let en = parse_simple_toml(&std::fs::read_to_string(dir.join("en.toml")).unwrap());
    let es = parse_simple_toml(&std::fs::read_to_string(dir.join("es.toml")).unwrap());
    assert!(!en.is_empty());
    for key in en.keys() {
        assert!(es.contains_key(key), "missing es key: {key}");
    }
}
