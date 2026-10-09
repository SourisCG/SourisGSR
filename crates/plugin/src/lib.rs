// SKELETON (T03): crate-level allow, remove as T10+ implementation lands.
#![allow(dead_code)]
//! Plugin ABI crate. Mirrors `plugin/plugin.h` + `src/plugins.c` (v5.10.2).

/// Overlay plugin trait (mirrors the C plugin interface).
pub trait Plugin {
    /// Plugin name.
    fn name(&self) -> &str;
}
