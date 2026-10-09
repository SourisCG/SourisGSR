//! Locale loading, language resolution and string catalog.
//!
//! Mirrors the user-facing strings of `src/args_parser.c` and `src/main.cpp`
//! (frozen reference v5.10.2). New surface (`--lang`) is `DEVIATION-ADD`.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const EN_TOML: &str = include_str!("../locales/en.toml");
const ES_TOML: &str = include_str!("../locales/es.toml");

/// Directory containing `en.toml` and `es.toml` (source of truth on disk).
pub fn locales_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("locales")
}

/// Parse a minimal `key = "value"` TOML subset (single-line values only).
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

/// Supported UI language. Default is English.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
}

impl Lang {
    /// Two-letter code.
    pub fn as_code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
        }
    }

    /// Parse `--lang` value (`en`, `es`, `auto` handled by caller).
    /// Returns `None` for unknown codes (caller falls back to English).
    pub fn from_code(s: &str) -> Option<Lang> {
        match s.trim().to_ascii_lowercase().as_str() {
            "en" => Some(Lang::En),
            "es" => Some(Lang::Es),
            _ => None,
        }
    }

    /// Parse a `LANG`-style locale (`es_ES.UTF-8`, `es-ES`, `C`, ...).
    /// Spanish when the primary subtag is `es` (case-insensitive), else English.
    pub fn from_locale_env(s: &str) -> Lang {
        let primary: String = s
            .split(['.', '@'])
            .next()
            .unwrap_or("")
            .split(['_', '-'])
            .next()
            .unwrap_or("")
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .collect();
        if primary.eq_ignore_ascii_case("es") {
            Lang::Es
        } else {
            Lang::En
        }
    }
}

/// Resolve the UI language from argv + `LANG` env value.
///
/// Pre-scans full argv (including program name at index 0) for
/// `--lang <code>` pairs in the C `key value` style; the last occurrence
/// wins. `auto` (or no flag) defers to `lang_env`. Unknown codes and a
/// trailing `--lang` without value fall back to English; the full parser
/// (T10) reports the underlying usage error later.
pub fn resolve_lang(argv: &[String], lang_env: Option<&str>) -> Lang {
    let mut flag: Option<String> = None;
    let mut iter = argv.iter().skip(1).peekable();
    while let Some(arg) = iter.next() {
        if arg == "--lang" {
            if let Some(value) = iter.next() {
                flag = Some(value.clone());
            }
        }
    }
    match flag.as_deref() {
        Some("auto") | None => match lang_env {
            Some(env) => Lang::from_locale_env(env),
            None => Lang::En,
        },
        Some(code) => Lang::from_code(code).unwrap_or(Lang::En),
    }
}

fn fallback_warned() -> &'static Mutex<HashSet<String>> {
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    WARNED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// String catalog for one language with English fallback.
pub struct Catalog {
    lang: Lang,
    en: HashMap<String, String>,
    current: HashMap<String, String>,
}

impl Catalog {
    /// Build a catalog for `lang` from the embedded locales.
    pub fn new(lang: Lang) -> Self {
        let en = parse_simple_toml(EN_TOML);
        let current = match lang {
            Lang::En => en.clone(),
            Lang::Es => parse_simple_toml(ES_TOML),
        };
        Catalog { lang, en, current }
    }

    /// Active language.
    pub fn lang(&self) -> Lang {
        self.lang
    }

    /// Look up `key`: current language, then English, then the key itself
    /// (programmer error, never localizable). English fallback warns once
    /// on stderr so missing translations are noticed without failing.
    /// Returns an owned string; CLI lookups are not a hot path.
    pub fn get(&self, key: &str) -> String {
        if let Some(value) = self.current.get(key) {
            return value.clone();
        }
        if let Some(value) = self.en.get(key) {
            let mut warned = fallback_warned().lock().expect("fallback set");
            let marker = format!("{}:{key}", self.lang.as_code());
            if warned.insert(marker) {
                eprintln!(
                    "i18n: missing {} key '{key}', using English",
                    self.lang.as_code()
                );
            }
            return value.clone();
        }
        key.to_string()
    }
}

/// Fill `{name}` placeholders in a template. Unknown placeholders stay as-is.
pub fn fill(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (name, value) in pairs {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}
