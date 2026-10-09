//! C ABI types mirroring `plugin/plugin.h` byte-for-byte.
//! Layout is asserted by tests (`#[repr(C)]` + explicit discriminants).

use std::ffi::{c_char, c_uint, c_void};

/// `GSR_PLUGIN_INTERFACE_MAJOR_VERSION`.
pub const INTERFACE_MAJOR: u32 = 0;
/// `GSR_PLUGIN_INTERFACE_MINOR_VERSION`.
pub const INTERFACE_MINOR: u32 = 1;

/// `GSR_PLUGIN_INTERFACE_MAKE_VERSION(major, minor)`.
pub const fn make_version(major: u32, minor: u32) -> u32 {
    (major << 16) | minor
}

/// `GSR_PLUGIN_INTERFACE_VERSION`.
pub const INTERFACE_VERSION: u32 = make_version(INTERFACE_MAJOR, INTERFACE_MINOR);

/// `gsr_plugin_graphics_api`. T14 always offers `EglEs` (no X11 `Glx` path).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GraphicsApi {
    EglEs = 0,
    Glx = 1,
}

/// `gsr_plugin_color_depth`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ColorDepth {
    Bits8 = 0,
    Bits10 = 1,
}

/// `gsr_plugin_draw_params`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct DrawParams {
    pub width: c_uint,
    pub height: c_uint,
}

/// `gsr_plugin_init_params`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct InitParams {
    pub width: c_uint,
    pub height: c_uint,
    pub fps: c_uint,
    pub color_depth: ColorDepth,
    pub graphics_api: GraphicsApi,
}

/// Optional per-frame callback (`draw` may be null, as in C).
pub type DrawFn = Option<unsafe extern "C" fn(*const DrawParams, *mut c_void)>;

/// `gsr_plugin_init_return`. `name` must be non-null, `version` non-zero.
#[repr(C)]
pub struct InitReturn {
    pub name: *const c_char,
    pub version: c_uint,
    pub userdata: *mut c_void,
    pub draw: DrawFn,
}

// SAFETY: plain data carriers across the ABI boundary.
unsafe impl Send for InitReturn {}
unsafe impl Sync for InitReturn {}

/// Exported `gsr_plugin_init`.
pub type InitFn = unsafe extern "C" fn(*const InitParams, *mut InitReturn) -> bool;
/// Exported `gsr_plugin_deinit`.
pub type DeinitFn = unsafe extern "C" fn(*mut c_void);
