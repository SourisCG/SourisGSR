//! Plugin crate: C ABI types plus a native Rust plugin trait.
//! Mirrors `plugin/plugin.h` (interface version 0.1) and the load protocol
//! of `src/plugins.c` (frozen reference v5.10.2). GL texture/color-conversion
//! setup around the plugins is capture-phase work (T21+); here we cover
//! ABI layout, loading, validation, draw dispatch and unload order.

pub mod abi;

pub use abi::{
    make_version, ColorDepth, DeinitFn, DrawFn, DrawParams, GraphicsApi, InitFn, InitParams,
    InitReturn, INTERFACE_MAJOR, INTERFACE_MINOR, INTERFACE_VERSION,
};

/// Maximum simultaneously loaded plugins (C `GSR_MAX_PLUGINS`).
pub const MAX_PLUGINS: usize = 128;

/// Native Rust plugin trait (in-process examples such as `triangle`).
/// File-based `-p` overlays use the C ABI above, like the original.
pub trait Plugin {
    /// Plugin name.
    fn name(&self) -> &str;
}
